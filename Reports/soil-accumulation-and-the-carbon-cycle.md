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

## 5. What it measures

**Read this section first and the census below second.** The question is
*how big is the pile around the trunk*, and `crown_census` answers it
directly — soil standing above the ground line. Everything after it is a
world-wide count that mixes in root uptake happening everywhere else, and
it is kept only because it is a trap worth documenting, not because it
answers the complaint.

`crown_census frames=60000 trees=8`, one build, same world seed:

| | soil above the ground line | band 0 (the 40 rows just above ground) | reaches |
|---|---|---|---|
| yield 1.0 | **15,039 cells** over 107 rows | soil 10,312, litter 99 | y 90 — 110 rows up |
| yield 0.05 | **2,166 cells** over 89 rows | soil 1,798, litter 1,523 | y 99, thin above ~110 |
| yield 0 | **0 cells** — the line does not print | soil 0, litter 1,375 | — |

**The pile falls about 7x at 0.05, and to nothing at 0.** That is the whole
result.

**And the floor does not disappear, it changes material.** Band 0 goes from
10,312 soil / 99 litter at 1.0 to 1,798 soil / 1,523 litter at 0.05. The
forest floor stops being a dirt mound and becomes standing leaf litter, which
is what it should have been — litter is no longer converted to permanent soil
as fast as it lands, so it stands and rots instead.

**0.05 and 0 give nearly the same *visible* floor** (1,523 against 1,375
litter cells), because the floor you see is litter either way. What 0.05 buys
over 0 is a trace of real soil accumulating — the mechanism kept alive at a
believable rate — for almost no pile. That is the argument for 0.05 over
simply switching it off, and it is a judgement about what the ground should
mean rather than a measurement.

## 5a. The world-wide census, and why it is the wrong instrument

`scene=forest`, one build, same seed, `DECAY_YIELD` the only thing varied.
The stand grows from 805 to ~13,300 cells of living tissue over the window.

| frame | yield 1.0 net soil | 0.05 | 0 |
|---|---|---|---|
| 8,000 | −319 | −615 | −635 |
| 14,000 | −637 | −1,529 | −1,613 |
| 20,000 | −819 | −2,435 | −2,766 |
| 26,000 | −832 | −3,060 | −3,432 |

The yield itself does exactly what it says — the `rot:` line reports 100%,
5% and 0% against 2,383 / 4,370 / 4,376 decay events at frame 26,000 — and
the reseed and shed counts move as they should.

**The surprise is the sign, and it is worth understanding before anyone
quotes these numbers.** The world-wide soil census *falls* in all three
arms, and falls further the lower the yield. That is not the floor
subsiding: it is root growth, which overwrites a soil cell to occupy it
(`plant.rs::displace_soil_water`). A stand in its expansion phase consumes
more soil into root tissue than its litter returns, so the global count is
dominated by a term that has nothing to do with the complaint.

**So the census answers a different question than the owner asked, and the
picture is what settles it** — this repo's own rule that a metric says *how
much* while an image says *what and where*. The burial is local: a mound
heaped around each trunk, where the litter actually falls. Rendered at
frames 14k / 26k / 38k, the 1.0 arm grows conical mounds that climb the
trunks until they are swallowing the lower canopy, and the 0.05 arm holds a
flat ground line across all three. A world-wide count cannot see a mound;
it averages it against root uptake happening everywhere else.

Anyone sizing this later wants a **local** census — soil depth in the
columns under a crown, against columns in the open — not the global count.
That instrument does not exist yet.

### The long horizon, and a second failure mode at the other end

Same scene, both arms run to 212,000 frames — long enough for the stand to
saturate at ~16,000–18,000 cells of tissue around frame 62,000, which is
where the root-uptake transient above finishes.

| frame | 1.0 | 0.05 |
|---|---|---|
| 32,000 | −922 | −3,891 |
| 62,000 | **−2,210** | −6,555 |
| 92,000 | −1,705 | −7,847 |
| 122,000 | −1,804 | −8,587 |
| 152,000 | −1,138 | −9,162 |
| 182,000 | −774 | −9,663 |
| 212,000 | **−491** | **−9,923** |

**At 1.0 the floor bottoms out and comes back.** Once roots stop expanding,
litter return exceeds uptake and the count climbs steadily from −2,210
toward zero and past it. That is the accumulation, finally visible in the
global census once the transient it was hiding behind is over — and it is
the trunk mounds, arriving in a number.

**At 0.05 it never turns around inside this window, and that is worth
taking seriously rather than filing as success.** The forest converts soil
into standing tissue through its roots and gets a twentieth of it back, so
the floor falls to roughly half the soil it started with (20,343 → 10,420).
The fall is decelerating hard — the last three intervals are −575, −501,
−260 per 30,000 frames — so within this run it is heading for a plateau
near −10,000 rather than stripping the world. But it plateaus **because the
stand stopped growing**, not because anything balances it.

**So the two arms bracket the defect rather than one of them solving it.**
1.0 accumulates and buries; 0.05 depletes and then stalls. Neither is a
cycle, because a cycle needs the return path *and* a sink, and this engine
has a broken version of the first and none of the second. The honest
reading of 0.05 is that it trades a visible failure for a slower and less
visible one, and buys time to build §4.

**Untested and worth knowing before this runs much longer: multiple
generations.** Every figure here is one cohort growing and saturating. When
those trees die, `rot_remains` sheds their tissue — roots included — to
litter, and at 0.05 only a twentieth of that mass returns to soil. Whether
a second and third cohort keep drawing the floor down is not measured, and
it is the question this report would ask next.

## 6. Provenance and what is not measured here

**A worked instance of this file's own trap, left in deliberately.** §5a was
written first and treated as the result. It is arithmetically correct and
answers a different question than the owner asked — the complaint is a mound
at a trunk, and a world-wide soil count averages that against root uptake
across the whole map, which is the larger term. It took the owner asking
*"why do we even care about the floor? all we care about is the pile"* to
put the right instrument (`crown_census`, which already existed) in front of
the right question. Grepping `Reports/instruments.md` before building the
measurement would have found it.


Everything in §2 is background knowledge, not measurement from this engine —
the figures are ranges from the soil-science literature as commonly cited,
and they are here to size the problem, not to be tuned against. Nothing in
this report should be read as a calibrated constant.

**§3's 0.05 is chosen from §2's range, not fitted to anything in this
engine.** §5 checks that it does what it says and that the mounds go away;
it does not derive 0.05, and no sweep has been run over the value. If the
floor later reads as too bare, moving it is a free change.

**§5 is one scene, one seed, one build.** Outcomes here are chaotic in the
seed and this repo's own convention is that six seeds is not a sweep, so
treat the table as "the mechanism fires and the mounds go" rather than as a
calibrated magnitude. The visual comparison is paired and same-seed, which
is what makes it worth more than the census here.
