# How to build an environment that selects for a behaviour

*Owner, 2026-09-05, correcting my framing: "not just population size and run
length but actually setting up the appropriate environment to select for
them. how do we do that?"*

The correction is right and this report is the answer. I had said the
remaining bottleneck was that the population is too small and too short-lived
for one weight to move. True, and second-order: **an infinite population over
infinite generations still evolves nothing in a world that does not
differentially reward the behaviour.** Population size is necessary; the
environment is what makes it sufficient.

**This is a method plus one worked demonstration, end to end.** It is not a
list of proposals.

---

## 1. The master constraint: there are exactly two currencies

Measured, `src/sim/creature.rs`: an animal dies in exactly two ways.

| how | where |
|---|---|
| **energy reaches zero** | `creature_tick`, two sites |
| **its cells are destroyed** — bitten, burnt, bladed | `reconcile_chain`, `slay` |

There is no thermal death, no desiccation, no old age, no disease. Heat is a
brain *input* (`TempAboveAmb`, which the ant wires to `Turn` at `-0.8`) and
nothing more: an ant walks away from a fire because it was authored to, not
because heat can hurt it.

**So every selective pressure that will ever exist in this engine has to
route through energy or through destruction.** That is the whole design
space, and it is the first thing to check against any proposed environment:
*which of the two does this change?* An environment that changes neither
changes nothing, however different it looks.

It is also why the free verbs mattered so much. Before 2026-09-05, digging,
trail-laying and hauling spoil touched neither currency
(`Reports/creature-behaviour-ceiling-2026-09-05.md`), so they were outside
the space entirely.

## 2. The four conditions

A behaviour B is selectable in environment E when all four hold. Each has
already failed at least once in this repo.

**(a) B must pay, in a currency.** The obvious one, and the one the free
verbs failed.

**(b) The gradient must be incremental.** A behaviour needing several
simultaneous mutations is unreachable however well it pays. Anything on a
single weight is reachable; anything needing a new *pathway* is not, at this
mutation rate.

**(c) B must be separable in the genome.** *This is the condition nobody
checks, and it silently invalidated a measurement in this very session* — see
§4.

**(d) E must not be dominated by a stronger pressure.** If everything starves,
nothing else is selected. `creature-direction.md` §13f already records this
as *"an ablation in a broken economy measures nothing"*, and this session
re-ran it: a dig-price sweep on `burrow_probe`'s foodless bank read flat
across every setting because the colony was dead by frame 8,000.

## 3. The method: never argue an environment, ablate it

**You cannot design a selective environment by reasoning. You test it.** The
test is one command, and it is the generalisation of Gate 2:

> Take the sense or the verb the behaviour needs. **Remove it.** Run the
> crippled animal against the intact one in the same bed. If the bed does not
> punish the loss, the bed cannot select for the behaviour — at any
> population size, over any number of generations.

`examples/creature_arena.rs` does this. It had fixed rungs (`lethal`,
`nofeed`, `notrail`, `random`); it now takes an arbitrary one:

```
creature_arena -- arm=ablate input=Bias output=Dig seeds=4 frames=24000
creature_arena -- arm=ablate input=Crowding            # every route out of one sense
```

and — the half that was missing, because *"does this environment select for
X"* is a question about the **environment** — it can now vary the economy in
the same run:

```
creature_arena -- arm=ablate input=Bias output=Dig digcost=0 emitcost=0 spoilweight=0
```

**An arena that can only vary the genome can only answer half the question.**

**Horizon is the standing trap.** An ant's founding grant is
`start_energy / (idle_cost_per_cell * cells)` = **12,000 frames**. Inside it
nothing can starve, so any arm that merely spends less wins by not spending.
Every number below is at 24,000.

## 4. The worked demonstration

**Question: does the bed select on the standing dig drive?** `ant.ron`'s
`(Bias, Dig, 0.4)` is unconditional — the thing that makes a bed become one
enormous hole. Ablate that single weight and race it against the intact ant.

Four seeds, 24,000 frames, mirrored, 52 ants against 8 founders:

| economy | non-digger's share of animals | seeds favouring it |
|---|---|---|
| **all three verbs free** (the world before 2026-09-05) | median **50.0%** (46.2 / 37.9 / 50.0 / 60.4) | 1 of 4 — 2 below, 1 tied |
| dig free, spoil and emit priced | median 70.0% | 3 of 4 |
| **shipped** — dig 6.0, emit 0.5, spoil 1.0 | median **63.6%** (56.5 / 63.6 / 64.9 / 53.3) | **4 of 4** |

