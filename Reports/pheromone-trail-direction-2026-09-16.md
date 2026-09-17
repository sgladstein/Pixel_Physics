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

---

## 6. Two questions the review did not reach, and the architectural answer under both

Raised by the owner on reading §2.0. Both have the same root, and it is a bigger
finding than the sign.

### 6.1 Why a descending reader is repelled, and what would fix it

`PheroBAlong = (ahead - here) / (ahead + here + SCALE)`. **Off the trail,
`here = 0`, so merely facing the trail makes the reading positive** -- there is
more ahead than underfoot. A descending reader answers a positive reading with a
low `Move`; it fails the roll, tumbles (`Tumble` is unwired in every species, and
`unit_scale(0.0, 1.0) = 0.5`, so a failed move re-orients at a flat 50%), and the
only headings that let it walk are the ones pointing away. It is gradient descent
on a ridge: descent leaves the ridge, and the flat it lands in has no gradient at
all, so nothing brings it back.

**The real gap is that nothing answers "am I on the trail".** The along-reader
answers *which way along it*, and it is the only channel-B reader any species
has. The Jones/Physarum triad this input list was modelled on had both: a
**front** sensor for acquisition and two laterals for steering. The laterals were
correctly abandoned here -- measured 0.000 on a surface, since at full offset
they point into air and rock -- but the **front sensors were never wired
either**, and `brain.rs` records that `PheroAFront`/`PheroBFront` have never
carried a weight in any species file in any commit.

Candidates, cheapest first, and the first two are complementary:

1. **Wire the front sensor.** `(PheroBFront, Persist, +w)` -- commit to a heading
   while standing on a trail -- or `(PheroBFront, Move, -w)` to slow down on one.
   `Persist` is attractive because it is unwired in every species, sits at its
   `unit_scale` default of 1.0 against a `PERSIST_MAX` of 2.0, and `creature.rs`
   calls it *"the number that decides whether a creature commutes or mills"* --
   and because it spends none of the `Move` budget.
2. **Make the ramp point at the food.** Then ascent does both jobs at once,
   because "more" is toward the trail *and* toward the food -- which is why the
   ascending arm in §2.0 acquires from off-trail at 13.4% while the descending
   one manages 0.0%.

