# Soil accumulation, and the carbon cycle this engine does not have

**Status: the 5% yield is landed; the sink is not, and the sink is the real
fix.** Written 2026-08-27, from the owner's report that trees were being
buried in their own leaf litter up to their crowns.

This report exists because the immediate fix and the actual defect are
different sizes, and the commit that lands the first will otherwise read as
having addressed the second. It has three parts: what the engine does, what
the real world does instead, and what it would take to do the real thing
here.

## 1. The observation, and the accounting behind it

Owner, 2026-08-27: trees drop leaves, the leaves break down into soil, and
the soil builds higher and higher around the trunk.

The pipeline is short and every step of it is 1:1:

| step | site |
|---|---|
| a shed leaf becomes one `litter` cell | `plant.rs::shed_to_litter` |
| a settled `litter` cell weathers into one `soil` cell | `decay.rs::tick`, on `litter.ron`'s `decays_into: "soil"` |
| the `soil` cell is permanent | `soil.ron` — no `decays_into`, no erosion, no `food_energy` |

**Soil has no weathering sink.** It does not decay, nothing eats it, and
there is no erosion pass, so the third row is a floor rather than a step.

It is not *strictly* terminal, and the difference matters if anyone measures
this: a root growing into soil overwrites the cell (`plant.rs::
displace_soil_water` moves its water to the neighbourhood and the root takes
the space), and soil is a `Powder`, so it can also fall out of the world at
an edge. `filmstrip`'s floor census has seen the count run negative for
stretches on that account. Root uptake is even a genuine *net* sink now: the
root cell is shed to litter when the plant dies, and at a 0.05 yield only a
twentieth of it comes back. But both are incidental, both are bounded by the
plant population, and neither is a weathering process. The stand still runs
a surplus.

The mass accounting is the whole bug, and it is worth stating in one line:
**a plant fixes an abstract carbon resource out of light and builds a solid
cell with it.** Matter enters the world from nothing. With a 1:1 rot, every
one of those cells becomes permanent soil. That is a source with no sink,
and a source with no sink does not have an equilibrium at any rate — it has
a slope. The forest floor rises for as long as the stand lives, and the
stand lives as long as the run.

An earlier draft of this analysis offered to measure "whether the floor
reaches equilibrium." It cannot, and the offer was incoherent given the
paragraph above it. There is nothing to converge to. The only measurable
question is *how long until it is visible*, which is a question about the
slope.

**The one existing sink is small and worth knowing about:** `ant.ron` eats
litter, so an ant removes a litter cell before it can rot. That is real, it
is bounded by the ant population, and it acts on litter rather than on soil.

## 2. What the real world does

The natural first guess — that this happens in reality too, just much more
slowly — is wrong in an important way. **It is not primarily a rate
difference. The great majority of a leaf's mass never becomes solid at
all.**

**Decomposition is respiration.** Fungi and bacteria oxidise the leaf's
carbon and exhale CO2. Of the carbon in annual litterfall, the fraction that
ends up as stabilised soil organic matter is somewhere in the **1–10%**
range depending on climate, mineralogy and whose numbers you take; the rest
is back in the atmosphere within a few years. A forest floor is a chimney,
not an accumulator.

**Volume collapses on top of that.** Fresh leaf litter runs about
**0.02–0.1 g/cm³** — it is mostly air and water. Mineral soil is
**1.1–1.6 g/cm³**. So even the fraction that persists shrinks by roughly two
orders of magnitude in *volume*, which is the quantity a cellular automaton
actually cares about. A cell-for-cell engine that converts one leaf cell
into one soil cell is wrong on volume before the respiration loss is counted
at all.

**Organic matter enriches soil; it does not add depth.** Temperate forest
topsoil is a few percent organic by mass. New soil *depth* comes from
bedrock weathering underneath, at something like **0.01–0.2 mm/year**.
Leaves darken and fertilise the A horizon. They do not raise the surface.

**Bioturbation carries it down rather than letting it pile up.**
Earthworms, ants, termites, burrowing mammals and tree-throw mix the O
horizon into the mineral soil below. Darwin's last book (1881) was about
exactly this — he measured stones and Roman ruins *sinking* a few mm/year
under worm action. The surface does not rise; the material goes down and is
stirred in.

**And some of it simply leaves.** Wind blows litter out of the stand, water
carries dissolved and particulate organic carbon to streams, fire burns it
off.

The result is a **standing litter layer at steady state**, because that
layer has both a source and a sink: standing mass ≈ input ÷ decay constant
k. Temperate deciduous forest k ≈ 0.3–1/yr, so the floor holds one to three
years of leaf fall, a couple of centimetres. Tropical rainforest k is high
enough that the floor is close to bare despite enormous litterfall.