**The first row is the null, and it is the finding.** With every verb free
the bed could not tell a digger from a non-digger — which is exactly what
*"the hole is selectively neutral"* claimed, now measured rather than argued.
The last row is the same bed after one change to the *environment*: it
reliably prefers the ant that does not dig.

**Nothing about the animal differs between those rows.** Same genome, same
ablation, same seeds, same horizon. Only the price of a verb moved, and a
behaviour went from invisible to selected. That is the whole answer to the
owner's question, in one table.

**And the middle row corrects a number in the report that preceded this
one.** `spoil_weight_cells` was measured at **+0.3% of movement cost** across
the population and called "honest rather than powerful". Across the
population it is. But selection does not act on a population average, it acts
on the *difference between two animals* — and a digger holds spoil while a
non-digger never does, so the whole of that weight falls on one arm. With the
dig price off and only spoil and emit priced, the non-digger still wins 3 of
4. **A term that is negligible in the aggregate can be decisive in the
contrast**, which is a general warning about sizing any lever by its share of
a total.

### What the same instrument could *not* answer, and why it matters

`arm=notrail` — blind the ant to both pheromone planes — loses badly (28.6%
and 22.2% on two seeds). It reads as *"the bed selects for following
trails"*. **It does not, and this is condition (c) failing.**

`ant.ron` wires `PheroAAlong` and `PheroBAlong` into hidden units 0–3, and
those units drive **`Move`** (`(0, Move, 2.5)`, `(1, Move, -2.5)` …). That is
the run-and-tumble mechanism: a laden ant walking away from the nest scent
computes a low `Move`, fails the roll, and re-orients. So blinding the
pheromone senses does not remove trail-following, it removes **the ant's
entire ability to modulate movement**. The arm measures "a broken navigator
loses", which is nearly tautological.

**There is currently no clean test of whether trails pay, because one
mechanism does both jobs.** No environment change can select for trail
quality while the trail sense and the walk are the same four weights. That is
a fact about the animal, not the world, and it has to be fixed in the genome
before any bed can be judged on it.

**The general lesson, and it outranks the specific one:** before designing an
environment for a behaviour, check that the behaviour is *separable* — that
there is something you can remove which removes it and nothing else. If there
is not, no environment selects for it, and an ablation will hand you a
confident number about something else.

## 5. What each named behaviour would actually need

Applying §1's constraint honestly — every row has to move energy or
destruction, and every row has to pass (c).

**Trails and foraging routes.** Blocked on separability first (§4). Then the
environment: food must be **clumped and far**, because a trail only pays when
re-finding a patch beats re-searching for it. In a bed where food is scattered
along one ground line within a few body lengths of everything, a random walk
is optimal and a trail is pure cost. The lab bed is that bed.

**Nest architecture.** Needs something outside that a chamber protects
against, and §1 says it must be energy or destruction. Today a burrow costs
energy and returns nothing — there is no weather that kills, no predator that
cannot follow, no thermal load. **A roofed cell is worth exactly as much as an
open one**, so no amount of digging skill can pay. This is the clearest case
of an environment that cannot select for the behaviour it was built to
produce: `burrow_probe` measures chamber shape beautifully and nothing in the
world cares what shape it is.

**Predator–prey cycles.** Blocked on a hard fact rather than an environment:
`beetle.ron` has no `reproduce_threshold`, so a beetle can never bud. One
authored field. Until then the predator is a fixed stock and there is no
cycle to find.

**Division of labour.** Needs two tasks with different optimal traits *and* a
way for the payoff to reach kin. The engine has no food sharing between
adults, so an ant that specialises away from feeding simply starves. Furthest
from reachable.

## 6. Where this leaves the programme

The order that falls out is not the order these were asked in:

1. **Separability audit before any environment work.** For each behaviour,
   name the weights whose removal removes it *and nothing else*. §4 shows
   what happens when you skip this. Cheap: it is reading `ant.ron`'s wiring
   against `brain.rs`'s slots.
2. **Beetles breed.** One field, and it converts "no cycle is possible" into
   a real question.
3. **Clumped food as a bed parameter**, which is the environment trails need
   and is a `LabBox` knob rather than an engine change.
4. **Something outside that kills** — the missing half of every shelter
   argument, and the largest of these.

Every one of them is checkable with §3 before it is believed.