**This was rejected on 2026-09-14, and a new proposal has to say how it
differs.** The rejection (`pheromone-lifetime-and-wiring-2026-09-14.md` §"The
front sensors stay unwired") rests on an *argument*, not on a measurement of the
fronts: *"A creature confined to a line does not need 'am I on it' -- it needs
which way along it"*, plus *"there is no fork on open ground"*. Its one
measurement, `pherolife mode=junction`, measures the **rival** input's
discriminating power, in a scene that harness's own doc admits no ant can walk.
**The acquisition data attacks the first premise directly**: an ant that only
needs "which way along" must still *get on the line*, and nothing serves that.
The rejection names its own reopening -- *"a lineage for which absolute
concentration is worth something can evolve the connection"* -- and the framing
that differs is **acquisition**, not the **selection** ("which branch is busy")
that §2a proposed and that rejection killed.

### 6.2 Reading a nest-peaked channel would push empty ants off the nest -- and that breaks more than it looks

The owner's objection, and the wiring makes it concrete rather than aesthetic.
**Every nest behaviour in `ant.ron` is gated on `AtNest`, a *contact* sense:**

| wire | what it does |
|---|---|
| `(AtNest, Drop, 1.0889)` | deliver food to the nest |
| `(AtNest, DropSpoil, 0.9)` | dump excavated soil |
| hidden units 5/6, `AtNest` +30 with `Crowding` +-6 -> `Dig` | excavate the nest |

An ant not standing on nest material can do none of them. So a scheme giving
empty ants a reading that *repels them from the nest* does not merely look wrong
-- it **disables digging, delivery and spoil disposal**, which is most of what a
colony does that is not foraging. The nest becomes what the owner describes: a
pile of food that ants are pushed away from.

The shipped ant escapes this, for a reason worth stating: empty ants do not read
channel A at all (units 0/1 are gated *shut* on an empty crop), so they are not
repelled from home, they simply diffuse. **Any change that opens a nest-peaked
channel to empty ants creates the defect.**

**Underneath it is the real architectural finding: `Move` is overloaded.** It
carries two different questions in one scalar:

- *Am I active or at rest?* -- `(Bias, 2.0)`, `(Energy, -1.75)`,
  `(Stillness, 1.5)`, `(Crowding, -0.3)`, `(FoodAdjacent, -1.16)`. This is the
  colony's whole division of labour: a fed ant rests, a hungry one forages.
- *Is this the right direction?* -- the two gated trail pairs.

Real colonies separate these: task allocation decides **whether** to forage, the
trail decides **where**. Collapsing both into `Move` is why every trail change
reallocates the rest/work budget, and why a stronger trail term mechanically buys
itself range by spending the colony's rest.

**The consequence for future trail work: prefer `Turn` and `Persist` over
`Move`.** Steering does not consume the activity budget, and a `Turn` wire cannot
push an ant off the nest, which answers the objection structurally instead of by
tuning. `PheroBAlong -> Turn` is expressible today and untried -- the direct
channel-to-`Turn` gains were removed on 2026-09-09 in favour of the gated `Move`
pairs, and the `Turn` route was never revisited after the along-reader replaced
the laterals.

### 6.3 The owner's foraging loop is the textbook model, and names the missing piece

Stated as: empty ants follow a trail to food, carry it back strengthening the
trail, repeat until the food is gone, then laden ants stop laying, the trail
dissipates, and search goes undirected again. **That is the Deneubourg/Beckers
recruitment model of *Lasius niger*, almost exactly.** The engine has the
autocatalysis, the decay and the carrying. It is missing two things, and this
report found both: the trail does not point at the food (§1c), and **nothing ever
stops laying** (§2.3) -- `Carrying` saturates on any successful trip, so the
trail cannot fade as its source empties. In the biological model that cessation
is what makes the loop terminate; without it a trail outlives its patch, which is
§Z7's own diagnosis reached from the other direction.

### 6.4 Where this engine does and does not follow the biology

**It does**, and more deliberately than is obvious: the along-reader is a
Weber's-law relative difference, which is what Perna et al. measured individual
Argentine ants to actually use, against the sigmoidal absolute response the
classical model assumes. Stigmergy proper, autocatalytic reinforcement,
trophallaxis (`KinNeed -> Share`) and `Crowding` as the anti-ossification term
are all real mechanisms, cited in `stigmergy-research.md`.

**It does not**, in four places, three of which this report has now hit:

- **No quality or quantity scaling of deposit, and no cessation.** The
  best-established recruitment biology there is, and absent.
- **No polarity mechanism.** Real ants get direction from trail *geometry*
  (asymmetric bifurcations -- Jackson, Holcombe & Ratnieks, *Nature* 432:907-909,
  2004) or from route memory. Stated carefully: concentration is not the primary
  polarity cue *in the species where this has been studied*, and the known
  solutions are geometric or memory-based. The engine asks concentration to
  supply it.
- **Path integration steers a real ant; here it only modulates emission.** Unit 4
  is a distance-since-home counter wired to `EmitA`, so it shapes a field other
  ants read. *Cataglyphis* steers by its home vector directly and barely uses
  trails (Wittlinger, Wehner & Wolf, *Science* 2006). **The analogy is weaker
  than it looks** -- path integration is an *internal* vector the animal steers
  by, while unit 4's information leaves the animal and returns through the world,
  which is stigmergy and different in kind. What survives is narrower and still
  useful: a distance-since-home term is what gives a trail a designed spatial
  ramp, and that is the half of this engine's stigmergy that works.
- **The front sensors are unwired** (§6.1), though real ants plainly do sense
  trail concentration and not only its gradient.

**The synthesis, and the thing to take from this report if only one thing is
taken: in reality the trail answers *am I on the road*, and something else --
geometry, memory, a compass -- answers *which way down the road*. This engine
asks the trail to answer both.** That single mismatch produces the acquisition
failure of §6.1, the polarity problem of §1c, and the overloading of `Move` in
§6.2. It is the same defect seen three times.

---

## 7. The colony-scale run, and five things it corrected on the way

Everything in §1–§6 is single-ant or plane-side. This section is what happened
when the question was put to colonies, and **most of it is corrections** —
three of them to claims made earlier in this same document or in its commits.

### 7.1 Re-gating channel B works at colony scale

`trailfollow mode=colony`, 6 seeds, near-target ant-ticks with a standing
food-ward trail against the same seed without one:

| arm | pooled | seeds up |
|---|---|---|
| shipped, genome untouched | 1,933 vs 0 | 2 of 6 |
| re-gated units 2/3 (`b2`) | **18,489** vs 0 | **6 of 6** |

**9.6x**, colony alive at 18–19 of 20 throughout.

**The control was wrong on the first attempt and read 8.6x.** `gate=saturated`
had not been the shipped ant since 2026-09-09: it writes `Carrying:0:75` where
`ant.ron` ships **45.5**, and `Bias:2:30` where the file ships **45**, so it
saturates the *homing* pair too and any channel-B delta measured against it
silently includes homing. `gate=shipped` (applies nothing) is the honest
control. The file's own `LANDED_NOTE` warns these preset names go stale; it had
gone stale again since that warning was written.

### 7.2 §Z7's headline null no longer reproduces

§Z7 records `trailfollow` returning **595 = 595 exactly, twice**, and
1,903 = 1,903 on an independent harness. On current `main` the *untouched*
shipped ant moves 2 of 6 seeds. Almost certainly the `u16` widening (#450) —
the same change that retracted `nest-design-2026-09-14.md` §5.1. **§Z7 wants
updating to say so.**

### 7.3 The 144-frame trail lifetime is a `u8`-era figure, and a prediction died on it

This branch predicted a trail-persistence threshold of **~19 commuters**, from
`round_trip / 144` where 144 frames is
`pheromone-lifetime-and-wiring-2026-09-14.md`'s *"a cell laid at `DEPOSIT` dies
in ~144 frames, 0.065x of a round trip"*.

**That measurement was invalidated by the `u16` widening the day after it was
written.** `DEPOSIT` went from 40 to `40 * SCALE` = 10,240. `pherolife` on
current `main`:

```
trail gone 1,476 frames (0.67x round trip)   stops steering 1,080 (0.49x)
```

Ten times longer. Corrected threshold ≈ **2 commuters, not 19.** Persistence
was never close to binding.

**The tell was `CLAUDE.md`'s tidiness signature**: `cells pk` read exactly 91 —
the full route length — in *every* arm across four populations and three seeds,
and again after a "fix" that moved the sampling window to `stop + 288`. A column
identical across every arm is an artifact. Chasing it is what surfaced the
stale constant.

### 7.4 The played bed cannot test any of this, and not for the reason first given

`played_bed` puts the colony at x=256 inside a **deliberate bare band
(210–310)**, nearest food ~75 cells out — the file records that moat being
worth 33 seated founders against 23 without it. Measured `deepest` across 18
runs: **7–49 cells, median 17**, against a nest band of **±26**. Most ants never
leave the nest's own footprint.

**But the moat does not stay a moat**, and an earlier draft's "they live on
litter drifting in" was wrong. `latecensus`:

```
frame    plnts   bare/band     seedJ    crpsJ   leafJ
    0       18    129/129      2,160        0       0
10000      176     80/129    109,560   37,080       0
30000      259      0/129    137,760   45,000       0
```

The band **fills in completely** — 18 plants to 259, 701 germinations. The ants
eat seeds from plants that colonised the gap, plus their own dead. So the bed
removes distance as a variable within half a session, and a trail has nothing
to serve.

**Consequence for Phase 1.0**: the homing sweep (shipped vs `noemit` vs
`nosteer`, six seeds) is **inconclusive, not negative**. Shipped beat both cut
arms on 2 of 6 with the lowest median — but cutting a homing circuit costs
nothing when nobody makes the trip. §12's "2 of 3 seeds" was very likely the
same noise.

### 7.5 `far_larder`'s colony dies of hunger, not distance

`far_larder.ron` is the right *shape* of bed — `founders: 0` so no plants
exist, colony at x=107, food pinned at x=470 (363 cells) and replenished
forever. Verified: `plnts` 0 and `bare/band` 129/129 across 30,000 frames. It
genuinely does not spread.

**But its colony starves regardless of distance.** 52 ants at two cells,
`idle_cost_per_cell` 0.05 and `move_cost_per_cell` 0.125 on a 6-frame tick, need
roughly **46,800 J** over 24,000 frames:

| supply | J | share of need |
|---|---|---|
| `far_larder`: 60 cells + 30 every 6,000 | 21,600 | **46%** |
| an earlier sweep here: 60 cells once | 7,200 | **15%** |

So *"363 cells starves the colony"* was published as a distance bracket and is
not evidence about distance at all — that colony dies at a gap of zero. The
arithmetic should have been done before the sweep, not after reading twelve
rows of `alive 0/0`.

### 7.6 With food replenished, a trail buys survival at 90 cells and nothing beyond — SUPERSEDED by §7.10

`trailfollow mode=gap`, 6 seeds, `food=200 refill=4000`, trail hand-laid to
frame 6,000 then released:

| gap | alive on (median) | alive off (median) | pooled | seeds up |
|---|---|---|---|---|
| **90** | **9.5** | **2** | **57 vs 13** | **4 of 6** |
| 150 | 3 | 4.5 | 24 vs 32 | 2 of 6 |
| 220 | 0.5 | 0 | 7 vs 0 | 3 of 6, rest dead both ways |
| 300 | 0 | 0 | 0 vs 0 | all dead |

**This is the first colony-level benefit a trail has bought anywhere in this
work** — 4.4x pooled survival at 90 cells — and it is gone by 150.

**Two corrections to this run, found on 2026-09-16 while building the larder
isolation §7.7 asked for. The survival comparison survives both; the
provisioning reading does not.**

*The energy figure was wrong by 4x.* This section first said the run supplied
"168,000 J, 3.6x need". That took the material table's face value and dropped
the gut filter: at the shipped **neutral** gut (`ant.ron` `traits` slot 0 =
0.0) `creature::diet_quality` returns 0.25 against either end of the food-class
axis, so a placed `corpse` cell is worth **30 J, not 120**. The run supplied
about **42,000 J against ~46,800 J of need** — roughly 90%, not 360%. It was
under-provisioned, and §7.5's whole point is that I should have run this
arithmetic before the sweep rather than after.

*And the colony had a better food source than the larder.* A placed
`Cell::new(corpse, 0)` carries `aux` 0 and is priced by `food_energy` (120 face,
30 J eaten). A **dead ant's** corpse is stamped with its `body_energy`, 480, and
`creature::food_value` prefers that stamp — **120 J eaten, four times the
larder**. So in a scene that kills most of its colony, the richest food in the
world was lying inside the nest, and it was free.

**What still stands:** both arms of every pair had the same corpses, on the same
seed, so the paired survival difference is real. **What does not:** any reading
of *why* those colonies survived. "A trail buys survival at 90 cells" is a
statement about survival only — it is not evidence that anything was eaten at
the target, and the instrument that could have said so did not exist yet.

### 7.7 `deliveries` does not measure provisioning, and a claim was withdrawn on it

`CreatureStats::deliveries` increments on **any drop while `at_nest`**,
whatever was dropped and wherever it came from. With 52 ants dying, the nest
fills with corpses, and an ant picking one up and setting it down scores a
delivery.

**The disproof is in the sweep's own data: at gap 300, `near on` is 0 in all
six seeds** — not one ant ever came within 10 cells of the food — **yet
`deliv on` reads 0, 0, 6, 4, 13, 10.** Deliveries occur with zero visits to the
food source.

So the column cannot attribute provisioning, and *"ants reach the food and
bring almost nothing home"* — stated in a commit here — **is withdrawn**; it was
inferred from a contaminated counter. The observed "trail lowers deliveries in
21 of 24 rows" is most economically read as *the trail arm has fewer ants at
home to shuffle corpses*.

**What is needed instead is food removed from the target heap**, which
nest-local handling cannot fake. Built on 2026-09-16 — §7.8.

### 7.8 The larder is isolable, and two instrument traps on the way to it

The owner's question was whether corpses, and anything else that is not the
deliberately placed larder, can be made non-food, so that a colony can only eat
what is intended and intake can be attributed. Yes — `trailfollow onlyfood=on`,
now the default.

**Zeroing `food_energy` is not enough, and corpse is exactly the material it
fails for.** `creature::food_value` prefers the *cell's* `aux` stamp wherever
the material sets `worth_in_aux`, and corpse is the **only** material in the
tree that sets it. Clearing the energy alone leaves every stamped corpse in the
world worth precisely what it was worth before. The flag has to come off too,
which is the whole difference between isolating the larder and appearing to.

**Trap one: the first provisioning column was counting rot.** It was
`placed − still standing`, which looks principled, and it read **240/300 in all
four arms of a two-seed control** — `CLAUDE.md`'s tidiness signature exactly.
`windfall.ron` sets `decays_into: "soil"`, and `decay.rs` checks every 200 ticks
at a 0.05 chance once damp, so about 87% of a placement leaves on its own over
8,000 frames. **240 cells vanished while the colony ate five.** A dead colony
scored the same "intake" as a live one. Replaced with
`EnergyLedger::harvested_plant`, which rot cannot reach.

**Trap two, from the same measurement: a rotting larder cannot host a distance
sweep at all.** `fruit`, `seed` and `moss` are the only edible materials in the
tree with no `decays_into`. The default larder is now `fruit` — 960 face, the
same 240 J/cell at the neutral gut that windfall had, so a `food=` setting means
what it meant before.

The isolation carries its own check rather than an assertion: intake off
anything that is not the larder is printed as `other J` and must read 0. That
column is read from `ColonyBooks::diet()`, which books intake **by the material
it came out of**, so it names a leak rather than merely totalling one — a
strictly stronger check than the `EnergyLedger::harvested_corpse` reading it
replaced, which could only ever see the `worth_in_aux` branch.

**Put the fault back and it goes red** — 3 seeds, gap 90, 8,000 frames, 20 ants:

| arm | corpse J | ate J on | ate J off |
|---|---|---|---|
| `onlyfood=on` | **0, 0, 0** | 1,200 / 1,404 / 228 | **0, 0, 0** |
| `onlyfood=off` | **138, 342, 228** | 960 / 5,971 / 1,416 | 0, 0, 0 |

Two readings, both first-of-their-kind here and both small:

- **`ate J off` is zero in every seed while `ate J on` is not**, and the
  control that says what that means is now in the table. An exactly repeated
  zero is `CLAUDE.md`'s tidiness signature, and it had two readings wanting
  opposite conclusions: the no-trail colony reached the food and declined to
  eat, or it never got there. Adding the no-trail arm's own near-target count
  separates them:

  | seed | near on | near off | ratio | ate J on | ate J off |
  |---|---|---|---|---|---|
  | 1 | 7,674 | **162** | 47x | 1,200 | 0 |
  | 2 | 15,128 | **726** | 21x | 1,404 | 0 |
  | 3 | 20,712 | **858** | 24x | 228 | 0 |

  **`near off` is non-zero in all three**, so the zero is not arithmetic — those
  ants did come within ten cells of the larder and ate none of it. But it is
  not a refusal either: they are there **21–47x less often**, and at that
  exposure eating nothing is unremarkable. So the honest statement is narrower
  than "with a trail the colony eats and without one it does not": **the trail
  moves exposure, and intake follows exposure.** The exposure ratio is the
  cleaner measurement of the two, because it does not depend on an ant
  happening to bite during a short visit.
- **`home on` is 0 throughout.** Not one larder cell ever stood inside the nest
  band. The ants eat where they find it and bring nothing back, which is what
  §7.9's "go eat over there, rather than provisioning" reading predicted, now
  measured on a counter that nest-local handling cannot fake.

Intake also runs at roughly 1–8% of `supply J`, so these colonies are not
short of food — they are short of ants that reach it. That points at range and
discovery, not at scarcity, and agrees with A.2's `near on` collapsing to
exactly 0 by gap 300. **`supply J` is a nominal figure and not a ceiling**: the
gut is heritable (`ant.ron` `trait_variance` slot 0 = 0.15), so an individual
drifted toward the plant end draws more from a class −1.0 larder than the
authored 0.0 does. Read it as "roughly how much was put out".

#### The isolation found an engine bug in the diet band

Worth its own heading because the isolation is what surfaced it, and because
nothing else would have: `EnergyLedger` had the joules right the whole time.

With `onlyfood=on` the placed larder is the only food in the world, so the
per-material diet band must attribute **all** intake to it. It did not. At 52
ants over 12,000 frames it read:

```
diet: empty     45,007 J
diet: fruit     13,996 J
```

**76% of intake attributed to a material that cannot be eaten.** `empty` is
`MaterialId(0)` — it has no `food_energy`, so no animal can ever have bitten it.

The cause is three lines in `creature.rs`'s birth-provisioning path — the
shortfall loop where a parent that cannot afford a child eats the ground to pay
for one:

```rust
if !bite_outcome.survived() {
    world.set(px, py, Cell::EMPTY);        // erase the cell
}
let material = world.get(px, py).material; // ...then ask what it was
```

The material was read *after* the cell was cleared, so every meal this path
took was booked against `empty`. **The tell was already in the function**:
`banked`, which chooses the account, reads the same cell *before* the bite — so
the two reads disagreed with each other, and the sibling bite site in `act`
names this exact hazard in a comment ("`worth` is read before the roll because
the roll rewrites the cell"). This site did it for the worth and not for the
material.

Fixed by reading it once, before the bite, and handing the same value to both.
Attribution-only: the joules are identical across the fix — 228 + 29,474 →
29,702, and 13,996 + 45,007 → 59,003 — and `empty` is gone.

**What it changes here: nothing, and that is checked rather than assumed.** The
three-seed control above re-runs byte-identical after the fix, because at 20
ants over 8,000 frames these colonies barely reproduce and the birth path
scarcely fires. It matters for any bed where the colony is growing, and it
means **every "what are they eating" reading taken on a breeding colony before
2026-09-16 understated the real food and attributed the difference to nothing.**
`harvested_plant` was never wrong, so a total was always right; only the split
by material was.

### 7.10 Re-measured with the larder isolated — SUPERSEDED by §7.11, the colony was founded on the food

The §7.6 sweep re-run on the fixed instrument — `onlyfood=on`, larder `fruit`,
6 seeds, 24,000 frames, 52 ants, trail hand-laid to frame 6,000 then released.
`other J` reads **0 in all 24 rows**, so the isolation held throughout.

| gap | alive on | alive off | seeds up | ate J on | ate J off | seeds up | rows eating anything, on / off |
|---|---|---|---|---|---|---|---|
| 90 | 139 | **158** | 2 of 6 | 501,349 | **576,214** | 2 of 6 | 6/6 / 6/6 |
| **150** | **137.5** | 54.5 | **5 of 6** | **521,564** | 221,429 | **4 of 6** | 6/6 / 6/6 |
| **220** | 0 | 0 | 1 of 6 | **1,302** | **0** | **5 of 6** | **5/6 / 0/6** |
| 300 | 0 | 0 | — | 0 | 0 | — | 0/6 / 0/6 |

Medians over 6 seeds; `alive` is the end count.

**This is very nearly the inverse of §7.6**, which reported a benefit at 90 that
was gone by 150. Corrected:

- **At 90 the trail does nothing.** Both arms thrive, and the control is
  slightly ahead on both survival and intake. The food is close and abundant —
  `supply J` around 300,000 against a need near 46,800 — so there is nothing for
  a trail to rescue. **§7.6's effect was a property of a starving colony**, not
  of the trail: those colonies ran at ~90% of need on 30 J corpse cells and
  topped up by eating their own dead.
- **At 150 the trail is doing real work.** 5 of 6 seeds survive with it,
  against a control whose median colony is down to 54; intake median 521,564
  against 221,429. Three of the six control colonies end at 0 or 4 ants while
  their paired trail arm ends at 120–159.
- **At 220 the trail is the only thing that finds the food at all.** `ate J off`
  is **exactly 0 in all six seeds** and `near off` is 0 in four of them — the
  control never reaches the larder. `ate J on` is non-zero in **5 of 6**, and in
  seed 5 the trail arm alone survives outright: 141 ants and 302,348 J against
  a dead control. The absolute intake is tiny in the other four (1,188–4,585 J),
  so this is "a few ants got there and ate" rather than a working supply line —
  but against a control that is flatly zero, it is the cleanest trail effect
  measured anywhere in this work.
- **At 300 nothing happens either way.** `near on` is 0 in every seed, so the
  ants never walk the trail that was laid for them. That is a range limit
  upstream of anything in Phase 2.

**What is not established, and the reason is structural.** `alive` and `near`
are entangled: a colony that dies has no ants left to be near anything, so a
`near` difference partly *follows* survival rather than causing it. The gap-220
block is the one place that confound is absent — colonies die in both arms in 5
of 6 seeds, and the trail arm still ate while the control ate nothing, which
can only have happened before they died.

**Seed 1 runs against the grain at every gap** (trail arm collapses, control
thrives) and is the single biggest contributor to the gap-90 column. With 6
seeds that is one bad draw, not a subgroup; it is recorded rather than dropped.

### 7.11 On a scene that contains a journey: the trail is decisive, and the ants cannot build one

§7.10 is void at gaps 90 and 150, and the fault was the scene rather than the
mechanism. `World::colony_stations` walks **outward** from the cursor taking the
first column that is a site, so founders who do not fit on the left are placed
on the right. With the nest pinned at x = 40, ten of 52 founders fit to the
left at the `COLONY_ANT_SPACING` of 4 and **the other 42 marched out to x =
208** — past the larder at every gap below 220. Measured: `founded x 8..208,
food at 190`. The ants were born on the food, `arrive@` read frame **1**, and
all three arms scored alike because none of them had anywhere to go.

The nest now sits clear of its own colony, and the founders' real positions are
**asserted** clear of the food, so an invalid scene refuses to run. Re-run with
20 ants (colony x 12..88), 6 seeds, 24,000 frames, trail hand-laid to 6,000 then
released:

| gap | arm | alive (med) | colonies alive | ate J (med) | ants reaching food | seeds arriving |
|---|---|---|---|---|---|---|
| **90** | **hand** | **102** | **4 of 6** | **317,486** | **1,299 / 1,415 (92%)** | **6 of 6** |
| 90 | self | 0 | 0 of 6 | 0 | 4 / 120 (3%) | 3 of 6 |
| 90 | mute | 0 | 0 of 6 | 0 | 3 / 120 (3%) | 2 of 6 |
| 150 | hand | 0 | 1 of 6 | 0 | 446 / 562 (79%) | 5 of 6 |
| 150 | self / mute | 0 | 0 of 6 | 0 | **0 / 120** | 0 of 6 |
| 220 | hand | 0 | 0 of 6 | 0 | 2 / 121 | 1 of 6 |
| 220 | self / mute | 0 | 0 of 6 | 0 | 0 / 120 | 0 of 6 |

**Three findings, and the second is the one this investigation was named for.**

**1. A laid trail is decisive at 90 cells.** 92% of every ant that ever lived
reaches the larder, against 3% without one; four colonies of six survive and
grow to a median of 102 from 20 founders, against **zero of six** in both
control arms, which die in every seed with food 90 cells away untouched. This
is not a marginal effect and it does not need an order statistic to see.

**2. The ants cannot bootstrap a trail of their own.** `self` — which lays and
reads channel B with the shipped `(Carrying, EmitB, 2.5)` wire and the re-gated
units 2/3 — is **indistinguishable from `mute`**, which cannot lay at all:
4 visitors against 3, zero survivors either way, zero joules either way, and at
gap 150 both read a flat **0 of 120 ants**. The colony's own stigmergy
contributes nothing. Whatever the re-gated reader can do, it can only do with a
trail somebody else laid.

**3. Even the arm that works does not provision the nest.** Across the whole
gap-90 `hand` block, **0.14%** of carrying happens inside the nest band — 1,796
ant-ticks of 1,275,776 — and the colony completes **7 round trips in six runs**.
With `reach` putting most ants past the food, the honest reading is that the
trail causes **relocation, not commuting**: it moves the colony to the larder,
and the colony then lives there. The owner's loop — out, pick up, carry home,
reinforce — does not occur in any arm at any gap.

**Range.** Decisive at 90; at 150 the trail still *delivers* ants (79% reach the
food, 5 of 6 seeds) but only one colony in six survives, so arrival stops being
enough; by 220 nothing arrives in any arm. Between 90 and 150 the limit is not
finding the food, it is what the journey costs.

**What this does not settle.** `hand` lays until frame 6,000 and colonies then
live to 24,000, and those ants can lay channel B themselves — so whether the
seeded trail is *maintained* or merely started a migration is not separated
here. `trips` of 7 and `carry@nest` of 0.14% point at migration. Six seeds.

### 7.12 Homing is upstream of the whole trail question, and congestion is not the fault

Two questions were put at 12 seeds: does the blocking rule cost more than it
buys, and — the owner's observation — is channel B failing *because* laden ants
never get home?

#### The homing dependency, confirmed

Channel B is laid only by laden ants. So if a laden ant does not walk home, it
draws a **wander-field rather than a route**, and no follower can climb it. That
predicts `self` ≈ `mute` for a reason that has nothing to do with the reader.

`carry->nest` is signed cells moved while holding larder, positive = homeward:

| gap | arm | net homeward cells | carrying ant-ticks | per 1,000 carried |
|---|---|---|---|---|
| 90 | hand | **−418** | 2,146,526 | **−0.19** |
| 150 | hand | −254 | 406,418 | −0.62 |
| 220 | hand | −550 | 409,998 | −1.34 |

**In the one arm where foraging plainly works, across 2.1 million ant-ticks of
carrying, net homeward displacement is negative.** Not weak — absent, with a
slight drift *away*. That sample is far too large for noise, and it agrees with
A.1's homing sweep, where shipped, `noemit` and `nosteer` do not separate at all.

**The consequence reaches past this report and into the plan.** §2.1 proposed
giving channel B a food-ward ramp off a unit-7 odometer. But *any* route-drawn
signal needs the drawer to walk a route: an ant that wanders while laying draws
the ramp on the wander. **Reshaping what charges the emitter cannot produce a
trail while homing is dead.** Homing is not a sibling task to the trail work, it
is upstream of it.

#### Pass-through: the blocking rule is expensive and is not the fault

`CreatureDef::passes_through_kin` (new, off by default, with a lab toggle) lets
a blocked body trade places with a nestmate — the pass-through half of dead ends
775/829, which `climbs_over_kin`'s own doc records as untested. It is a **swap**,
because a cell holds one material and one `organism_id`, so co-occupancy is not
representable; a swap also keeps the owner's reason for the rule, since two ants
are never drawn in one pixel.

It does what it was built to do. Blocked moves, 12 seeds: gap 90 `hand`
**44,496 → 4,338 (−90%)**, gap 150 −72%, gap 220 −96%.

It does not buy foraging, and it costs the arm that works:

| gap 90 `hand` | off | on |
|---|---|---|
| intake, summed over 12 seeds | **2,681,502 J** | **1,743,676 J** (−35%) |
| ant-ticks in the food band | 8,339 | 5,055 (−39%) |
| alive, median | 92 | 4 |
| colonies surviving | 7/12 | 6/12 |

Ants that can pass each other **disperse instead of following**.

The one thing it buys is an occasional breakthrough, and the distribution is
brutal. The no-trail arms reach food far more often (`self` 3.7% → 47.9%, `mute`
7.7% → 58.4%) and their intake sums jump from 1,200 to 267,116 J and from 9,603
to 455,311 J — but **"seeds with any intake" stays at 1 of 12 in both.** One
colony in twelve produces all of it. And **`mute`, which has no pheromones at
all, out-earns `self`**, so this is wandering luck rather than a trail being
unlocked.

**Verdict: congestion is a real tax and not the broken thing.** Removing it costs
the arm that works and does not rescue the arms that do not.

#### Two instrument faults found on the way, both mine

**The swap froze every ant it displaced, and the first 12-seed run was that bug.**
`creature_tick`'s guard says it outright — *"the active site sits on the head"* —
and a tick arriving to find a stranger's cell reconciles and returns no site:
*"not dead, not scheduled, just an orphan standing in the world forever."* The
mover was fine; the animal swapped *out* was not. Frozen ants were never
charged, never starved, and read as colonies surviving 24,000 frames on **zero
food** — with a uniform 12/12 survival across nine arm/gap combinations, the
tidiness signature. `ticks` per 1,000 ant-frames against the 167 a 6-frame
interval implies:

| arm | off | on, before fix | on, after fix |
|---|---|---|---|
| hand | 172 | — | 172 |
| self | 172 | **42** | 170 |
| mute | 173 | **24** | 171 |

Fixed by scheduling a fresh site for the displaced animal. The arithmetic tell
was available without any of this: 20 founders hold 4,000 J and idling alone
costs ~400 J each over the run, so survival on no intake is impossible.

**`route pk` was reading our own dying trail as maintenance.** It returned the
full route length — 88 at gap 90, 146 at 150, 213 at 220 — in **12 of 12 seeds at
every gap**, including gap 220 where eleven colonies of twelve die and only two
seeds ever land an ant on the food. A full-route trail in a seed where nobody
walked the route is not the ants'. The sampling margin was mis-derived: `stop +
1500` was set against the ~1,476-frame lifetime of a cell laid **once** at
`DEPOSIT`, but `lay` re-deposits every `relay` frames for the whole of `stop` —
about a hundred times — so those cells saturate far above a single deposit. The
repair is a **`hmute` decay baseline** (our ramp laid, the ants silenced) rather
than a longer window guessed at: whatever `hand` holds above `hmute` is the
colony's own.

What survives the correction is the one column with a working control: `self`
holds about **3.4 route cells of 90, in 2 seeds of 12**, against `mute`'s clean
**0 of 12**. The few ants that find food do lay — and it is not a trail.

### 7.13 Homing is necessary and not sufficient: the binding constraint is discovery

Five arms, 12 seeds, 24,000 frames, larder isolated. `hmute` lays our ramp with
the ants' `EmitB` zeroed — the decay baseline. `homeA` lays no food trail and
instead supplies a **channel A homing ramp**, testing whether a colony that can
find its way home can then build a trail of its own.

| gap | arm | survived | intake, summed | seeds with intake | reached food | route pk |
|---|---|---|---|---|---|---|
| 90 | hand | **7/12** | **2,681,502** | 11/12 | **91.4%** | 88.0 |
| 90 | hmute | 3/12 | 1,455,113 | 12/12 | **85.4%** | 88.0 |
| 90 | self | 0/12 | 1,200 | 1/12 | 3.7% | 3.4 |
| 90 | mute | 0/12 | 9,603 | 3/12 | 7.7% | 0.0 |
| 90 | **homeA** | **0/12** | 3,608 | 1/12 | **4.1%** | 8.9 |
| 150 | hand | 2/12 | 661,342 | 4/12 | 67.4% | 146.0 |
| 150 | hmute | 3/12 | 820,982 | 4/12 | 73.9% | 146.0 |
| 150 | self / mute | 0/12 | 0 | 0/12 | 0.0% | 1.5 / 0.0 |
| 150 | **homeA** | **0/12** | **0** | 0/12 | **0.0%** | 4.5 |
| 220 | hand | 1/12 | 547,934 | 2/12 | 62.6% | 213.0 |
| 220 | hmute | 1/12 | 489,211 | 2/12 | 61.8% | 213.0 |
| 220 | **homeA** | **0/12** | **0** | 0/12 | **0.0%** | 1.6 |

**1. `homeA` fails, so homing is necessary and not sufficient.** Handing the
colony the homing gradient it cannot build — the exact thing §7.12 showed
missing — changes nothing that matters: 0 of 12 colonies survive at every gap,
**0.0% of ants reach food at 150 and 220**, and 4.1% at 90 against `self`'s 3.7%
and `mute`'s 7.7%. The causal chain in §7.12 is real, and repairing its first
link on its own buys nothing.

**2. The binding constraint is discovery, and the circularity is now measured
rather than argued.** Every arm without a laid trail reaches food at **0–8%**;
every arm with one reaches at **62–91%**. The colony cannot lay a trail to food
it has never found, and it cannot find food without a trail. That is the loop
this plan opened by naming — *"to get a naturally-laid trail you need ants to
commute; to get them to commute you need a trail worth following"* — and it is
now a measurement rather than a premise.

**3. `route pk` is identical in `hand` and `hmute` at every gap** — 88.0/88.0,
146.0/146.0, 213.0/213.0. The baseline arm cannot lay at all, so that column is
**entirely our own decaying deposit**. Maintenance is zero everywhere, and the
reading that suggested otherwise was the instrument.

**4. The one place the colony's own laying adds anything is gap 90.** `hand`
beats `hmute` there on survival, **7/12 against 3/12**, and on intake, 1.8x.
That is the only evidence of autocatalysis anywhere in this work — the ants
reinforcing a trail they were given — and it **vanishes at 150 and 220**, where
the two arms are indistinguishable on every column. Twelve seeds, so a 7-vs-3
split is suggestive rather than settled; the intake ratio rests on which
colonies happened to live.

**What this means for the plan.** §2.1 and §2.2 reshape what *lays* channel B.
Nothing in this table is limited by what lays it: `hmute`, which lays nothing at
all, matches `hand` at two gaps of three. **The work that would move these
numbers is whatever lets a scout find food 90+ cells out** — range, exploration,
or a scene where food is discoverable — and that is not a pheromone mechanism.
Phase 2 should not be built against this evidence.

### 7.14 Why `homeA` clipped exploration: `Carrying` is true for dig spoil

§7.13 left one thing unexplained. Supplying the homing gradient made
exploration **worse** — `homeA` lost to `mute` on 10 of 12 seeds, put **zero
ants of 240** past the food, and more than doubled occupancy in the band beside
the nest (392 vs 174). A homing signal that reduces range is not obviously
wrong — homing *means* "go back" — but the size of it wanted explaining.

**The first hypothesis was a leaking gate, and the control that already existed
killed it.** Units 0/1 read `PheroAAlong` gated on `Carrying`; the pair is a
mirror (+6 and −6 on `along`) and cancels exactly only if `squash` is linear,
which it is not. `onetrail mode=arith` has carried the row for this since it was
written, labelled *the specificity control: the pair that is shut in this state
must not respond, or the gate is not a gate*:

```
empty, channel A [gate shut] |  0.200  0.200  0.200  0.200  0.201  0.202  0.205  0.209
laden, channel A (units 0/1) |  0.200  0.277  0.396  0.486  0.641  0.740  0.799  0.819
                       along |   0.00   0.01   0.03   0.05   0.10   0.20   0.50   1.00
```

An empty ant moves 0.200 → 0.209 across the **whole** range, and real trails
read `along` ≈ 0.026 where the residual is ~0.0005. The gate is a gate.

**The row above it is the answer.** A *laden* ant on a channel A gradient runs
at **0.641 against a 0.200 baseline** at a modest `along` of 0.10. So the
question is not whether the gate leaks but what is opening it — and
`creature.rs`'s `sense` says:

```rust
inputs[I::Carrying as usize] = crop_fill.max(state.spoil.map_or(0.0, |_| 1.0));
```

**`Carrying` is 1.0 whenever the ant holds dig spoil.** Measured with a new
`Carrying: laden / of which SPOIL` column, gap 90:

| arm | laden ticks | of which spoil |
|---|---|---|
| self | 19,998 | **100.0%** |
| mute | 20,448 | **100.0%** |
| homeA | 8,448 | **100.0%** |
| hand | 116,232 | 3.2% |
| hmute | 661,875 | 8.8% |

In every arm that never finds food, `Carrying` is **entirely spoil**. So in
`homeA` every tick that opens the homing gate is a tailings haul, and a digging
colony handed a standing homing ramp is railroaded home by its own spoil. That
is the clipping.

**It is the same flaw §2.0 already recorded from the other side**:
`(Carrying, EmitB, 2.5)` fires for spoil too, so tailings are written onto the
plane a forager reads as a route. One sensor, three consumers, and only one of
them wants the honest answer:

| consumer | wants |
|---|---|
| `(Carrying, Drop, 0.2)` | "are my mandibles full" — the shipped meaning, and correct |
| units 0/1, the channel A homing gate | "am I carrying **food**" |
| `(Carrying, EmitB, 2.5)`, the channel B emitter | "am I carrying **food**" |

#### What removing spoil from `Carrying` buys: one seed in twelve

`SPOIL_IS_CARGO=0` takes the pellet out. Paired, 12 seeds, gaps 90 and 150, all
five arms — and **the switch is neutral everywhere except the arm the hypothesis
named**: `hand` 7/12 → 5/12 and `hmute` 3/12 → 5/12 survive (noise in opposite
directions), `self` and `mute` unchanged, and all of gap 150 unchanged.

At gap 90, `homeA`:

| | shipped | `SPOIL_IS_CARGO=0` |
|---|---|---|
| seeds with any ant reaching food | 7/12 | 9/12 |
| seeds with a surviving colony | 0/12 | **1/12** |
| seed 8 alone | 0 alive, 3,608 J, 4/23 reached | **159 alive, 565,010 J, 421/443 reached** |

**The pooled "reach 4.1% → 65.5%" is not a 16x effect and must not be quoted as
one.** It is seed 8, and the percentage is inflated by its own success: the
denominator is *ants that ever lived*, so a colony that finds food explodes to
443 ants who then mostly stand at the food. The eleven other seeds did not move.

What looked genuinely notable was qualitative: **seed 8 is the first colony
anywhere in this investigation to build a working forage loop with no hand-laid
food trail.** But one occurrence is the same 1-in-12 shape as §7.12's
pass-through breakthrough, so it bought a replication rather than a design
change.

#### The replication: it does not reproduce

24 fresh seeds (13–36), `homeA` only, gap 90, paired:

| | surviving colonies | seeds with any intake | total J |
|---|---|---|---|
| shipped | **0/24** | 3/24 | 11,261 |
| `SPOIL_IS_CARGO=0` | **0/24** | 6/24 | 9,475 |

**Zero breakthroughs in either arm.** Seed 8 was a lottery win. The only signal
left is that more seeds *touch* food with the switch on — 3 → 6 fresh, 4 → 7
over all 36 — and it comes with **less total energy**, so it is more colonies
nibbling rather than more colonies succeeding.

**The prediction recorded before the run is falsified.** It said `homeA`'s
past-halfway share would rise toward `mute`'s 23% and the far reach bucket would
stop reading zero. On survival it did neither.

**That verdict was "the confound is real and fixing it is not worth 24 brain
slots", and it is WITHDRAWN — the survival result above is true and the
conclusion drawn from it was not.**

#### Withdrawn: the repair does work, on an endpoint that is not a lottery

The owner's objection, 2026-09-17: *"You say these are not the fault but I don't
think that is correct. cargo sensing seems like a big issue but it is only 1
part of the multi issue problem."*

**The inference was invalid.** "Removing spoil did not rescue a colony" was read
as "cargo sensing is not the fault". In a system where several necessary
conditions are broken, repairing one changes no *outcome* until the others are
repaired too. And the outcome used — colony survival — is decided by a discovery
event with a base rate near **1 in 36**, so it cannot detect a change in any one
condition at affordable sample sizes. §7.13 had already established that
discovery is the binding constraint and this section failed to apply it.

**Re-analysed on an endpoint that needs no discovery at all**, from the same
12-seed paired data, gap 90. Exploration — the share of ants that ever get past
halfway to the food — is continuous, requires nobody to win anything, and moves
at n=12:

| arm | shipped | `SPOIL_IS_CARGO=0` | paired sign test |
|---|---|---|---|
| **homeA** | **15.0%** | **27.5%** | **up in 10 of 12 seeds, down in 0** (p ≈ 0.002) |
| homeA, seeds reaching past the food | **0/12** | **2/12** | the hard zero is gone |
| `self` | 15.0% | 22.5% | — |
| **`mute`** (no channel A ramp) | 21.9% | 20.0% | **unchanged — the specificity control** |
| `hand` | 95.3% | 79.7% | already at ceiling; see below |

**`mute` is what makes this a mechanism rather than a correlation.** It has no
homing ramp, so a spoil-opened gate has nothing to railroad it along, and it does
not move. `homeA` — the one arm with a standing ramp — moves in 10 of 12 seeds
with **zero reversals**. That is the predicted effect, in the predicted arm,
absent from the control.

**`hand` moves the other way (95.3% → 79.7%) and that is reported rather than
buried.** With a hand-laid food trail the ants are already at ceiling on this
measure, so it is not an arm where a homing confound can show; its survival is
a clean null too (7/12 against 5/12, and a **6/12** sign split on intake).

**Standing verdict: cargo sensing is a real defect with a measured,
mechanism-specific effect on exploration, and it is one of several broken
conditions rather than the single fault.** Whether it is worth 24 brain slots is
a separate question that survival cannot answer and exploration alone does not
settle. `SPOIL_IS_CARGO` remains a measurement switch defaulted to the shipped
behaviour, because the shipped behaviour is itself a measured fix (`Drop`).

#### Pre-registered, before the 50-seed confirmation

Recorded here before the run, because three separate single-seed results in this
investigation were rationalised after the fact:

- **`homeA`'s past-halfway share rises**, sign test at least 2:1 up over down.
- **`mute` does not move.** If it moves as much as `homeA`, the mechanism story
  is wrong and the switch is doing something non-specific to digging.
- **Survival and intake stay flat.** That is expected and is *not* a failure of
  the hypothesis — it is the multi-condition point. Movement there would be a
  bonus, not the test.
- Read the **paired sign test**, never the pooled share: a pooled percentage is
  inflated by any colony that succeeds and grows its own denominator, which is
  the artifact behind the withdrawn "16x reach" figure above.

#### The 50-seed confirmation: every prediction held, including the falsifier

Gap 90, 50 seeds, paired, `RAYON_NUM_THREADS` pinned.

**Primary endpoint — past-halfway share, paired sign test:**

| arm | shipped | `SPOIL_IS_CARGO=0` | up / down | ratio | predicted |
|---|---|---|---|---|---|
| **homeA** | 15.0% | **25.0%** | **34 / 8** | **4.2:1** | rises ≥2:1 ✓ |
| `self` | 20.0% | 25.0% | 35 / 15 | 2.3:1 | between ✓ |
| **`mute`** | 24.4% | 20.0% | 20 / 23 | **0.9:1** | **does not move ✓** |

34-up against 8-down is a sign test far below p = 0.001, and **`mute` — the
falsifier — stayed flat.** An arm with no channel A ramp has nothing for a
spoil-opened gate to be railroaded along, and it did not move. `self` lands in
between because its ants lay their own weak channel A. The effect is
mechanism-specific, not a general consequence of changing what digging ants do.

**Downstream, pre-registered as expected-flat, and flat:**

| arm | surviving | intake | seeds reaching past the food |
|---|---|---|---|
| homeA | 2/50 → **2/50** | 307,774 → 986,908 | 7/50 → **11/50** |
| self | 2/50 → **2/50** | 688,132 → 876,226 | 10/50 → **13/50** |
| mute | 2/50 → **1/50** | 730,437 → 275,756 | 17/50 → **14/50** |

Survival does not move in any arm, exactly as recorded beforehand. The
"seeds reaching past the food" column is the robust ordinal version of the
primary endpoint and agrees with it: **homeA and self up, mute down.**

#### And in the arm where discovery is already solved, it changes nothing

The owner's second ask: run `hand` with cargo sensing on. **`hand` is the one
arm where the discovery lottery is removed** — ~88% of ants reach the food
because a trail is laid for them — so it is the only place the repair can be
measured on the *loop* rather than on whether a colony got lucky. 50 seeds:

| endpoint | shipped | `SPOIL_IS_CARGO=0` | up / down |
|---|---|---|---|
| colonies alive | 17/50 | 18/50 | 18 / 11 |
| intake J | 7,073,527 | 7,821,647 | 27 / 20 |
| carrying ticks | 6,194,128 | 6,644,680 | 27 / 23 |
| **carried home** | **7,129** | **4,541** | 12 / 16 |
| **round trips** | **24** | **16** | 6 / 11 |
| past-halfway | 76% | 80% | 29 / 20 |
| ants reaching food | 87.9% | 89.0% | — |

**Flat on everything, and transport if anything falls** — carried-home share
0.115% → 0.068%, round trips 24 → 16. A well-powered null at n = 50.

#### What the two results say together

The confound has exactly one consequence, and it is now pinned:

> **A spoil-laden ant is pulled along whatever channel A gradient exists.**

- Where a gradient exists and **discovery is not solved** (`homeA`, `self`),
  that clips exploration, and removing spoil releases it — 4.2:1 and 2.3:1.
- Where there is **no gradient** (`mute`), there is nothing to be pulled along
  and nothing changes — 0.9:1.
- Where **discovery is already solved** (`hand`), channel A is not what is
  steering behaviour, so the confound is irrelevant — flat on every endpoint.
- **It never touches transport.** Carried-home and round trips do not improve in
  any arm, at any n. The return leg fails for an independent reason: laden
  movement has **no homeward component at all** (§7.12, −418 net cells over
  2,146,526 carrying ticks).

**So cargo sensing is a real defect with a measured, mechanism-specific effect,
and it is one broken condition among at least three** — cargo gating, homing,
and discovery. Repairing it moves the endpoint it touches and nothing
downstream, which is precisely why reading it through colony survival produced
the withdrawn "not the fault" verdict. That is the general lesson: **in a
multi-condition failure, a downstream endpoint cannot price an upstream
repair**, and choosing one that can is the whole of the experiment.

**Method note, because it cost a wrong headline and nearly a second one.** The
replication was launched as `seed0=13 seeds=36` to be independent of seed 8. The
parser read `arg("seed")` while the header *printed* `seed0=`, so the flag was
silently ignored and the run re-executed seeds 1–36 — returning the very seed it
was built to exclude. Salvageable only because seeds 13–36 were fresh anyway.
Third instance of `CLAUDE.md`'s *"an unknown argument is silently ignored"* in
one session, after `gaps=`; **a knob whose echoed name is not its accepted name
is worse than an unknown one, because the header reads as confirmation.** Fixed
to accept both.

#### The design fork, and why the switch is not the answer

**`SPOIL_IS_CARGO=0` is a measurement switch and must not ship.** The shipped
behaviour is itself a *measured* fix: with `Carrying` reading the crop alone the
`Drop` gene is asked a question about food, and `labnest` read **30–35 of 52
ants standing laden with `digs` 121 against 881** — a colony that looked from
outside like it had stopped digging.

So the repair, if the replication justifies one, is to let the ant **tell food
from spoil**, leaving `Carrying` honest for `Drop`. Two cheaper routes were
considered and rejected:

- **Grading `Carrying`** (spoil 0.5, food `crop_fill`) needs no slot and is
  wrong: it overloads one channel with two meanings, and a weight tuned against
  it reads a *code* rather than a quantity. `CLAUDE.md` is explicit — when a
  rule must tell apart two things that can look identical, state the difference
  as **data**.
- **Gating `EmitB` on food inside `creature.rs`** authors "channel B is the food
  channel" in Rust, which is the objection
  `creature-genome-flexibility-2026-09-02.md` §2b raised against the old
  hardcoded odometer. No channel has a meaning the engine knows, and that is the
  point of the design.

The codebase's own idiom supports the appended sense: `ant.ron` already carries
**`(Carrying, DropSpoil, 0.2)` beside `(Carrying, Drop, 0.2)`**, so food-cargo
and spoil-cargo are already different things — the engine just resolves it at
the *output* side, where separate verbs exist. The pheromone mechanisms have one
`EmitB` and one channel A gate, so nothing resolves it for them.

**The price, named:** an appended `BrainInput` is **24 live slots** (16 outputs +
8 hidden), `live_slots` 870 → 894, `mutation_rate` re-derived in **all thirteen**
species files that wire `Carrying`, and every breeding baseline void, since
`brain::mutate` draws one value per live slot. That is the same bill §2.2 priced
for `FoodAmount`, and it is why the replication comes first.

### 7.15 Why the return leg fails: the homing ramp inverts the moment a colony forages

> **This section replicates, and §7.17 adds the condition it was missing.** The
> inversion needs a colony foraging against a **replenishing** larder
> (`trailfollow refill=`); on the one-shot default the colony starves at six ants
> and the question cannot be posed. Re-measured on the committed tree at three
> refill settings, every foraging colony's ramp points at the food — 4/4, 7/7,
> 3/3, r = +0.69 to +0.85. **What is wrong below is the mechanism, not the
> result**: step 1 derives the odometer's decay from `w_rec` alone and concludes
> the per-ant charge is flat. It is not — it falls 78% across a 141-tick trip
> (§7.16). The plane inverts because it **integrates traffic**.


§7.12 measured laden ants having **no homeward component at all** — −418 net
cells over 2,146,526 carrying ticks — and left the cause open. This is the cause,
and it is not that the ants ignore channel A. **They follow it, and it points at
the food.**

#### The arithmetic that started it, and why it was not the answer

Channel A is laid by hidden unit 4, an odometer charged by `AtNest` and decaying
at `recurrence 0.99995`, explicitly fitted for a **3,000-tick** decay
(`ant.ron`: "0.992 → 0.072 across 3,000 ticks"). The journeys here are nowhere
near that long:

| condition | 90-cell trip | share of odometer range | charge decays to |
|---|---|---|---|
| fed, P(move) 0.20 | 450 ticks | 15% | 0.978 |
| on a trail, P(move) 0.64 | 141 ticks | **4.7%** | **0.993** |

Over the reader's own 6-cell sensor offset the charge differs by **0.0005–0.0015**,
against the `along` ≈ 0.05 a laden ant needs to lift `P(move)` off its 0.200
baseline. So the *intended* ramp — strong near home, faint far away — is
essentially flat at play distances. **That is true and it is not the failure.**

#### What is actually on the plane

Measured on the route, gap 90, 12 seeds, as a **running peak** — an end-of-run
sample reads 0 for any colony whose plane has decayed, which is the same error
`route pk` made for channel B and which this measurement made first time round:

| | |
|---|---|
| peak channel A on the route | **10,269 – 54,311** |
| route cells ever holding any | **55 – 91 of 91** |

`DEPOSIT` is 10,240, so cells reach *past a full deposit*, accumulated across
ants. **Channel A is laid abundantly.** The first reading of this section said it
was "barely laid at all" and that was the end-of-run artifact, caught before
publication.

#### And it points the wrong way, in exactly the colonies that forage

Polarity averaged over every sample that had a trail to measure. **Positive =
taller at the nest**, the shape homing needs; negative = taller at the food.

| colony behaviour | `AtNest` share | n | channel A polarity |
|---|---|---|---|
| **stays home** | 14.0 – 18.5% | 5 | **positive in 5 of 5**, mean **+0.0375** |
| **forages** | 0.4 – 1.9% | 7 | **negative in 6 of 7**, mean **−0.0704** |

Correlation between `AtNest` share and polarity: **r = +0.77 (n = 12)**. The
inverted magnitudes (−0.061 to −0.158) are comfortably above the ~0.05 the
reader acts on, so this is not a faint wrong-way bias — it is a readable signal
pointing away from home.

#### The mechanism, end to end

1. ~~The odometer's decay is **~20–60× too slow** to grade a 90-cell journey, so an
   ant lays channel A at nearly constant strength wherever it goes.~~
   **WRONG — see §7.16.** Measured against the engine's own odometer readout, the
   shipped emission runs **0.819 → 0.177 across a 141-tick trip, a 78% fall**.
   The per-ant charge grades steeply. The correct step 1 is that the *plane* sums
   every visit, and a foraging colony's visits are ~21:1 in the food end's favour
   against a per-visit strength ratio of 4.6:1.
2. A plane laid at constant strength records **where ants spent time**, not how
   far they are from home.
3. A foraging colony spends its time **at the food** — `AtNest` falls from ~15%
   to ~1.5% precisely when foraging starts.
4. So channel A accumulates at the food end and the ramp **inverts**.
5. A laden ant ascending channel A is therefore driven **toward the food**.

**The homing signal is self-defeating: the better a colony forages, the more
firmly its own homing ramp points away from home.** That is the −418 explained —
laden ants are not failing to follow the homing channel, they are following it
faithfully in the wrong direction.

**It is the same structural failure as §1c's channel B age ramp**, in a different
costume: in both cases the plane's shape is set by *laying behaviour* rather than
by the quantity the design intended to encode, and no deposit value or decay rate
fixes it, because the shape follows from *when and where* cells were written.

**What this does not settle.** The 12 seeds are one gap and one colony size, and
the homebound/foraging split is observational rather than assigned — foraging
colonies differ from homebound ones in more than their `AtNest` share. The
prediction it makes is sharp and cheap to test, though: an odometer whose decay
is scaled to the actual journey (tens of ticks, not thousands) should keep the
ramp nest-ward *while* the colony forages, and that is one weight.

### 7.16 The odometer's decay is not the lever, and finding that out corrected §7.15 — re-run at a known scene in §7.18

§7.15 closed on a sharp, cheap prediction: *"an odometer whose decay is scaled to
the actual journey should keep the ramp nest-ward while the colony forages, and
that is one weight."* That weight was swept. **The prediction is false**, and why
it is false replaces §7.15's mechanism with a better one.

#### What was swept

`examples/trailfollow` gained a `recur=` rider patching `brain::hh_slot(4)` on the
cloned genome, beside the existing `Gate::apply` seam. Gap 90, 12 seeds,
`arms=hand` (the arm where discovery is already solved, so the return leg is what
is being measured), 24,000 frames, `RAYON_NUM_THREADS` pinned.

Half-lives were chosen to bracket the 141–450-tick journey: shipped `0.99995`
(~13,900 ticks), then `0.999` (~693), `0.995` (~138), `0.99` (~69), `0.98` (~34).

**The rider's own controls ran first**, because a knob that changes nothing is
indistinguishable from one that is not connected: `recur=0.99995` is *refused* by
an assert as a no-op, which is what proves the slot is the one claimed.

#### The result: no window, and no collapse either

Polarity over the foraging colonies (`AtNest` < 5%), which are the ones the
section is about. Positive = taller at the nest.

| `recur` | half-life | n foraging | negative | mean polarity | peak range | r(`AtNest`, polarity) |
|---|---|---|---|---|---|---|
| **0.99995** (shipped) | ~13,900 | 7 | 6/7 | **−0.0704** | 10,269–54,311 | +0.77 |
| 0.999 | ~693 | 5 | 3/5 | −0.0478 | 10,263–35,325 | +0.69 |
| 0.995 | ~138 | 6 | 5/6 | −0.0692 | 9,925–34,380 | +0.83 |
| 0.99 | ~69 | 4 | 2/4 | −0.0219 | 8,254–17,122 | +0.56 |
| 0.98 | ~34 | 7 | 4/7 | −0.0372 | 8,233–22,760 | +0.67 |

Against the three predictions recorded before the run:

- **"Polarity rises toward positive, monotone, positive by 0.995 or sooner"** —
  false on both counts. It is not monotone (0.995 is *worse* than 0.999) and no
  arm's foraging mean is positive. Paired on the seeds that forage in **both**
  arms, the shift is real but small and noisy: +0.016, +0.037, +0.030, +0.067,
  up in 3/5, 4/5, 3/4, 4/5.
- **"`a_peak_amt` falls — the counter-risk"** — false, and this is the useful
  negative. The peak *floor* moves 10,269 → 8,233, a 20% fall against a `DEPOSIT`
  of 10,240. **The signal-collapse trade this sweep was built around does not
  exist at these settings**, so "there may be no window" is not what happened
  either: there is plenty of headroom and the polarity simply does not follow.
- **"Transport is not expected to follow"** — held. `trips` and `carry→nest` stay
  where §7.13 puts them.

**The tell was tidiness in the wrong place.** `r(AtNest, polarity)` stays between
**+0.56 and +0.83 in all five arms**. A 400× change in the decay weight left the
correlation §7.15 is built on entirely intact — which is not a weak effect, it is
a knob that is not attached to the quantity.

#### Why: `w_rec` is not what sets this odometer's decay

It is not attached because `brain.rs` says so, in the fitting test's own comment,
and §7.15 did not read it:

> **The decay is dominated by `squash`, not by `w_rec`** […] `eval_brain` computes
> `h = squash(w_rec * h)`, and `squash` compresses: at `h = 0.08` it takes about
> 8% off every tick, swamping a `w_rec` of 0.9999.

§7.15 computed the per-trip decay as `0.99995^141` = **0.7%** and concluded the
charge is flat over a journey. Simulating the real recurrence
(`h ← squash(w_rec·h + w_in·AtNest)`, `emit ← squash(w_out·h + bias).clamp(0,1)`):

| config | t=0 | t=70 | **t=141** | t=300 | t=2999 | fall over a 141-tick trip |
|---|---|---|---|---|---|---|
| **shipped** (`w_out 32`, no bias) | 0.819 | 0.293 | **0.177** | 0.094 | 0.010 | **78.4%** |
| `recur` 0.99 | 0.816 | 0.217 | 0.086 | 0.015 | 0.000 | 89.4% |
| `recur` 0.98 | 0.813 | 0.149 | 0.033 | 0.001 | 0.000 | 95.9% |

**Positive control**: the same simulator at the fitting test's chosen weights
(`w_in 0.05, w_rec 0.99995, w_out 900, bias −0.2`, 5-tick touch) returns
`0.992 → 0.072`, `t1000 = 0.402` — and the engine's own readout
(`cargo test --release --lib -- --ignored --nocapture what_an_odometer_emits`)
prints `emit 0.992 -> 0.072 (t=1000 0.402, t=2000 0.185)`. Three decimals on
three points, so the simulator is reproducing `eval_brain` and not a model of it.

**So the odometer already grades steeply**, and the sweep was turning the one
weight in the unit that barely moves the thing it names.

#### The corrected mechanism: the plane integrates traffic

The per-ant deposit is graded. The **plane** is a sum over every visit, and that
is where the shape is decided:

| | nest end | food end | ratio |
|---|---|---|---|
| ant-ticks (`occupancy/1k`, a foraging colony) | 30 | **627** | **~21 : 1** |
| per-visit emission (shipped, t=0 vs t=141) | 0.819 | 0.177 | 4.6 : 1 |

**Traffic beats grading by about 4.5×, and the ramp inverts.** Every step of
§7.15's chain survives except the first, and the closing prediction fails for a
reason the table makes obvious: at `recur` 0.98 grading reaches 25:1 against
traffic's 21:1, which is *exactly why 0.98 came closest to flipping* — mean
−0.037 against the shipped −0.070 — and still did not, because a ratio that
barely exceeds the traffic ratio only cancels it.

#### A documentation defect found underneath it, and it is load-bearing

Hidden unit 4 ships **three weights and no bias**: `(AtNest, 4, 0.05)`,
`(4, 0.99995)`, `(4, EmitA, 32.0)`.

- **`creature.rs` names a different set as the shipped one.** Its odometer comment
  says *"the weights `ant.ron` actually authors — `w_in = 0.0005`, `w_rec = 0.9999`,
  `w_out = 1609.1` and a `-0.35` bias"*. That is the **dead pre-fix** version;
  `w_in = 0.0005` is below `W_EPS` and the fitting test's own first row labels it
  `authored (dead: w_in < W_EPS)`, emitting `0.000 -> 0.000`. The comment names
  this exact failure mode two paragraphs later, about a *previous* set of numbers.
- **`ant.ron` quotes a curve its own weights do not produce.** Beside
  `(AtNest, 4, 0.05)` it records *"Emits 0.992 → 0.072 across 3,000 ticks after a
  five-tick nest touch … rms 0.084 … Saturated for 85 of those 3,000 ticks."* That
  is verbatim the fitting test's chosen fit at **`w_out 900, bias −0.2`**. The
  weights it sits next to give **0.819 → 0.010, saturated 0 ticks**. §7.15 quoted
  that comment as evidence about the shipped ant, and it is evidence about a fit
  that was not shipped.

#### What this points at instead: the emission floor

`ant.ron` deleted `(Bias, EmitA, −0.35)` deliberately, and records why:

> **No offset any more.** The −0.35 it carried existed to hold down a unit whose
> output weight was 1609.1; at 32.0 the level never approaches saturation and
> **the offset would only clip the bottom of the gradient**.

**Clipping the bottom of the gradient is the thing that is needed.** The bottom of
the gradient is what a *dwelling* ant lays — 627 ticks in every 1,000 — and a
floor is the only term that can beat an occupancy ratio, because it makes the
grading ratio **unbounded** (a dwelling ant lays nothing) rather than 4.6:1. No
setting of a decay weight can do that; a cut-off can.

A `biasa=` rider now patches `brain::io_slot(Bias, EmitA)`, with two asserts: the
shipped value is refused as a no-op, and a value inside `W_EPS` is refused because
`eval_brain` would skip the wire and the arm would silently be the shipped one.
Both were watched firing. **Pre-registered** for the sweep at −0.05, −0.10, −0.18,
−0.35 against shipped, same 12 seeds and gap:

- polarity in foraging colonies **flips positive** at some floor, unlike `recur`;
- `a_peak_amt` holds near one `DEPOSIT` — a floor does not change what an ant
  fresh from the nest lays;
- **the counter-risk is reach, not amplitude.** A single control run at −0.18 took
  the route's channel-A cells from **86 to 45 of 91**. A floor that makes polarity
  positive over a 10-cell stub near the nest has bought nothing, so this reads
  `a_polarity` and route coverage together;
- transport still does not follow — discovery is the binding constraint (§7.13),
  and a correct ramp remains necessary rather than sufficient.

### 7.17 §7.15 stands. Its log could not say which scene it ran, and that cost a day

§7.16's sweep disagreed with §7.15's baseline on the same command and the same
twelve seeds. Two numbers that had to be identical were not. The chase, and the
answer, are worth more than the sweep was.

#### What was ruled out, in order, and all of it was wrong

| hypothesis | test | result |
|---|---|---|
| the harness is non-deterministic | same binary, same pin, back to back | **byte-identical** |
| parallelism differed | `RAYON_NUM_THREADS` unset, 1, 2, 4, 8, 16 | none reproduce it (box is 4 cores; unset ≡ 4) |
| CPU contention breaks determinism | five copies at once on a 4-core box | **all five byte-identical to the idle run** |
| my comment edits moved it | rebuild, re-run | identical to the pre-edit run |
| `SPOIL_IS_CARGO` was dropped | re-run with `=0` | differs from both, and *kills* every colony |
| corpses were feeding them | re-run with `onlyfood=off` | no change; not the diet |
| the binary was not the tree | clean `git worktree` at `f8bd2179`, full build | reproduces today's run, not the sweep's |

Every one of those is a code or environment hypothesis, and the answer was in the
**scene**. `CLAUDE.md` says it in one line — *a scene that contradicts the code
will look like a bug in the code* — and six tests were spent before it was read
that way.

#### The answer: `refill`, which the header does not print

The tell was one line neither run's *header* carries, printed at the end:

| | larder put out over the sweep |
|---|---|
| §7.15's run | **48,960 – 314,160 J** |
| my re-run | **48,000 – 48,000 J** |

`refill` re-places the larder every N frames. It defaults to **0** — a one-shot
larder — and until this change it was **not echoed in the header**, so two runs
on completely different scenes printed *identical* parameter lines. Reproduced:
`refill=4000` puts out **48,960–324,720 J**, the same low end and nearly the same
high end as §7.15's run.

**At `refill=0` the scene cannot pose §7.15's question.** The one-shot larder is
48,000 J against a stated need near 46,800 — 1.03x subsistence. Four seeds of
twelve keep any ant alive and the best colony is **six ants**. There is no colony
foraging hard enough to sit at the food, so there is nothing for the ramp to
invert *toward*.

#### Restored, the inversion reproduces — on the committed tree, at three settings

Shipped ant, gap 90, 12 seeds, `arms=hand`, `RAYON_NUM_THREADS=4`, current tree:

| `refill` | seeds with ants alive | foraging colonies | of those, ramp points at the food | mean polarity, foraging | mean, stays home | r(`AtNest`, polarity) |
|---|---|---|---|---|---|---|
| **0** (one-shot) | 4/12 | 5 | 1 | +0.0082 | +0.0176 | +0.19 |
| **500** | 5/12 | 4 | **4 of 4** | −0.0561 | +0.0266 | **+0.69** |
| **2000** | 7/12 | 7 | **7 of 7** | −0.0759 | +0.0518 | **+0.85** |
| **4000** | 3/12 | 3 | **3 of 3** | −0.0631 | +0.0441 | **+0.84** |
| §7.15's original | 7/12 | 7 | 6 of 7 | −0.0704 | +0.0375 | +0.77 |

**Every foraging colony at every refill setting has a ramp pointing at the food**
— 4/4, 7/7, 3/3 — and §7.15's original numbers sit in the middle of that spread.
The finding replicates. What failed was never the result; it was that the log
could not say which scene produced it.

#### What is void, and what is not

- **§7.15 stands**, with the qualifier it always needed made explicit: the
  inversion is a property of a colony foraging against a *replenishing* food
  source. On a one-shot larder the colony starves before it can express it.
- **§7.16's arithmetic stands and is independent of any run**: the odometer's
  decay is dominated by `squash` rather than `w_rec`, the shipped emission falls
  **78% across a 141-tick trip** (0.819 → 0.177), and the simulator saying so
  reproduces `what_an_odometer_emits` to three decimals on three points.
  `recur` is not the decay knob whatever the scene is.
- **§7.16's documentation defects stand**, being file reads rather than runs.
- **§7.16's `recur` sweep is provisional.** Its five arms shared one driver and
  therefore one refill, so the comparison *between* them is sound; but the refill
  is unrecorded, so it is re-run rather than quoted.
- **The emission-floor sweep is void.** It ran at `refill=0`, where two to five
  colonies forage weakly and the shipped ramp is already mildly nest-ward. It
  measured a scene with nothing in it. Re-run at `refill=2000`, the setting with
  7 of 12 alive and 7 of 7 foraging.

#### The process failure, stated so it is not repeated

The rider controls run before the sweep all passed — the shipped value refused as
a no-op, `recur=0.99` firing and moving the readout. They were run on the same
scene as the sweep and were correct. **A control that validates the knob does not
validate the scene**, and this harness printed a header that made two different
scenes look like one run repeated. `refill` is now echoed. The general rule is the
one `CLAUDE.md` already states for the other direction — *a knob nobody can see
the value of is a knob nobody can tell is disconnected* — and the missing half is
that nobody can tell it is **connected** either, which is the more expensive way
round: the sweep looked reproducible and was not.

### 7.18 Both levers fail on the proper scene: the inversion is not a weight

§7.16 and the emission-floor hypothesis were both re-run at `refill=2000` — the
setting with 7 of 12 seeds alive and 7 of 7 colonies foraging, so the question is
posable. Gap 90, 12 seeds, `arms=hand`, `RAYON_NUM_THREADS=4`, every arm's knob
echoed in its own header. **Neither lever flips the sign.**

| arm | seeds alive | foraging | ramp points at food | mean polarity | route cells (med) | peak floor | r(`AtNest`, pol) |
|---|---|---|---|---|---|---|---|
| **shipped** | 7/12 | 7 | **7 of 7** | −0.0759 | 78 | 8,234 | +0.85 |
| `biasa` −0.05 | 6/12 | 5 | 4 of 5 | −0.0653 | 76 | 8,409 | +0.74 |
| `biasa` −0.10 | 7/12 | 6 | **6 of 6** | **−0.1015** | 68 | 9,111 | +0.91 |
| `biasa` −0.18 | 5/12 | 4 | **4 of 4** | −0.0845 | 57 | 8,231 | +0.84 |
| `biasa` −0.35 | 3/12 | 3 | **3 of 3** | −0.0628 | 52 | 8,184 | +0.81 |
| `recur` 0.999 | 5/12 | 5 | **5 of 5** | −0.0820 | 76 | 8,211 | +0.86 |
| `recur` 0.995 | 3/12 | 3 | 2 of 3 | −0.0439 | 76 | 8,265 | +0.65 |
| `recur` 0.99 | 3/12 | 2 | **2 of 2** | −0.1291 | 65 | 8,254 | +0.64 |
| `recur` 0.98 | 6/12 | 6 | 5 of 6 | −0.0517 | 70 | 8,054 | +0.76 |

**Across all eight intervention arms, 31 of 34 foraging colonies still have a ramp
pointing at the food**, and the mean polarity is negative in every single arm. The
paired per-seed shifts are +0.048, −0.014, +0.037, −0.002 for the floors and
+0.004, +0.054, −0.021, +0.035 for the decays — both signs, no trend, noise.

#### Against the predictions recorded before the run

- **§7.16's `recur` result holds on the proper scene.** It was marked provisional
  because its refill was unrecorded; re-run at a known one it says the same thing.
  A **400x** change in the decay weight does not flip the ramp.
- **The emission-floor prediction is wrong, and it was mine.** §7.16 argued a
  floor beats an occupancy ratio because it makes the grading ratio *unbounded* —
  a dwelling ant lays **nothing** rather than a little. It does not happen. The
  tell is in the table: **the peak floor does not move** (8,184–9,111 against
  shipped's 8,234) at any setting including −0.35. If the floor were silencing
  dwelling ants, the food end's amplitude would collapse; it does not, so **the
  ants laying at the food end are not the low-charge ones** and the cut-off never
  reaches them. The charge model of who-lays-what is wrong somewhere upstream of
  both levers.
- **The counter-risk was real and is the only thing either lever reliably did.**
  Route coverage falls monotonically with the floor — **78 → 76 → 68 → 57 → 52**
  of 91 cells — and colonies die with it, 7 → 6 → 7 → 5 → 3 seeds alive. A floor
  shortens the trail toward the nest without re-grading it.

#### What this leaves

**The one quantity that survives every intervention is the correlation itself**:
`r(AtNest, polarity)` is **+0.64 to +0.91 in all nine arms**, shipped and
intervened alike. Two independent weights — the decay that sets how fast a
charge falls and the floor that sets when it stops being laid at all — move
colony survival, trail length and amplitude, and leave that correlation exactly
where it was.

**So the inversion is not a tuning problem, and this is the second time this
report has had to learn that.** §7.15 named it correctly in its closing line and
then proposed a weight anyway: *the plane's shape is set by laying behaviour
rather than by the quantity the design intended to encode*. A plane that every
ant adds to, everywhere it walks, is an integral over occupancy; no per-ant
weight changes what an integral over occupancy is. `CLAUDE.md` states the
remedy for exactly this shape — *when a rule must tell apart two things that
can look identical, state the difference as data* — and here the two things are
"far from home" and "where ants happen to be".

**What that points at** (none of it measured, all of it structural rather than a
weight): lay channel A only on the **return** leg, so the plane integrates
journeys home rather than time spent anywhere; or make it a **distance field**
the world maintains from the nest outward rather than a deposit ants make; or
give the ant a homing input that is not a pheromone plane at all. Which of those
is worth building is the owner's call, and it is a larger question than this
report was opened to answer.

### 7.9 What this leaves standing

- **A laid trail is decisive and the colony cannot lay one itself** (7.11).
  With a journey actually present in the scene, 92% of ants reach food at 90
  cells against 3%, and four colonies of six live where none do; `self` and
  `mute` are indistinguishable. §7.6 and §7.10 are both superseded — the first
  by a starving, cannibalising colony, the second by a colony founded on top of
  its own larder.
- **Nothing provisions the nest in any arm**: 0.14% of carrying happens at
  home, and 7 round trips across six runs. The trail relocates the colony
  rather than supplying it.
- **Homing, not the reader and not congestion, is the fault** (7.12). Laden
  movement has no homeward component at all — −418 net cells over 2.1 million
  carrying ant-ticks — so channel B is drawn on a wander and cannot be
  route-shaped whatever lays it. This blocks §2.1 rather than running beside it.
- **Blocking nestmates is expensive and is not the fault** (7.12). Pass-through
  cuts blocked moves 90% and *reduces* intake 35% in the arm that works.
- **Homing is necessary and not sufficient, and the real constraint is
  discovery** (7.13). A hand-laid homing gradient leaves 0 of 12 colonies alive
  and 0.0% of ants reaching food at 150 and 220. Arms without a laid trail reach
  food at 0-8%; arms with one reach at 62-91%. **Phase 2 reshapes what lays
  channel B, and nothing measured here is limited by what lays it.**
- **The return leg fails because the homing ramp INVERTS when a colony forages**
  (7.15). Channel A is laid abundantly (peaks of 10,269-54,311 against a
  `DEPOSIT` of 10,240, on 55-91 of 91 route cells) but the odometer that shapes
  it decays ~20-60x too slowly to grade a 90-cell trip, so the plane records
  **where ants spent time** rather than distance from home. Homebound colonies
  get a correct nest-ward ramp (positive in 5 of 5); foraging colonies get a
  **food-ward** one (negative in 6 of 7), r = +0.77 against `AtNest` share. A
  laden ant ascending it is driven away from home -- which is §7.12's -418 net
  homeward cells, explained.
- **`Carrying` is true for dig spoil, and it gates both pheromone mechanisms**
  (7.14). In every arm that never finds food it is **100% spoil**, so the
  channel A homing gate is opened exclusively by tailings and a laden ant on a
  ramp runs at 0.641 against a 0.200 baseline — which is why 7.13's `homeA` arm
  *clipped* exploration. **Removing spoil raises exploration in 10 of 12 seeds
  with zero reversals (15.0% → 27.5% past halfway), and leaves the `mute`
  control unmoved** — a real, mechanism-specific defect. It does not move
  survival, and an earlier verdict that read that as "not the fault" is
  withdrawn in §7.14: survival is decided by a ~1-in-36 discovery lottery and
  cannot price any single condition in a multi-condition failure.
- A trail works as **"go eat over there" for individuals** rather than as
  provisioning for the nest — survival rises, the larder is eaten, and nothing
  comes home. No longer an inference for the hauling half: §7.8 measures it on
  a material census and reads `home` flat at zero. The intake half is measured
  but thin — 3 seeds, and an exactly-zero control arm that still wants its own
  near-target count beside it.
- Every bed available either removes distance (`played_bed`) or starves the
  colony (`far_larder` as shipped). A usable bed needs food **far, fixed,
  non-spreading and sufficient**, which is `far_larder` with its larder resized.

## §7.19 The inversion was the instrument: the polarity metric reads a blob's POSITION, not its shape

**Measured 2026-09-17, on `main` at `c40c1712` (the merge of #464). This
retracts §7.15's finding, the nine-arm sweep in §7.18 that was aimed at it, and
the master report's §3.3, §3.4, §3.5 and §1 item 3.** The mechanism it named is
not there. The measurements were all real; the statistic was not measuring what
its name says.

### The control that had never been built

`homeA` — a hand-painted nest-ward channel-A ramp, reading **+0.115** against an
independent prediction of +0.12 — was the metric's only control, and it is a
*positive* one. `CLAUDE.md` asks for the other half — *put the fault back and
watch it go red* — and `homeA` structurally cannot supply it: it fills **91 of
91 route cells**, so the `here > 0 || ahead > 0` admission gate and a `&&` gate
admit exactly the same cells and it is blind to the edge defect §5 item 5
accuses the metric of.

So two **negative** controls were added (`trailfollow arms=flatN,flatF`): a
**constant-amplitude** channel-A blob over the nest half and over the food half,
with the ants' own `EmitA` silenced so the plane holds only what was painted. No
ramp anywhere in either. A correct metric reads 0.

| arm (6 seeds, gap 90, `refill=2000`) | `POLARITY\|\|` | `POLARITY&&` | `SPAN` |
|---|---|---|---|
| `homeA` — a perfect nest-ward ramp | **+0.115** | +0.115 | **+0.026** |
| `flatN` — FLAT blob, nest half | **+0.19498** | +0.18061 | **+0.00000** |
| `flatF` — FLAT blob, food half | **−0.19498** | −0.18061 | **+0.00000** |

**A field with no gradient in it anywhere read ±0.195** — larger in magnitude
than a perfect ramp, and far larger than the −0.076 that §7.15 called an
inversion. Byte-identical on all six seeds, because with `EmitA` muted the plane
is a pure function of the paint schedule; that is the one place a tidy result is
the expected one.

### It is not the admission gate, and that mattered

Tightening the gate to `&&` moved ±0.19498 only to ±0.18061 — **7%**. The
shoulders doing the work are not edge cells admitted by `||`; they are genuine
interior gradients that `DIFFUSE` puts on any finite blob.

The first repair — anchoring the scan to the trail's own occupied span — **was a
no-op, and its failure is the finding**. It reproduced the `&&` figure to five
decimals because the span search ran `nest_x..=target_x`, so `hi` was clamped to
the route. The blob has diffused *past* the route's end, so the shoulder that
would cancel the one being counted is **off the measured segment**, and no window
drawn inside the route can ever find it.

**The root cause, stated once:** the scan window is fixed to the route while the
trail is not. A blob has two shoulders; whichever one happens to fall inside the
route gets counted and the other does not. A blob nearer the food contributes its
rising shoulder and reads negative; a blob nearer the nest contributes its falling
shoulder and reads positive. **That is the whole of the "inversion".**

Searching the span over the **whole row** fixes it: both shoulders are then always
found and a ramp-free blob cancels to exactly zero, while a real ramp still reads
positive. That is `SPAN`, and it is the only one of the three columns that passes
a negative control *and* a positive one.

### What the real arms say under a metric that passes its controls

`trailfollow mode=gap gate=b2 gaps=90 seeds=12 arms=hand,hmute,self,mute,homeA
onlyfood=on larder=fruit food=200 frames=24000 refill=2000`, archived at
[`Reports/data/baseline-5arm-12seed-2026-09-17.log`](data/baseline-5arm-12seed-2026-09-17.log).

**This build reproduces §7.15's headline exactly**, which is what says the
disagreement below is the metric and not the tree: foraging mean
`POLARITY|| = −0.07594` against the reported −0.0759, and
`r(AtNest, POLARITY||) = +0.849` against the reported +0.85.

| `hand`, 12 seeds | foraging mean | negative | range | r(AtNest, ·) |
|---|---|---|---|---|
| `POLARITY\|\|` | **−0.07594** | 7 of 7 | −0.161 … −0.035 | **+0.849** |
| `POLARITY&&` | −0.07502 | 7 of 7 | −0.162 … −0.032 | +0.693 |
| **`SPAN`** | **−0.00539** | **5 of 7** | −0.014 … **+0.007** | **−0.010** |

- The inversion falls **14x** and stops being consistent in sign — two of the
  seven foraging colonies point the other way.
- Against the reference scale, a real ramp reads `SPAN` **+0.026**. A foraging
  colony's −0.005 is **a fifth of a real ramp, in the other direction, on five
  seeds of seven**.
- **`r` goes from +0.849 to −0.010.** The correlation between "where ants spend
  time" and "which way the ramp points" — which §7.18 swept nine arms against and
  which the master report's §1 quotes as `r = +0.64…+0.91` in all nine — is not a
  relationship between two quantities. It is one quantity twice: `||` was reading
  where the channel-A mass sits, which is what `AtNest` reports directly.

The `self` arm makes it starker still. `POLARITY||` reads **+0.133, positive on
12 of 12 seeds** — a confident "strong nest-ward ramp". `SPAN` reads **+0.00215,
positive on 6 of 12**. A coin flip.

### What this does and does not retract

**Retracted:** "the homing ramp inverts in a foraging colony"; the traffic-versus-
grading arithmetic built on it (§7.15, §7.16); the nine-arm `recur`/`biasa` sweep
aimed at flipping the sign (§7.18) — those arms were sweeping a knob against a
number that was not measuring a ramp; and the master's §1 item 3 and §3.3–§3.5.
**"31 of 34 foraging colonies still point at the food" should be read as "31 of
34 foraging colonies had their channel-A mass nearer the food", which is what
`occupancy/1k` already said.**

**Not retracted, and untouched by this:** §7.13's finding that a laid trail is
decisive and the colony cannot build one (`hand` 92% reach against `self`/`mute`
3%; `self ≡ mute`); §7.11's `hmute` decay baseline; the `u16` widening; §3.7's
`homeA` exploration clip; and the cargo-sensing defect. None of those is a
polarity measurement. **Deliveries were always the stated success criterion
(§3.7) and they still are** — this removes a rival criterion that was never
measuring what it claimed, which is the one thing it was doing for the line.

### The rule, for the file that collects them

`CLAUDE.md` already says *ask what your number counts when nothing is wrong* and
*run the positive control*. This is the sixth recurrence of a related shape and it
adds one clause the file does not have: **a positive control built from a case
that saturates the instrument cannot test the instrument.** `homeA` fills every
cell it measures, so it has no boundary — and boundary handling was the entire
defect. A control has to be the case the instrument finds *hard*, not the case
that makes it look best.


## §7.20 The return leg, measured directly — the symptom survives the retraction

**2026-09-17.** §7.19 retracted the *explanation* for why food does not come
home. It did not touch the symptom, and the symptom wants a number that does not
pass through any polarity statistic. This is that number.

`trailfollow mode=gap gate=b2 gaps=90 seeds=12 arms=hand onlyfood=on
larder=fruit food=200 frames=24000 refill=2000 stop=6000`, **shipped genome, no
riders**. The `hand` arm is deliberate: it is the only arm where ants reach food
in quantity, so it is the only one that can ask what a *laden* ant does.

| | |
|---|---|
| ant-ticks carrying larder | 4,260,372 |
| of those, inside the ±26 nest band | **2,177 — 0.051%** |
| completed round trips (nest → within `near` of food → nest) | **11**, over 12 seeds of ~200 ants |
| net cells moved while laden, signed toward the nest | **+883** |
| **…per carrying tick** | **+0.0002** |

Seven of the twelve seeds deliver **exactly zero** while carrying for 500,000 to
800,000 ticks each. **A laden ant's net motion toward home is not slow, it is
nil** — `carry_toward_nest` is the column built to separate "aimed home and slow"
from "not aimed home", and it answers the second.

This reproduces the corpus's own `−418 net homeward cells over 2,146,526 carrying
ticks` (§5 item 4's rescued figure) in sign-agnostic magnitude: both are zero to
three decimal places per tick. Two independent runs, one on `u8` and one on
`u16`, agree that the return leg does not exist.

**What this changes about how to read everything else here.**

- **Any mechanism whose subject is "the trail a homing ant lays" is untestable on
  this bed until this number moves.** That is not a statement about those
  mechanisms; it is a statement about the substrate. The food-charged `EmitB`
  odometer was rejected on this bed and the rejection has been corrected in
  `dead-ends.md` to say so — it was never tested, and its re-test condition is
  now this number rather than anything about trail shape.
- **It explains the shape of the four-arm result without any claim about the
  plane.** `emit_cost_in_moves` charges per unit laid; the odometer holds a unit
  charged for hundreds of ticks after one food contact where the shipped wire
  fires only while laden; the bed runs at 1.03x subsistence. More emission is
  more cost for a return that never arrives, monotone in exactly the order the
  arms came out (7/12, 8/12, 5/12, 3/12 colonies as emission rises).
- **It is the reason §3.2's "discovery is the binding constraint" needs a second
  clause.** Discovery binds for *one* ant. The *loop* needs the return leg, and
  the return leg reads 0.05%. Both are broken; fixing discovery alone cannot
  close the loop, because a scout that finds food and never gets home recruits
  nobody.

**The order of work this implies:** the return leg first, and everything about
trail shape after it, because trail shape is downstream of a journey that is not
happening. That is also the one place where §7.19's retraction does *not* let the
line off — it removed the reason to believe channel A's ramp inverts; it did not
supply a reason to believe homing works, and this says it does not.


## §7.21 The homing circuit is not detectable at six seeds — build the bearing

**2026-09-17.** `nesthome scene=bed arm=shipped|noemit|nosteer`, **six seeds**,
on `u16`, archived at
[`Reports/data/nesthome-gating-6seed-2026-09-17.log`](data/nesthome-gating-6seed-2026-09-17.log).

This is the sweep `nest-design-2026-09-14.md` §12 **pre-registered**, and round
37's Lane 3 restated: *"build the bearing only if laden-at-door still reads as
floor-level on the seeds where the round trip fails"*, at six seeds, because the
`u8` version of the question had needed six to say "nothing".

| arm | laden-at-door, median (range) | trips, med | deliveries, med | alive, med |
|---|---|---|---|---|
| `shipped` | **15.2%** (2.8 – 31.1) | 9 | 468 | 77 |
| `noemit` — lays no channel A at all | **16.5%** (0.1 – 47.7) | 8 | 443 | 86 |
| `nosteer` — lays A, never reads it | **16.9%** (1.7 – 35.1) | **15** | 510 | 78 |

**Cutting the homing circuit out of the genome entirely is not detectable.** Both
ablations are *above* the shipped circuit on the median, `nosteer` has the most
trips, and every deliveries figure sits inside that instrument's own **3.6x**
noise floor between mechanically equivalent arms.

Per seed, the shipped circuit beats **both** cut arms on **2 of 6** — which is
what chance gives when one of three arms must be highest.

**This reverses the three-seed reading it was written to check.** Round 37's Lane
3 reported the shipped circuit beating both cut arms on 2 of 3 seeds after the
`u16` widening, and drew the cautious conclusion that *"the circuit is
detectable"* and that the home bearing might therefore not be needed — while
saying in the same breath that three seeds is not a sweep. It was right to say
so. At six the signal is gone, and the spread is why: laden-at-door runs from
**0.1% to 47.7%** across seeds of one arm, so three samples cannot separate arms
whose medians differ by 1.7 points. `CLAUDE.md`'s *six seeds is not a sweep* case
(§S2's 1.64x over six becoming 1.08x over the next twelve) is the same shape one
rung further down, and the honest reading of **this** table is likewise that it
licenses the weak claim only: **the circuit is not detectable, not that it is
provably absent.**

**So the pre-registered condition is met and step 4 is NOT cancelled — the home
bearing gets built.** That was the cheapest possible outcome to check for and it
did not come off.

### Three independent lines now say the return leg is the fault

| evidence | bed | what it says |
|---|---|---|
| §7.20 | `trailfollow mode=gap`, flat, 90-cell gap | 0.051% of carrying ticks reach the nest band; **+0.0002 net homeward cells per carrying tick** |
| §7.21 (this) | `nesthome scene=bed`, sloped | deleting the whole homing circuit is undetectable at 6 seeds |
| §T2 | played bed | 1,651 pickups, 4 deliveries |

Three beds, three instruments, one answer. **And note they do not agree on
magnitude** — laden-at-door is 2.8–31.1% on the lab bed against 0.051% on the gap
bed, two to three orders apart. That difference is not noise and it is worth
naming: the lab bed has slopes and short distances, the gap bed is flat with a
90-cell run, and **§R4 says `Turn` is near-inert precisely on flat ground**. So
the return leg is not uniformly broken; it is broken worst in exactly the
geometry the foraging loop needs, which is also the geometry in which the engine
cannot currently steer a walking animal at all.

**That is the constraint the home bearing has to be designed around**, not a
detail to be discovered afterwards: wiring a bearing to `Turn` and measuring it
on `mode=gap` would reproduce §R4's null and read as "the mechanism failed".
Either land §R4's own recommended counter first — how often a `Turn` request is
discarded because the side it asked for scored zero — or drive the bearing
through `Move` in the run-and-tumble idiom the trail readers already use, which
is the one steering path measured to work on flat ground (92% of ants reach food
on a hand-laid trail, and that is `Move`, through hidden units 2/3).

## Instruments

- `examples/onetrail.rs` — `mode=arith` (shipped genome, nothing overridden),
  `mode=walk` (one ant, standing trail, mirrored arms), `mode=timing` (§1c).
- `examples/nesthome.rs` — `channel A` and `channel B` profiles by 32-column
  band, `scene=bed`.
- `examples/trailfollow.rs` — the colony-scale question and the `gate=`
  presets. Note its presets each overwrite **both** gate pairs, so no row of
  its `mode=arith` table is the shipped ant, which is a mix.

## Appendix A. Raw per-seed data

Kept in full because outcomes here have enormous spread, and every headline in
§7 is a pooled or median figure over six seeds. A reader who wants to check a
claim, or re-derive a different statistic from the same runs, needs the rows.
All four datasets were produced on this branch between 2026-09-16 14:00 and
17:30 and existed nowhere but a scratch directory until now.

### A.1 Laden homing on the played bed — Phase 1.0, six seeds, three arms

`nesthome scene=bed`, 40,000 frames. `shipped` is the untouched genome;
`noemit` pins `ho_slot(4, EmitA)` to zero, so no channel A is laid at all;
`nosteer` cuts the reader. This is the six-seed version of the sweep
`nest-design-2026-09-14.md` §12 asked for on `main`.

| arm | seed | alive | pickups | deliv | trips | deepest | laden nest% | laden food% |
|---|---|---|---|---|---|---|---|---|
| shipped | 1 | 75 | 6,943 | 290 | 35 | 28 | 9.2 | 29.8 |
| shipped | 2 | 80 | 6,975 | 646 | 6 | 17 | 31.1 | 39.5 |
| shipped | 3 | 122 | 17,669 | 7,505 | 10 | 13 | 18.8 | 19.3 |
| shipped | 4 | 30 | 3,126 | 139 | 0 | 7 | 2.8 | 84.1 |
| shipped | 5 | 85 | 10,164 | 2,853 | 8 | 10 | 11.6 | 39.9 |
| shipped | 6 | 61 | 6,011 | 218 | 21 | 35 | 30.3 | 24.8 |
| noemit | 1 | 90 | 7,144 | 34 | 42 | 25 | 0.1 | 37.3 |
| noemit | 2 | 82 | 7,356 | 529 | 12 | 17 | 26.0 | 42.5 |
| noemit | 3 | 120 | 17,396 | 7,667 | 4 | 11 | 17.6 | 13.9 |
| noemit | 4 | 51 | 3,745 | 48 | 0 | 7 | 4.0 | 81.9 |
| noemit | 5 | 97 | 11,170 | 3,105 | 3 | 8 | 15.4 | 47.0 |
| noemit | 6 | 77 | 7,057 | 357 | 22 | 45 | 47.7 | 21.1 |
| nosteer | 1 | 85 | 7,429 | 45 | 46 | 38 | 2.8 | 24.3 |
| nosteer | 2 | 71 | 6,640 | 839 | 15 | 18 | 35.1 | 34.9 |
| nosteer | 3 | 159 | 17,419 | 4,878 | 15 | 17 | 16.7 | 16.1 |
| nosteer | 4 | 45 | 3,085 | 63 | 1 | 18 | 1.7 | 83.0 |
| nosteer | 5 | 113 | 12,251 | 3,707 | 5 | 45 | 17.2 | 47.8 |
| nosteer | 6 | 68 | 5,480 | 181 | 15 | 49 | 17.5 | 36.9 |

Medians:

| arm | laden nest% | deepest | trips |
|---|---|---|---|
| shipped | **15.2** | 15 | 9 |
| noemit | 16.5 | 21 | 8 |
| nosteer | 17.0 | 28 | 15 |

**Read `laden nest%`, not `deliv`.** The delivery column here is the same
contaminated counter §7.7 disproves — any drop inside the nest band scores one
— so it cannot attribute provisioning in this sweep either. `laden nest%` is
positional (the share of *laden* ant-samples standing in the nest band) and is
not affected by that defect.

On that column the arms do not separate, and **the shipped arm has the lowest
median of the three** — cutting the homing circuit does not measurably reduce
the share of laden ants found at home. Per §7.4 this is **inconclusive rather
than negative**: the bed's moat fills in, so by the time these samples are
taken there is food inside the nest band and a laden ant has no trip to make.
The one column that does move is `deepest` — cutting the reader (`nosteer`)
nearly doubles the median excursion, 15 to 28 — which is what a circuit that
pulls ants home would do, and which says nothing about whether they carry
anything when they get there.

### A.2 The gap sweep, provisioned — the data behind §7.6

`trailfollow mode=gap`, `food=200 refill=4000`, trail hand-laid to frame 6,000
then released, 24,000 frames. `alive` is written as *end/min*. `near on` counts
ant-ticks within 10 cells of the food in the trail arm.

**Taken before `onlyfood` existed** (§7.8), so every row here had corpses as
edible as the larder and four times as rich per cell, and the larder itself was
`corpse` at 30 J. The paired `alive` comparison is unaffected — both arms of a
pair share a seed and the same corpses — but nothing here attributes intake.
To reproduce these exact rows: `onlyfood=off larder=corpse`.

| gap | seed | alive on | alive off | deliv on | deliv off | trips on | near on |
|---|---|---|---|---|---|---|---|
| 90 | 1 | 10/10 | 0/0 | 4 | 5 | 7 | 190,921 |
| 90 | 2 | 5/5 | 5/2 | 1 | 78 | 6 | 186,196 |
| 90 | 3 | 9/9 | 1/1 | 9 | 16 | 7 | 163,819 |
| 90 | 4 | 1/1 | 3/2 | 13 | 18 | 8 | 166,015 |
| 90 | 5 | 17/17 | 2/1 | 14 | 20 | 6 | 183,173 |
| 90 | 6 | 15/15 | 2/2 | 9 | 13 | 13 | 215,979 |
| 150 | 1 | 1/1 | 4/3 | 0 | 36 | 7 | 141,798 |
| 150 | 2 | 0/0 | 9/8 | 32 | 16 | 6 | 82,799 |
| 150 | 3 | 13/13 | 10/10 | 10 | 13 | 13 | 118,433 |
| 150 | 4 | 4/4 | 0/0 | 4 | 28 | 4 | 142,351 |
| 150 | 5 | 2/2 | 4/2 | 19 | 56 | 7 | 139,517 |
| 150 | 6 | 4/4 | 5/5 | 1 | 22 | 11 | 174,781 |
| 220 | 1 | 0/0 | 0/0 | 31 | 58 | 10 | 4,554 |
| 220 | 2 | 3/2 | 0/0 | 7 | 19 | 6 | 17,845 |
| 220 | 3 | 2/2 | 0/0 | 16 | 31 | 15 | 25,611 |
| 220 | 4 | 0/0 | 0/0 | 18 | 20 | 6 | 6,114 |
| 220 | 5 | 0/0 | 0/0 | 22 | 13 | 6 | 5,292 |
| 220 | 6 | 2/2 | 0/0 | 0 | 36 | 7 | 13,146 |
| 300 | 1 | 0/0 | 0/0 | 0 | 30 | 4 | 0 |
| 300 | 2 | 0/0 | 0/0 | 0 | 41 | 5 | 0 |
| 300 | 3 | 0/0 | 0/0 | 6 | 60 | 10 | 0 |
| 300 | 4 | 0/0 | 0/0 | 4 | 37 | 4 | 0 |
| 300 | 5 | 0/0 | 0/0 | 13 | 12 | 4 | 0 |
| 300 | 6 | 0/0 | 0/0 | 10 | 48 | 10 | 0 |

The gap-300 block is §7.7's disproof in one glance: `near on` is **0 in every
seed** — no ant ever came within 10 cells of the food — while `deliv on` reads
0, 0, 6, 4, 13, 10.

Note also that `near on` does not collapse gradually. It is ~180,000 at gap 90,
~130,000 at 150, ~12,000 at 220 and exactly 0 at 300. The trail stops being
reachable somewhere between 150 and 220, which is a **range** limit rather than
a reading limit, and it is upstream of everything in Phase 2.

### A.3 The same sweep under-provisioned — the data behind §7.5

`food=60`, placed once, no refill: 7,200 J against a ~46,800 J need.

| gap | seed | alive on | alive off | deliv on | deliv off | trips on | near on |
|---|---|---|---|---|---|---|---|
| 90 | 1 | 0/0 | 0/0 | 5 | 21 | 8 | 88,224 |
| 90 | 2 | 0/0 | 0/0 | 17 | 33 | 7 | 96,726 |
| 90 | 3 | 0/0 | 0/0 | 46 | 67 | 14 | 86,424 |
| 150 | 1 | 0/0 | 0/0 | 0 | 65 | 6 | 61,200 |
| 150 | 2 | 0/0 | 0/0 | 20 | 27 | 7 | 88,344 |
| 150 | 3 | 0/0 | 0/0 | 29 | 44 | 13 | 82,674 |
| 220 | 1 | 0/0 | 0/0 | 0 | 17 | 6 | 1,386 |
| 220 | 2 | 0/0 | 0/0 | 2 | 20 | 5 | 540 |
| 220 | 3 | 0/0 | 0/0 | 20 | 121 | 13 | 1,098 |
| 300 | 1 | 0/0 | 0/0 | 0 | 19 | 4 | 0 |
| 300 | 2 | 0/0 | 0/0 | 4 | 7 | 8 | 0 |
| 300 | 3 | 0/0 | 0/0 | 11 | 64 | 11 | 0 |

**Every row reads `alive 0/0`, at every gap including the shortest.** That is
what a supply of 15% of need looks like, and it is why no distance conclusion
can be drawn from it. Running the energy arithmetic before the sweep would have
cost a minute and saved the run.

### A.4 What a laden ant on the bed is actually carrying

`foodroad`-style census of `played_bed`, seed 1, 40,000 frames, sampled every
50:

```
ant-samples:  empty 811,919   crop-only 24,205   spoil 1,091   both 1,523
laden total 26,819  =  3.2% of all samples
of laden samples:  crop 90.3%   spoil-involved 9.7%
digs 378   pickups 6,943   deliveries 290   forage trips 35   moves 18,382
plane totals:  A 22,738     B 549,429
```

Three things to take from it, each load-bearing for Phase 2.1:

- **An ant is laden 3.2% of the time**, so channel B is written by a small
  minority of moves — but that minority writes **24x more plane mass than
  channel A**, because `(Carrying, EmitB, 2.5)` is a direct wire while `EmitA`
  comes off a decaying odometer.
- **9.7% of laden samples involve spoil**, not food. That fraction of channel B
  is dig tailings, written onto the plane a forager is supposed to read as a
  route.
- **6,943 pickups against 35 forage trips.** Almost all laden movement is
  local handling, not commuting — which is the dilution §2.0 describes, and the
  reason the fix is to charge the emitter from food rather than from `Carrying`.


### A.5 The gap sweep on an isolated larder — the data behind §7.10

`trailfollow mode=gap gate=b2 seeds=6 frames=24000 food=200 refill=4000
stop=6000 ants=52 onlyfood=on`, larder `fruit`. `alive` is *end/min*.
`other J` is the isolation check and reads 0 in every row.

```
   gap  seed    alive on   alive off   ate J on  ate J off   supply J other J   home   near on  near off
------ ----- ----------- ----------- --------- --------- --------- ---------
    90     1     0/0       183/51         41536     802746      95520       0      0    113072   2176916
    90     2   170/11      174/22        546058     610139     312960       0      0   1525123   1801244
    90     3   122/40       13/12        506286     371066     314160       0      1   1867710    759723
    90     4   113/7       156/51        393067     913395     302640       0      1   1043411   2891584
    90     5   156/12      160/50        496412     542288     294960       0      2   1541142   1884488
    90     6   167/51      154/26        602677     513930     324720       0      1   2134041   1208837
```

Read `near off` against `ate J off` at gap 220: the control arm is at zero on
both in four of six seeds, so its zero intake is arithmetic rather than a
refusal — it never arrives. At gap 150 the control does arrive (`near off`
50,899–2,155,667) and still loses on survival in 5 of 6.


### A.6 The gap sweep on a scene that contains a journey — the data behind §7.11

`trailfollow mode=gap gate=b2 seeds=6 gaps=90,150,220 frames=24000 food=200
refill=4000 stop=6000 ants=20 onlyfood=on`, larder `fruit`, colony founded
x 12..88. Intake off anything but the larder is asserted 0 in every row.

| gap | arm | carry | carry@nest | share | trips |
|---|---|---|---|---|---|
| 90 | hand | 1,275,776 | 1,796 | **0.14%** | 7 |
| 90 | self | 1,752 | 0 | 0% | 0 |
| 90 | mute | 3,234 | 0 | 0% | 0 |
| 150 | hand | 350,748 | 0 | 0% | 0 |
| 220 | hand | 789 | 0 | 0% | 0 |

The `hand` arm at gap 90 carries larder for 1.28 million ant-ticks and brings it
inside the nest band for 1,796 of them. That is the provisioning claim, measured
on a counter nest-local handling cannot fake, and it is essentially zero in the
one arm where foraging plainly works.