**The counter-example proves the rule and is worth keeping in mind for
worldgen.** Where decomposition genuinely stalls — waterlogged, anoxic,
cold — organic matter *does* accumulate metres deep. That is peat. And
burial genuinely does kill trees: root-collar burial on aggrading
floodplains is a real mortality mode. So the engine is not simulating
something impossible. It is simulating a peat bog everywhere, at speed.

## 3. What was landed, and what it does not do

`Material::decay_yield` — a per-material fraction of decays that leave a
cell of `decays_into` behind, the rest leaving nothing. Default **1.0**, so
no existing material changed. `litter.ron` sets **0.05**.

Ash keeps 1.0 and that is not an oversight: **ash is mineral.** It is what
is left after fire has already taken the carbon, so essentially all of it
stays put. Ash and litter sharing the decay *channel* while differing on
yield is the same shape as their already differing on rate — the channel is
generic, the material states its own physics.

**A yield rather than a slower rot rate**, and the two are not
interchangeable. `decay_chance_*` sets how long a cell *waits*: lowering it
makes litter pile up on the floor while producing exactly the same soil in
the end. The yield sets how much *survives*. Only the second is the quantity
the real world reduces, and the first would have traded a soil problem for a
litter problem.

`DECAY_YIELD` (env) overrides every material's yield, so 0 / 0.05 / 1 can be
compared without a rebuild. That exists because materials are `include_str!`d
and a `.ron` edit is invisible to an already-built harness — a gotcha this
repo has already lost a multi-hour study to.

**What this does not do, stated plainly so the commit is not misread: it
does not fix the accumulation.** 5% of an unbounded stream is unbounded.
The floor still rises monotonically, at 1/20th the rate. This buys time and
correctness-of-magnitude, not a cycle.

## 4. What an actual fix looks like

Ranked by how much of the real mechanism they buy against what they cost.
None of these is scoped or approved; this section is the menu, not a plan.

**A. Litter enriches the soil below instead of becoming soil.** The real
mechanism, and the highest-value one: rot writes fertility into the existing
soil cell underneath rather than adding a cell. This is what makes a forest
floor rich without making it tall, and it would give the plant economy a
reason to care where litter fell.

**The obvious version of this is a known dead end and must not be
retried.** Having new soil inherit or receive *moisture* was built and
reverted: it manufactures water, and it closes a pump — tree sheds litter,
litter wets the soil, tree drinks it, grows, sheds more. Measured:
`a_tree_eventually_stops_growing` went 1,718 → 2,652 cells and still
climbing (`decay.rs`'s own comment carries this). So A needs a **fertility
channel that is not water**, and a channel needs a named writer *and* a
named reader before it is worth building — this engine has shipped three
channels missing one end.

**B. Bioturbation: worms and ants carry surface material downward.** There
is already a `worm` material and ants already eat litter, so the agents
exist. This is the mechanism that most directly answers "why is the trunk
not buried" — the surface does not rise because the material is taken down
and mixed in. It is also the most *visible* of these, which by this repo's
ethos matters: it is a verb, performed by a creature the player can watch.

**C. A soil sink: erosion, compaction, or settling.** The structurally
honest fix, because it is the missing half of the ledger. Wind and water
moving surface soil, or deep soil compacting under load. Cost is the
problem: a per-cell erosion pass over all exposed soil is sweep-scale work
on a system that currently costs nothing, and `CLAUDE.md` is explicit that
frame cost is a hard constraint rather than a tiebreaker. Worth scoping
against the dirty-rect skip before anyone builds it.

**D. Volume collapse: N litter cells rot into 1 soil cell.** Models the
density change directly and is more physical than a probabilistic yield.
Needs neighbour counting in the decay path, and it is a *smaller* idea than
A–C — it corrects the exchange rate without adding a sink. Probably only
worth it if the yield roll turns out to read badly.

**E. Do nothing more, and let burial be a real death.** The peat and
floodplain cases are real. If soil eventually swallows a stand, that is a
succession event rather than a bug — provided it is slow enough to read as
one, and provided the world is large enough that it is local. Worth
revisiting after M10 streaming, not before.

**The recommendation, if this is picked up: B first.** It is the only one
that is simultaneously the real mechanism, visible on screen, performed by
an agent that already exists, and free of the water-pump trap that A has to
solve first.

## 5. Provenance and what is not measured here

Everything in §2 is background knowledge, not measurement from this engine —
the figures are ranges from the soil-science literature as commonly cited,
and they are here to size the problem, not to be tuned against. Nothing in
this report should be read as a calibrated constant.

**§3's 0.05 is chosen from §2's range, not fitted to anything in this
engine.** Nobody has yet measured how the floor reads at 0.05 against 0 or
1 in a long run. That measurement is the obvious next step and it is not in
this report.
