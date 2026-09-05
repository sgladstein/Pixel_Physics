# Why the creatures are not interesting — measured, 2026-09-05

*Owner's question: "I am not seeing interesting behavior from our creatures
but I am uncertain if we need to improve the engine/mechanics (these
behaviors are not currently possible) or they are possible, but I haven't
evolved the right creatures... or I have evolved the right creatures but it
is one out of 100 and the other 99 are just digging a giant hole in the
middle of the screen."*

Three hypotheses. This is a measurement of which one is true. **It is a
diagnosis, not a change: nothing is tuned here.** That follows the owner's
standing direction of 2026-08-30 — *"stop balancing, start exposing... a
default that looks wrong is something to register and report, never to
tune."*

---

## The answer in one line

**It is the second hypothesis, in a form none of the three named: the world
scores exactly one thing, so evolution has exactly one axis to climb.**
Across 24 random genomes, survival correlates **+0.895 with how much an ant
eats** and with nothing else — `travelled` +0.151, `commute` +0.236. There is
one income channel in the whole engine and every other verb is free or a
cost, so any behaviour more elaborate than *get to food, spend less* is a
debit selection removes. More generations climb the same hill.

The giant hole is a separate finding and a simpler one: **it is authored, it
is free, and no number of generations can remove it.**

---

## 1. The behaviour space is real — hypothesis (a) is false

