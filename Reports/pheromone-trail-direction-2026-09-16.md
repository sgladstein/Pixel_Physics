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

## §7.22 The homing circuit is correct, and switched off 98% of the time

**2026-09-17.** The decision trace asked for in plain terms — *put an ant on a
trail, laden, and record every decision and why* — pointed at the **colony**
rather than at a bare slab, because the bare slab already passes (§1a, +104 of
112 cells). It localises the 750x gap between the two in one number.

`trailfollow mode=gap gate=b2 gaps=90 seeds=6 arms=hand onlyfood=on
larder=fruit food=200 frames=24000 refill=2000 stop=6000 trace`, archived at
[`Reports/data/decision-trace-hand-6seed-2026-09-17.log`](data/decision-trace-hand-6seed-2026-09-17.log).
**570,660 traced decisions of ants actually carrying larder.**

### The finding

```
homing gate: opens at Carrying >= 0.9890
             OPEN on 10,509 of 570,660 laden decisions  (1.84%)

Carrying histogram: [0.3]25k [0.4]66k [0.5]96k [0.6]125k [0.7]157k [0.8]68k [0.9]33k
```

**`Carrying` is not a boolean.** It is `crop.worth() / crop_capacity`
(`creature.rs`), and `worth()` is `unit * cells`. `ant.ron` authors
`crop_capacity: 1440.0`; one cell of the harness's `fruit` is **960 J**. So an
ant that has picked up one food item reads **0.667**, and the modal laden ant
sits in exactly that bucket.

The homing pair is gated `Bias -45, Carrying +45.5`, so it only leaves
saturation at `Carrying >= 45/45.5 = 0.989` — which needs **two** cells, since
`worth()` accumulates (`cells.saturating_add(1)`) and 2 x 960 clamps to 1.0.
**One pickup is not enough, and digesting only moves it further down.**

### It is not weak. It is off, and when it is on it is excellent

Split by the sign of `PheroAAlong` — the run-and-tumble test, since the shipped
circuit raises `P(move)` up-gradient so the ant runs, and drops it down-gradient
so the ant stalls and tumbles:

| | n | `P(move)` | cells homeward |
|---|---|---|---|
| **gate OPEN**, facing up-gradient | 1,073 | **0.8155** | **+0.0130/tick** |
| **gate OPEN**, facing down-gradient | 9,060 | **0.1572** | +0.0000/tick |
| *pooled (98% gate-shut)*, up-gradient | 147,513 | 0.6468 | −0.0033/tick |
| *pooled*, down-gradient | 372,147 | 0.5897 | +0.0017/tick |

**With the gate open the mechanism works better than the isolated harness
does** — `P(move)` 0.8155 against 0.1572, a swing of **+0.658**, where
`onetrail`'s bare-slab figure is 0.641 against 0.200. And the displacement is
**homeward**, so the channel-A ramp the colony builds for itself points at the
nest.

**Read only the gate-open rows for direction.** The pooled rows appear to say
the opposite — ants facing up-gradient drifting *away* from home — and that is a
confound, not a finding: with the pair saturated it cannot respond to
`PheroAAlong` at all, so whatever moved those ants was some other term and the
correlation with `along` is not causal. Splitting on the gate is what tells them
apart, and it reverses the apparent sign.

### What the decomposition shows, and why it needed the hidden layer

`PheroAAlong` reaches `Move` through **no direct wire**. It enters hidden units
0/1 at ±6.0 and leaves them into `Move` at ±2.5, so a decomposition built from
`creature::probe`'s inputs alone shows every reason the ant moved except the
trail. `probe_full` (added with this) returns the hidden activations, and the
harness asserts `squash(sum of named terms) == outputs[Move]` so the
decomposition cannot quietly be arithmetic this file invented.

Mean `Move` pre-squash terms over all 570,660 laden decisions:

| term | mean | |
|---|---|---|
| `h2` | −2.42236 | the channel-B pair, also saturated |
| `h3` | +2.40296 | |
| **`h0`** | **−2.26960** | **the trail — saturated shut** |
| **`h1`** | **+2.10225** | **and its mirror, cancelling it** |
| `Bias` | +1.99984 | |
| `Energy` | −1.30254 | |
| `KinNeed` | +0.34329 | |
| `Crowding` | −0.25500 | |
| `FoodAdjacent` | −0.18577 | |

The trail pair nets **−0.167** against a `Bias` of +2.0. Both halves are pinned
near ±1 by `squash` and cancel, which is what saturation looks like from
inside a decision.

### What this explains, and what it retires

- **The 750x gap.** `onetrail::hold_gate_laden` folds the authored `Carrying`
  weight into the unit's `Bias`, i.e. it evaluates the circuit **at
  `Carrying = 1.0`** — a value the colony reaches on 1.84% of laden decisions.
  The isolated instrument has been testing a configuration the engine almost
  never produces. Its +104-of-112 is real and is not evidence about the colony.
- **§Z7 is half a diagnosis.** The 2026-09-09 `b2` re-gate moved the *open*
  value from +30 to +0.5 and that was correct. It does not help, because the
  gate does not reach its open value: the fault is the **input**, not the
  authored open state.
- **It is upstream of every channel-A result in this report**, including
  §7.21's "cutting the homing circuit is undetectable at six seeds" — a circuit
  that is switched off 98% of the time is one whose deletion should be hard to
  detect, and that is now the expected result rather than a puzzle.

### What it does not say

- **The threshold is read from the founder genome.** Bred ants mutate, so each
  individual's own threshold moves a little; the histogram is what carries the
  claim, and it shows the population sitting at 0.3–0.9 whatever the exact bar.
- **`fruit` is the harness's larder.** A food worth >= 1,425 J per cell would
  open the gate on one pickup, so the *number* is diet-dependent even though the
  mechanism is not. The lab bed's foods have not been checked against this.
- **It does not say the fix.** Lowering `crop_capacity`, rescaling the gate, or
  giving the pair a food/spoil-split input are all candidates and they trade
  differently; none is measured. What is measured is that the circuit is sound
  and its enabling condition is almost never met.

**Determinism:** traced and untraced runs are identical over 3 seeds
(`probe_full` copies the hidden state rather than writing it back), so the
instrument does not perturb what it measures.

## §7.23 What the homing gate SHOULD do — the design, before the fix

**2026-09-18.** §7.22 found the gate open on 1.84% of laden decisions. Three
repairs are available and they trade differently, so this settles the design
question first: **at what load should an ant head home?**

### The threshold: essentially any load, and it is not close

`gain` and movement cost are charged to the **same** `state.energy`
(`digested - spent`, `creature.rs`), so they compare directly. Every constant is
read from the tree:

```
round trip over a 90-cell gap:  out 22.5 + laden return 33.8  =  56.2 energy
   (90 cells x move_cost_per_cell 0.125 x body 2; x3 laden, "a pellet weighs
    one body cell")
one fruit cell, gain = unit * quality * (1 - overhead), unit = 960:
   at quality 1.0 :  17.1x the round trip
   at quality 0.3 :   5.1x the round trip
against the ant's WHOLE budget (start_energy 200):  4.8x
```

**One item is worth five to seventeen round trips.** There is no load at which
an ant should decline to carry it home, and waiting to fill the crop buys
nothing while risking death out there. The correct threshold is **just above
zero**; the shipped gate sits at **0.989**, needing two cells since one reads
`960 / 1440 = 0.667`.

**This is the rare case where the desired behaviour is not a tuning judgement.**
The margin is an order of magnitude, so no plausible re-derivation of `quality`,
`overhead` or distance moves it.

### The shape: a LATCH, not a slope — and this is the owner's correction

The first draft of this section argued for a graded *weight*, on `CLAUDE.md`'s
"an outcome is a distribution, not a binary". **The owner's objection killed
it**, 2026-09-18: a weight that scales with load gives *every* laden ant the
*same weak homeward bias, all the time* — one uniformly half-hearted cloud. What
is wanted is that **some ants commit to going home while others keep foraging**,
and that is **per-ant persistent state**. A per-tick coefficient cannot produce
a population split.

**Half of the objection does not apply, and it is recorded so nobody re-raises
it.** There is no per-tick "go home / go away" decision to flip-flop between.
`heading` is persistent organism state; `Move` only gates *whether the ant steps
along the heading it already has* — `if draw.unit_f32() < p_move {
step_chain(..) } else if .. { tumble(..) }`. Direction changes only on `tumble`,
in the branch where the ant did **not** step. That is run-and-tumble, the bias
comes from persistence, and §7.22 measures it working: `P(move)` **0.8155**
up-gradient against **0.1572** down, gate open.

So load should set the **rate of committing**, and commitment should persist and
decay out slowly. Whether an individual has latched depends on its own history,
so at any instant a fraction have — the population split, by hysteresis rather
than by a threshold. It is also the more biological answer: real ants have
discrete outbound and homebound states, not a blended drive.

### There is no hidden-to-hidden path, and that dissolves the obvious design

The natural implementation — a `Carrying`-charged recurrent unit **gating** the
homing pair — **is not buildable.** `HH_END = IH_END + HIDDEN_SLOTS` and
`hh_slot(h) = IH_END + h`: **one weight per hidden unit, pure self-recurrence.**
There is no hidden-to-hidden matrix. A hidden unit reaches *outputs* only.

This also dissolves the slot contest it appeared to create (spend `ant.ron`'s
last free unit 7 on the latch, or on §7 step 5's trail-concentration sense, or
grow `BRAIN_HIDDEN`): **a slot does not buy the wiring**, so neither side was
purchasing what it thought.

**What is buildable, cheapest first:**

- **(a) Recurrence on units 0/1 themselves.** `hh_slot(0)` and `hh_slot(1)`
  exist in every genome and are zero. Setting them makes the homing pair
  hysteretic in its own activation — that *is* the latch, in **two numbers in
  `ant.ron`**: no new unit, no `live_slots` change, no `mutation_rate`
  re-derivation, no `genome_manifest` move. The idiom unit 4 already uses.
  **Caveat to measure rather than assume:** the recurrence holds the unit's
  whole sum — gate *and* gradient — so it is persistence in "am I running up the
  homing gradient while laden", not purely in "am I laden". That may be the
  better object: it gives a commuter that rides through local gradient wobble.
- **(b) A latch unit driving `Persist` and `Tumble`.** Both are outputs, so a
  hidden unit can reach them, and they are the run-and-tumble knobs directly
  (`PERSIST_MAX` 2.0 is the straight-ahead score; `Tumble` re-rolls the
  heading). A laden-latched ant that raises `Persist` and drops `Tumble` is a
  committed straight-line walker. A real job for a slot, if one is ever spent.

**If a slot is ever needed: expand, do not drop.** One extra hidden unit costs
`live_slots` **870 → 917** (+47, **5.4%**) and `mutation_rate`
0.0036552 → 0.0034678 in six species files; `GENOME_LEN` is unchanged and no
existing weight moves, because `HIDDEN_SLOTS` is **64** against 8 live — a
reserve built for exactly this. The repo has absorbed it twice (809→846 `Fly`,
846→870 `Stillness`) and once at 318→524, **65%**, for the 4→8 expansion.
Dropping the trail-concentration sense instead spends an **owner-level scope
decision** on a roadmapped mechanism to save 5.4% of mutable surface: the
re-derivation is the cheap half, the decision is the expensive one. **And take
one unit, not a power of two** — 8→16 would be 870 → **1,246 (+43%)**.

### The order, and the tension in it

1. **Rescale the gate** — one line in `trailfollow`'s `GATES` table, which
   already parameterises exactly these three numbers and asserts each arm is not
   a no-op. It is the arm that says whether saturation alone was the problem.
2. **Then recurrence on units 0/1.** Measuring a latch on top of an unrescaled
   gate would measure nothing, because the saturated pair cannot respond either
   way.
3. Only then `crop_capacity`, and only if one item still lands somewhere silly
   on the 0..1 axis. It moves the step's *position* and keeps the step, and it
   side-effects digestion, delivery value and the colony economy.

**The tension to measure, not assume:** the ±45 magnitudes are deliberate — they
make units 0/1 a **conditional**, reading channel A only when laden. A shallower
gate means an empty ant partially reads the homing plane and is steered home
while it should be searching. That is what §Z7's 2026-09-09 `b2` tuning was
navigating, and `plainspeak.rs`'s `GATE_DOMINANCE = 3.0` decides whether the lab
still calls the unit a conditional at all. **The falsifier is the `self`/`mute`
arms**: a shallower gate that helps laden ants and wrecks empty ones shows up as
intake falling while `carry@nest` rises.

**And what distinguishes the latch from the rescale is not mean drift but the
SHAPE of the population** — report the distribution of per-ant homeward
displacement, not its mean. The rescale alone predicts one cloud; the latch
predicts two groups. A mean cannot tell them apart, and quoting one would leave
the objection above unresolved.

### A second defect, found on the way: the gate opens for dirt, not for food

`SPOIL_IS_CARGO` is a **measurement switch, default ON** (`creature.rs`), not
the food/spoil split the roadmap remembers:

```rust
inputs[Carrying] = if spoil_is_cargo() { crop_fill.max(spoil ? 1.0 : 0.0) } else { crop_fill };
```

So an ant holding **spoil reads 1.0 and the homing gate opens**, while an ant
holding one food item reads 0.667 and it stays **shut**. One sensor, three
consumers, and only `Drop` wants the honest answer.

**This is a live confound in §7.22's own 1.84%**: a laden ant that also holds
spoil reads 1.0, so some of those 10,509 gate-open decisions may be spoil-driven.
The trace needs a `spoil` column and a re-run at `SPOIL_IS_CARGO=0` **before**
the rescale — if the gate-open count collapses, the shipped homing circuit is
open only for ants carrying dirt.

## §7.24 Half the open gate is dirt — and opening it six times wider buys nothing

**2026-09-18.** §7.23 named the spoil confound as the first thing to check
before rescaling the gate. It is real, **my prediction about it was wrong in an
instructive direction**, and the correction is the more useful half.

`trailfollow ... arms=hand seeds=6 ... trace`, run at the shipped default and at
`SPOIL_IS_CARGO=0`. Archived at
[`spoil-confound-on-6seed-2026-09-18.log`](data/spoil-confound-on-6seed-2026-09-18.log)
and [`-off-`](data/spoil-confound-off-6seed-2026-09-18.log).

### The confound is real: 47.4%

