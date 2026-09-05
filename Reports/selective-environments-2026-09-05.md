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
checks, and checking it shallowly is worse than not checking it* — §4 has
both, on the same arm.

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

### The separability check, and the mistake I made doing it

**Condition (c) is the one nobody checks, and the first version of this
report got it wrong in print.** It said `arm=notrail` could not mean what it
says — that blinding the pheromone senses removes the ant's whole ability to
modulate movement rather than its trail-following — because `ant.ron` wires
`PheroAAlong`/`PheroBAlong` into hidden units 0–3 and those units drive
`Move`.

**That reasoning was too shallow and the conclusion was wrong.** The four
weights feed **differential pairs**:

```
h0 = squash(-45 + 75*Carrying + 6*PheroAAlong)      h0 -> Move  +2.5
h1 = squash(-45 + 75*Carrying - 6*PheroAAlong)      h1 -> Move  -2.5
h2 = squash(+45 - 75*Carrying + 6*PheroBAlong)      h2 -> Move  +2.5
h3 = squash(+45 - 75*Carrying - 6*PheroBAlong)      h3 -> Move  -2.5
```

With the pheromone term zeroed, `h0 == h1` and `h2 == h3` **exactly**, so the
pairs contribute exactly 0.0 to `Move` and the ablated ant is left with its
baseline `Bias -> Move (2.0)`, `FoodAdjacent -> Move (-1.5)` and
`Crowding -> Move (-0.3)`. **Ablating the trail sense is arithmetically
identical to standing in a world with no trail in it**, which is precisely
what the arm is supposed to mean. `notrail` is separable, and the measurement
stands: **3 seeds of 4 below the null, median 37.8%** (28.6 / 22.2 / 56.2 /
37.8) at 24,000 frames.

**Noting that weights reach an output is not the check. Tracing what the
pathway is worth is.** The shallow version cost a wrong claim in a pushed
report; the arithmetic took ten minutes.

### …and doing it properly found something better

Running the same numbers across the input range says the trail mechanism is
**gated on carry state by saturation**, and the window is narrow:

| `Carrying` | pheromone's share of the `Move` sum | change in p(move) at full signal |
|---|---|---|
| 0.0 (empty) | +0.03 | **+0.003** |
| 0.5 | +1.66 | **+0.119** |
| 1.0 (full) | +0.06 | **+0.007** |

The gate is `-45 + 75*Carrying`, which only sits near zero — where a `±6`
pheromone term can still move a saturating `squash` — at **Carrying ≈ 0.6**.
Away from that band the unit is pinned at ±0.97 and the trail signal is lost
in the saturation. So an ant reads its trail hard at about three-fifths
laden, and is **effectively blind to it empty or full**.

Gating trail-following on load is sensible — a laden ant is the one that
wants to go home. **The width of that window is not a designed quantity**: it
falls out of the ±45 / ±75 / ±6 magnitudes, and nothing records choosing it.
It also means the mechanism's selective weight depends on **where the bed
puts the crop-fill distribution**, which is a property of the world rather
than the animal: measured on the shipped bed, fill runs 25–75% with nothing
below a quarter, so ants do spend time in the live band — by luck.

### The audit, for the ant as authored

Every live weight in `ant.ron`, grouped by whether removing it removes one
behaviour and nothing else:

| behaviour | ablate | separable? |
|---|---|---|
| trail-reading | `PheroAAlong`/`PheroBAlong` -> h0–h3 (4 weights) | **yes** — pairs cancel at zero signal |
| excavation | `(Bias, Dig)`, and `(FoodAdjacent, Dig)`, `(MoistureGrad, Dig)` | **yes** — `Dig` has no other writer |
| feeding | every weight into `Feed` | **yes** |
| unloading | `Drop` / `DropSpoil` rows | **yes**, but the two share `AtNest` and `Carrying` with each other |
| homing (nest scent) | `AtNest` -> h4, h4 -> `EmitA`, h4 self-recurrence | **yes** — h4 writes only `EmitA` |
| heat avoidance | `(TempAboveAmb, Turn)` | **yes** — the only `Turn` writer at all |
| baseline locomotion | `(Bias, Move)` | not a behaviour; the floor everything else modulates |

**The ant is more separable than it looked**, which is the useful result:
every named behaviour has an ablatable set. The one caution is `Carrying`,
which appears in eight places (both hidden pairs, `EmitB`, `Drop`,
`DropSpoil`) — ablating *that* input is not a clean test of anything.

## 5. What each named behaviour would actually need

Applying §1's constraint honestly — every row has to move energy or
destruction, and every row has to pass (c).