`examples/creature_space.rs`, 24 random genomes + two controls, 12,000
frames, scarce food (2 trees), 52 ants, 9 beetles, generated wetland terrain.
The horizon matters and is checked: this scene's `START_ENERGY` is 90 against
`idle_cost_per_cell` 0.05 on ~2 cells, so the founding grant is ~5,400
frames. At 12,000 the bed has teeth. (Gate 2's own finding, and the reason
this run is not at `labbatch`'s 9,000-frame default.)

| genome | survival |
|---|---|
| **authored ant** | **2.107** |
| best random (`r003`) | 1.121 |
| zeroed brain (control) | 0.420 |
| median random | 0.397 |

Behaviour-space coverage: **7 of 16 cells occupied** (2 bins on each of
travelled, commute, feeding, depth). The space is not flat, the authored ant
sits at a genuine high point, and it beats the best of 24 random draws by
1.88x. **Selection has something to act on, and the world discriminates.**

The negative control behaves: the zeroed brain scores 0.420 against the
authored 2.107, so the harness is not blind.

## 2. …but the landscape has one dimension — hypothesis (b), sharpened

Correlation of survival against each descriptor, over the 24 random genomes:

| descriptor | r |
|---|---|
| **feeding** | **+0.895** |
| depth | −0.469 (confounded — the deep genomes are the ones that also fed) |
| commute | +0.236 |
| travelled | +0.151 |

Movement without feeding is punished, not rewarded: `r016` travelled 218
cells, ate nothing, and scored **0.137 — the worst in the sample**, below the
brain that does nothing at all.

**And 14 of 24 random genomes (58%) never moved and never ate**, clustering
at survival 0.372–0.409 — *below* the zeroed brain's 0.420. The reachable
space is wide but most of it is inert, and the part that pays is one ridge.

### Why it has one dimension, in the source

**There is exactly one income channel.** `creature.rs`'s only credit to an
animal's bank is `diet_yield` on a food cell in the head's 8-neighbourhood
(`adjacent_food`, ~line 2711). There is no other way to gain energy.

**Expenditure is `spent = idle + synapse_tax + sight_tax`** (`creature.rs:1967`)
plus `move_cost_per_cell` per cell travelled. That list is the whole economy.

So of the twelve brain outputs, the ones outside the economy entirely are:

| output | costs | pays |
|---|---|---|
| `Dig` | **nothing** | nothing |
| `EmitA` / `EmitB` | **nothing** | nothing |
| `Drop` / `DropSpoil` | **nothing** | nothing |
| `Move` | per cell | only via food reached |
| `Feed` | — | **the only credit** |

Trail-laying, excavation, nest architecture, caching, division of labour:
each is either free (so selection is blind to it) or a movement cost with no
return (so selection removes it). **A behaviour that cannot be paid for
cannot be evolved.** This is `CLAUDE.md`'s channel-with-no-reader failure
wearing an economic costume.

### The diet axis has two points, not a range

`gut_bias` is heritable and continuous over `-1..=1`, and
`diet_quality` is `(1 - |gut - class|/2)^2`. But **every food material in the
world is authored at `food_class: -1.0` or `+1.0`** — plant matter (leaf,
deadleaf, flower, fruit, litter, moss, seed, windfall) or flesh (ant, beetle,
corpse, chitin, worm, ancestor). Nothing sits between.

So the gut gene is a **binary switch**, not a niche axis: a generalist at 0.0
absorbs 0.25 of every mouthful where a specialist absorbs 1.0, and is
strictly worse at both. Two ways to make a living, one of which needs corpses
to already exist.

## 3. Generations are slow, and the demography is geometric — hypothesis (c)

`examples/labstats.rs`, 60,000 frames, 32 plant founders, one colony of 52
ants, seed 1. The population **stabilises** — it does not boom or crash:

```
frame 59,580:  plants 90   animals 54   gen p7 a7   lines 33, biggest 20%
ANIMALS BORN 157   DIED 156
REFUSED  NO ROOM 1171   NO SLOT 0
ANT HUNGRY 4 OF 54
```

Three things in that block:

- **Animal generation 7 at 60,000 frames.** ~8,600 frames per generation, so
  the owner's 60–70 generations is ~500,000–600,000 frames of *this* regime.
  The count is real; it is just expensive.
- **1,171 births were denied for want of a free cell beside the parent,
  against 157 actually born.** 88% of affordable births fail on geometry.
  Placement is deterministic — `creature.rs`'s own test says *"who gets born
  is decided by energy, and nothing else... a bank against a threshold, then
  `DIRS` order for placement"* — so the filter that actually decides who
  reproduces is **where you happen to be standing**, which is only weakly
  heritable.
- **Only 4 of 54 ants are hungry.** This population is *space*-limited, not
  food-limited. The one axis the world scores is the axis that is not
  currently binding.

### And in the bed the owner actually plays, generation reaches 1

The coordinator note already warns that the 8-founder default is not the bed
being played. Re-run at 64 founders and 3 colonies (156 ants), seed 1:

| frame | animals | born | died | animal gen |
|---|---|---|---|---|
| 0 | 156 | 0 | 0 | 0 |
| 5,400 | **83** | 0 | 73 | 0 |
| 10,740 | 65 | 5 | 96 | 1 |
| 16,140 | 57 | 16 | 115 | 1 |
| 21,420 | 42 | 21 | 135 | 1 |
| 26,940 | **42** | 27 | 142 | **1** |

Half the colony dies before a single ant is born. Deaths outrun births 5:1,
and **27,000 frames buys one generation.** This is the owner's outcome (2)
and (3), and in it *no evolution happens at all* — there is nothing wrong
with the search, there is barely a search.

The difference between the two beds is the founding ant:plant ratio. Too many
ants and the stand never establishes; the right number and the colony sits at
replacement rate, space-capped, well fed.

## 4. The giant hole is authored, free, and cannot be selected away

`assets/species/ant.ron` carries, as an authored instinct:

```
(Bias, Dig, 0.4),
```

a **standing, unconditional drive to dig**, added deliberately — its own
comment records that with `FoodAdjacent` as the only route to the output, an
ant facing a bank of soil computed a dig urge of exactly zero, *"0 digs
against 19,114 moves and 27,928 blocked ticks"*. *"Ants dig. It does not need
a reason in the world; it needs a reason in the animal."*

That reasoning is sound and the consequence is the hole:

1. **Digging deducts no energy.** The dig block (`creature.rs:3436–3523`)
   empties the cell, stores the spoil, increments `digs`, calls
   `line_burrow` — and charges nothing. It is absent from `spent`.
2. **It only fires when nothing edible is in reach**, so it does not even
   cost a feeding opportunity.
3. Therefore the weight is **selectively neutral**. Drift moves it; selection
   cannot.

**No mutation rate and no generation count removes the hole**, because
nothing in the world can tell an ant that digs from one that does not. The
hole is not evolved behaviour — it is generation zero's instinct, running
forever, in a system with no way to price it.

## 5. Beetles cannot breed, and the owner's guess is exactly right

> *"beetles likely need to be able to breed to find an equilibrium with the
> ants if it is possible"*

`assets/species/beetle.ron` **has no `reproduce_threshold` field at all**. It
is `#[serde(default)]` on `CreatureDef` (`organism.rs:3085`), so it is `0.0`,
and `creature::reproduce_at_of` is gated `(def.reproduce_threshold > 0.0)` —
it returns `None`. **A beetle can never bud, under any energy, at any age, in
any bed.**

So the two outcomes the owner sees are the only two available: beetles are a
fixed stock that either eats the colony down or is swamped by it. **A
predator–prey equilibrium is not rare in this build — it is unreachable**,
and it is one authored line away from being reachable. Whether it *would*
equilibrate is then a real question; today it is not being asked.

Note also that beetles carry `sight_range: 64` while the ant's ancestor
carries 32 and the shipped ant carries none — the predator sees and the prey
does not.

## 6. So: is it 1 in 100?

No. Across the runs here the outcome is not a lottery with a rare good
ticket; it is a deterministic consequence of the economy, and the spread
between seeds is smaller than the spread between *beds*. `labbatch`, 10
seeds, 30,000 frames, 32 founders + 1 colony: animals 30–68 (2.27x), all ten
qualitatively identical — a stable, space-capped, well-fed colony. The
per-seed spread is real (and is the noise floor any comparison must clear,
2.4–3.1x) but it does not contain a different *kind* of animal.

The variance the owner is seeing between sessions is the **ant:plant ratio at
founding**, not the genome.

---

## What this changes about next steps

The bottleneck is not the brain, not the mutation rate, and not the
generation count. It is that **the world has one thing to be good at.** In
rough order of how much each would change the answer, and framed as things to
expose or to make possible rather than to tune:

1. **Three of the twelve verbs are outside the economy.** Dig, emit and drop
   cost nothing. Until they have a price, selection is blind to excavation,
   trail-laying and cargo, and *no* nest-building or trail behaviour can be
   selected for at any horizon. This is the single largest gap.
2. **There is one income channel.** Any second way to be paid — a resource
   that must be carried, tended, cached or reached cooperatively — is a
   second dimension for selection. One axis produces one strategy, however
   many weights the genome has.
3. **The diet gene has two reachable values** because every food in the world
   is at `food_class` ±1.0. Intermediate food classes would make the gut a
   niche axis instead of a switch.
4. **Beetles cannot breed** — one field. That is the difference between "no
   equilibrium is possible" and "an equilibrium might be found".
5. **88% of births fail on adjacency.** Demography is currently geometry.
   Worth *exposing* (the number is already on the stats page) before deciding
   whether it should be changed.
6. **Species parameters are still compiled in** via `include_str!` —
   `reproduce_threshold`, `mutation_rate`, `dig_force`, `gut_bias` are not on
   the parameters page. Round three's note called this *"the largest single
   gap between the lab and 'I can figure it out myself'"* and the parameter
   half is still the larger one.

**One thing this report cannot settle**, and it should be settled before any
of the above is built: *interesting* is undefined. Trails, chambers, castes,
territory, predator–prey cycles and farming are six different mechanisms with
six different prices, and (1) and (2) look different depending on which one
is wanted. That question is the owner's, and it is cheaper to answer than to
guess.

## 7. Built, same day: the free verbs have a price — and what it bought

The owner picked *price the free verbs* off §"What this changes about next
steps", naming trails, nest architecture and predator–prey cycles as targets,
and adding the answer that reframes them: *"the real answer is anything
visually interesting. Clear variety in behavior. Different methods of
movement. Visual collection of food (although this might just be a coloration
thing). Any change in behavior over time."*

Three fields on `CreatureDef`, all `#[serde(default)]` and all shipping at
**0.0**, all three on the lab parameters page under Ants:

| field | prices |
|---|---|
| `dig_cost_in_moves` | excavating one cell |
| `emit_cost_in_moves` | one full-strength deposit, pro-rated on what was laid |
| `spoil_weight_cells` | a held pellet, while walking |

**One correction to §2 of this report, found while building it.**
`carried_cells` reads `state.crop` **only**, so it was never just the three
verbs — hauling the spoil was free as well, for up to `SPOIL_LIFT` (160)
cells. The excavation loop was unpriced end to end: free to cut, free to
haul, free to put down. Carrying *food* has always cost.

Priced in multiples of one step, following `LAUNCH_COST_IN_MOVES` rather than
inventing a unit: the price then scales with the animal exactly as walking
does, and *"digging a cell costs three steps"* is a sentence a tuner can hold
where a joule figure needs `start_energy` beside it.

### The measurement, and it is not the flattering answer

One bed, **one seed**, 40,000 frames, 32 plant founders, one colony:

| `dig_cost_in_moves` | standing animals | born | died | deepest animal generation |
|---|---|---|---|---|
| 0 | 57 | 88 | 83 | 6 |
| 4 | 36 | 45 | 61 | 3 |
| 16 | 33 | 47 | 66 | 5 |

A price roughly **halves the colony and halves its births**, and deaths
overtake births. Read that as one seed — the generation column is
non-monotone and this bed's seed-alone spread is 2.4–3.1x, so only the
population and birth effects are large enough to survive it.

**Why it costs rather than teaches, and the number that says so.** The dig
drive is `(Bias, Dig, 0.4)` — *unconditional*, on the bias input, so an ant
cannot dig less in response to anything. Selection has to walk that one
weight down. At `mutation_rate: 0.0058456` a given slot is touched about
**once per 171 births**; a run here produces 45–88 births, so that weight is
touched **about once in a whole run**. The gradient is now real and the
population is far too small and too short-lived to climb it.

So pricing the verb is **necessary and not sufficient**, and the gap is not a
tuning: it is that this bed does not run long enough, or breed enough
animals, for one weight to move. That is the same finding as §3 arriving from
the other side.

### A first measurement that measured nothing, kept because it is the trap

The first sweep was run on `burrow_probe arms=colony`, whose bank is bare
soil with **no food**, at its 8,000-frame default. Chamber size read 69 → 68
→ 67 cells across prices 0 → 2 → 8: essentially flat. Two faults at once,
both of them named elsewhere in this report:

- **8,000 frames is inside the ant's 12,000-frame founding grant** (§"How to
  re-run"), so nothing could starve and a charge that only bites at the
  bottom of the bank could not show.
- Re-run at 24,000 frames the rows came back **byte-identical apart from the
  frame column** — the tell `CLAUDE.md` names for a disconnected knob. The
  cause was neither: the colony is dead by 8,000 frames in a bed with no
  food, so there was nothing left alive to charge. *"An ablation in a broken
  economy measures nothing"* (`creature-direction.md` §13f).

The number that finally moved was measured in the lab bed, which has plants
in it.

### Not built, and why

**`Drop`/`DropSpoil` are left free.** §2 listed them as unpriced, and the
carrying charge above is the reason not to charge them again: an animal
already pays to hold a load every step it takes, so the release is the moment
it *stops* paying. Charging the drop as well would price one act twice and
would specifically tax delivering food to the nest, which is the behaviour
worth having.

**Nothing was re-derived, and no default moved.** `start_energy`,
`reproduce_threshold` and the authored dig weight were all fitted against
free digging, and `CLAUDE.md` requires budgeting their re-derivation as part
of any change that reallocates the shared budget. At 0.0 that bill is not
owed. It comes due the moment a non-zero default is proposed, and the table
above is the first instalment of the evidence for one.

### The other half of the owner's answer, not addressed here

*"Visual collection of food (although this might just be a coloration
thing)"* is a live question with a recorded prior, and it is not this change.
`OrganismState::crop` is internal state, never a cell in the world, and its
own doc says a carried cell was rejected as *"no payoff at all at the zoom a
creature is seen at"* — a judgement the owner has now contradicted.
`render.rs`'s gut-tint constants carry the rest: a hue A/B was posted and the
owner chose **untinted**, because *"an ant is one or two cells at play zoom,
the readable signal at that size is contrast against the ground rather than
hue"*, and the note says the untried axis is **brightness**, to be posted as
its own blind A/B rather than argued. `Crop` already carries `material` and
`shade`, so the data for a laden ant to look different is present and unread.
That is a review-queue question, not a report one.

## How to re-run any of this

```
cargo run --release --example creature_space -- genomes=24 seeds=1 frames=12000
cargo run --release --example labstats      -- frames=60000 founders=32 colonies=1 seed=1
cargo run --release --example labstats      -- frames=30000 founders=64 colonies=3 seed=1
cargo run --release --example labbatch      -- arm=seeded runs=10 frames=30000 founders=32 colonies=1
```

**Horizon is the trap in all of them.** An ant's founding grant is
`start_energy / (idle_cost_per_cell * cells)` ticks — 12,000 frames for the
shipped ant, ~5,400 in `creature_space`'s cheaper scene. Any creature result
read inside that window is a reading of who was given more, not who earned
more, which is Gate 2's own finding and the reason nothing here is measured
at `labbatch`'s 9,000-frame default.