On the shipped build, of the gate-open decisions on seed 1, **4,978 of 10,509 —
47.4% — are ants that are also holding spoil.** Every ant counted is carrying
larder (the trace's own condition), so nearly half of the homing circuit's
already-tiny availability is owed to **dig tailings the ant happens to be
carrying as well**, not to its food. `Carrying` is
`crop_fill.max(spoil ? 1.0 : 0.0)`, and one pellet of dirt reads 1.0 where one
fruit reads 0.667.

### The prediction was that turning it off would collapse the open gate. It did the opposite

| | colonies | ate J (med) | carry@nest | trips | **gate open** |
|---|---|---|---|---|---|
| `SPOIL_IS_CARGO` ON (shipped) | 4/6 | 434,069 | **1,536** | **5** | 26,157 / 2,164,119 — **1.21%** |
| `SPOIL_IS_CARGO=0` | 3/6 | 501,585 | **1,029** | **3** | 132,926 / 1,869,727 — **7.11%** |

**The gate opens 5.9x more often with the confound removed, not less.** The two
runs are different worlds — the switch is a behaviour change, so trajectories
diverge — and the mechanism is visible in the shipped comment at the `Carrying`
fill site: `ant.ron` authors `(Carrying, Drop, 0.2)` as its whole away-from-nest
putting-down rule, so when dirt reads as cargo the `Drop` gene fires on dirt and
ants put things down more. With spoil silenced they hold on, accumulate a second
food cell, and `crop_fill` clamps to 1.0 honestly.

### And that is the finding: more open gate is not more delivery

**`carry@nest` 1,536 → 1,029 and `trips` 5 → 3**, against a **5.9x** rise in how
often the homing circuit is available. Per seed it scrambles rather than trends
(1212→78, 0→0, 0→587, 60→276, 264→88, 0→0), which is what six seeds of a chaotic
bed look like; the honest statement is **no detectable improvement, and
emphatically not the 6x that the availability change might have suggested.**

**This independently replicates §7.14 from a different instrument.** That section
found cargo sensing to be a real defect on *exploration* with **no effect on
transport**; this reaches the same verdict through the decision trace rather than
through a 50-seed replication.

### What it means for §7.23's plan, and it is a caution

**Gate-open frequency is not the binding constraint on its own.** Going from
1.2% to 7.1% availability moved nothing that matters. So **the rescale (step 3)
should be expected to buy little by itself**, because it does the same thing by
a different route — it makes the gate reachable, it does not make an ant commit.

That is evidence *for* the owner's latch argument rather than against it: the
prediction "availability up, outcome flat" is exactly what "a uniformly
half-hearted colony is not the same as committed commuters" says should happen.
It is weak evidence — 6 seeds, tiny counts — but it points the same way.

**Revised expectation for the next session:** run the rescale sweep as planned,
but do **not** read a null there as "the gate was not the problem". Read it as
"availability alone is not sufficient", and go on to the latch
(`hh_slot(0)`/`hh_slot(1)`) which is the part that changes the *shape* of the
population rather than the *fraction* of time the circuit is live.

**Not yet answered:** whether `SPOIL_IS_CARGO=0` should ship anyway. It removes a
sensor that lies, and its cost here is inside the noise — but the `Drop`
behaviour it changes is the reason it was left on, and that trade has not been
measured on the lab bed where digging matters.

## §7.25 Crop fill buys stillness, not homing — the response curve, and a harness fault

**2026-09-18.** §7.23 asked for the gate to be rescaled and then latched, and
§7.24 found that opening it 5.9x bought nothing and read that as evidence *for*
the latch. This measures the thing both were arguing about — **what a laden ant
does as a function of how full it is** — and the answer is that the curve the
design wants does not exist at any setting, so neither repair could have moved
it.

The owner's statement of the target, 2026-09-18: *a full crop should be a 100%
chance to move toward home, a half crop about 50%, and so on.* That is a claim
about a **shape over fill**, and no aggregate in this corpus could see it.

### The instrument

`examples/trailfollow.rs` already binned `Carrying` into tenths and printed the
**counts** — the histogram §7.22 quotes. It threw the outcome away, so it could
say the modal laden ant sits at 0.7 and nothing about whether 0.7 behaves
differently from 0.3. Two columns were added to the same per-decision loop:

- **the response curve** — per fill bin, `P(move)`, `P(home)`, `P(away)` and net
  cells/tick. `P(home)` and `P(away)` are steps, from the far side of the call,
  against `P(move)`, which is what the brain asked for;
- **the conversion test** — gate OPEN against gate SHUT, pooled over gradient
  direction, as two halves of **one run**. `tr_up_open`/`tr_down_open` split the
  open half by which way the ramp points and have no shut counterpart, so
  nothing could be subtracted from them.

**Positive control: the archived run reproduces exactly.** `n 570,660`, mean
`PheroAAlong -0.16732`, `net cells homeward 308`, gate open `10,509 (1.84%)`,
up-open `1,073 / P(move) 0.8155 / +14`, down-open `9,060 / 0.1572 / 0`, spoil
`4,978 (47.4%)` — every figure §7.22 and §7.24 report, unchanged. The columns are
additive.

### The curve, and it goes the wrong way

`arms=hand`, 6 seeds, gap 90, `stop=6000` — the scene §7.22 measured:

```
     fill          n   P(move)   P(home)   P(away)    cells/tick
  0.3-0.4      25091    0.6314    0.0177    0.0176     +0.000120
  0.4-0.5      66377    0.6220    0.0168    0.0165     +0.000316
  0.5-0.6      95976    0.6457    0.0184    0.0157     +0.002980
  0.6-0.7     124777    0.6376    0.0161    0.0153     +0.000850
  0.7-0.8     156731    0.6339    0.0153    0.0156     -0.000249
  0.8-0.9      68392    0.5797    0.0134    0.0142     -0.001009
  0.9-1.0      33316    0.3283    0.0064    0.0064     +0.000000
```

**`P(home)` and `P(away)` are a dead heat in every bin.** 0.0177/0.0176,
0.0168/0.0165, 0.0161/0.0153, 0.0153/0.0156, 0.0134/0.0142, 0.0064/0.0064.
There is no homeward bias at any fill — a laden ant is a symmetric random walker
whether it holds a third of a crop or a full one.

**And `P(home)` *falls* with fill, 0.0177 → 0.0064, a factor of 2.8.** The
fullest ants are the least likely to take a homeward step, because the only
thing fill does is lower `P(move)` (0.63 → 0.33), which brakes them equally in
both directions. **Crop fill is wired to stillness, not to direction.**

The bottom row is the design target's own case: an ant with a full crop, the one
that should be sprinting home, moves on a third of its decisions and splits those
**213 home / 213 away**, for a net of exactly zero.

### The conversion test: the homing circuit is worth eleven cells

```
  gate OPEN     n    10509   P(move) 0.2409   cells homeward   11 (+0.001047/tick)
  gate SHUT     n   560151   P(move) 0.6178   cells homeward  297 (+0.000530/tick)
  => open minus shut: P(move) -0.3769   cells homeward +0.000517/tick
```

**Over six seeds and 24,000 frames the entire gate-open population produces
eleven net homeward cells**, and opening the gate *lowers* `P(move)` by 0.38.
The homing gate is a brake whose net yield is eleven cells.

**This does not contradict §7.22 — it prices it.** §7.22's `+0.658` swing is a
*differential over 1,073 decisions* and is real; what it never stated is the
magnitude on the other side of it. `+0.013048/tick` on a population of 1,073 is
**fourteen cells**. `CLAUDE.md`'s *"ask what your number counts"*, in the shape
where the number is a ratio and the question was a quantity.

### Why: the engine says so in a comment, and it has always said so

`creature.rs:4156`, untouched:

> *"`Move` is the run probability, and the brain drives it from the
> along-heading gradient: a laden ant walking away from the nest scent computes
> a low `Move`, fails the roll, and re-orients. **That is the whole of the
> homing mechanism — there is no steering toward the nest anywhere**, because on
> a surface there is nothing to steer on."*

`Move` gates whether the ant steps **along the heading it already has**. It
cannot turn it. Direction changes only in `tumble` (`creature.rs:11066`), which
re-rolls **uniformly among the viable directions** — its own doc names this as
the only mechanism available "to something whose lateral sensors read zero".

So a laden ant pointed away from home with the gate open does not turn around.
It brakes (`P(move)` 0.1572), stands still, and waits for a uniform re-roll to
happen to point homeward. That is a random search with a brake, and the curve
above is what it yields. **86% of gate-open decisions are ants facing
down-gradient** (9,060 of 10,509) — when the gate is open the ant is *less*
likely to be facing homeward (10.2%) than the laden population at large (25.9%).

### What this retires, and what it promotes

- **The rescale (§7.23 step 1) cannot work, and §7.24 already measured that.**
  Opening the gate wider produces more braking, not more homing, because the
  fill→direction channel does not exist to be widened. §7.24's null was not weak
  evidence for the latch; it was the direct prediction of this curve.
- **The latch (§7.23 step 2) has almost nothing to hold.** Recurrence on units
  0/1 makes "running up the homing gradient while laden" persistent, and only
  10.2% of open-gate decisions are running up it at all.
- **`crop_capacity` (§7.23 step 3) moves the step's position along a curve that
  is flat.** Nothing to buy.
- **What is promoted is the owner's rule**, because it is the only proposal on
  the table that writes to *direction*. It is also what `nest-design` §8C and
  §6's path-integration reading were reaching for, at a fraction of the price —
  see §7.26.

### The harness fault, found by the control failing

`stop=` **was never echoed in the header**, and it defaults to `0`. It decides
whether the hand-laid trail stands for the whole run (the *pull* question) or is
seeded and released (the *loop* question) — two different experiments on one arm.
The same command at `stop=6000` and at the default reports **n 570,660 / 1.84%
open** against **639,100 / 1.25%**, under **byte-identical parameter lines**.

This is the second time this exact fault has been found in this file: §7b records
`refill` being fixed for it days ago. `stop` now prints too. The
`nostop` log is archived beside the other so the pair is on the record, and the
two scenes disagree on a published sign — gate-open up-gradient reads **+14
cells** at `stop=6000` and **−48** at the default, so §7.22's *"positive means
the ramp points at the NEST"* is a statement about one scene, not about the
colony.

**Data:** [`Reports/data/fill-response-curve-hand-6seed-2026-09-18.log`](data/fill-response-curve-hand-6seed-2026-09-18.log)
(`stop=6000`, the §7.22 scene) and
[`fill-response-curve-nostop-hand-6seed-2026-09-18.log`](data/fill-response-curve-nostop-hand-6seed-2026-09-18.log)
(the default).

## §7.26 The fill-weighted tumble — the design, before the fix

**2026-09-18. Designed, not built.** §7.25 says crop fill is wired to `P(move)`
and not to direction. This is where a fill-to-direction channel goes, and what it
costs. **The rule is the owner's**, stated 2026-09-18: *a full crop should be a
100% chance to move toward home, a half crop about 50%.*

### There is exactly one site that sets direction, and it is not `Turn`

`Move` gates whether the ant steps **along the heading it already has**
(`creature.rs:4156`). `Turn` is the obvious alternative and **§R4 is open against
it**: on level ground both outer candidates lose at every `Turn` value, measured
byte-identical with `(PreyBearing, Turn) = -2.5` wired. `trailfollow mode=gap`
builds a flat floor, and so does most of the bed the loop needs.

That leaves `tumble` (`creature.rs:11066`), which re-rolls the heading
**uniformly among the viable directions** and is the only place in the walk where
a new direction is chosen. Its own doc already names it as the only mechanism
available "to something whose lateral sensors read zero". **It works on flat
ground by construction** — it selects among directions already filtered for
footing, so it cannot ask for a step the body cannot take, which is precisely how
§R4 kills `Turn`.

### The rule

In `tumble`, with probability **`crop_fill`**, pick the viable direction whose
`DIRS` vector best matches the home vector; otherwise re-roll uniformly, exactly
as today.

- **`crop_fill`, never `Carrying`.** `Carrying` is
  `crop_fill.max(spoil ? 1.0 : 0.0)` (§7.24), so keying on it would send ants
  home for a pellet of dirt — 47.4% of today's open gate. The food/spoil split
  the roadmap keeps deferring is not needed here; the honest number is already in
  scope at the call.
- **The home vector is free.** `forage_anchor` (`organism.rs:5888`) is anchored
  at spawn and **re-anchored at every nest contact**, so `(anchor - head)` is the
  bearing with **zero new state and no integrator to drift** — the reason
  `nest-design` §8C chose it. Do not accumulate displacement: `step_crossing` and
  `step_flight` return early before the deposit site, so a flying ant would
  silently desynchronise the integrator.
- **An empty ant is byte-identical to today.** At `crop_fill = 0` the draw never
  fires. That answers §7.23's falsifier by construction rather than by
  measurement: a shallower *gate* risks steering empty ants home while they
  should be searching, and this cannot, because the term does not exist for them.

### Why this produces the population split the latch was for

§7.23's objection to a graded weight is right and does not apply here. A graded
weight gives every laden ant the same weak bias every tick — one uniformly
half-hearted cloud. **This draws once per tumble, and `heading` is persistent
state**, so an ant whose re-roll landed homeward *runs* homeward until something
stops it. At `crop_fill = 0.5` half the re-rolls commit and half keep wandering:
**two groups, not one cloud**, which is what the latch was being built to buy —
reached through the direction channel instead of through hysteresis, and without
spending a hidden unit or a `BRAIN_INPUTS` bump.

It is also the ethos's first law applied to the return leg: the outcome is a
distribution over ants and moments rather than a threshold that is off 98% of the
time.

### What it costs, against step 4

| | fill-weighted tumble | §7 step 4 (`HomeBearing`) |
|---|---|---|
| `BRAIN_INPUTS` | unchanged | 30 → 34 |
| `live_slots` | unchanged | 870 → 966 |
| `mutation_rate` | unchanged | re-derived in **six** species files |
| genome / `genome_manifest` | untouched | pinned, moved |
| works on flat ground | **yes** | blocked by §R4 |
| guards expected red | none | `brain.rs:2007`, `:2058`, `:1652` |

### The tension to measure, not assume

- **The existing gate may fight it.** A full ant's `P(move)` is **0.3283**
  (§7.25) — pointed home and refusing to step is a live outcome, and whether the
  two channels cooperate depends on the local sign of the channel-A ramp, which
  §7.25 shows is **scene-dependent** (+14 cells at `stop=6000`, −48 at the
  default). **So the gate rescale is not cancelled — it is demoted to
  conditional and reordered**: run it *after* the direction rule exists, where
  its job is to stop braking an ant that now knows which way home is. Measuring
  it first, as §7.23 ordered, measures a brake on a walker with no destination.
- **It moves homing out of the genome**, which is the axis the master's *"things
  deliberately not in this plan"* flags against the `ActiveSpace` field. The
  mitigation is to author the strength as a species parameter so it stays
  evolvable rather than hardcoding 1.0; ship it behind a switch for the
  measurement arm either way.
- **Biologically it is closer, not further.** §6's reading is that real ants home
  by path integration — a private home vector, run straight — and use the trail
  only as a contextual modulator. This is a coarse path integration; the
  gradient-reading it supplements is the part with no biological counterpart.
- **Determinism holds but every baseline moves.** The draw comes from the same
  `rng::Rng` already threaded into `tumble`, so the stream stays reproducible —
  and it shifts for every ant from the first tumble on, so §3 and §7.11–§7.25
  are re-taken, not compared against.

### Acceptance

**The instrument is already built: §7.25's response curve is the test.**
`P(home)` must climb with fill and separate from `P(away)`; today it *falls*
(0.0177 → 0.0064) and the two are a dead heat in every bin. That is a direct read
of the owner's rule rather than a proxy, and it is visible in one 6-seed run
before any survival or delivery number is quoted.

Then, and only then, `carry@nest` and `trips` on an order statistic over 12
seeds, with a seed sweep for regression in reach — the plan's standing criterion,
which nothing here replaces.

### The alternative site, recorded so it is not re-derived

`step_chain`'s candidate scan already scores directions and picks with
`choose_weighted` (`creature.rs:9505`). A home term there would bias **every
step** rather than every tumble, which is stronger and less legible: it competes
with footing and crowding scores that were calibrated without it, which is
`CLAUDE.md`'s *"a correct mechanism at inherited constants is a regression"*.
`tumble` is the cheaper first arm because its re-roll is currently **uniform** —
there are no weights to re-derive.

## §7.27 The steering works, and it starves the colony — every carry ends in a drop

**2026-09-18. Built (§7.26's design), measured, and the result is a stop.**
`CreatureDef::home_bias` ships at `0.0` and this is the sweep over it.
`arms=hand`, 6 seeds, gap 90, `stop=6000`.

### The control is bit-identical, which is the claim the rest rests on

At `home_bias: 0.0` the run is **identical to the archived log, line for line**
— `n 570,660`, net `308`, gate open `10,509`, every seed row. The gate on
`home_bias` and on a non-empty crop is checked before `draw` is touched, so the
default arm takes no RNG draw and the whole corpus in `Reports/data` stays
comparable. `tumbles_homeward` reads **exactly 0** in every row, which is the
counter's negative control.

### The steering works

| | `home_bias` 0.0 | 1.0 |
|---|---|---|
| net cells homeward / laden tick | +0.000540 | **+0.027191** (50x) |
| `P(home)` / `P(away)`, fill 0.6–0.7 | 0.0161 / 0.0153 | **0.0415 / 0.0142** |
| `carry@nest`, summed over seeds | 1,536 | **5,646** |
| laden ant-ticks | 570,660 | 7,098 |

The dead heat §7.25 found is gone: a laden ant is now **2.9x** more likely to
step homeward than away. `arrive@` is **identical across every arm**
(804, 600, 564, 1050, 534, 528), so discovery is untouched — the empty-ant path
really is unchanged by construction.

### And it kills the colony

Colonies alive at the end, of 6 seeds:

| `home_bias` | 0.0 | 0.1 | 0.25 | 0.5 | 1.0 |
|---|---|---|---|---|---|
| colonies alive | **4** | 2 | 3 | **0** | **0** |
| `ate J`, summed | 2,924,202 | 1,538,574 | 2,910,833 | 5,333 | **0** |
| `trips`, summed | 5 | 1 | 1 | 0 | 3 |

**At `w >= 0.5` every colony in every seed dies**, by frame 7,200–8,832, having
eaten **nothing**, with no ant ever born — the founding 20 and no more. Six
seeds is not a sweep (`CLAUDE.md`), and the per-seed scatter at 0.1 and 0.25 is
this bed's usual chaos; the twelve-of-twelve at 0.5 and 1.0 is not.

**`trips` does not rise at any setting.** The plan's stated success criterion is
`carry@nest` *and* `trips`, and only the first moved.

### Why: the delivery destroys the meal

Food is eaten by **holding it**. `digesting` accumulates in the crop and a cell
is absorbed only when it reaches `c.unit` — 960 J for the harness's `fruit`.
The drop path's own comment states the consequence: dropping before maturity
*"forfeits the progress **and** the meal, which is starvation rather than an
exploit."*

`ant.ron` authors **`(AtNest, Drop, 1.0889)`**. So arriving home *is* the drop
trigger. Give an ant a homeward run and the sequence becomes: pick up, walk 90
cells, arrive, drop — with `digesting` discarded every time.

The counter says so:

| | drops | cells digested (`ate J` / 960) | share of carries that ended in a drop |
|---|---|---|---|
| `home_bias` 0.0 | 5,990 | 3,046 | ~66% |
| `home_bias` 1.0 | 285 | **0** | **100%** |

**The prediction that produced this counter was wrong, and the correction is
the finding.** It predicted drops would *rise* — ants thrashing at the nest.
They **fell 21x**, because far fewer ants ever carry anything once the colony
stops growing. The count was never the question: **the rate is.** In the
control a third of carries end in a meal; at `home_bias: 1.0` **none of them
do**. `CLAUDE.md`'s *"ask what your number counts"*, caught by running the
control rather than by reasoning.

### What this means, and it is bigger than the dial

**The return leg was not the last missing piece.** It is now the *second* to
last. Food put down at the nest is an ordinary loose cell on the ground — there
is **no larder**, nothing that stores it, and nothing that eats from it — so
the act of delivering subtracts the meal that paid for the journey. A colony
that commutes is strictly worse off than one that grazes, and the engine has
been telling us so through the only channel it had: ants that would not go home.

That is the ethos's second law in its sharpest form. **The verb now works and
what it produces is nothing.** The right reading of §7.25's "eleven net homeward
cells" is not only that homing was broken; it is that homing had **nothing to be
for**.

### What to build next, in order

1. **A delivery that stores rather than forfeits.** Either the drop at a nest
   banks the cell's worth to the colony, or `digesting` survives a drop and
   resumes on re-ingest. The first is a larder and is the bigger change; the
   second is a one-field fix to `Crop` and is the cheaper arm to measure first.
2. **Only then re-sweep `home_bias`.** Measuring a commuting rule against an
   economy that punishes commuting measures the economy. Everything in the table
   above is conditional on step 1 and must be re-taken after it.
3. **Do not read the 0.1 / 0.25 rows as "a safe setting exists."** They are
   near-neutral because at low `w` most ants never commit, which is the shipped
   animal wearing a dial.

**Data:** `Reports/data/homebias-{0.1,0.25,0.5,1.0}-hand-6seed-2026-09-18.log`
and the drop-counter pair `homebias-drops-{control,1.0}-hand-6seed-2026-09-18.log`.

## §7.28 The granary — design of record, and the criterion that stops this drifting

**2026-09-18. Nothing here is built.** §7.27 ended with the return leg working
and starving the colony. This is the plan out of that, written before any code,
and the first section exists because the owner stopped the work to ask for it.

### 0. The criterion, and why it is first

**The goal of this line is not that colonies survive. It is that a colony lays
its own food trail.** Stated by the owner, 2026-09-18, on being shown the
foraging-economy plan: *"I want to make sure we don't lose pheromone context if
that part isn't finished."*

The risk is specific and was already live. **Every `home_bias` measurement in
§7.27 was taken on `arms=hand`** — a bed with a trail already laid down for the
ants. The pheromone question lives in the **`self`** arm, and no run in that
section touched it. A foraging-economy fix that makes colonies survive on
`hand` would look like success and answer nothing.

> **ACCEPTANCE, for everything below:** `arms=self`, and **`route pk` rises off
> **3.4 of 90**. That is §3.2's number for a colony left to build its own trail,
> and it is indistinguishable from `mute` (channel B zeroed). Survival, `ate J`
> and `carry@nest` are *diagnostics on the way*; none of them is the finish.

**Why the return leg is upstream of that rather than a detour from it.**
`ant.ron` drives channel B from exactly one wire:

```
(Carrying, EmitB, 2.5)
```

**A food trail is, by construction, the track of a laden ant walking home.** No
laden return journey, no channel B along a route, no recruitment — structurally,
not weakly. §7.20 stated the same thing from the other side: *"any mechanism
whose subject is 'the trail a homing ant lays' is untestable on this bed until
this number moves."*

So the chain is: `self ≡ mute` ← no channel B laid ← no laden return ← homing
did not steer (§7.25, now fixable) ← turning it on starves the colony (§7.27)
← delivering destroys the meal. **The granary is three links down and every link
is load-bearing.**

**And §7.27's trail columns do NOT answer this** — recorded so nobody quotes
them. At `home_bias: 1.0` the colony's own trail reads `end 0`, `along +0.0000`
and a B profile pinned at **8291–8292 in all six seeds**, which is
`CLAUDE.md`'s tidiness tell: those colonies were dead by frame 7,200–8,832 and
the measurement window opens at 7,500. It is hand-laid residue. The question is
**unanswered**, not answered in the negative.

### 0a. This was investigated three weeks ago, and the prior work changes the plan

**Found by grepping `dead-ends.md` for the mechanism before building it, which
is `CLAUDE.md`'s rule and which nearly did not happen — §0–§7 below were drafted
first.** Recorded in full because every item either redirects a step or forbids
one.

**[`larder-reachability-2026-08-30.md`](larder-reachability-2026-08-30.md) asked
this exact question and answered it.** Its verdict: *"the granary end is an
empty set"* — not for want of a pile, but because nothing can spend one.

1. **A birth cannot be paid from the world, and it is a code fact.**
   `creature::try_bud` gates on `state.energy` and charges `state.energy`;
   `adjacent_nest` is read by a brain input, the drop branch and a visit
   counter, and **never by anything that looks at what is in the nest
   neighbourhood.** *"A granary of ten thousand cells would fund exactly zero
   births."* The only route from a pile to a child is **indirect** — an ant eats
   from it, its own bank rises, it buds — and that is precisely what §2's wire
   targets. A *direct* nest-funded birth is the larger change and is that
   report's §6 item 1.
2. **The thrash is not new and is not mine.** Measured at colony scale over 18
   seeds: **140,202 pickups against 137,945 drops**, and **87% of what an ant
   puts down it puts down away from the nest.** The report names the cause in
   its §6 item 2 — *"with the pickup branch ahead of the drop branch and no
   stored bit, a colony cannot hold a pile larger than its own carrying rate:
   what is put down is picked back up."* §7.27's 285 drops against zero meals is
   the same phenomenon with `home_bias` concentrating it at the door.
3. **The pile is a flow, not a store, and that is measured rather than argued.**
   145 entries against 143 exits over 15,000 frames, `resident` ending at **0** —
   nothing that was in the first pile is still there. A standing count of ten
   cannot tell a store of ten from ten in transit; it is ten in transit.
4. **A granary can physically stand — just not near ants.** A hand-planted
   40-cell pile in a colony-free world settles to 22–23 and holds for 18,000
   frames on all 18 seeds. Add a colony and the paired difference is **−14 cells,
   down on 15 of 18.** So persistence is not the missing piece; the colony is the
   sink.
5. **`larder_probe` already exists** and asks the right question — *"is there a
   standing pile of food beside the nest, and is it a store or a flow?"*, banded
   by Chebyshev distance to the nearest nest cell, priced in what the gut can
   digest rather than face value, **with both controls in the binary**
   (`mode=control` plants the same pile with no colony; `mode=turnover` separates
   a store from a flow).
   > ⚠️ **CORRECTED 2026-09-18, and the first version of this line was wrong in
   > the way that matters.** It said this was "requirement 7's metric built in
   > advance". **It is not, because it cannot be aimed at this line's bed.**
   > `larder_probe.rs:89` is `const PRESET: &str = "wetland"` and the argument
   > list is `mode, frames, every, seeds, plant` — **there is no `scene=`**. It
   > builds a 512x160 wetland with a 74-cell nest strip, and nothing points it
   > at `trailfollow`'s `LabBox` gap bed, which is where the pheromone criterion
   > in §0 lives.
   >
   > And `trailfollow` has **no standing-food census at all** — `carry@nest`
   > counts *ant-ticks carrying larder inside the nest band*, which is ants, not
   > a pile. So **requirement 7 is outstanding**: on the bed this line is
   > measured on, there is no number for "is there a store, and is it growing".
   > Per the requirement's own terms that has to exist **before** the mechanism,
   > or `ate J` will under-report a working loop exactly as §7.25's numbers did.
   >
   > Use `larder_probe` for what it can do — auditing the prior report, which
   > used this same tool on this same bed — and build the gap-bed census
   > separately.

**`TRAIT_STORE_IN_BODY` was specced and deliberately not built
(`dead-ends.md`, 2026-08-31), and its reasoning is the strongest argument for
§2's wire.** The gene was redundant *because the `Feed`/`Drop` output contest is
already the granary-versus-replete mechanism*: those weights are heritable,
mutate at every birth, and are conditioned on everything the brain senses. A
scalar trait beside them is *"a second knob on one quantity and a strictly weaker
one, because it is unconditioned."* **So the mechanism is to be reached by
changing what that contest reads — which is exactly a missing `Energy → Feed`
wire — and NOT by adding a trait, a flag or a new verb.**

**The order that report settled on, which supersedes any I would invent:**
(1) make a birth payable from a nest-adjacent store, (2) stop stored cells being
re-taken, (3) **re-derive whatever was calibrated against the current
behaviour**, only then (4) write the gene. It flags step 3 as *"not optional and
the expensive one"* — `hunger_fraction`, `reproduce_threshold` and `drop_urge`
are all balanced against a world where the pile is inert. That is requirement 6
below, arrived at twice independently.

> ⚠️ **AND A WARNING AIMED SQUARELY AT THIS PLAN.** `dead-ends.md`, 2026-09-08:
> *"'The colony starves, so selection cannot have teeth in this bed'; reasoned
> from a real observation and refuted — **assuming it does sent a session at the
> larder instead of at the horizon.**"*
>
> A previous session saw starvation dominating mortality and went at the larder.
> That was the wrong call. **This plan must not be the same move wearing a new
> number.** What makes it different, stated so it can be checked rather than
> asserted: the claim here is not *"colonies die, therefore fix food"* — it is a
> mechanism measured end to end, that **100% of carries end in a drop and none in
> a meal** against a control's ~66/34, with the digestion forfeit named in the
> engine's own comment and `(AtNest, Drop, 1.0889)` as the trigger. **The
> falsifier is §2's own: if the wire moves `drops` and `ate J` not at all, this
> is the larder detour again and the plan stops.**

**One correction to §7.27's reading, before anyone builds on it.** I read the
control's non-zero `end` (51, 19, 19) as a colony maintaining some trail of its
own. **The corpus says otherwise and should be believed**: `route pk` reads 88 of
91 in every `hand` arm alike, *including an arm where the ants lay no channel B
at all*, which `dead-ends.md` records as independently reproducing §3.2 — **ant
maintenance of a laid trail is zero.** Those `end` cells are most likely
hand-laid residue decaying at different rates in a live colony against a dead
one. Do not quote them as colony trail.

### 0b. What the prior granary evidence can and cannot bear — owner's correction, checked

**Owner, 2026-09-18:** *"The granary fail happened when we had a fully not
functioning pheromone/forage loop, so I wouldn't place too much weight on the
failure."* Correct, and §0a as first written leaned on it too hard. The point is
not that the numbers are wrong — they reproduce — it is **which world they are
numbers about.**

`dead-ends.md` entries carry *the condition their rejection depends on* for
exactly this reason. Here that condition is **a colony with no return leg**, and
it is the condition §7.28's whole plan exists to change. So the larder findings
are **suspended, not binding**, and the re-test is the same event.

**Ran `larder_probe` rather than citing it**, which is the other half of the
owner's note (*"when using pre-built tools, make sure they are doing what you
think they are doing"*) — and §0a had quoted it without ever executing it. Two
hygiene checks it passes that `trailfollow` did not: it **echoes its own
parameters** in the header, and it **panics on an unknown argument** instead of
ignoring it (`larder_probe.rs:122`). Three things the run shows that change how
its findings read:

- **The colony is dying for the whole census.** `seeds=2 frames=4000`: the `ants`
  column runs **52 → 47 → 45 → 37 → 36 → 29 → 28 → 24 → 21 → 18 → 16.** Every
  standing-pile figure in `larder-reachability` is therefore measured on a colony
  in decline, and *"the pile does not accumulate"* cannot be separated from *"a
  shrinking colony accumulates nothing."* That is the owner's point, in the
  instrument's own output.
- **The bands do not discriminate on this bed.** `<=2`, `<=4`, `<=8` and `<=16`
  read identically in most rows (5,5,5,5 — 10,10,10,10 — 16,16,16,16), because
  the scene's nest is a **74-cell strip** (`nest_x=16..90`), so "within two of the
  nearest nest cell" is most of the colony's world. The banding is not wrong; it
  is uninformative here, and a conclusion resting on band contrast is not
  available.
- **It is a different bed from the pheromone work.** `scene=wetland 512x160
  ants=52 trees=2`, against `trailfollow`'s `LabBox` gap bed. Nothing about the
  larder findings transfers to the gap bed without being re-taken there.

**What survives regardless of the condition, because it is a code fact and not a
measurement:** `try_bud` gates on and charges `state.energy`, and nothing reads
what is in the nest neighbourhood. A pile still funds zero births *directly*, on
any bed, working loop or not. The indirect path — eat, bank, bud — is unaffected
by the correction and remains what §2's wire targets.

**What is now explicitly suspended**, and must be re-taken on a bed where ants
complete a round trip before it is quoted again: *"the pile is a flow, not a
store"*, *"87% of drops are away from the nest"*, *"the colony is the sink"*, and
the 140,202/137,945 pickup–drop identity. Each is consistent with *"ants wander
at random while holding food"*, which is precisely what §7.25 measured the
shipped animal doing.

**And one number does survive and is worth keeping in view:** 463 deliveries by
frame 4,000 against a standing pile of 16. Delivery without accumulation is real
and reproduces here; what it *means* is what the suspension is about.

### 1. What the engine already has, verified

| | state |
|---|---|
| food dropped at a nest | persists as an ordinary world cell — `world.set(dx, dy, unit.into_cell(world))`. **The worth is not destroyed.** |
| `Share` (trophallaxis) | **works**, evolvable, transfers **`energy`** downhill to `neediest_kin`, gated on `KinNeed` |
| `Feed` vs `Drop` | already compete for one tick: `choose_weighted(&[feed_urge, drop_urge], ..)` |
| `(AtNest, Drop, 1.0889)` | a heavy thumb on **Drop** at the nest |
| `Energy → Feed` | **does not exist.** Hunger does not make an ant eat. |
| digestion | happens **in the crop**; a drop before `c.unit` forfeits the progress *and* the meal |
| starvation immunity | **does not exist.** `life_half_life: 0.0` is immortal for *old age* only; the energy death is `creature.rs:11028`, `state.energy <= 0.0` |

**The floor larder already half-exists and defeats itself.** Food is dropped at
the nest and persists; any ant that picks it up there meets
`(AtNest, Drop, 1.0889)` and puts it straight back down, and nothing makes a
hungry ant eat instead. That is a thrash loop, and it is the likeliest reason
§7.27 measured 285 drops and **zero** meals.

### 2. Step one — one wire, before any mechanism

**`(Energy, Feed, -w)` in `ant.ron`.** A hungry ant eats what is beside it.

- **Why this first:** it is the missing half of a contest that already exists. If
  it breaks the thrash, the granary **already exists** and the whole defect was
  one absent wire. That is the cheapest possible outcome and it must be checked
  for before anything is built.
- **It is a gene, not a hardcode** — it mutates, so a lineage can evolve how
  hungry it has to be before it eats its cargo. Point 2 of the owner's
  considerations (below) is satisfied by construction.
- **Division of labour for free, with no castes and no age.** A *full* ant
  carries and drops; a *hungry* ant eats. Energy varies naturally across a
  colony, so **one genome produces both behaviours** — a distribution rather
  than a binary, which is the ethos's first law rather than a special case.
- **Derive `w` before running it, do not fit it.** Follow §7 step 5's own
  discipline: **write out what the unit computes** at
  `Energy ∈ {0.0, 0.25, 0.5, 1.0}` × `AtNest ∈ {0, 1}`, through `squash`, and
  check that a starving ant at the nest beats `drop_urge` while a fed one does
  not. `Feed` currently sums `(Bias, 0.4)` and `(FoodAdjacent, 0.8)`; `Drop`
  sums `(Bias, -0.2)`, `(AtNest, 1.0889)`, `(Carrying, 0.2)`,
  `(MoistureGrad, 0.169)`, `(SurfaceCurvature, 0.169)`.
- **Falsifier:** if `drops` stays high and `ate J` stays at 0 across the sweep of
  `w`, the thrash is not the drop contest and the granary is a real build.
- **Watch for the sweep trap:** `CLAUDE.md`'s *"when every setting of a sweep
  fails the same way, suspect the sweep"* — run the control that strips the
  rider, which here is §4 below.

### 3. Step two — the pheromone test, immediately

**The moment colonies survive with `home_bias` on, run `arms=self,mute`.** Not
later, not after tuning: this is the criterion in §0 and the reason the rest
exists. `hand` has answered every question it can answer.

### 4. The fallback — starvation immunity as a confound stripper

**The owner's suggestion, 2026-09-18**, and it is methodologically the right
shape rather than a shortcut: *"you could artificially make it so ants cannot
die from hunger to just check if they are following pheromones and foraging
correctly, without the confound of ants dying because they are not eating."*

This is exactly `CLAUDE.md`'s remedy for a sweep whose every setting fails the
same way — **the mechanism with every rider stripped out.** Starvation is a
rider that arrived *with* `home_bias` and is constant across every data point in
§7.27's table, which is precisely the shape that reads as "the approach is
wrong" and is not.

- **Cost:** one env-gated early return at the energy death (`creature.rs:11028`),
  measurement-only, same idiom as `SPOIL_IS_CARGO` and
  `PIXEL_PHYSICS_TROPHALLAXIS`. Small.
- **What it buys:** it separates *"the loop does not work"* from *"the loop works
  and the colony cannot afford it."* Those want completely different repairs and
  nothing measured so far can tell them apart.
- **What it costs in trust, and the guard against it:** an immortal colony is not
  a colony, so **no number taken under it may be quoted as a result** — it is a
  diagnostic arm only, and its own header must say so. The engine has been
  burned by exactly this: §7.26's `onetrail::hold_gate_laden` evaluated the
  homing circuit at `Carrying = 1.0`, a value the colony almost never reaches,
  and its +104-of-112 was read as evidence about a colony for weeks.
- **When to reach for it:** if step 1 and step 2 both fail. Not before — it is a
  scalpel for a confound, and reaching for it early would hide the economy
  problem rather than isolate it.

### 5. The granary proper — only if steps 1–2 fail

**What it is:** food delivered to a nest accumulates as a *visible, persistent,
spatial* store that the colony eats from.

**Why a store and not ant-to-ant sharing, which was the first recommendation and
was wrong.** The owner's objection, and it is decisive: *"I don't know if that
would be visible cuz they're very tiny and there's many of them. Can I tell the
difference between them sharing and just standing or walking next to each
other?"* **No.** An ant is two cells; trophallaxis is two ants adjacent for a few
ticks, which is pixel-for-pixel identical to two ants passing. It would need an
invented render marker. This repo has already paid for that mistake once —
`Reports/plant-appearance-design.md`, where three levers all fired, all counted,
and moved nothing on screen because they only changed *which cell got a label*.
**A granary changes the silhouette of the nest**, which is a *what and where*, the
one thing an image can answer.

**The biology supports the store, and the earlier claim that it did not was
wrong.** Trophallaxis is the **liquid** pathway. Solid food is stored or fed to
brood, and a physical store is the *more* general pattern across superorganisms,
not the less: harvester-ant seed granaries, leafcutter fungus gardens, honeybee
comb, termite fungus combs, honeypot repletes. **"A superorganism accumulates a
visible store at its home site" generalises across ants, bees, wasps and
termites; trophallaxis is narrower.** That satisfies the owner's requirement
that this not be hardcoded for ants.

**Keep `Share`.** Energy-sharing already works and is already evolvable. The two
are complementary and match the real division: **solid food → store; digested
energy → share.** The granary is an addition, not a replacement.

### 6. Requirements on the granary, from the owner's considerations

1. **It must work, which means it must not be complicated.** Ranked first by the
   owner. This is why steps 1 and 2 come before any of §5 — the cheapest thing
   that could possibly work is a wire, and it has not been tried.
2. **Biology as inspiration, not as hardcoding.** The mechanism must be "a
   creature with a crop, a `Drop` and a home marker accumulates a store", with
   the strengths as *genes*. No ant-shaped special case; the lab's other species
   and the held world's creatures must get it for free or it is wrong.
3. **The foraging loop must be visible.** Judged by eye, not by counter. A pile
   that grows and shrinks is the deliverable.
4. **A store is a target.** A visible pile at a nest is something beetles and
   rival colonies can raid — free emergent drama, biologically real, and it turns
   storage from bookkeeping into a **stake**. Design toward it even if the first
   build does not include it.
5. **A store with no sink is a hoard.** If food only accumulates the colony
   solves hunger for ever and the tension dies. Bound it with brood consumption,
   spoilage and season — `spoil`, `rot_remains` and weather all already exist. A
   pile that *shrinks when neglected* is also more visible, not less, and is the
   ethos's first law again: graded, not binary.
6. **It recalibrates the whole economy, and that is where the time will go.**
   Banking food makes starvation rare, which moves birth rates, which moves every
   constant tuned against the current economy. `CLAUDE.md`: *a correct mechanism
   at inherited constants is a regression.* **Name the constants before starting
   or the change is not scoped, it is merely started.**
7. **The success metric has to exist before the mechanism.** With a store, food
   can be delivered and not yet eaten, so `ate J` will **under-report a working
   loop** — the §7.25 trap exactly, a number that is arithmetically right and
   about the wrong question. Build *stored cells at the nest*, and its turnover
   rate, **first**.
8. **This is `engine`, not `lab`.** It lands in the outdoor game, the lab and the
   held world at once. `Reports/two-games-one-repo-2026-08-30.md` before
   proposing any scoping of it.

### 7. What not to build

- **Crop-sharing as the primary loop.** Invisible at play scale (§5). It may
  still be worth having *behind* a granary, for the liquid pathway.
- **Caste or age-based division of labour.** Real, and real complexity. §2's
  hunger split gives the same population effect from one genome and no new state.
  Revisit only if the hunger split provably cannot produce two groups.
- **A larger `home_bias` sweep before the economy is fixed.** Every row in
  §7.27's table is conditional on an economy that punishes commuting; re-running
  it finer measures the economy, not the dial.

## §7.29 291 ticks to digest, 4 ticks held at the nest — the arithmetic that kills step 1

**2026-09-18.** §7.28 planned `(Energy, Feed, -w)` as the cheap first move and
said to derive `w` through `eval_brain` before running anything. Derived, and
**the step is falsified without a single sweep** — along with a better answer than
the one it was looking for. `trailfollow mode=feedgate`.

### The two numbers

**A `fruit` cell needs 291 ticks in a crop.** `digest_rate: 3.3` per tick against
a cell `unit` of 960 J, absorbed only when `digesting` reaches `unit`. At the
ant's 6-frame tick that is **1,745 frames**.

**At the nest a cell is held for 4.** P(drop) per tick is **0.2507** with
`AtNest = 1`, and **0.0000** without it.

So an ant that walks home **puts its food down about seventy times sooner than it
could ever absorb it**, and the drop discards `digesting` entirely. Going home is
fatal to the meal, quantitatively, and this is the whole of §7.27's
"100% of carries end in a drop and none in a meal."

It also explains the shipped animal's survival: with `home_bias: 0.0` an ant
wanders, rarely touches nest, and P(drop) away from nest is nil — so it holds the
cell the ~291 ticks it needs and eats. **The colony lives by NOT delivering.**

### Dropping is two gates, and reading one of them overstates it 4x

`creature::act` sets `prefer_drop` from
`choose_weighted(&[feed_urge, drop_urge], 0.1, ..)` and **then rolls again**
against `drop_urge` itself (`let p = drop_urge; if draw.unit_f32() < p`). The
per-tick probability is the **product**.

The first draft of this readout printed only the contest and reported 0.4809 at
the nest where the truth is 0.2507. Recorded because the error is the standing
one: **a number that is arithmetically right and one step short of the decision**
— and it was caught by reading `act` again rather than by anything going wrong.

### Why `(Energy, Feed, -w)` cannot do the job

`Energy` is *fullness*, `state.energy / start_energy`, so the wire contributes
`-w × Energy` — **zero at E = 0**. It cannot raise a hungry ant's feed urge,
because at the point of maximum hunger the term vanishes. What it does instead is
*suppress* feeding when full, which pushes P(drop) at the nest **up**:

```
       w   AtNest    E=0.00    E=0.25    E=0.50    E=0.75    E=1.00
    0.00      yes    0.2507    0.2507    0.2507    0.2507    0.2507
    1.00      yes    0.2507    0.2753    0.3106    0.3629    0.4402
    3.00      yes    0.2507    0.3629    0.5081    0.5081    0.5081
```

**Every row is identical at E = 0.00.** No setting of `w` moves a starving ant by
one digit, and the whole point of the step was to move exactly that ant. A
positive `Bias` term would move the baseline, but it would move it for *every*
ant at every fullness, which is not a hunger response at all.

`CLAUDE.md`'s *check that a planned step can demonstrate itself before promising
it will* — **which cell does this rule actually evaluate?** The answer was "a full
ant", and the step was written for a hungry one. One readout instead of a lane.

### What the numbers say to build instead

The gap is **291 against 4**, and only three things close it:

1. **`digesting` survives a drop and resumes on re-ingest.** Then delivery stops
   destroying the meal, cumulative nibbling works, and *a pile at the nest is
   digestible by the colony over many visits rather than by one ant in one
   sitting*. **This is the granary, and the arithmetic says it is the right
   shape** — it is a change to `Crop`, not a new verb, a flag or a trait (which
   `TRAIT_STORE_IN_BODY`'s rejection forbids).
2. **A nest-side consumption path** — `larder-reachability` §6 item 1, a birth
   payable from a nest-adjacent store. Larger, and it is the one that makes the
   pile *mean* something rather than merely survive.
3. **Re-derive `digest_rate` or `crop_capacity`.** Cheapest to type and the
   worst-grounded: it moves a constant calibrated against a world where nobody
   delivers, and 70x is not a tuning distance.

**Nothing here needs `home_bias` turned down.** The steering was never the
problem; the 4-tick hold at the nest is.

### One caveat on this readout, stated rather than left for the next reader

The synthetic inputs set `MoistureGrad` and `SurfaceCurvature` to **0**, and
`ant.ron` authors both into `Drop` at 0.169. A real world does not. So
**"0.0000 away from the nest" is a floor, not a field value** — colony-scale data
records 137,945 drops, most of them away from nest, which those two terms and a
real gradient account for. The nest figure is the one to trust, because `AtNest`
dominates it at 1.0889.

## §7.30 `gap=` is not the journey — the ants start 40 cells nearer than it says

**2026-09-18.** Owner: *"Make sure the ants are being placed far enough away from
the food. They place in a spread and we have results earlier where they were
being placed closer to the food than expected."* Checked, and it is real,
constant, and was invisible in every row this harness has ever printed.

### The founders are a band, and `gap` measures from its centre

`found_colony_of` lays the colony in a band about `ants * 4` wide **centred on
the nest**, and `gap` is the **nest**-to-food distance. At `ants=20` the band's
food-side edge sits a constant 40 cells in front of the nest, so:

| nominal `gap` | nest | food | nearest founder | farthest | nearest as % of nominal |
|---|---|---|---|---|---|
| 60 | 48 | 108 | **20** | 96 | 33% |
| 90 | 48 | 138 | **50** | 126 | 56% |
| 140 | 48 | 188 | **100** | 176 | 71% |
| 200 | 48 | 248 | **160** | 236 | 80% |

**The shortfall is a constant 40 cells, not a constant fraction**, so it distorts
short gaps worst. At `gap=60` with `near=10` the nearest ant starts **ten cells
from the food's edge** — that is not a foraging journey, it is a standing start.

**The existing assertion does not catch this and is not meant to.** It fires only
when a founder lands *on* the larder (`fh < target_x - near`), which is the
extreme case fixed in the 2026-09-16 box-widening. Between "on it" and "a gap
away" lies the whole range above, and nothing printed it. **It prints now**, on
the `founded` line, as nest, food, nearest and farthest against the nominal.

**The corpse half of the owner's warning is already guarded and did hold**:
`ate_other_j` is asserted to **0** on every row when `onlyfood=on`
(`trailfollow.rs:2346`), so no run in §7.25–§7.29 was feeding on its own dead. A
hard assertion rather than a column nobody reads, which is the right shape.

### And the bed has a narrow working range, which nobody had measured

Same run, `arms=hand`, 3 seeds, `stop=6000`:

| nominal gap | nearest founder | colonies alive | `ate J`, best seed | `arrive@` |
|---|---|---|---|---|
| 60 | 20 | **1 of 3** | 1,004,726 | 330 / 150 / 90 |
| 90 | 50 | **2 of 3** | 904,047 | 804 / 600 / 564 |
| 140 | 100 | **0 of 3** | 5,117 | 2,532 / 1,680 / 2,016 |
| 200 | 160 | **0 of 3** | **0** | 5,124 / 3,984 / **0** |

**Past about 50 cells of real founder distance the colony dies in every seed and
eats essentially nothing**, and at `gap=200` one seed never reaches the food at
all (`arrive@ 0`). `trailfollow.rs:980` says *"the distance at which a food trail
is both necessary and survivable is somewhere between, and nobody has swept it"*
— this is that sweep, and the answer is that the survivable window closes between
a real 50 and a real 100.

**Three seeds is not a sweep** (`CLAUDE.md`), and the per-seed scatter here is
enormous — gap 60 reads 1,004,726 J on seed 1 against 5,520 and 3,120 on the
other two. Treat the 60/90 rows as indicative. The **6 of 6 deaths across 140 and
200** are firmer, being the same shape as §7.27's twelve-of-twelve.

### What this constrains — and the conclusion this section first drew was BACKWARDS

**`gap=60` is not a shorter journey to compare against; it is a standing start**,
and that part stands: 20 real cells against a food radius of 10.

> ⚠️ **CORRECTED 2026-09-18, by the owner, the same day it was written.** This
> section first concluded that *"the `arms=self` criterion can only be asked
> where a colony survives at all, which is `gap=90`"*, and that **a gap sweep
> cannot be used until the colony stops dying.** Both are the wrong way round.
>
> Owner: *"They are not surviving at larger distances because we haven't solved
> the pheromone loop."* **The deaths at 140 and 200 are the symptom under
> investigation, not a bed limitation that blocks investigating it.** A lone
> scout cannot keep a colony alive a hundred cells out — 3.7% of ants reach food
> unaided (§3.2) — and a *recruited column* can. That is the entire claim of the
> stigmergy mechanism.
>
> So **survival at 140 and 200 is the success signal**, and a run that tests only
> 90 has removed the outcome it is looking for. `gaps=90,140,200` is now
> `trailfollow`'s default rather than something to remember.
>
> The error is worth naming because it is a general one: *a measurement taken
> where the mechanism is absent was read as a property of the apparatus.* The
> same shape as §7.28b's suspended larder findings, one section apart, which is
> how easily it recurs.

What the founder-distance finding does still constrain is **arithmetic, not
scope**: quote real founder distance, never nominal `gap`, because the two differ
by a constant 40 cells and the difference is largest exactly where the numbers
flatter most.

**Quote real founder distance, never nominal `gap`,** in anything downstream. The
two differ by 40 cells and the difference is largest exactly where the numbers are
most flattering.

## §7.31 The self arm's trail is mostly dirt — and a returning ant does lay a real one

**2026-09-18.** Owner: *"2 is dependent on 3. Ants lay the trail when they are
returning. If they don't return they cannot lay their own trail."* Correct, and
the framing in the previous message had it as "we are solving 2" with 3 as the
path, which invites exactly the drift this line keeps having. **The goal is 2;
the active work is 3; and 2 is the test of whether 3 worked.**

This is the first direct measurement of that dependency, and it was available
without fixing anything.

### The self arm can be read from frame 0, which nothing had noticed

`route pk` waits for `stop + 1500` **only when this harness lays a trail of its
own** — `ours_is_down = trail || paint != PaintA::None`, and the `self` arm is
`("self", false, false, PaintA::None)`. So on `self` the window is open from
frame 0 and samples every 100 frames. **No fix to the digestion problem was
needed to ask this**; the colonies die around frame 7,600 and still give ~76
samples, and `route pk` is a peak.

### Most of what the colony lays is dig tailings

`arms=self,mute`, 6 seeds, gap 90, against the same run with `SPOIL_IS_CARGO=0`:

| seed | route pk, spoil ON | spoil OFF | B profile ON | B profile OFF |
|---|---|---|---|---|
| 1 | 33 | **11** | [0,12,0,0,0] | [0,0,0,0,0] |
| 2 | 22 | **0** | [3,3,0,0,0] | [0,0,0,0,0] |
| 3 | 26 | **0** | [0,8,0,0,0] | [0,0,0,0,0] |
| 4 | 84 | 55 | [0,10,17,16,1] | **[0,0,119,16,1]** |
| 5 | 52 | 30 | [4,34,0,0,18] | **[0,0,0,76,0]** |
| 6 | 31 | **0** | [0,4,0,0,0] | [0,0,0,0,0] |

**In every seed where no ant ever reached the food, silencing spoil takes the
trail to exactly zero.** `Carrying` is `crop_fill.max(spoil ? 1.0 : 0.0)` and
`(Carrying, EmitB, 2.5)` is channel B's only emitter, so **an ant holding dig
tailings lays food-trail pheromone**, and it does it where it digs — the nest.
Seeds 2, 3 and 6 were laying a puddle of dirt-scent at their own door and
scoring it as a trail.

### And where an ant did return, the trail is real — and sharper without the dirt

The three seeds with a finder (1, 4, 5 — `arrive@` 4914, 1416, 3018) keep their
trail with spoil silenced, and **the mass moves out of the nest bands into the
route**: seed 4's band 2 goes **17 → 119**, seed 5's band 3 goes **0 → 76**.
The food trail was being *masked* by the nest-side puddle, not produced by it.
(Consistent with §7.24: silencing spoil makes ants hold cargo rather than drop
it, so there are more laden ticks along the route to lay from. The two arms are
different worlds, not one world measured twice.)

**So a returning ant does lay a route trail.** Part 3 feeds part 2, measured
rather than argued, and the owner's ordering is the right one.

### The real bottleneck on this arm is discovery, and it is severe

Every self-arm colony died, at every gap, in both `home_bias` arms. `ate J` is
**0** in all six seeds and `trips` is **0**. The reason is upstream of everything
this line has been working on:

| | seed 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| distinct ants ever reaching food, of 20 | 0 | 1 | 0 | **1** | **2** | 0 |
| `arrive@` | 0 | 3480 | 0 | 1416 | 4494 | 0 |

**Three of six colonies never find the food at all**, and the best manages 2 ants
of 20. One ant reaching a larder once cannot lay a 90-cell trail, so the return
leg has almost nothing to act on here. `home_bias` did visibly work on the one
seed that had a finder — seed 4's `carry@nest` goes **24 → 924**, 38x — and it
changed no other seed, because there was nothing to change.

This is §3.2's *"discovery is the binding constraint"* with the second clause the
master already added (*"discovery binds for one ant; the loop needs the return
leg"*) — **and on the `self` arm the first clause binds so hard the second cannot
be tested.**

### Two corrections to §7.28's criterion, which this invalidates as written

1. **The baseline is wrong.** §0 sets the bar at *"`route pk` rises off 3.4 of
   90"*. The shipped self arm reads **22–84** here, not 3.4. Whatever
   configuration produced 3.4 is not this one, and a criterion whose baseline
   does not reproduce cannot be passed or failed.
2. **`route pk` is the wrong instrument.** It counts cells between nest and food
   holding any channel B, so **a 30-cell puddle at the nest scores 30** — which
   is exactly what seeds 2, 3 and 6 did while never seeing food. **The criterion
   must read the outer bands of the `B nest->food` profile**, which separates a
   route from a door-step, and it should be taken at `SPOIL_IS_CARGO=0` or the
   number is part dirt.

## §7.32 The compass was built weeks ago and wired to nothing

**2026-09-18.** Four questions from the owner, and two of them overturn a thing
this line has been reasoning from.

### The odometer is alive, and `brain::what_an_odometer_emits` reads as if it is not

The readout's first row prints

```
authored (dead: w_in < W_EPS)   emit 0.000 -> 0.000   rms 0.385
```

and it is a **historical control, not the current animal.** That row hardcodes
`w_in = 0.0005`, the §Z5 value that sat under `W_EPS`. `ant.ron` authors
`(AtNest, 4, 0.05)` — fifty times `W_EPS` — with recurrence `0.99995` and
`(4, EmitA, 32.0)`. The odometer charges at the nest and decays with time away,
exactly as designed.

**Read the first row as the present state and you conclude channel A has no
ramp at all**, which is wrong and was one sentence away from being written down
here. The row deserves a name that says *was*, not *is*.

### But the plane it builds is not nest-tall, and the reason is the population

Surviving colony, `arms=hand`, gap 90, endless larder — channel A nest→food:

```
end nest->food [0, 194, 1034, 7323, 1520]
```

**It peaks in the third band, not at the nest.** The odometer makes each ant lay
more when freshly home; with `AtNest` at **1.37%** almost no ant is freshly home,
and the plane integrates where the bodies are. *The mechanism works and the
population defeats it.*

### The gate fix moved that, measurably, and did not finish it

Two surviving colonies, one per gate:

| | food:nest occupancy | `AtNest` |
|---|---|---|
| old gate (`Carrying`, threshold 0.989) | **111 : 1** | 0.22% |
| new gate (`CarryingFood`, boolean) | **27 : 1** | **1.37%** |

The owner's prediction — *"ants were milling about the food because their gate
never opened; we might not have the blob now"* — is **right in direction and
incomplete in degree**. The blob shrank fourfold and time at home rose sixfold.
It is still 27 to 1.

### The compass exists, has shipped for weeks, and nothing steered with it

`OrganismState::forage_anchor` is a home vector: set at spawn, **re-anchored at
every nest contact**, so it cannot drift and needs no integrator. `creature.rs`
says what it was for, at the site that maintains it:

> *"**Measurement only** — nothing downstream reads it, and an ant still has no
> idea where home is."*

Every consumer was telemetry — the reach histogram in `app.rs`, the lab roster's
RANGE row and `forage_max` percentiles, one `params.rs` row. **No simulation code
read it until `home_bias`** (`creature.rs:11351`), which is the first thing in
this engine to steer a body with it.

**So path integration was half-built.** The hard half — a drift-free home vector
with no accumulation to desynchronise — has been sitting in the tree as a debug
readout, and the walk home was never wired to it. `home_bias` is not a
workaround for missing path integration; **it is the missing half of it.**

### What real ants do, and why it retires the plan to fix channel A

Real foragers home by **path integration**: a private home vector, accumulated
from their own movement, run straight from anywhere with no trail at all. The
pheromone trail is a *contextual* cue — "you are on a known route" — and it is
**isotropic**: a real trail carries no direction.

This engine asks a concentration gradient to be a compass, which no real trail
is. That is `pheromone-master` §1's structural statement reached from the
biology: *"Channel A and channel B have the same laying rule and need opposite
ones."* Channel A cannot be tuned into a compass, because the thing it is
modelled on is not one.

**The consequence for the plan: stop trying to make channel A point home.** The
compass is `forage_anchor`; channel A's job is the contextual one. Every §7.23
repair aimed at the ramp's polarity — and §7.19's whole retracted metric — was
work on a signal that was never going to carry direction.

## §7.33 Ants do not follow the hand-laid trail, and the reader pair is why

**2026-09-18.** Owner: *"I would also recommend reading a few ants' exact brain
decisions at each tick, unless the results are very clean."* They were not clean
— one seed of eighteen runs — so this reads one ant, per tick, and the answer is
neither homing nor the gate.

### The sweep that prompted it

`arms=hand`, `gate=shipped`, endless larder, trail off at 6,000, gaps 90/140/200,
6 seeds, `home_bias` ∈ {0, 0.25, 0.5, 1.0}:

| gap | hb 0 | 0.25 | 0.5 | 1.0 |
|---|---|---|---|---|
| 90 | 0/6, 0 trips | 0/6, 1 | 0/6, 0 | **1/6, 23 trips, 13.7M J** |
| 140 | 0/6 | 0/6 | 0/6 | 0/6 |
| 200 | 0/6 | 0/6 | 0/6 | 0/6 |

`home_bias: 1.0` turned seed 6 from dead into **480 alive, 12,131 of 12,499 ants
reaching food, 22 trips**. The same seed at 0 is dead with 2 visitors. One seed is
not a result; it is a reason to look at a tick.

### Two instrument gaps had to close first, and the first is the serious one

**The focal ant was chosen from larder-carriers only.** So in a seed where nobody
reaches the food there was no focal ant and the CSV was empty: **the per-tick
instrument could see every run except the ones that fail.** `focalany` now takes
the first ant seen, carrying or not, and `focalx=N` takes the one nearest a
column — because the first ant is the westernmost, and the west of this colony is
somewhere the question does not live.

The row also carried only the homing half (`PheroAAlong`, h0/h1), so it could not
answer *"why did this ant not follow the food trail"*. It now carries
`CarryingFood`, `crop_cells`, `heading`, `PheroBAlong`, `PheroBFront`, h2, h3 and
`p_tumble` beside `p_move`.

### The trail does not cover half the colony

The hand-laid trail runs `nest_x..=target_x` = **48 to 138**. Founders span
**12 to 88**. **Every ant founded west of 48 starts off the trail entirely.**
Traced, the westernmost ant lived its whole life at **x 4–15** and sensed channel
B on **0 of 3,875 ticks**. It is not failing to follow a trail; there is no trail
where it stands.

That is worth fixing, and it is not the main fault.

### An ant standing ON the trail does not follow it either

`focalx=85` — a founder inside the trail's span, seed 2, the same run:

- sensed channel B on **4,061 of 4,061 ticks**, mean `|PheroBAlong|` **0.585**;
- lived its whole life at **x 56–85**, never passing 85 toward food at 138;
- faced **down**-gradient on 2,652 ticks against **up** on 1,379, nearly 2:1 away
  from the food.

And the response to that full-strength signal:

```
facing UP-gradient (toward food)   n 1379   mean p_move 0.7622
facing DOWN-gradient               n 2652   mean p_move 0.7244
                                            difference  +0.0378
```

**A 0.585 signal buys a 0.038 change in the chance of stepping.**

### Why: units 2/3 are still saturated, and only units 0/1 were ever repaired

`ant.ron` gates the food-trail reader `Bias +45, CarryingFood -75`. For an **empty**
ant the sum is `45 + 6·along`, and `squash(x) = x/(1+|x|)` is flat there:

```
along +1 -> squash(51) = 0.98077
along -1 -> squash(39) = 0.97500
spread                    0.00577
across the antisymmetric pair into Move at +-2.5:  0.0288
```

**Predicted 0.0288 against a measured 0.0378** — the same mechanism, with the
other `Move` terms making up the remainder.

This is **§Z7's saturation, alive in the shipped animal.** The homing pair (units
0/1) was moved to a `+0.5` on-state on 2026-09-09; **units 2/3 never were.** The
food-trail reader has been parked in the state the repair was written for, for
nine days, and every "the colony will not follow a trail" result on this bed sits
downstream of it.

### It is a `dead-ends.md` re-test, and the condition it was rejected under is gone

Re-gating units 2/3 was built and rejected 2026-09-09: *"the repair is correct,
it does exactly what its arithmetic promises, and it makes the animal decisively
worse."* Its recorded re-test condition is **"what a food trail is worth in this
bed, not the gate"** — and three things in that bed have since changed:

1. the homing gate opened at all (§7.22 → today: 1.84% → 100%);
2. the sensor stopped calling dirt cargo (§7.31), so channel B is no longer
   laid at the nest by diggers;
3. `home_bias` gave laden ants a way home that does not depend on channel A.

**Do not re-run it as it was.** The 2026-09-09 arm re-gated 2/3 against a colony
that could not return, on a sensor that laid food-scent while digging. Re-test it
on today's animal, and read the per-tick `p_move` split above as the acceptance
number rather than survivor counts — it is the quantity the repair is about.

## §7.34 The reader repair works, and the ant starves holding its meal

**2026-09-18.** §7.33 found the food-trail reader saturated. This is the repair,
and one ant's whole biography under it.

### The owner's argument that unblocked it

`dead-ends.md` rejected re-gating units 2/3 on 2026-09-09 — *"correct, and makes
the animal decisively worse"* — because a working reader sends empty ants to
patches the colony has already eaten. The owner's objection, 2026-09-18:

> *"only laden ants lay channel B, so once the patch depletes, they should stop
> laying the path. If they don't that is a separate fix. This again sounds like a
> multi-step fix that we are not trying because it failed at step 1."*

**Both halves check out.** `ant.ron` has exactly one channel-B emitter,
`(CarryingFood, EmitB, 2.5)`, and `pherolife` measures a laid trail **gone at
1,476 frames**. So a depleted patch stops being marked and its trail dies: the
self-limiting behaviour is already in the emitter. The rejected step was rejected
for a downstream consequence that has its own remedy.

### The repair, and it is four numbers

`(Bias, 2|3, 45.0) → 0.5` and `(CarryingFood, 2|3, -75.0) → -45.5`, mirroring
what units 0/1 got on 2026-09-09.

| empty ant, sum = `Bias + 6·along` | squash spread | into `Move` |
|---|---|---|
| `Bias +45` (shipped) | 0.00577 | +0.029 |
| `Bias +0.5` (repaired) | **1.71282** | **+8.564** |

A laden ant stays shut — `0.5 − 45.5 = −45`, leak 0.00577, identical to the
homing pair's shut state — so an ant carrying food still ignores the food trail.

### It does what the arithmetic promised, on the same ant, same seed, same bed

| | saturated | de-saturated |
|---|---|---|
| `p_move` facing toward food | 0.7622 | 0.7580 |
| `p_move` facing away | 0.7244 | **0.3023** |
| **difference** | **+0.0378** | **+0.4556** |
| x range (nest 48, food 138) | 56–85 | **84–133** |
| ticks lived | 4,061 | 8,891 |

**Twelve times the trail response, and the ant walks to the food** instead of
milling fifty cells short. The return leg moves too: **+0.0289 cells/tick toward
the nest while laden, against §7.20's shipped +0.0002 — 145x.**

### And then it starves, holding the food

The owner asked what happened at x=102. The last row of the trace answers it:

```
frame 8891  x 102  CarryingFood 1.0  crop_cells 1  Energy 0.0028
```

**It died carrying a fruit cell.** Not a drop, not a turn — it ran out of energy
on the way home with its meal in its crop. Its whole life:

| | |
|---|---|
| picked food up | **17 times** |
| lost it again | **16** |
| digestions that credited energy | **2** |
| laden ticks within the nest band | **0** |
| `Energy`, first pickup → death | 0.5076 → 0.0028 over 6,023 ticks |
| longest unbroken laden run | 570 ticks (291 needed for one 960 J cell) |

**Seventeen pickups, two meals.** It held long enough to digest most of them —
570 against 291 — and fifteen of seventeen ended with the cell back on the ground
before the timer finished, each forfeiting the progress *and* the meal. It never
once reached the nest while carrying.

This is §7.29's 291-against-4 in one animal's biography rather than as an
aggregate, and §7.27's wall reached from the other side: **carrying and eating
are the same act, so a forager that commits to the journey starves holding its
cargo.**

### Colony level, and it is not resolvable at six seeds

| gap | saturated hb0 | repaired hb0 | saturated hb1.0 | repaired hb1.0 |
|---|---|---|---|---|
| 90 | 0/6, 0 trips | **2/6**, 0 | 1/6, **23 trips** | 0/6, 2 |
| 140 | 0/6 | **1/6** | 0/6 | 0/6 |
| 200 | 0/6 | 0/6 | 0/6 | 0/6 |

Survival improves without `home_bias` (0→2 at gap 90, 0→1 at 140) and the
`home_bias` arm's 23 trips came from **one seed**. Six seeds cannot separate
these on a bed whose per-seed spread is this wide; the per-tick numbers can, and
they are unambiguous. **Do not read this table as an effect in either
direction.**

### What is left

Exactly one thing, and it is the same one from both directions: **a forager
cannot afford the journey while carrying and eating are one act.** The candidates
are unchanged — `digesting` surviving a drop so cumulative nibbling works, or a
nest that can be delivered into.

## §7.35 The ant drops food *on the pile*, because a pile is curved ground

**2026-09-18.** Owner: *"Why does the ant drop the food 16 times on its way back
to the nest?"* It does not. It never drops on the way back, and finding that out
corrects §7.34's own reading.

### The correction first

§7.34 counted 17 pickups against 16 losses and called them drops **on the return
leg**. The count was right and the location was assumed. Logged properly — the
tick before each loss:

```
frame 3077  x 122  drop_urge 0.08540  Moist 0.11368  Curv 0.83333  AtNest 0.0000
frame 3767  x 116  drop_urge 0.01990  Moist 0.09797  Curv 0.41667  AtNest 0.0000
frame 5789  x 130  drop_urge 0.03890  Moist 0.21733  Curv 0.41667  AtNest 0.0000
frame 6113  x 128  drop_urge 0.11321  Moist 0.31655  Curv 0.83333  AtNest 0.0000
        ... 16 of 16, all x 116-130, AtNest 0.0000 every time ...
```

**Every loss is at x 116–130 with `AtNest` exactly 0.** The food sits at 138 with
`near=10`, so that band *is* the larder. None of them is a completed digestion
either — no energy credited at any of the sixteen.

**This also means §7.29's `(AtNest, Drop, 1.0889)` story, true as arithmetic, is
not what killed this ant.** It never reached the nest carrying anything — 0 laden
ticks in the nest band across its whole life — so the nest-drop wire never fired
for it. The 4-ticks-at-the-nest figure still describes an ant that *gets* home;
this one never did.

### What actually fires

`ant.ron` authors five wires into `Drop`, and away from the nest only three can
move it:

```
(Bias, Drop, -0.2)  (Carrying, Drop, 0.2)
(MoistureGrad, Drop, 0.169)   (SurfaceCurvature, Drop, 0.169)
```

At the heap, `SurfaceCurvature` reads **0.417–0.833** against a mean of **0.441
at x ≥ 115 versus 0.361 elsewhere**, and `drop_urge` lands at **0.019–0.113 per
tick**. Compounded, that gives a **mean hold of 104 ticks against the 291 one
960 J cell needs to digest** — so most pickups are put down before they can ever
pay. The ant's longest hold was 570 and did digest; sixteen shorter ones did not.

**`mode=feedgate` could not have found this.** It sets `MoistureGrad` and
`SurfaceCurvature` to zero and reports `P(drop) = 0.0000` away from the nest —
the caveat §7.29 recorded as *"a floor, not a field value"*. The two terms it
zeroes are precisely the two that fire at a food pile. A synthetic readout cannot
see a term whose whole value comes from the terrain.

### The mechanism is a wiring mismatch, not a tuning value

Curvature-drives-drop exists so a **builder** puts material down on uneven
ground, and it is correctly wired to `DropSpoil` (`SurfaceCurvature, DropSpoil,
0.169`). It is **also** on `Drop`, which carries food. **A food heap is curved
ground by construction**, so the larder triggers the put-it-down reflex of an ant
standing on it. The forager picks a cell off the pile, carries it a few cells,
and puts it back on the pile.

### The candidate, and it is one number

**Remove `(SurfaceCurvature, Drop, 0.169)`**, keeping it on `DropSpoil` where the
construction argument holds. `MoistureGrad` is the same shape and worth testing
separately — at the heap it reads 0.10–0.32, a smaller contribution than
curvature's 0.42–0.83, so curvature is the one to move first.

**Do not read this as the whole return-leg fix.** It buys longer holds at the
pile, which is necessary for a forager to leave with something and sufficient for
nothing. §7.34's wall stands: carrying and eating are one act, so even a
successful carry starves the carrier unless it can digest on the move or deliver
into something.

## §7.36 Digestion survives a drop, and the journey gets 21 cells longer

**2026-09-18.** Two steps, measured separately, both on the owner's instruction
(*"do curvature and moisture and then crop change"*).

### Step 1 — `Drop` stops reading the terrain

`(SurfaceCurvature, Drop, 0.169)` and `(MoistureGrad, Drop, 0.169)` removed;
**both kept on `DropSpoil`**, where the construction argument holds. Same ant,
same seed, same bed:

| | terrain drops on | removed |
|---|---|---|
| pickups / losses | 17 / 16 | **2 / 1** |
| mean hold | 104 **frames** | **1,464 frames** |
| longest hold | 570 | 1,740 |
| digestions | 2 | **32** |
| frames lived | 8,891 | 16,985 |
| died at x (nest 48) | 102 | 91 |

> **CORRECTED 2026-09-19 — these columns are FRAMES, and this section called
> them ticks.** The row beneath the table read *"Mean hold goes to five times
> the 291 ticks one 960 J cell needs"*, and it is wrong by
> `CreatureDef::tick_interval`, which is **6** for the ant. The focal-ant CSV
> writes one row per **frame** — `examples/trailfollow.rs` pushes inside the
> per-frame organism sweep — while §7.29's 291 is in **ticks**, because
> `digest_rate: 3.3` is charged once per tick.
>
> One 960 J `fruit` cell therefore needs **291 ticks = 1,746 frames**. The
> post-fix mean hold of 1,464 frames is **244 ticks — less than one cell**, so
> the shipped ant holds its food for about **0.84 of a digestion**, not 5x.
>
> **Verified against two long-lived ants rather than re-derived on paper**
> (seed 1, gap 90, `hand`): one laden for 7,693 frames = 4.41 cells' worth of
> clock recorded **4** completed digestions, and one laden for 8,718 frames =
> 4.99 cells' worth recorded **5**. The model is exact.
>
> **Every duration in this section and §7.38 is in frames.** Divide by 6 before
> comparing any of them against §7.29, and see §7.42 for what the corrected
> arithmetic implies about the return leg.

Mean hold goes to **0.84 of** the 291 ticks (1,746 frames) one 960 J cell needs,
and the ant eats sixteen times more often.

### Step 2 — the digestion timer survives an empty crop

`Crop::digesting` was already carried across a drop by the `..c` update *while
cells remained*. **The loss was only ever at `left == 0`**, where the whole struct
goes `None` — "remainder and all" — and the progress dies with it.

That choice is right and stays: `crop.is_some()` must mean *is carrying*, and a
timer on an empty stomach once had `ascii` reporting 18 ants carrying when none
held a cell. So the timer gets a home outside the crop:
**`OrganismState::digest_carry: Option<(MaterialId, f32)>`**, parked when the last
cell leaves (by drop *or* by absorption) and resumed on the next ingest **of the
same material** — different material starts fresh, because the two have different
`unit` and crediting one against the other would mint joules.

| | before | after |
|---|---|---|
| x range (nest 48, food 138) | 84–133 | **63–131** |
| ticks lived | 16,985 | 17,819 |
| energy rises | 32 | 29 |
| died | x 91, **holding a cell** | x 81, **crop empty** |

**The westward reach improves by 21 cells** — 84 → 63, against a nest at 48. The
forager now gets three quarters of the way home, and dies empty rather than
starving on top of its own dinner.

### Colony level, and it is still six seeds

| gap | step 1 | step 2 |
|---|---|---|
| 90 | 2/6 alive, 4 trips | 1/6 alive, **5 trips** |
| 140 | 0/6 | 0/6 |
| 200 | 0/6 | 0/6 |

Survivors move the wrong way and trips the right way, by one each, on six seeds
of a bed whose per-seed spread runs three orders of magnitude. **That is not a
result in either direction and is not offered as one.** The per-tick numbers are.

### The counter this section asked for, and what it retracts

The paragraph that stood here said there was no "did it fire" counter for
`digest_carry`, that the 21-cell improvement was indirect evidence, and that the
counter should be built **before** these numbers were quoted as the mechanism's.
It has been built — `CreatureStats::digest_parked` / `digest_resumed` /
`digest_resumed_face`, printed by `trailfollow` as `chew parked / resumed (J)` —
and it does not support the attribution.

Same command, six seeds, `hand` arm:

| gap 90, seed | parked | resumed | resumed J | **J per resume** |
|---|---|---|---|---|
| 1 | 19 | 14 | 12 | **0.86** |
| 2 | 13 | 6 | 8 | **1.3** |

**A fruit cell is 960 J and takes 291 ticks.** The mechanism is moving **under
one joule** per resume — about a quarter of one tick of chewing, against a cell
that needs 291. Colony-wide over 24,000 frames it transfers **12 J**. That cannot
move a forager 21 cells, and the headline above is therefore a sample from the
distribution, not the mechanism: `digest_carry` changes the tick on which a cell
finishes absorbing, one changed decision tick makes a different world, and
`CLAUDE.md` is explicit that a single run against a remembered number is not a
comparison. **The 84→63 row is withdrawn as evidence for step 2.** The code is
still correct and still lands; what is withdrawn is the claim that it is what
moved the ant.

**Why it is worth so little here is the interesting half, and it is step 1's
doing.** There are two parking sites and only one of them carries real money:

- the **drop** site forfeits up to 290 ticks of chewing, and it is the site the
  17-pickups-2-meals disaster of §7.34 was made of;
- the **absorb** site parks `matured - c.unit`, the overshoot in the tick that
  crosses the threshold. That is bounded by *one tick's increment* by
  construction — it can never be more, whatever the cell is worth. At 3.3 J per
  tick a mean of 0.86 J is exactly what it should read.

Every joule in the table above is the absorb site. The drop site fired **zero
times**: `drops 0` on every seed, because step 1 took `Drop` off the terrain and
`(AtNest, Drop, 1.0889)` — the other trigger — needs an ant to *reach the nest
carrying*, which none does. **Step 1 had already closed the hole step 2 was built
for.** `digest_carry` is insurance against a loss that stopped happening one
commit earlier, and its value reappears the moment ants start arriving home
laden, which is the whole point of the exercise.

This is the shape `CLAUDE.md` names twice over: an "it fired" counter paired with
an effect counter from the far side of the call, and the effect counter saying
the mechanism fires constantly and feeds nobody. `digest_resumed` alone reads 14
and looks like a working mechanism; only the joules say otherwise.

### What is honestly not established

The loop still does not close: no seed at gap 140 or 200 has ever produced a
colony, and the forager still dies short of the nest. **Step 1 is the step that
is carrying the per-tick numbers** — mean hold 104 → 1,464 ticks, 2 → 32
digestions — and step 2 is a correctness fix whose measurable value in this bed
is 12 J.

## §7.37 Nine of twenty ants home correctly, to the wrong place

**2026-09-18.** `home_bias` aims at `forage_anchor`, and `forage_anchor` is set
to the **birth cell**. For a founder born off the nest comb that is a private,
permanently wrong home, and the mechanism then steers it there correctly.

**This section was published with two wrong numbers and is corrected in place;
the retraction is kept below because the mistake is one this file warns about.**

### The measurement

Focal ant, seed 2, gap 90, `hand` arm, `homebias=1` — the ant §7.36 traced, with
`anchor_x` and `since_nest` added to the per-tick CSV:

| `forage_anchor.x` | ticks |
|---|---|
| **84** | **17,447** |
| 70..63 (walking the comb) | 372 |

**98% of its life anchored at x 84**, its own birth cell. It reached nest
material for the first time at frame **17,448 of 17,819** — 371 frames before it
starved. Picking food up at x 138, `home_bias` scored the eight viable headings
against the vector to (84, y) and walked it **east, away from the nest**, with
the mechanism working perfectly. Its x range of 84–133 is not an ant that fell
short of home. **It is an ant that arrived.**

### How much of the colony this is — the number that decides it

Censused directly, at frame 1, over all three gaps (`trailfollow` now prints it):

```
nest cursor 48   MATERIAL x 26..70   11 of 20 founders born on it
```

**Nine of twenty founders are born off the comb**, and those nine carry a wrong
home for life. The focal ant, born at x 84, is fourteen cells east of the nest's
east edge — in that minority, not typical of the colony.

### Why

`creature.rs`'s birth block sets the anchor unconditionally:

```rust
// Starts *at* the nest as far as scent goes: an ant that has just
// hatched has, by construction, just been at home.
state.since_nest = 0;
state.forage_anchor = (x, y);
```

Right for a *hatched* ant — an egg is laid in the nest, so its birth cell is a
nest cell. False for a founder: `paint_nest_patch` lays a masked comb over
`nest_x ± COLONY_HALF_WIDTH` (26 columns each way, 25 cells with gaps between the
teeth), while `colony_stations` spreads the bodies about `ants * 4` wide — x
12..88 at twenty ants. The two spans are different and nothing reconciles them.

### The retraction, and why it is kept

The first version of this section reported **"the nest is an eight-cell band at
x 63–70"** and **"roughly 18 of 20 carry a private wrong home."** Both are wrong.
The nest spans x 26..70 and the figure is nine of twenty.

The error was to take the rows where the focal ant's `AtNest` fired, tabulate
them by x, and report that as the nest's extent. `AtNest` is honest — it is a
plain 8-neighbour read of nest material — and the table was arithmetically
correct. **It was a census of where one ant went, published as a census of where
the nest is.** That is `CLAUDE.md`'s single worst-recurring failure verbatim, and
the tell was there to be read: the table was *tidy*, eight adjacent columns in a
neat run, which is what one ant's walk looks like and not what a masked comb
looks like.

The instrument that settles it existed nowhere, which is the other half of the
lesson. `trailfollow`'s header printed **`nest 48`** — the founding *cursor* —
in the slot a reader takes for the nest's position, and no output in this repo
said where the material was. It now prints the material's span and the count of
founders born on it, because that count is the precondition every other number
in the bed is conditional on.

### What this does and does not explain

It explains the nine. It does not explain the eleven: those founders are born on
the comb, anchor correctly, and the loop still does not close for them. **So the
anchor is a real defect and not, by itself, the blocker** — and the next test is
the one that separates them, a run narrow enough that every founder starts on the
comb.

### What this says about the counter

`CreatureStats::tumbles_homeward` read 1,150 of 15,291 tumbles on this seed and
every one was a correct aim at a wrong target. That is the sharpest case yet for
`CLAUDE.md`'s pairing rule and it also defeats the pair as it stands: the "it
fired" counter and the `P(home)` effect counter beside it **both** report a
working mechanism, because the aim fired and the body moved as aimed. Only the
per-tick trace, once it carried the anchor, could see it.

### Three instrument repairs made here

- **`anchor_x` and `since_nest` are focal CSV columns.** Without them a laden ant
  walking confidently to the wrong place and one that will not steer at all
  produce identical rows.
- **The header names the nest material's span and the founders born on it**, in
  place of a bare cursor that reads as a location.
- **Every arm of a `trace` run wrote the same file.** `mode=gap` writes
  `/tmp/trailfollow-focal-seed{seed}-gap{gap}.csv` once per arm, so a four-arm run
  announces four traces and leaves one on disk — the 17,819-row `hand` trace was
  silently replaced by a 10,085-row `mute` one, and the two look equally
  plausible. Run one arm at a time until the path carries the arm name.

### The fork, undecided

- **Put the founders on the comb**, so the birth rule's assumption is true. That
  is what the experiment is meant to model — ants that hatched at home.
- **Do not anchor an ant that was not born on nest material.** The more correct
  engine rule, and it costs the experiment: an ant with no anchor never homes.

Measure before choosing.

## §7.38 The loop closes, and what was holding it was the size of the stomach

**2026-09-18.** Food reached the nest for the first time. 107 deliveries on one
seed and 37 on another, at gap 90, against **zero on every run this harness has
ever produced in its shipped configuration**.

### The number

`DELIVERED` is `CreatureStats::deliveries` — a drop the drop site saw at the
nest. It is new here because neither existing column is delivery: `drops` counts
food put down anywhere, and `carry@nest` counts *ant-ticks* spent carrying inside
a ±26 band, which is time rather than cargo. (This section's first draft quoted
`carry@nest` as "food arriving home". It is not, and the column's own definition
two hundred lines away says so.)

Six seeds, gap 90, `hand` arm, fruit larder, everything else identical:

| | shipped `crop_capacity: 1440` | `cropcap=2880` |
|---|---|---|
| drops | 0, 0, 0, 0, 0, 0 | 0, 0, 0, **107**, **37**, 0 |
| delivered | 0, 0, 0, 0, 0, 0 | 0, 0, 0, **107**, **37**, 0 |

**Every drop is a delivery.** That is not luck: `849d6d06` took `SurfaceCurvature`
and `MoistureGrad` off `Drop`, so `(AtNest, Drop, 1.0889)` is the only trigger the
verb has left and an ant now puts food down **only at the nest**. The earlier
commit was load-bearing for this one.

### Which arm these numbers are from — added 2026-09-19

**Every delivery figure in this section was measured at `homebias=1`, and
`CreatureDef::home_bias` ships at `0.0`.** The rider is not named beside the
numbers above, and a reader would take them for the shipped animal. Measured on
the same 18 seeds, gap 90, `hand` arm, one binary:

| `home_bias` | deliveries | seeds delivering |
|---|---|---|
| **shipped, 0.0** | **124** | 5/18 |
| 1.0 (the rider) | **511** | 9/18 |

**The loop does close without the rider** — the three fixes in this section are
sufficient on their own, and an ant that is never told which way home is still
gets food there by foraging until it happens to arrive. What the return leg buys
is a multiple, not the mechanism. §7.41 is the sweep that prices it, and it is
the re-take §7.27 asked for: *"Measuring a commuting rule against an economy that
punishes commuting measures the economy. Everything in the table above is
conditional on step 1 and must be re-taken after it."* Step 1 is this section.

### What was holding it

The ant could not carry more than one meal. `ant.ron` authors
`crop_capacity: 1440.0` and explains it in the same breath:

> **1440 is three leaves at the shipped table (480 each), and three is a floor
> rather than a taste.** Food only leaves the crop a whole cell at a time, so an
> ant that can hold exactly one leaf is under one leaf within a tick of ingesting
> and **can never deliver again**.

This bed's larder is **fruit at 960 J**. One cell is `0.6667` of the crop —
measured on the focal ant, whose `Carrying` read exactly 0.6667 on all 1,740
ticks it held anything. Two do not fit. Its `crop_cells` was **1 on 1,740 ticks,
0 on 16,079, and never 2 in 17,819 ticks of life.**

So the shipped ant in this bed is in precisely the state its own comment
forbids. It picks up one cell, metabolises it over the 291 ticks the walk takes
many times over, and arrives empty. `drops 0` is not a steering failure or a
trail failure. **There was never anything left to put down.**

### It is joules, not cells — which took a wrong turn to establish

The obvious repair is a 480 J food, so that three cells fit at the shipped
capacity. It does not work, and the two candidates fail differently:

| arm | cells that fit | J in a full crop | delivered (6 seeds, gap 90) |
|---|---|---|---|
| fruit 960 @ 1440 (shipped) | 1 | 960 | **0** |
| **moss 480 @ 1440** | 3 | 1,440 | **0** |
| deadleaf 480 @ 1440 | 3 | 1,440 | 32, 15 — *but see below* |
| **fruit 960 @ 2880** | 3 | 2,880 | **107, 37** |

`moss` is the clean comparison — a `Plant`, static like fruit, 480 J — and it
delivers **nothing on all six seeds** while fitting three cells. `deadleaf` fits
three cells too and does deliver, and it is not a result: **`deadleaf` is a
`Powder`.** Four hundred cells dropped at the target slump westward until the pile
meets the ants, and the run reports `visitors 0/20` with colonies surviving on
6/6 seeds — a colony that never went anywhere, fed by food that came to it. The
bed's whole geometry is gone. It reads as the strongest result in the table and
is the weakest.

So what separates the arms is **how many joules the crop holds**, not how many
cells. 1,440 J is not enough to survive the walk with a whole cell left over;
2,880 J is, on two seeds in six.

### Which means the crop was sized against a trip that does not exist

`ant.ron` derives 1440 from an assumed round trip:

> at `forage_probe`'s 87-cell gap and P(move) 0.67 a round trip is **~130 ticks**,
> and losing ~30% of a 1,440 crop over it wants ~3.3

A 30% loss over 130 ticks. The real journey in this bed is not 130 ticks — the
focal ant lived 17,819 decisions and never completed one — so the forager
metabolises **more than the whole load**, and the constant that was derived to
make the trip "visibly cost the load" instead makes it cost everything. This is
`CLAUDE.md`'s *fixing a bug often exposes a constant that was compensating for
it*, arriving from the other side: the constant is honest, its input was wrong,
and nothing downstream ever checked.

### `digest_carry` was not worthless after all

§7.36 measured `digest_carry` at **0.86 J per resume** and withdrew the claim
that it mattered, because the only site that fired was the absorb site, whose
remainder is bounded by one tick's chewing. That was right *and* it was right
about why: the drop site fires only when ants actually deliver, and none did.

In the arm where they do:

| | shipped | `cropcap=2880`, seed 4 |
|---|---|---|
| resumes | 6 | **103** |
| face value carried across | 8 J | **61,256 J** |
| per resume | 1.3 J | **~595 J** |

Three orders of magnitude, and ~595 J is most of a 960 J cell rather than a
quarter of one tick. (`digest_resumed_face` is **throughput, not stock** — a cell
parked, resumed and parked again is counted twice — so read it for its order of
magnitude, which is the whole point here.) The mechanism was not weak; it was
waiting for the loop to close.

### Still open

- **Four of six seeds deliver nothing even at 2,880**, and gaps 140 and 200
  deliver nothing at any setting. Two seeds is a result that the loop *can*
  close, not that it does.
- **2,880 is a rider, not a fix.** Doubling `crop_capacity` on the shipped ant
  reaches the lab and the held world, and `ant.ron`'s value is load-bearing for
  the reproduction arithmetic directly under it. The alternatives are a cheaper
  journey or a slower gut, and neither has been measured.
- **§7.37's anchor defect is real and is not this.** Its within-run control —
  trips by ants born on the comb against ants born off it, inside the same run —
  reads **0.0152 against 0.0122 trips per ant**. Knowing exactly where home is
  buys nothing while there is nothing to carry there.

## §7.39 The stomach grows, the appetite gate does not pay, and both are kept

**2026-09-18.** Owner's instruction: *"We will build the granary later. build the
hunger-graded gate against `reproduce_threshold` and increase `crop_capacity`."*
Both built, measured separately. One is a large win and ships on; the other is
correct, measures twice as bad, and ships **off** with the rider to turn it on.

### Why a gate was on the table at all

`digest_rate` is one scalar doing two jobs — how fast an animal feeds itself,
and how long cargo survives in its crop — because `matured += digest_rate` ran
every tick with no gate on need. `ant.ron` derives 3.3 from two brackets, and
measured against the real journey they contradict:

| bracket | wants |
|---|---|
| *the trip must visibly cost the load*, re-derived at the measured 436–873 tick leg instead of the assumed ~130 | **≤ 0.50** |
| *a child must be reachable inside a lifetime* | **≥ 2.6** |

Five to ten times apart, no overlap. **No setting of one scalar satisfies both**
— `CLAUDE.md`'s *when a rule must tell apart two things that can look identical,
state the difference as data*, which four support models failed before a bit on
the cell settled it. Appetite is that data.

### Step 1 — `crop_capacity` 1440 → 2880

The old value was three cells of a **480 J** food. Fruit is **960 J**, so the
crop held exactly one, which is the state `ant.ron`'s own comment forbids. Gap
90, `hand`, **eighteen** seeds:

| | before | after |
|---|---|---|
| deliveries | **0** | **511** |
| seeds delivering | 0/18 | **9/18** |
| median | 0 | 6 |

Zero to 511 is not a tuning result. It is the loop existing.

### Step 2 — the appetite gate, and it does not pay here

`CreatureDef::digest_hunger_weight`, `0.0` for every species that has not
authored it and bit-identical there. At 1.0 the gut scales by hunger, ramped
across `start_energy .. reproduce_threshold`.

**The first curve was wrong and measuring caught it.** `1 - energy /
reproduce_threshold` reads **0.82** for an animal at exactly `start_energy`, so a
subsistence ant paid an 18% cut to its intake while holding no surplus to
protect: deliveries 144 → 36 and survivors 8 → 0 over six seeds. Ramping from
`start_energy` instead — full rate at or below subsistence, falling to zero at
the bar — took it to 46 and 2. The mechanism was right; the curve was not.

Then the honest sweep, because six seeds is not a sweep and the first six were
unrepresentative — they showed 144 deliveries where eighteen show 511:

| gap 90, 18 seeds | gate off | gate on |
|---|---|---|
| deliveries | **511** | 255 |
| median | 6 | **0** |
| seeds delivering | 9/18 | 7/18 |
| survivors | 59 | 29 |

Robust: drop each arm's best seed and it is 392 against 199. The survivor column
is **not** a result — seed 10 alone carries 37 of the 59, a colony that ate
78,644 J and delivered nothing; without it, 22 against 21.

### Why it cannot pay in this bed, and the counterweight nobody priced

The gate withheld about **1%** of the gut's throughput, because almost no ant in
this bed ever gets above `start_energy`. It has no surplus to protect and still
costs the few ants that do.

And the engine already charges for carrying, **twice**: `carried_cells` bills
`move_cost_per_cell` for whatever is in the crop, and crop fill lowers `P(move)`
— measured as the *only* thing fill did, over 570,660 laden decisions (§7.25).
So protecting cargo keeps the ant heavy and slow for longer, and a slower ant
has a longer journey, which is the quantity the gate exists to survive. **Not
confirmed** — it is the mechanism that fits, and it is exactly the shape of
`CLAUDE.md`'s *a constant nobody can tune in either direction may be a
counterweight*.

### Disposition, and what would change it

Shipped at `digest_hunger_weight: 0.0`. Built, wired, countered
(`digest_appetite_ticks` / `digest_appetite_held`), rider-controlled
(`hungergate=`), and one number from on.

Kept rather than reverted because the contradiction it resolves is real and does
not go away by ignoring it. **Its usefulness is conditional on a colony that gets
rich**, which is the granary — so the condition for re-testing is explicit:
build the granary, then re-measure at eighteen seeds before assuming either way.

### A counter of mine that lied, fixed before it was quoted

`digest_appetite_held` first bumped on **every** tick, including the ~90% where
the crop is empty, so it summed a rate nobody was going to spend: it read
13,986 J withheld against 13,440 J absorbed, i.e. the gate looked like it was
halving the gut. Gated on the crop actually existing it reads **2,020 of
13,440** — about 15%, and about 1% on the final curve. A **seven-fold**
overstatement, arithmetically correct throughout, answering a different question
than the one asked. Caught by this file's own rule before a single number from
it reached a conclusion.

## Instruments

- `examples/onetrail.rs` — `mode=arith` (shipped genome, nothing overridden),
  `mode=walk` (one ant, standing trail, mirrored arms), `mode=timing` (§1c).
- `examples/nesthome.rs` — `channel A` and `channel B` profiles by 32-column
  band, `scene=bed`.
- `examples/trailfollow.rs` — the colony-scale question and the `gate=`
  presets. Note its presets each overwrite **both** gate pairs, so no row of
  its `mode=arith` table is the shipped ant, which is a mix.

## §7.40 The terrain wires: one of them was deleted, not moved, and CI is what said so

`cargo run --example ascii` went red on the branch that closed the loop, on
the scene named for terrain-driven deposition: `no ant ever dropped anything
-- the verb never fired`. The message is accurate and points the wrong way.

**§7.35 took two wires off `Drop` and said both would live on `DropSpoil`.**
They did not. `main`'s `DropSpoil` block is `(AtNest, 0.9)`, `(Carrying,
0.2)`, `(SurfaceCurvature, 0.169)` — there was **no moisture term there to
stay on**, so moving curvature was a no-op and moving moisture deleted it
from the species. `wiki/ants.md` went on describing the preference for
another day.

**The obvious repair is the expensive one, and it is measured.**
`trailfollow mode=gap gaps=90 seeds=18 gate=shipped homebias=1`, deliveries
on the `hand` arm — the arm §7.38's headline is quoted from:

| `Drop` wiring | deliveries | ascii guard |
|---|---|---|
| no terrain terms (shipped) | **511** | red, wrong message |
| `(MoistureGrad, Drop, 0.169)` back | 180 | still red |
| moisture and curvature both back | 113 | green |

Restoring moisture alone is the worst row available: it costs two thirds of
the headline **and** leaves CI red, because the guard was riding on
curvature, not on moisture. That row is where this was heading when the work
changed hands, and it is recorded because it looks like the cautious move.

**The mechanism behind the whole table is one line of arithmetic.** `Bias
-0.2` against `Carrying +0.2` puts a laden ant's away-from-nest drop urge at
exactly `squash(0) = 0`, so on `Drop` the terrain terms are not a bias on the
rate — they *are* the rate. That is why `AtNest` being the only trigger left
makes every drop a delivery (§7.38), and equally why any terrain term put
back is a forager abandoning its dinner somewhere short of home.

**So the scene moved to the verb that owns deposition, and the moisture claim
came off the bar.** `examples/ascii.rs` now lays a diggable `soil` lattice
instead of food and reads `spoil_dumped` instead of `drops`; the marker is
`spoil`, which nothing but an ant putting a pellet down can produce, so the
attribution is sound rather than merely careful. Attributed events go **17 →
59** and the moisture field stops being saturated (steep 2.10 against flat
0.11, margin 1.16 against 0.34) — the channel has range for the first time.

What it cannot do is carry the preference claim, and that is filed as §Z28
rather than patched: `DropSpoil` has **no `Bias` wire**, so `Carrying` alone
puts its urge at `squash(0.2 + terrain)` and the pellet goes down within a
few ticks of being cut. A verb that fires at the face cannot choose a site.
Adding `(MoistureGrad, DropSpoil, 0.169)` moves the ratio 1.30x → 0.92x and
the sign test 31.8% → 45.5% — a null either side of no-effect — while moving
`hand` deliveries 511 → 121 and failing
`a_share_is_booked_on_both_sides`. One line, three things move.

The bar it replaces is the instrument's own: every dump the engine counted
is accounted for by the attribution, and most are credited to a cell. Both
were checked by putting the fault back — recounting settled pellets every
frame reads `83 credited + 27 unmatched + 3 discarded != 77 dumped` and goes
red.

## §7.41 The return leg re-swept against the new economy, and the laden leg measured at last

**2026-09-19.** Two things §7.38 left owed. `trailfollow mode=gap gate=shipped
gaps=90 seeds=18 seed0=1 ants=20 relay=60 near=10 food=400 refill=400
stop=6000`, one binary for the whole sweep, `hand` arm.

### The sweep, and why its totals must not be read as a ranking

| `home_bias` | deliveries | seeds delivering | ants alive | ate J |
|---|---|---|---|---|
| **shipped 0.0** | 124 | 5/18 | 51 | 270,755 |
| 0.1 | 510 | **2**/18 | 1,301 | 13,220,496 |
| 0.25 | 331 | **1**/18 | 634 | 7,735,816 |
| 0.5 | 215 | 6/18 | 788 | 13,134,614 |
| 1.0 | 511 | 9/18 | 59 | 140,377 |

**The two delivery columns disagree, and the per-seed data says which to
believe.** At 0.1 the entire 510 is seeds 1 and 18 — 188 and 322 — and those
same two seeds carry **536 and 722 ants alive**. At 0.25 it is one seed, 331
deliveries against 600 ants. Those are not foraging results; they are colony
explosions, and a colony of six hundred delivers incidentally. `CLAUDE.md`'s
*ask what your number counts*: total deliveries counts colony size times per-ant
rate, and here the first term moves by two orders of magnitude between seeds.

At 1.0 the 511 is spread over **nine** seeds with **no colony above 37 ants**.
Same total, entirely different object: small colonies each bringing food home,
which is the thing being asked about.

**So read breadth, not totals** — 1.0 delivers on 9/18, 0.5 on 6/18, shipped on
5/18, and the two arms whose totals look best deliver on 2 and 1.

### The answer to §7.27's re-take: still no, and for the same reason

§7.27 shut the dial because `home_bias: 1.0` starved the colony, and said the
table *"must be re-taken"* once delivering stopped forfeiting the meal. This is
that re-take, and **the starvation reproduces**: at 1.0, 59 ants alive and
140,377 J eaten against 1,301 and 13.2 M at 0.1. `digest_carry` fixed the
*carrier's* loss — chewing survives a drop — and did nothing about the
*colony's*, because a delivered cell is still an ordinary loose cell that
nothing stores and nothing eats from.

**So the return leg buys reliability and is still paid for in bodies, and
`home_bias` should stay at 0.0 until there is a granary.** That is §7.28's
design, and it is now the prerequisite rather than the next nice thing: the
dial cannot be priced against an economy that has no way to bank what the dial
delivers.

### The laden leg, measured rather than bracketed

§7.38 left it at *"436–873 ticks by three data points"*. `Track` now records the
frame of the last sample within `near` of the food and the frame the trip closes
at the nest, and emits the raw samples so they pool across seeds — a median of
per-run medians is not a median when a run contributes one trip.

| arm | n | p25 | **median** | p75 | p90 | max |
|---|---|---|---|---|---|---|
| shipped, laden at close | 10 | 835 | **1,909** | 3,025 | 4,249 | 4,447 |
| `home_bias` 1.0, laden at close | 42 | 913 | **1,837** | 3,565 | 4,573 | 14,551 |
| shipped, all closed trips | 29 | 3,025 | 4,699 | 7,867 | 11,713 | 17,233 |
| 1.0, all closed trips | 84 | 1,633 | 4,195 | 7,489 | 10,849 | 15,835 |

**The bracket was low by two to four times.** Two independent samples — a
different dial, four times the n — agree at **~1,850 frames**, which is the
number to size anything against from here. The laden subset is roughly *half*
the all-trips figure, which is worth keeping: an ant that is carrying gets home
substantially faster than one that is not, so the cargo is not what slows the
journey.

**Data:** `Reports/data/homebias-resweep-{shipped,0.1,0.25,0.5,1.0}-18seed-gap90-2026-09-19.log`
and `legs-{shipped,hb1}-18seed-gap90-2026-09-19.log`, whose `LEGS` lines carry
every raw sample the table pools.

**And it sits above the hold — corrected 2026-09-19.** §7.35 measured a post-fix
mean hold of 1,464 **frames** (it said ticks; see the correction block there). A median laden leg of ~1,850 against that is the shape
of the remaining failure — the journey outlasts the meal that pays for it — and
it is offered as the comparison to make next rather than as a demonstrated
mechanism, because the two numbers come from different beds and one of them is a
single ant.

## §7.42 The ant at the larder: the gate opens onto nothing, and the plane stops 25 cells short

**2026-09-19.** Traced tick by tick, on the owner's question: an ant with
`Carrying` at 1.0000 and the homing gate open — what is it deciding instead of
going home, and why?

### What it does

Ant 16, seed 1, gap 90, `hand`. Longest gate-open run 1,728 frames (288 decision
ticks) at x≈131, food at 138, nest band ending at 74.

```
frame    x   heading   moved  p_move  PheroAAlong    h0      h1   trail_term
21552  131   S              0.3134     0.0000     0.3333  0.3333    0.0000
21558  131   SE       NO    0.3114     0.0000     0.3333  0.3333    0.0000
21588  131   NE       NO    0.2941     0.0000     0.3333  0.3333    0.0000
21648  131   SW       NO    0.3608     0.0000     0.3333  0.3333    0.0000
21684  131   N        NO    0.3899     0.0000     0.3333  0.3333    0.0000
```

`moved` is **NO on every tick**. The heading spins through all eight directions
and no step is taken; net displacement over the window is **2 cells**.

**Why each choice comes out that way.** `PheroAAlong` is **0.0000**, so `h0` and
`h1` pin at a constant **0.3333** and their contribution to `Move` is
**0.0000** — the homing pair is wired in and carrying nothing. What is left is
`(Bias, +2.0)`, `(Energy, −1.75)` and `(FoodAdjacent, −1.16)`, giving
`move_presquash` −0.60..−0.26 and P(move) 0.31–0.40. `p_tumble` is a flat
**0.5000**, an uninformed re-roll. A stationary random walker.

### The control, and it is decisive

The one cohort member that went home, at the **same x**:

| | `PheroAAlong` | P(move) facing home | facing away | net x |
|---|---|---|---|---|
| ant 16 (stayed) | 0.0000 flat | 0.5480 | 0.5352 | 131 → 129 |
| ant 19 (went home) | −0.012 … +0.015 | **0.4099** | **0.2557** | 131 → **119** |

`ant.ron` calls that ratio *"the homing mechanism"*. It is intact. Its input is
zero.

### Why the input is zero — and the first answer was wrong

**Outbound ants lay the plane.** An earlier reading on this branch said it never
extends because no ant returns to lay it; that is false and `aprofile` (added
with this section) shows it directly. Seed 1, gap 90, amplitude along the route:

```
f=6000  63:155  68:1351  73:2011  78:1135  83:837  88:715  93:916
        98:1768 103:595 108:590 113:1784 | 118:27  123:4  128:12  133:0  138:0
```

Strong and continuous from the nest to **x≈113**, about 85% of the way, then a
cliff. **133 and 138 read 0 in every sample of every window.** The food is at
138 and the ants that reach it stand at 128–138 — past the edge.

**It is transient as well as short.** Down to 21–120 across the whole route by
f=7500 and zero nearly everywhere by f=9000. The traced window above is
f=21,552, long after anything existed anywhere.

### The loop that holds the edge short

Two rules colliding, both already in the engine:

1. **Deposits happen only on a successful move** — `creature.rs`,
   `if moved { ..deposit.. }`. A stationary ant lays nothing.
2. **Arriving at food is what stops an ant moving** — `(Energy, Move, −1.75)`
   and `(FoodAdjacent, Move, −1.16)` both bite at once on a fed ant standing on
   the larder.

So: reach food → fed and on food suppress `Move` → stop walking → **deposit
nothing** → no channel A at the larder → no homing gradient → no reason to move
→ stay stationary. **The plane's outer edge sits where ants are still walking,
and arrival is what ends walking**, so it cannot reach the thing ants walk
toward.

**It is not density.** The far band is the *most* crowded of the eight, at 127
per 1k. The ants are there; they are not moving.

### What this predicts, and the cheap test

An ant that kept moving at the larder would lay the missing stretch itself. That
is testable as an **ablation switch** rather than a genome edit — zero
`(FoodAdjacent, Move)` for one arm and read `aprofile` at 128–138 — and it is
the control to run before proposing any change to the wires.

**Instruments added for this**: `focaln=N` (cohort tracing with an `id` column,
because one ant is n=1), the return ledger, and `aprofile`/`aprofevery`/
`aprofstep`. A `p_tumble` column bug was found and fixed on the way: it printed
the raw brain output clamped instead of `unit_scale`d, reading **0.0000 on every
row of every ant**, whose obvious reading — *the ants never change direction* —
is the opposite of the truth.

## §7.43 The homing plane made four times more readable, and not one extra round trip

**2026-09-19.** §7.42 found an ant at the larder reading `PheroAAlong = 0.0000`
and concluded the plane was unreadable where the ants stand. This is the sweep
that followed, and it refutes the repair rather than confirming it.

### The plane is a scatter of bursts, not a ramp

First, what `aprofile` shows once it is time-averaged rather than read off one
frame. Over 3 seeds x 18 samples, channel A's **median is 0 from x=78 outward**
while its means run 100-470, and route cells are nonzero **55% of the time near
the nest, 25-40% mid-route, 3.7% at x=134 and 0% at 136-138.**

Per cell over time it is a sawtooth — x=62 reads 2857, 809, 2476, 114, 720, 32,
0. An ant walks over, deposits, and it decays to nothing before the next one
arrives. **So the ramp in §7.42 is an artifact of averaging bursts, and no ant
ever stands on it.** That is `CLAUDE.md`'s *ask what your number counts*, and the
mean profile walked the previous section into it.

### The guard was not the suppressor

§7.42 proposed re-deriving `sense`'s `guard = SCALE` on the grounds that it is
95% of the denominator where the plane is faint. Swept post-hoc at guard 256
against 16, readability moves **0-4 points at every x and every baseline** — the
failure is both cells reading exactly zero, and `(0-0)/(0+0+g)` is zero for any
guard. **That proposal is withdrawn.**

### The sweep the register's own conditions had opened

`DIFFUSE` and `DECAY_RHO` both carry re-test conditions tied to the plane being
`u8` (`dead-ends.md:1200-1202`), and the `u8` -> `u16` widening landed
2026-09-15 without either being re-swept. `arho=` / `adiffuse=` reach channel A
alone; B is untouched, because a food trail and a homing ramp want different
lifetimes.

36 seeds, gap 90, `hand` arm, one binary. `READ%` is the fraction of route-cell
samples on which an ant facing the nest gets a homeward `along` of at least 0.02
— the bar a real ant was observed to act on (§7.42's homing cohort member did it
on ~0.01).

| channel A decay | READ% | plane lit % | ants reached food | **round trips** | ants alive |
|---|---|---|---|---|---|
| **0.03, shipped** | 13.7 | 25.9 | 5,359 | **47** | 683 |
| 0.01 | 21.2 | 37.3 | 1,514 | 29 | 304 |
| 0.0075 | 23.5 | 40.9 | 460 | 23 | 120 |
| 0.005 | 27.4 | 45.1 | 390 | 27 | 108 |
| 0.002 | 40.2 | 59.5 | 375 | 38 | 87 |
| 0.0 | **54.6** | 68.5 | 814 | **48** | 124 |

Diffusion is nearly inert here — 0.05 against 0.25 at fixed decay moves `READ` by
1-3 points — so the rows above vary decay alone. That is itself a correction:
`DIFFUSE`'s doc argues it dominates *an isolated cell's lifetime*, and for
readability across a route it does not.

**Readability rises fourfold, colony size falls eightfold, and round trips do not
move.** 47 at shipped, 48 at the most readable setting, 23-38 everywhere between.

### The rate that looked like a win was a denominator collapse

`back/reached` reads 0.88% shipped against 5.90% at decay 0 — 6.7x, and the
paired sign test on `came back` is **20 seeds better, 6 worse, 10 tied**, which
is p < 0.01 against no effect. Both are real and both are about the wrong thing:

| | shipped | decay 0 |
|---|---|---|
| round trips | 47 | **48** |
| ants that reached food | 5,359 | **814** |
| ants ever alive | 5,890 | **1,244** |

The rate improved because the denominator fell 6.6x. Per-seed the difference is
**median +1.0 but mean +0.03**, min **-30**, max +3 — many small wins and one
catastrophic loss, which is exactly the distribution that makes a median and a
total disagree. **The honest statement is that round trips are flat.**

### What this rules out, and what it points at

**Readability is not the binding constraint.** A plane four times more readable,
lit on 68% of route cells instead of 26%, produces the same ~47 round trips. So
the ant reads a homeward gradient and does not convert it into homeward
displacement.

That points at §7.42's D4 rather than its D1: `p_move` ~0.5 against a flat
`p_tumble` of 0.5 gives a heading change every ~4 ticks — **a mean run of about
2 cells** — and the homing pair modulates *whether to step*, not *which way*.
A ratchet that biases stepping cannot accumulate displacement when the heading
is re-rolled uniformly every two cells. `BrainOutput::Persist`'s own doc names
this: *"the single number that decides milling versus commuting — median net
displacement was 2% of path length."*

### Why it cannot convert — the ant reads its own deposit (§Z29)

Chasing the flat outcome into the per-tick traces found the cause, and it is
upstream of everything above. **Laden ants only**, since the homing pair is
gated shut for an empty one:

| run | facing HOME gives along > 0 | facing AWAY gives along < 0 |
|---|---|---|
| seed 1, shipped | **8.2%** | 60.3% |
| seed 6, shipped | **1.2%** | 78.7% |
| seed 1, decay 0 | **7.5%** | 91.9% |
| seed 6, decay 0 | **2.5%** | 20.9% |

Facing the nest gives a homeward reading on **1–8% of laden ticks**. The
reading is negative *whichever way the animal faces*.

`step` deposits channel A at the ant's own head cell after a successful move,
and `sense` reads that same cell as `here`. So the cell underfoot is the
freshest thing in the neighbourhood and `ahead − here` is negative by
construction. Reconstructed from `along` and `PheroAFront`, median `ahead` is
**0.0** against a median `here` of **105 and 457**, with `here > ahead` on
**75–87%** of laden ticks.

**This is why a four-times-more-readable plane bought nothing**: the signal got
bigger and stayed negative in every direction. It also explains the measured
ratchet of +0.04 to +0.11 where `ant.ron`'s own note describes 0.84 against 0 —
`|along|` is healthy (median 0.23–0.53 against the 0.486 that note needs), and
what is broken is the **correlation between heading and sign**, which no census
of the plane or the reading can see.

Filed as §Z29 with fix candidates; it is not §Z7, whose homing half was fixed
2026-09-09 and which concerns the gate rather than its input.

***Re-test this decay sweep when:*** §Z29 is fixed. Until then it measures a
reader that is looking at its own footprints, and every row above will
reproduce.

**Data:** `Reports/data/achannel-decay-*-36seed-gap90-2026-09-19.log`,
`aplane-profile-seed1-2026-09-19.txt`.

## §7.44 The §Z29 fix works, and the colony starves doing it

**2026-09-19, same night as §7.43.** §Z29 says the ant stands on its own
freshest deposit so `along` reads negative whichever way it faces. This is the
repair, measured — and the one place it fails is the one the register predicted.

### The repair

`PIXEL_PHYSICS_DEPOSIT_AT=vacated` lays the mark on the cell the ant just left
rather than the head it arrived on. Unset is bit-identical; P-11 is untouched,
the deposit still happening only on a successful move.

**Alone it does nothing** — 41 round trips against 47, paired 11 better / 7
worse, total −6. The traces say why: it removes the negative bias without
creating a positive one, because the ambient plane it then reads has no slope.
Facing home gives `along > 0` on 4.0% and 13.4% of laden ticks against 8.2% and
1.2% shipped: the reading goes from negative to roughly zero.

**With a persistent plane it is the strongest result of the session.** 36 seeds,
gap 90, `hand` arm:

| arm | READ% | ants ever | round trips | trips per 1k ants | deliveries | alive |
|---|---|---|---|---|---|---|
| **shipped** | 13.7 | 5,890 | **47** | 8.0 | 322 | 683 |
| vacated | 13.7 | 7,067 | 41 | 5.8 | 162 | 885 |
| vacated + decay 0.02 | 16.0 | 1,433 | 21 | 14.7 | 77 | 169 |
| vacated + decay 0.01 | 20.3 | 907 | 20 | 22.1 | 59 | 127 |
| vacated + decay 0.005 | 27.6 | 803 | 36 | 44.8 | 151 | 60 |
| vacated + decay 0.002 | 41.7 | 830 | 46 | 55.4 | 506 | 62 |
| **vacated + decay 0** | 53.9 | 857 | **61** | **71.2** | **693** | 73 |

Paired on round trips against shipped, `vacated + decay 0` reads **21 seeds
better, 6 worse, 9 tied**, median +1.0, total **+14** — the only arm all session
to move trips up on both the total and the paired test. The per-ant rate rises
**monotonically with persistence, 8.0 → 71.2 per thousand, a factor of nine.**

The ratchet is the strongest measured: P(move) **0.4155 facing home against
0.2629 facing away** (+0.1526) where shipped reads +0.0630. And it works by
*stalling an ant pointed away* rather than speeding one pointed home — which is
how run-and-tumble is supposed to work.

### And then the colony starves

| arm | deliveries | **ate J** | born | starved | alive |
|---|---|---|---|---|---|
| shipped | 322 | **6,151,294** | 5,173 | 755 | 683 |
| vacated | 162 | 7,918,778 | 6,359 | 592 | 885 |
| vacated + decay 0.002 | 506 | **234,086** | 110 | 588 | 62 |
| vacated + decay 0 | 693 | **270,810** | 137 | 595 | 73 |

**Intake falls 23-fold while deliveries double.** Births go 5,173 → 137. The
starvation *rate* goes from 13% of ants ever alive to **69%**. Per ant,
deliveries rise about eightyfold while the colony that makes them cannot feed
itself.

The mechanism is not subtle and it is not new: **a delivered cell is left on the
ground and nothing banks it**, so an ant that walks its meal home has spent the
journey and given the colony a cell it does not eat. Shipped ants survive by
*not* delivering.

**The alternative reading, stated because it is the honest rival:** the colony
might collapse first for another reason, leaving delivery as a survivor effect.
Per-ant deliveries argue against — 5.06 per ant against 0.06 — but this is one
bed and the confound is real.

### What this settles about the granary

Earlier in this session the claim "the granary gates all of this" was withdrawn
on the grounds that banking a delivered cell cannot make a gradient readable.
**That withdrawal was right about the mechanism and wrong to drop the gate.**
The two claims are separate and both are now measured on this branch:

1. The granary does **not** fix channel A. §Z29 is a sensing bug with a sensing
   fix, and the repair above needs no economy change to work.
2. The granary **does** gate whether working homing is survivable. Tonight it
   reproduced twice by different routes — `home_bias 1.0` (59 alive against
   1,301, §7.41) and `vacated + decay 0` (73 against 683, here).

So the order is: §Z29's repair is a real fix to a real bug and should land on
its own terms; **turning the persistence up to exploit it is gated on a larder**
(§7.28), exactly as §7.27 said and for exactly the reason it gave.

***Re-test the decay ladder when:*** a nest drop banks the cell's worth. Until
then every row above trades colony for commuting and the trade is the economy's,
not the sensor's.

**Data:** `Reports/data/z29-*-36seed-gap90-2026-09-19.log`.

## §7.45 The P(move) column was in the wrong units, and correcting it moves the diagnosis

**2026-09-19, overnight after §7.44.** Everything below follows from one line
of the harness being wrong, and the correction makes the homing circuit look
**better** than reported while moving the blame somewhere else entirely.

### The bug

`examples/trailfollow.rs` wrote the `p_move` column as
`brain::unit_scale(out, 1.0)` = `(out + 1) / 2`. `src/sim/creature.rs:4368`,
which is what actually rolls the step, uses `out.clamp(0.0, 1.0)`. Those are
different functions. `unit_scale` is the right convention for `Tumble`,
`Persist` and `Caution` — and `Move` is the one output that does not use it.

They differ most exactly where this ant lives: **every negative `Move` output
prints as something between 0 and 0.5 under `unit_scale` and is rolled as a
hard zero.**

**The control cost nothing and was already on disk**, which is the part worth
carrying: the cohort traces carry positions, so the observed step rate per
printed bucket says which function the engine is using.

| printed `p_move` | ticks | ants stepped | if the column were P | if P = `clamp(2p−1)` |
|---|---|---|---|---|
| 0.15 | 4,328 | **0.0%** | 15% | 0% |
| 0.25 | 1,373 | 0.0% | 25% | 0% |
| 0.45 | 1,458 | 0.1% | 45% | 0% |
| 0.55 | 1,907 | 12.3% | 55% | 10% |
| 0.75 | 1,096 | 48.3% | 75% | 50% |
| 0.95 | 51 | 76.5% | 95% | 90% |

Weighted absolute error over 13,248 ticks: **1.4 points for `clamp`, 27.7 for
`unit_scale`.** In the four lowest buckets the column claimed 5–35% and the
ants stepped **0 times in 6,819 ticks**.

### What it changes

Every `P(move)` figure in §7.41–§7.44 is in the wrong unit. The corrected ones,
`gate=shipped gaps=90 arms=hand`:

| | as reported | corrected |
|---|---|---|
| ratchet, P(move) up-gradient − down | +0.0630 / +0.1526 | **+0.2620 / +0.6255** |
| engine P(move) exactly **zero** | not measured | **48–72% of all ticks** |

The old number was not merely small; it was a mean over ticks where the homing
term is **disconnected**. Zero plus a small number is still zero, so on 62% of
this ant's ticks the gradient cannot move the behaviour at all whatever it
reads. A mean across that boundary is `CLAUDE.md`'s "ask what your number
counts" with the clamp doing the hiding.

### And split out, the circuit is doing exactly what run-and-tumble should

`not resting %` is the share of ticks the clamp has *not* already pinned at
zero; `P(move)|>0` is the mean over only those. Twelve seeds, gap 90:

| arm | facing up-gradient | | facing down-gradient | |
|---|---|---|---|---|
| | not resting | P(move)\|>0 | not resting | P(move)\|>0 |
| shipped | **90.8%** | 0.636 | **16.4%** | 0.046 |
| vacated | 88.3% | 0.559 | 22.0% | 0.055 |

**The homing drive is almost entirely the rest gate and barely at all the
speed** — an ant facing up the ramp is five and a half times more likely to be
in a state where it can step at all, and when both are moving they move at
similar rates. That is the mechanism the design asked for, working.

### So the bottleneck is not the ratchet. It is that the reading is almost
### never positive

Same runs, the count the split is taken over:

| arm | laden ticks facing **up** | facing **down** | ratio |
|---|---|---|---|
| shipped | 11,255 | 169,037 | **15.0 : 1** |
| vacated | 12,521 | 173,300 | 13.8 : 1 |

An ant reads the homing plane as pointing the way it is facing on about **6% of
its laden ticks**. `§Z29`'s vacated deposit moves that 15.0 → 13.8 and no
further. **Everything downstream is fine; there is nothing coming in.**

And the arithmetic closes: facing up-gradient the ant nets **+0.0142 cells/tick**
homeward, facing down **+0.0011**. Pooled that is +0.0017/tick, so 90 cells
takes about **53,000 frames** — more than twice the whole run.

### T0: the guard is not the suppressor, and this is now settled

§7.42's D5 said `sense`'s `guard = SCALE` (256 raw) swamps a faint plane —
95% of the denominator at the food end against 2.5% at the nest. Post-hoc over
36 recorded profiles (3 seeds × frames ≤ 6,000), recomputing what an ant facing
home would read at every x:

| x | g256/b6 | g16/b6 | g256/b30 | g16/b30 |
|---|---|---|---|---|
| 78 | 17% | 19% | 47% | 53% |
| 108 | 28% | 28% | 36% | 39% |
| 138 | 3% | 3% | 22% | 22% |

**A sixteenfold cut in the guard moves readability 0–5 points at every x and
every baseline.** The baseline does about twice as much — and 30 cells is the
radar reach already rejected on plausibility. **Part 1 of the plan is dead**,
and cheaply: no code was written.

### T1: the plane is not spiky. It is absent

D2 ("spikes that locally invert") rested on one sample. Time-averaged over 3
seeds × 18 frames:

| x | mean | median | nonzero% |
|---|---|---|---|
| 48 (nest) | 228.4 | **6.5** | 55.6% |
| 78 | 196.9 | **0.0** | 48.1% |
| 108 | 30.9 | 0.0 | 25.9% |
| 122 | 56.7 | 0.0 | 25.9% |

**Median zero from x=78 outward, and a quarter of samples lit past x=106.** The
mean-to-median ratio of 35:1 at the nest is the signature: this is a scatter of
short-lived bursts, not a ramp with noise on it. So D2 is the wrong reading of
D1 — there are no spikes to smooth, there is nothing there most of the time.

### T2: the run is over before the ant has moved once

Run-length census over five cohort traces, one row per tick:

| | shipped | vacated | vacated + decay 0 |
|---|---|---|---|
| run length, ticks (mean / median) | 3.35 / 2 | 3.15 / 2 | 3.13 / 2 |
| run length, **cells** (mean) | **0.35** | 0.30 | 0.37 |
| net displacement / path length (median) | 12.0% | 15.7% | 4.7% |
| **actually stepped on** | **13.0%** of ticks | 12.4% | 16.4% |

The plan predicted "a mean run of ~2 cells". It is **0.35**. The heading turns
over about six times faster than the body moves, because the tumble roll fires
in the `else` of a *failed* move — so an ant the ratchet has correctly stalled
re-rolls the heading it was stalled for, about every other tick.

### The constant that was lost in a refactor, and did not turn out to matter

`dead-ends.md:1083` records the measurement: re-orienting on **every** failed
move roll took food discovery from 33 pickups to **1**, and
`TUMBLE_ON_FAILED_MOVE = 0.35` was the fix. That const became
`BrainOutput::Tumble`, whose silent output is `unit_scale(0.0)` = **0.5**, and
`ant.ron` authors no `Tumble` wire — so the shipped ant has been re-rolling on
half its failed rolls against an authored answer of 0.35, and nothing said so.
`Persist` is the same shape: an anonymous `0.15` became a silent **1.0**.

Restoring them, and the gradient-into-`Tumble` wiring that output's own doc
asks for. Twelve seeds, gap 90, `arms=hand`, paired within seed against
shipped on **`came back / reached food`** — a rate, because two arms scored
`reached food` in the thousands on colonies that founded rather than navigated:

| arm | reach/seed | back/seed | homing rate | sign b/w/t | median Δ |
|---|---|---|---|---|---|
| shipped | 9.0 | 1.0 | 6.61% | — | — |
| vacated | 10.0 | 0.0 | 3.68% | 2/7/3 | −2.09 |
| + tumble 0.35 | 13.0 | 0.0 | 3.14% | 3/6/3 | −0.97 |
| + tumble 0.20 | 11.5 | 1.0 | 7.19% | 5/4/3 | +0.00 |
| + tumble 0.10 | 7.5 | 0.0 | 9.62% | 4/5/3 | +0.00 |
| + persist 1.5 | 11.5 | 0.5 | 6.47% | 4/5/3 | +0.00 |
| + persist 0.5 | 10.5 | 0.0 | 0.21% | 3/6/3 | −2.50 |
| + `PheroAAlong→Tumble` −3.0 | 12.0 | 0.0 | 2.96% | 3/6/3 | −2.50 |
| + tumble 0.35, persist 1.5 | 10.0 | 1.0 | 0.99% | 5/4/3 | +0.00 |

**Null, every arm.** Best sign test 5/4/3, median difference zero. L3 does not
fix homing.

**Read the totals in that sweep as the trap they are.** `persist 0.5` scored
`reached food` **6,615** against shipped's 121 and `born` 6,638 against 37 —
and its per-seed median reach is **10.5 against 9.0**. One or two seeds founded
a colony and the rest did not; the thousands are population, not navigation.
The pooled homing *rate* on that arm is **0.21%**, the worst in the table.

### What this leaves

The sensor is fine. The ratchet is strong and correctly shaped. The plane is
the whole of it, and the number to move is the **15:1 down:up ratio** — which
needs to know whether the ramp points at the food (§7.15's polarity inversion)
or whether a milling ant builds a mound of its own deposits and reads downhill
in every direction. `A READ` now prints the **foodward** reading beside the
homeward one, which separates those two for the first time: a ramp pointing at
the food gives one high and one low, a mound gives both low.

**Data:** `Reports/data/runlength-12seed-gap90-2026-09-19.log`.

## §7.46 The homing plane was being erased faster than an ant can walk home

**2026-09-19, following §7.45.** With the sensor cleared (§7.45: the ratchet is
strong, correctly shaped, and reads a positive gradient on 6% of laden ticks),
the remaining suspect was the plane. `dead-ends.md:1202` had left the decay
sweep open — *"a wider-precision plane would need re-sweeping"*, a condition
met when `u8` → `u16` landed on 2026-09-15 and never acted on. This is that
sweep, and it is the strongest result on this line.

### The dose-response

36 seeds, gap 90, `arms=hand`, paired within seed on **`came back / reached
food`** — a rate, for §7.45's reason. `arho` is channel A's decay per pass;
`DECAY_RHO` ships at 0.03 on both trail planes.

| channel A rho | homeward readable | foodward | plane lit | homing rate | sign b/w/t | median Δ |
|---|---|---|---|---|---|---|
| **0.03 (shipped)** | 13.6% | 12.9% | 26.1% | **0.88%** | — | — |
| 0.01 | 20.5% | 15.0% | 35.4% | 1.02% | 12/8/16 | +0.00 |
| 0.005 | 27.9% | 17.6% | 44.8% | 1.51% | 16/8/12 | +0.00 |
| **0** | **53.2%** | 15.6% | 69.2% | **14.84%** | **23/6/7** | **+8.37** |

Four settings, monotone in both the mechanism number and the outcome, which is
better evidence than any single arm. The sign test on `rho 0` is 23 better, 6
worse, 7 tied over 36 seeds — **p ≈ 0.002** two-sided over the 29 non-ties.

Note the **foodward** column barely moves while homeward quadruples. The plane
does not merely get louder; it becomes a **ramp that points home**, 3.4:1
against the shipped arm's 1.05:1 coin flip.

Adding §7.45's tumble constant on top gives 24/5/7 and median **+11.81**; tumble
**alone** is 9/11/16, median 0.00. So run length is not a second lever, it is a
multiplier on a plane that is readable in the first place.

### Why only zero works, and it is about how slowly this ant walks

`rho 0.005` is null. The required lifetime is not a tuning matter: §7.45 measured
the laden ant netting **+0.0142 cells/tick** homeward at best, so a 90-cell walk
is tens of thousands of frames, and at `PHEROMONE_INTERVAL = 12` a run of 24,000
frames is 2,000 decay passes. `0.995^2000` is 4.5e-5. **Nothing but zero survives
the trip.** Channel A was being erased between one ant's visit and the next.

### "Rho 0" does not mean the plane never forgets, and the control says so

The obvious objection is that a plane which does not decay is a plane that never
clears and never sleeps. `examples/ascii scene=pheromone` is the control, and it
is titled for exactly this claim — *"a blob spreads, drains to zero, and the
plane goes back to sleep"*:

| | at deposit | after 400 frames | after 4,000 |
|---|---|---|---|
| shipped | max 200 | max 60 | **max 0** |
| `A_RHO=0` | max 200 | max 166 | **max 0** |

**Diffusion, not decay, is what erases this plane** — which is what
`set_channel_diffuse`'s own doc already said in a different context (the blend
takes 16.7% per pass against decay's 2.9%, so decay "is close to inert because
it is the smaller term"). A 3x3 mean of a thin trail rounds to zero at low
values, so a weak plane still dies; what `DECAY_RHO` was adding was a second,
faster eraser on top of one that already worked.

### How it works, and it is not the number §7.45 pointed at

§7.45 named the **15:1 down:up ratio** as the bottleneck — the ant reads the
plane as pointing its way on 6% of laden ticks. A persistent plane does not fix
that. It makes it *worse*:

| arm | up-gradient share of laden ticks | cells/tick homeward **when facing up** | pooled cells/tick |
|---|---|---|---|
| shipped | **17.0%** | +0.0026 | +0.00068 |
| channel A rho 0 | **6.6%** | **+0.0602** | **+0.00541** |
| + tumble 0.35 | 7.1% | +0.0589 | +0.00543 |
| rho 0.005 | 8.3% | +0.0301 | +0.00287 |
| rho 0.01 | 11.0% | +0.0208 | +0.00245 |

**What changed is not how often an up-gradient reading arrives, it is whether
the reading is true.** On the shipped scatter an "uphill" reading mostly points
at the nearest random burst, and the ant walks toward noise: an up-gradient tick
is worth +0.0026 cells homeward. On a persistent plane uphill means *the nest*,
and the same tick is worth **+0.0602 — twenty-three times as much**. The share
falls because a plane lit on 69% of cells instead of 26% converts "no readable
gradient" ticks into readable ones, most of which are downhill.

Pooled, the laden ant's homeward drift goes **+0.00068 → +0.00541 cells/tick**,
eightfold. Over a 24,000-frame run that is 21.6 cells of net progress against
2.7 — still well short of the 90 the trip needs, which is exactly why the rate
lands at 14.8% rather than at 90%. **The arithmetic pins the outcome**, which is
the check a rate this much improved otherwise invites.

`d:u` was a count, and a count cannot see whether the things it counts are
right. That is `CLAUDE.md`'s "ask what your number counts" arriving on a number
this report had just finished promoting.

### Run length did not move, and that is the point

Acceptance criterion 2 asked for mean run length and net displacement to rise
off ~2 cells / ~2%. Re-measured on the winning arm, three seeds, one row per
tick:

| | shipped | channel A rho 0 |
|---|---|---|
| run length, ticks (mean) | 3.04–3.50 | 2.98–3.18 |
| run length, cells (mean) | 0.37–0.42 | 0.26–0.40 |
| stepped on | 14.3–17.6% of ticks | 11.4–16.8% |

**Unmoved, and net/path is noise at three seeds** (two of three better). So the
criterion **fails as written and the change works anyway**: the ant still mills,
it simply mills *biased*. The fix is entirely in what the ant can read, and
nothing about how it moves had to change — which is the cheapest possible shape
for it and the reason the tumble arm is a multiplier rather than a lever.

### Frame cost: measured, and it is not the gate

`CLAUDE.md` requires the cost of anything that keeps tiles awake, read as
`PheromoneStats::tiles_processed` and `ascii`'s worst frame. `scene=ants`,
12,000 frames:

| channel A rho | tiles/pass | mean ms | worst ms |
|---|---|---|---|
| 0.03 (shipped) | 16.7 | 0.779 | 9.95 |
| 0.01 | 16.6 | 0.780 | 6.66 |
| 0.005 | 16.0 | 0.756 | 6.03 |
| **0** | **17.4** | **0.773** | 6.86 |

**A 4% rise in awake tiles and no measurable frame cost.** Read the *mean*: the
worst column swings 6.0–9.9 ms across arms whose tile counts differ by 4%, which
fails `CLAUDE.md`'s pinning test (mean × frames nowhere near worst), so the worst
here is an order statistic over many similar frames and is noise wearing a
number. And the counter moving **up** is what rules out the other failure —
a cost that vanishes because the work vanished.

### The instrument built for this question, which nobody asked

`examples/pherolife` exists to answer *"how long does a trail live once nobody
is re-laying it, and what holds one up?"* — `Reports/instruments.md` says to
grep it before building a harness, and the trailfollow sweep above was built
first. Its `sweep=rho`, on a 120-cell ramp with a **2,200-frame round trip**:

| rho | trail gone | of a round trip | stops steering | of a round trip |
|---|---|---|---|---|
| **0.03 (shipped)** | 1,476 frames | **0.67x** | 1,080 | **0.49x** |
| 0.10 | 612 | 0.28x | 480 | 0.22x |
| 0.25 | 288 | 0.13x | 180 | 0.08x |
| **0** | 4,488 | **2.04x** | 2,328 | **1.06x** |

`stops steering` is the frame the ant's own run drive — `ant.ron`'s authored
path from `PheroAAlong` into `Move`, so it is what the animal does rather than
what the plane holds — falls under a tenth of its baseline.

**At the shipped decay a homing trail stops being worth reading at half a round
trip. At zero it clears one, barely.** That is this whole section in the
instrument's own units, it was answerable without a single new line of harness,
and it is an independent check: `pherolife` builds its planes with its own
`rho=` and never reads `TRAIL_A_RHO`.

It also predicts the size of the win rather than just its sign. One trail laid
and abandoned covers **1.06** round trips at rho 0 — so a colony gets home when
traffic re-lays the route and not otherwise, which is exactly an outcome of
14.8% rather than 90%.

### The food trail wants the opposite, which settles how this ships

The awkward part of shipping this is that A is the homing plane **only because a
species wires it that way** — the 2026-09-02 genome refactor exists to make that
a species' choice rather than the engine's. So the alternative worth measuring is
that neither trail plane decays and diffusion sets both lifetimes, needing no
per-channel rule at all. Measured, 36 seeds, gap 90:

| arm | ants reaching food / seed | homing rate |
|---|---|---|
| shipped | **11.0** | 0.88% |
| channel A rho 0 | 9.5 | 14.84% |
| channel **B** rho 0 | **3.0** | 2.46% |
| both planes rho 0 | **3.0** | 10.45% |

**Killing the food trail's decay destroys the outbound leg** — a third as many
ants ever reach the larder. That is `set_channel_rho`'s own §Z7 argument
arriving as a measurement: a trail that outlives its patch keeps recruiting to
an exhausted one. So the two planes want opposite settings, the engine already
has that idea (`ALARM_RHO` is a third lifetime for a third plane), and a
per-plane constant is the honest shape. **The evolvable version — lifetime as a
species field rather than an engine constant — is the next step and is not this
change.**

### What the longer gaps say, which is that they cannot say anything

T5 asked for gaps 90 **and** 140. At 140 and 200 the outbound leg fails first:
`reached food` is **2.5 and 1.0 ants per seed** against gap 90's 11.0, and the
homing rate is 0.00% in *both* arms. There is nothing to be paired. That is
§7.42's finding — the hand-laid trail does not get ants to food past ~140 —
and it makes gap 90 the only bed on which this question is currently askable.

### A correction to §7.44: the starvation was a pooled-total artifact

§7.44 reported that the persistent plane starved the colony — *"intake falls
23-fold while deliveries double"*, 6,151,294 J → 270,810, births 5,173 → 137.
**Those are pooled sums over 36 seeds**, and §7.45 showed what that does here:
one or two seeds found a runaway colony and own the total. Paired per seed, same
runs:

| arm | intake/seed (median) | born | starved | alive | sign test on intake |
|---|---|---|---|---|---|
| shipped | 7,116 | 2 | 17 | 1 | — |
| channel A rho 0 | 5,164 | 2 | 17 | 0 | **15/21/0** |
| + tumble 0.35 | 4,848 | 2 | 18 | 0 | 16/20/0 |

A median dip of about a third that **does not clear a sign test at 36 seeds**,
with births, starvations and survivors flat. So the honest reading is
**unresolved, not absent** — and it is nothing like a 23-fold collapse.

**This reverses §7.44's ordering.** That section concluded *"turning the
persistence up to exploit it is gated on a larder"*. On the paired statistic it
is not gated on anything: the plane's lifetime is a sensor question, it pays for
itself four-fold on the outcome it was changed for, and its cost to the economy
is inside the noise of a bed where every colony starves in every arm.

### A second bed, and the cost is real after all — correcting the correction above

The section above corrected §7.44's *"persistence starves the colony"* to
*"unresolved"* on a paired sign test of 15/21. **That was right about that bed
and wrong to generalise it.** Re-run on a second one — `refill=2000` instead of
400, a third as much food arriving, both arms sharing it — 36 seeds, gap 90,
paired within seed:

| column | old (rho 0.03) | new (rho 0) | sign b/w/t | median Δ |
|---|---|---|---|---|
| **round trips closed** | 0.88% | **2.92%** | **20/10/6** | **+6.12 pts** |
| what a homing ant can read | 13.5% | **55.3%** | — | — |
| ants that reached the food | 15.0 | 11.5 | **9/24/3** | −2.5 |
| intake, J | 10,842 | 6,654 | **11/25/0** | −2,650 |
| born | 6.5 | 3.5 | **9/23/4** | −2.0 |
| alive | 4.0 | 1.0 | 10/18/8 | −0.5 |
| starved | 17 | 18 | 18/13/5 | +0.5 |

**The mechanism replicates and so does a cost that bed one could not resolve.**
Round trips are up on 20 of 30 non-ties; intake, births and food contacts are
each down on about three quarters of seeds, which at 36 seeds is real.

**The fall in `reached food` is downstream of the fall in births, not a
navigation effect** — per ant born it goes *up*, 2.3 → 3.3 — and the homing
pair cannot steer an empty ant anyway: `ant.ron` gates units 0/1 at
`(Bias, 0, -45.0)` / `(CarryingFood, 0, 45.5)`, so a forager with nothing in its
mandibles has them saturated off. What falls first is **intake**.

**And that is §7.44's argument, arriving with a paired statistic instead of a
pooled one.** An ant that successfully walks its meal home has spent the journey
and handed the colony a cell nothing banks; an ant that eats where it stands has
not. On a bed with food to spare the trade is invisible; on a food-limited one
it is the dominant term. So the ordering §7.44 gave was right in substance and
its *evidence* was still a pooled-total artifact — both things are true, and the
part this report got wrong was reading "the totals are an artifact" as "the
effect is not there".

**What this does not change:** the sensor fix stands on its own terms. Channel A
was being erased faster than an ant can walk and now is not, which is a defect
either way — §7.28's larder is what decides whether a colony can *afford* to use
it, exactly as §7.27 said. **What it does change** is that this is a trade rather
than a free win, and on the bed where food is scarce the colony currently pays
more than it earns.

***Re-test the whole of §7.46 when:*** a nest drop banks the cell's worth. That
is the same condition §7.44 set, and it is now set on better evidence.

**Data:** `Reports/data/planerho-36seed-2026-09-19.log`.

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