**Trails and foraging routes.** Separable after all (§4), and the bed already
selects for it weakly — 3 seeds of 4, median 37.8%. To select for it *well*: food must be **clumped and far**, because a trail only pays when
re-finding a patch beats re-searching for it. In a bed where food is scattered
along one ground line within a few body lengths of everything, a random walk
is optimal and a trail is pure cost. The lab bed is that bed.

**Nest architecture.** **Experimented on 2026-09-05, and it is the deepest of
the four — the answer is not the environment.** Three hazards were put on the
other side of `arm=ablate input=Bias output=Dig`, whose null is known: with no
hazard the *non*-digger wins 4 seeds of 4. If shelter pays, that flips.

| hazard | non-digger's share | flipped? |
|---|---|---|
| none (the null) | median 63.6%, 4 of 4 | — |
| **ten beetles** | median 60.0%, 3 of 3 | **no** |
| **exposure at 1x the cost of living** | median 60.9%, 2 of 3 | **no** |

Exposure is not a weak tax — it is **18.2% of the colony's whole energy
burn**, larger than trail-laying and digging combined — and it still does not
make digging pay. Three findings, in the order they rule things out:

- **Not a generations problem.** The ablation is a direct fitness race
  between two arms in one bed; it needs no evolution to answer, and it
  answers the same way at every hazard tried.
- **Predation cannot be the pressure, because an ant cannot perceive a
  predator.** `PreyNear` is gated on `sight()` and the shipped ant authors no
  `sight_range` at all, so it defaults to 0 — **the ant is blind**. The only
  sense that fires on a beetle is `Crowding`, which counts *any* creature
  cell within r=2 and cannot tell a beetle from a nestmate (its own test
  records it reading "partly a body-size sensor"). There is no input a weight
  could connect to a retreat, so no bed can select for fleeing indoors.
- **And the hole they dig is the wrong shape.** Measured with the shelter
  census: ants are in the open on **66.2% of creature ticks** — under a roof
  a third of the time, incidentally, while cutting. `(Bias, Dig)` digs along
  the heading, which from a surface start is a **pit, open to the sky**. A
  pit is not shelter. Tunnelling into a bank face is, and nothing in the
  genome distinguishes the two.

So the missing piece is **in the animal, not the world**: no sense of threat,
and no drive that prefers roofed ground. `exposure_cost_per_cell` ships (at
0.0) because the hazard half is now real and switchable, and because it is
the half that was genuinely absent — but on its own it is a flat tax on being
alive, and a flat tax selects for nothing.

**Predator–prey cycles.** ~~Blocked on a hard fact rather than an
environment.~~ **Unblocked 2026-09-05, and the result is a capability rather
than a cycle.** `beetle.ron` had no `reproduce_threshold` at all, so
`reproduce_at_of`'s `> 0.0` gate returned `None` and a beetle could never bud
in any bed at any energy. It now authors **2550**, derived from its own birth
cost the way the ant's was: `grant_fraction(1.0) * 1600 = 1600` plus a
`200 * 4`-cell stamp is **2400**, and the ant's authored 1100 sits 6% over its
own 1040 — the same 6% here.

**The verb fires** — a beetle bred (6 → 7) inside 40,000 frames, the first
beetle birth this engine has produced. **Whether it changes the ecology is
not established**, and two seeds is why:

| | breeding | control (`beetlebreed=0`) |
|---|---|---|
| seed 1, 40,000 frames | ants 15, beetles 5–7 | ants **53**, beetles **1** |
| seed 2, 32,000 frames | ants 34, beetles 2 | ants 21, beetles 3 → 1 |

Seed 1 reads exactly like the owner's complaint being fixed: the control arm
*is* *"the ants have multiplied so much that a few beetles have no effect"*,
beetles decaying to one while the colony climbs back to 53, against a
breeding arm holding it at 15. **Seed 2 reverses it.** At this bed's 2.4–3.1x
seed spread two runs are a lottery ticket, so the honest reading is:
capability verified, consequence unmeasured. It wants a seed sweep read at an
order statistic — which is now a one-command question rather than an
impossible one.

**Division of labour.** Needs two tasks with different optimal traits *and* a
way for the payoff to reach kin. The engine has no food sharing between
adults, so an ant that specialises away from feeding simply starves. Furthest
from reachable.

## 6. Where this leaves the programme

The order that falls out is not the order these were asked in:

1. ~~**Separability audit.**~~ **Done for the ant, §4** — every named
   behaviour has an ablatable set, and the trail pathway turned out separable
   after a first reading said otherwise. The live caution is `Carrying`,
   which appears in eight places and is a clean test of nothing.
2. **Beetles breed.** One field, and it converts "no cycle is possible" into
   a real question.
3. **Clumped food as a bed parameter**, which is the environment trails need
   and is a `LabBox` knob rather than an engine change.
4. **Something outside that kills** — the missing half of every shelter
   argument, and the largest of these.

Every one of them is checkable with §3 before it is believed.
