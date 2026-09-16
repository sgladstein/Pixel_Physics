# The food trail points at the nest

**Investigation, 2026-09-16. `engine`.** Started from the owner's question
*"do pheromone trails work at all?"* and ended somewhere else: they do, the
half that looks broken is one sign away from working, and the reason nobody
found that is that the gate in front of it is welded shut.

Supersedes nothing. Extends `open-bugs-handoff.md` §Z7, whose channel-B half
is still open, and takes its standing conclusion — *"the thing to fix is what
a food trail is worth, not what the ant can read"* — as **half right for a
reason §Z7 did not have**.

---

## 1. What was measured

Three findings, in the order they arrived. Instruments:
`examples/onetrail.rs` (PR #462), `examples/nesthome.rs`' channel-B profile
(PR #464), and `onetrail mode=timing` (this report).

### 1a. The mechanism works; the food half does not read

One ant on a bare stone slab — no food, no nest, no nestmates, no colony, no
plants — with a trail that cannot decay (`step_pheromones` never called) and
the ant's own `EmitA`/`EmitB` weights zeroed so it cannot write the plane it
is tested on. 12 seeds x both mirror directions, 4,000 frames, median cells
travelled toward the ramp's high end:

| arm | median |
|---|---|
| laden ant, channel A ramp | **+104** of 112 available |
| empty ant, channel B ramp | **+1** |
| empty ant, channel B ramp, units 2/3 re-gated | **+104** |
| no trail at all | +2 |

Controls: every arm mirrored, so the sweep's left-to-right chunk order and the
movement rules' asymmetry cancel — the no-trail arms read a mean of **0.0**
over 24 runs. A `flat` trail reads `PheroBAlong` of **exactly 0.0000** and
moves the ant **0.0** cells, because the reader is a difference and a trail of
uniform concentration is unfollowable by construction. And `decay=on` makes
the standing-trail assertion fire, so its silence is evidence.

Arithmetically, on the shipped genome through `eval_brain`, `P(move)` against
the along-gradient: laden/channel A runs **0.200 -> 0.641**; empty/channel B
runs **0.200 -> 0.201**, and at `along = 1.00` — the absolute maximum the
reader can express, since it is bounded by +-1 — only **0.209**. Units 2/3 sit
at a gate sum of **+45**, where `squash` is saturated, so *open* is
indistinguishable from *shut*: the empty/channel-B row matches the
empty/channel-A **[gate shut]** row to three decimals. No trail geometry that
could ever exist moves it.

### 1b. Channel A has a designed shape; channel B has none

`EmitA` comes off hidden unit 4 — the **odometer**, charged by `AtNest`,
recurrence `0.99995`. Channel A is laid *weaker the further out an ant is*, so
it climbs toward home **by construction**. `EmitB` is the single direct wire
`(Carrying, EmitB, 2.5)`, with **no distance term anywhere**.

On the played bed at 40,000 frames, 32-column band totals from x=0:

```
seed 1  A  0 0 0 0 0 147 0 0 1640 19733
        B  0 0 0 185954 132553 86917 1052 0 12028 86839
seed 2  A  0 0 996 6254 0 0 645 55295 10021 0
        B  4735 0 47705 26199 10017 9376 148357 46838 11875 0
seed 3  A  0 0 0 0 0 0 9266 27265 35435 45021
        B  0 0 0 0 215499 129651 205015 19417 59052 264587
```

A is concentrated and rises toward the nest in all three. B is multi-modal,
points no consistent way, and carries several times A's total mass — a loud
signal with no shape against a quiet one with a good shape.

**Read `forage_trips` and `deepest` first**: 35 / 6 / 10 trips at excursions of
28 / 17 / 13 cells. This colony barely commutes, and a trail is a mechanism for
recruiting over a distance nobody is travelling.

### 1c. Timing *does* give B a gradient, and it climbs toward the nest

The owner's question, and the one this report exists for. A laden ant lays
channel B on every step of its walk **home**, so the cell at the food end is
written first and has been decaying longest by the time the ant arrives. That
should leave a ramp. It does.

`onetrail mode=timing`: no ant and no brain, one cell written per `per_cell`
frames along a 112-cell route from food to nest, the real `Pheromones::step`
running throughout, then the profile and the gradient a reader would compute.

```
per_cell=1  value food->nest:  1966 2989 3325 3712 4897 5731 6631 10240
            PheroBAlong facing nest: 0.151 -0.002 0.059 0.065 -0.000 0.088 0.108 **-0.976**
per_cell=4  value food->nest:   306  656  850 1126 1551 2266 3718 10515
            PheroBAlong facing nest: 0.231 0.040 0.045 0.051 0.063 0.084 0.125 **-0.976**
per_cell=8  value food->nest:    58  164  256  403  655 1126 2266 10537
            PheroBAlong facing nest: 0.153 0.047 0.058 0.069 0.082 0.104 0.180 **-0.976**
```

**The last column is the trail's own end and it is printed here because an
earlier draft of this report dropped it.** The 6-cell sensor samples past the
stamped span into zero, so it reads about -0.976. That is an edge artifact and
it is *not* negligible: it is 6 cells of however long the trail is — 5% at
span=112, **21% at span=28**, which is this bed's actual excursion depth — and
it sits at the **nest end**, which is exactly where empty ants are.

**"Monotone" is true of the eight probe values and false of the gradient the
ant computes.** Censusing every interior reading at the real 6-cell offset:

| `per_cell` | mean | min | max | readings <= 0 |
|---|---|---|---|---|
| 1 | +0.0365 | -0.0166 | +0.1505 | **47 of 106** |
| 2 | +0.0553 | +0.0286 | +0.2251 | 0 of 106 |
| 4 | +0.0746 | +0.0388 | +0.2312 | 0 of 106 |
| 8 | +0.0975 | +0.0433 | +0.3557 | 0 of 106 |

At `per_cell=1` nearly half the readings are non-positive, because
`PHEROMONE_INTERVAL = 12` makes the trail a staircase with 12-cell treads and a
6-cell sensor sits inside one tread half the time — `CLAUDE.md`'s
decaying-gradient quantization arriving in a new costume. It clears from
`per_cell >= 2`. The realistic figure is `per_cell ~ 8` (`tick_interval: 6`,
`P(move)` around 0.7, 8-way steps), so the bottom row is the fair one; the
`per_cell=1` row is shown because it is the one that disagrees.

**The control this needed, run: lay the identical trail nest -> food and every
reading mirrors exactly** — mean +0.0365 / +0.0553 / +0.0746 / +0.0975 becomes
-0.0365 / -0.0553 / -0.0746 / -0.0975, min and max mirroring with them
(`onetrail mode=timing reverse=on`). So the ramp is a property of laying
**order**, not of the scene, the chunked sweep, or the harness. Without this
the whole section was a claim about +x.

**And the magnitude is larger on a realistic trail, not smaller**: at
`span=28`, `per_cell=8`, the interior mean is **+0.2093** with 0 of 22 readings
non-positive.

**Not decomposed here**: how much of the ramp is `DECAY_RHO` and how much is
`DIFFUSE`. `pheromone.rs`'s own `DECAY_RHO` doc records the blend dominating on
a one-cell line (16.7% per pass against decay's 2.9%), so "decay" above should
be read as "the plane's step", not as the decay term alone.

**So channel B is not shapeless after all — it is shaped backwards.** §1b's
multi-modal bed profile is many ants' backwards ramps laid over each other
from many pickup points, not an absence of structure.

### The consequence: a proposed mechanism, not a demonstrated one

**Channel B, as laid and as wired, may be a second and noisier copy of channel
A.** Both ramps climb toward the nest, and `ant.ron` authors
`(PheroBAlong, 2, +6.0)` with the same sign as channel A's
`(PheroAAlong, 0, +6.0)`, so an empty ant with the gate opened is steered
**home**.

**This is a candidate explanation for §Z7's result, and it is not the only
one, and it is not established.** §Z7 and `dead-ends.md` already carry a
different mechanism from the same measurement: a colony whose laden ants home
on A and whose empty ants search at random is a central-place forager, and
giving empty ants B makes the search converge on patches *already eaten* —
with evidence (`unvisited` 57% -> 74%, the 16-48 band eaten out 284 -> 49, the
>128 band untouched 1,285 -> 1,514). Both mechanisms predict the same coarse
numbers. **Nobody has run the arm that separates them.**

**And the number to quote is not the one an earlier draft quoted.** §Z7's split
table reads: both pairs re-gated **25.0%** (0 of 6 seeds up), **food pair only
47.6% (2 of 6)**, homing pair only 63.6% (5 of 6). The shipped ant now carries
the homing half, so the arm that bears on channel B is the **47.6%** row —
roughly neutral against a zeroed-brain control's 15.4% — not the 25.0% both-
pairs figure. "Re-gating makes the colony worse" is doing work that row
qualifies.

*(The single-ant derivation above is also not measured under real colony
traffic, and §5 item 1 is the measurement that would settle it. See §1b's own
`forage_trips` 35 / 6 / 10: on the played bed ants are laden for about 3% of
samples and only 35 of 290 delivery episodes are forage trips, so the long
food-to-nest walk this derivation models is a minority of B-laying. Short-range
shuffling near the nest and larder lays B with no consistent direction, and
that is a cheaper explanation of §1b's multi-modal profile than many backwards
ramps overlaid.)*

*(One further leak in the premise: `Carrying` is `crop_fill.max(spoil ? 1.0 :
0.0)`, so an ant hauling a **dig pellet** lays channel B at full strength.
Measured at ~9.7% of laden ant-samples on one seed — not repeated, so treat it
as an order of magnitude. "B is laid only by an ant that found food" is
therefore about 90% true rather than true.)*

---

## 2. What I suggest

**Revised twice. The first draft proposed a food-charged odometer; the second
demoted it in favour of a sign flip; an independent review then measured the
sign flip and it does not work. The odometer is back at the front.**

### 2.0 The sign flip is dead, and this is why

The proposal was: negate `(PheroBAlong, 2, +6.0)` and `(PheroBAlong, 3, -6.0)`
so an empty ant **descends** channel B toward the older, food end. The
arithmetic is exact — the pair's contribution is
`2.5 * (squash(b + 6a) - squash(b - 6a))`, odd in `a`, so negating both weights
negates the response precisely. It reverses the reading. It does not produce a
trail follower.

`onetrail mode=walk span=112`, 8 seeds x both mirrors, 4,000 frames:

| arm | on-band | mean cells |
|---|---|---|
| re-gated **ascend**, starting on the trail | 100.0% | +50.4 |
| re-gated **descend**, starting on the trail | **29.5%** | -71.0 |
| re-gated **ascend**, starting 24 cells off the trail's end | **13.4%** | +14.1 |
| re-gated **descend**, starting 24 cells off the trail's end | **0.0%** | +0.2 |
| shipped (undirected) control, 24 cells off the end | 4.3% | +0.1 |

**A descending reader cannot acquire a trail — it is actively repelled from
one.** Off the trail, `here = 0` and `ahead > 0`, so the along-reading is
*positive* exactly when the ant is pointed at the trail; a descending reader
answers positive along with a low `P(move)`. It freezes facing the trail and
walks when facing away. At 0.0% on-band it is **worse than the undirected
control's 4.3%**, while the ascending arm from the identical start reaches the
band 13.4% of ticks and climbs — so the scene is not the limit.

And on the trail it overshoots: 29.5% on-band, because it walks down to the
end and keeps going into the zero beyond, where `along` is exactly 0 and it
reverts to an undirected walk wherever it landed.

**This also means a null from the sign flip could never have falsified the
diagnosis**, because the acquisition failure predicts a null whether the
direction story is right or wrong. It was not, as an earlier draft claimed,
"the cheapest decisive test". It was a guard that could not go red.

**A recorded dead end sits near it, and the ordering matters.** The reason to
drop this is the measurement above, not the register — and the register is a
record of *conditions*, not a blacklist. Every entry states the condition its
rejection depended on precisely so it can be retried when that changes, and
`CLAUDE.md` says in terms to *re-test any do-not-retry entry of that shape
after something changes its condition*. The entry here is
`Reports/dead-ends.md`'s
*"`assets/species/ant.ron` hidden units 2/3 ... re-weighted off saturation the
same way units 0/1 were; measured 2026-09-09"*, whose `Re-test when:` clause
reads *"The rejection depends on **what a food trail is worth in this bed**,
not on the gate"*, and which says in terms **"Do not retry the weight change
alone; it is not the variable."** An earlier draft proposed that retry and never
consulted the register at all, which is the process failure — `CLAUDE.md`:
*proposing, building or retrying any mechanism -> `dead-ends.md` first*.

**But the register did not turn out to be why the variant fails, and saying
otherwise would be the wrong lesson.** Its stated condition is about what a
food trail is *worth*; the sign flip died of something else entirely and new —
it cannot acquire a trail. Adding a sign genuinely was a different proposal
from the one recorded, and it deserved the test it got. What the register is
for is telling you what was already tried and on what the rejection hung, so
you can say how your variant differs. It is not a list of forbidden ideas, and
a measured failure is the only thing that closes one.

### 2.1 Measure the gradient a real ant reads, before building anything

The local along-reading on the played bed, at the real 6-cell offset, as a
**distribution** and split laden/empty. §1b's 32-column band totals are a mass
distribution and cannot answer it; §1c's derivation is a single ant in a world
with no traffic. This is the one measurement everything else is downstream of,
and it costs one instrumented run.

### 2.2 Then the food-odometer, ascended — and paired with depletion, not before it

Mirror the odometer onto **hidden unit 7, which is free** (0-3 are the gate
pairs, 4 the odometer, 5/6 the `Dig` pair, nothing touches 7): charge it from
`FoodAdjacent` rather than `AtNest`, decay it, wire it to `EmitB`. B is then
laid strongest just after leaving food, so it ramps **up toward the food** and
an ant *ascends* it. Ascending is the shape that works: it acquires from off
the trail and it terminates at a maximum rather than running off an end.

**Two things this needs that an earlier draft missed.**

- **The existing wire has to go or be budgeted with it.** `(Carrying, EmitB,
  2.5)` stays unless removed, so `EmitB = squash(2.5*Carrying + w7*h7)` and an
  *empty* ant near food would lay B — breaking the "only laden ants lay B"
  property the rest of this report rests on.
- **Pair it with depletion in the same arm.** `dead-ends.md` says the rejection
  depends on what a food trail is *worth*, and §Z7's competing mechanism
  predicts a *better* food trail is **worse** without a cessation signal —
  recruitment delivered more efficiently to patches already being eaten. If
  this is an epistasis problem then depletion is in the minimal set, and
  measuring recruitment without it measures a component rather than the
  mechanism.

### 2.3 The depletion signal, which is the thing genuinely missing

`Carrying` is **graded, not binary** (`crop_fill = worth / capacity`, and
`worth = unit x cells`, so richer food fills the crop faster) — an earlier
draft said binary and was wrong. What saturates is the **trip**: an ant that
fills its crop reads ~1.0 whether the patch has three cells left or three
thousand. So the trail cannot fade as its source empties, which is §Z7's "a
trail that outlives its patch keeps recruiting to nothing", and it is the
biological mechanism (§4.4) that makes real recruitment pay. Nothing in the
engine senses *how much is left where I found this*.

### 2.4 Whatever is run, re-derive the `Move` row

Every term in it was fitted where units 2/3 contribute ~0.001: `(Bias, 2.0)`,
`(Energy, -1.75)`, `(FoodAdjacent, -1.16)`, `(Crowding, -0.3)`,
**`(Stillness, 1.5)`**, `(KinNeed, 1.25)` and `(Alarm, -1.0)`. An earlier draft
listed only the first four, in a paragraph whose whole point is that the list
must be complete; `Stillness` at 1.5 is comparable to the +-2.5 a woken gate
pair would start delivering. `Crowding` is the one to watch — it is the
documented anti-ossification term and -0.3 was sized for a colony that never
follows trails.

### 2.5 Forks and crossings, unaddressed

Not simulated, and the arithmetic is forced: a **descending** reader takes the
*weaker* branch at a fork, inverting the differential reinforcement that
`pheromone.rs`'s `DIFFUSE` doc calls "the entire path-selection algorithm" and
that `pherolife mode=junction` measures at 0.845-0.969. At a crossing — a local
maximum — both headings read negative and the ant leaves undirected. One
`pherolife mode=junction` run would settle both. This is a further count
against any descending scheme and does not bear on 2.2.

## 3. The owner's concerns, as stated

1. **"I'm worried we're solving step one of a complex multi-step process and
   we're rejecting it because the first step doesn't fix everything."**
2. **"I'm worried if we highly engineer how the trail is laid and read, it
   will just become a single signal instead of a way for multiple creatures to
   communicate and potentially evolve different ways to communicate and use
   the tool. Are we turning a complex tool into a simple one? Of course, if
   the complex tool doesn't work, a simple functional one is better."**
3. **"Why is there not a gradient just based off of timing?"**
4. **"How do these mechanisms really work biologically?"**

---

## 4. My thoughts on them

### 4.1 On step one being rejected for not fixing everything — the concern is right; my proposed rule is withdrawn

The concern is sound and it is the most important of the four. A component of a
mechanism, measured alone, tells you about the component and not about the
mechanism, and §Z7's conclusion was drawn from exactly such an arm.

**But I over-claimed twice and both need retracting.**

First, an earlier draft said this was "now demonstrated rather than suspected".
It is not. Nobody has run gate+sign, and §2.0 now shows that particular pairing
does not work at all — so the epistasis I proposed is not merely unverified, it
is the one pairing measured and rejected. The only *measured* non-additivity in
the record is A-gate x B-gate (63.6% / 47.6% / 25.0%), which is §Z7's, not mine.

Second, I proposed this as a new `CLAUDE.md` rule. **Withdrawn.** That file
already carries the same failure in another costume — *"Ask which **pixels** a
lever moves, before ranking it by silhouette"*, where three levers demonstrably
fired, were ranked high, and moved nothing because a prerequisite they did not
touch was absent. Its remedy question is the one I was proposing. The
neighbouring *"When every setting of a sweep fails the same way, suspect the
sweep"* is the same shape with a harmful rider instead of a missing partner.
`CLAUDE.md`'s removal criterion is explicit that a file which only grows
dilutes every rule in it, and it ran +2,583/-365 over its history; promoting an
unverified hypothesis into a rule is what that criterion exists to stop. If
anything survives it is a clause on the existing pixels rule, and it should
wait until something has actually been measured.

What remains true and useful, as a working note rather than a rule: **before
measuring one part of a mechanism, ask what else has to be true for it to
work.** Here the answer was "the gradient has to point at the food *and* an ant
has to be able to find the trail in the first place", and nobody had checked
either.

### 4.2 On turning a complex tool into a simple one — the concern is right, and the current design is the one that prevents evolution

Take the concern seriously first, because the engine already agrees with it.
`pheromone.rs`'s own header: *"`Channel::A` and `Channel::B` carry **no
semantics**... Which is which lives in a species' instinct weights, never here
— that is what lets a second species reuse, or parasitize, the same planes."*
That is exactly the property the owner does not want to lose, and it is
load-bearing for the lab, where lineages are supposed to diverge.

So the line to hold is: **semantics live in the genome; the engine supplies
physics only.** Measured against that line:

| change | where it lives | preserves the property? |
|---|---|---|
| sign flip on units 2/3 | `ant.ron` weights | **yes** — one mutation reverses it |
| food-odometer on unit 7 | `ant.ron` `hidden_wiring` | **yes** — a zero weight is one mutation from existing |
| per-channel `rho` / `diffuse` | already parameters | **yes** — this is volatility, i.e. physics |
| multiplying `EmitB` by food worth inside `creature.rs` | engine code | **no — don't** |
| naming a channel "food" in `pheromone.rs` | engine code | **no — don't** |

All three suggestions in §2 sit above the line. None of them teaches the engine
what a channel means.

**But the sharper answer is that the concern points the other way here.** A
saturated gate is not a rich tool awaiting an evolutionary use — it is a
channel that **selection cannot see**. At a gate sum of +45 the difference
between reading the trail and not reading it is ~0.001 on `P(move)`; there is
no fitness gradient, so no lineage can climb toward using channel B. The
current state is the one that forbids evolution, and opening the gate is what
creates the axis for it.

**One claim in an earlier draft of this paragraph was simply wrong and is
withdrawn**: that `MUT_ABS_FLOOR` puts a sign change one mutation away.
`brain.rs` mutates by `width = MUT_ABS_FLOOR + MUT_REL * |w|` with
`MUT_ABS_FLOOR = 0.04` and `MUT_REL = 0.10`, stepping uniformly in +-width — so
at `w = 6.0` the largest single step is **0.64** against the **12.0** a sign
change needs, and the step shrinks as `|w|` does. That is on the order of 25
consecutive all-downward mutations, not one. `MUT_ABS_FLOOR` is the term that
lets a *zero* slot become a live connection, which is a different claim and is
the one that holds: 0.04 is comfortably above `W_EPS` (0.01), so the unit-7
odometer of §2.2 genuinely is one mutation from existing. The evolvability
argument survives for *adding* a connection and does not survive for *flipping*
one.

So: authoring generation zero's wiring is not the same as fixing the meaning.
`ant.ron` already authors channel A's odometer, and that is why homing works at
all; the lab is free to mutate away from it. The risk the owner names is real
for engine-side changes and small for genome-side ones.

**Where I would genuinely hold back**: adding a third channel, or a new
`BrainInput` that senses "food quantity at source" as a dedicated slot.
`stigmergy-research.md` §8's standing rule — resist a third channel until a
concrete consumer exists — is right, and step 3 above should be looked at very
carefully for exactly the owner's reason.

### 4.3 On the timing gradient — you were right, and it is worse than "there isn't one"

Measured in §1c: sequential laying plus the plane's own step produces a ramp,
at a magnitude the reader can act on, and the reversal control confirms it is a
property of laying **order** rather than of the scene. The gradient exists. It
climbs toward the nest, because B is laid **only while laden** and laden means
homeward, so the food end is always the older end. No deposit value and no
decay rate turns that around — it follows from *when* cells are written.

This is why the question was worth asking and why I had not answered it. I had
argued B was shapeless from a band profile; it is shaped, and a band total is
not the quantity an ant reads.

**What it did not license was the fix I then proposed.** "The gradient points
the wrong way" suggests "reverse the reader", and §2.0 shows a reversed reader
cannot find a trail at all. The direction finding is real; the obvious
inference from it was wrong.

### 4.4 On the biology

Four things from the real literature bear directly on this, and three of them
are uncomfortable for the current design.

**Trail polarity is a genuine problem that concentration alone does not
solve.** An ant standing on a scalar trail cannot tell outbound from inbound by
smelling harder. Pharaoh's ants (*Monomorium pharaonis*) solve it with **trail
geometry**: bifurcations are asymmetric and the fork angle encodes direction
(Jackson, Holcombe & Ratnieks, *Nature* 432:907-909, 2004). Others use visual
landmarks or route memory. An earlier draft put this as the flat assertion
*"real ants do not get direction from the concentration gradient"*; that is too
absolute for a large and varied literature. The defensible form is that
**concentration is not the primary polarity cue in the species where this has
been studied**, and that the known solutions are geometric or memory-based.

**And this argues against §2.0, not for it** — which an earlier draft did not
notice, having put the biology and the proposal in the same document without
checking them against each other. Asking an empty ant to read direction off B's
incidental age ramp is asking for the thing the biology says does not work.
It is not an argument against §2.2, which gives the trail a *designed* spatial
ramp rather than an incidental one.

**Ants modulate laying by quality and, crucially, stop.** *Lasius niger* scale
deposition with sucrose concentration and cease when the source is exhausted
(Beckers, Deneubourg & Goss, 1993). That negative feedback is what stops
recruitment to a dead patch. It is the biological form of §2.3 and it is the
piece genuinely missing here rather than merely mis-signed.

**Path integration is the real navigational analogue — but the analogy to
hidden unit 4 is weaker than an earlier draft claimed.** Desert ants
(*Cataglyphis*) navigate by a stride integrator plus a polarised-light compass
(Wittlinger, Wehner & Wolf, *Science* 2006), barely using trail pheromones at
all. Unit 4 is an internal decaying counter, which is superficially similar —
but path integration is an **internal vector the animal steers by directly**,
whereas unit 4 *modulates an external field that other ants read*. That is
stigmergy: the information leaves the animal and returns through the world.
Different in kind, so the earlier draft's "that is not a coincidence worth
ignoring" claimed a coincidence that is not there. What survives is narrower
and still useful: **a distance-since-home term is what gives a trail a designed
spatial ramp**, and that is the half of this engine's stigmergy that works.

**Real channels differ by physics, not by built-in meaning** — the biology's
answer to concern 4.2. Alarm pheromones are small volatile molecules (fast
diffusion, seconds); trail pheromones are heavier and less volatile;
cuticular hydrocarbons are non-volatile identity cues read by contact. Meaning
lives in which behaviour reads them. The engine has the matching knob already —
per-channel `rho` and `diffuse` — and that, not semantics in code, is the axis
along which species should differentiate. Honeybees make the point from the
other side: distance and direction travel by **waggle dance**, a different
modality entirely, because a scent field is bad at direction.

---

## 5. What would settle it

Reordered after review. None of this has been run.

1. **The local gradient a real ant reads on the played bed**, at the 6-cell
   offset, as a distribution, laden and empty separately. §1c is a single ant
   in an empty world and §1b is a mass distribution; this is the measurement
   that says whether the derivation survives 290 delivery episodes of which 35
   commute. Everything below is downstream of it.
2. **The unit-7 food odometer, ascended, paired with a depletion signal**, as
   one arm — per §2.2 and §2.3, and per `dead-ends.md`'s standing condition
   that the rejection depends on what a food trail is worth.
3. **Re-derive the `Move` row** with the full term list of §2.4 if anything
   above moves.
4. **`pherolife mode=junction`** for the fork behaviour of §2.5, if any
   descending scheme is ever revisited.

**Not** the bare sign flip of §2.0: it is a recorded dead end, it cannot
acquire a trail, and a null from it is uninformative by construction.

And one thing not to do: **do not judge any of these on the colony's size
alone.** Foraging range, deliveries, `unvisited` and `deepest` are what a trail
is supposed to move; a population count is several steps downstream and is
exactly the readout that produced the original reject.

---

## Review

Reviewed 2026-09-16 by an independent agent, which reproduced §1a byte-for-byte
and the arithmetic exactly, confirmed the hidden-unit map, the `Carrying`
grading and the `EmitB` wiring, and then found the sign-flip proposal
non-viable, the `-0.976` column dropped, "monotone" overstated, the
`MUT_ABS_FLOOR` claim wrong, the `dead-ends.md` entry uncited, the competing
§Z7 mechanism unmentioned, and two biology overstatements. Every one of those
was checked against the source before being accepted here; the off-trail
acquisition measurement in §2.0 was re-run in-tree rather than taken on
report, and the first attempt at it was mis-built (a start 20 cells off centre
on a 224-cell span is still on the trail). The reversal control and the
interior-gradient census are now permanent modes of `onetrail` rather than
throwaway harnesses.

## Instruments

- `examples/onetrail.rs` — `mode=arith` (shipped genome, nothing overridden),
  `mode=walk` (one ant, standing trail, mirrored arms), `mode=timing` (§1c).
- `examples/nesthome.rs` — `channel A` and `channel B` profiles by 32-column
  band, `scene=bed`.
- `examples/trailfollow.rs` — the colony-scale question and the `gate=`
  presets. Note its presets each overwrite **both** gate pairs, so no row of
  its `mode=arith` table is the shipped ant, which is a mix.
