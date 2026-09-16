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
            PheroBAlong facing nest: 0.151 -0.002 0.059 0.065 -0.000 0.088 0.108
per_cell=4  value food->nest:   306  656  850 1126 1551 2266 3718 10515
            PheroBAlong facing nest: 0.231 0.040 0.045 0.051 0.063 0.084 0.125
per_cell=8  value food->nest:    58  164  256  403  655 1126 2266 10537
            PheroBAlong facing nest: 0.153 0.047 0.058 0.069 0.082 0.104 0.180
```

Monotone, in every arm, and the gradient facing the nest is **positive along
the whole route**. The magnitude is *usable*: the working gate delivers
`P(move) = 0.641` at `along = 0.10`, and this route offers 0.04–0.23.

**So channel B is not shapeless after all — it is shaped backwards.** §1b's
multi-modal bed profile is many ants' backwards ramps laid over each other
from many pickup points, not an absence of structure.

### The consequence nobody had

**Channel B, as laid and as wired, is a second and noisier copy of channel A.**
Both climb toward the nest; `ant.ron` authors `(PheroBAlong, 2, +6.0)` with the
same sign as channel A's `(PheroAAlong, 0, +6.0)`, so an empty ant with the
gate opened is steered **home**. That is a mechanistic explanation for the
§Z7 measurement that re-gating makes the colony worse: it does not merely fail
to help, it actively converts searching ants into homing ants.

*(Confidence: the timing derivation and its measurement are solid. That this
is the dominant term in a real bed's channel B is inferred, not measured —
§1b's 32-column bands are a mass distribution, not the local gradient an ant
reads at a 6-cell offset. See §5.)*

---

## 2. What I suggest

**Revised by §1c.** Before that measurement I proposed building a food-charged
odometer on hidden unit 7. That is no longer the first thing to try.

**Step 1 — flip the sign, and open the gate, together. Two numbers.**
`(PheroBAlong, 2, -6.0)` and `(PheroBAlong, 3, +6.0)`, plus units 2/3's bias
moved off saturation to the `b2` value units 0/1 already carry. An empty ant
then **descends** channel B, walking down the age gradient toward the older
end — which is the food. Nothing is built; these are four authored weights in
`ant.ron`, all of them mutable.

**The known weakness, stated up front:** descending a gradient walks you to
where the signal *vanishes*, not to a peak, and `pheromone-lifetime-and-wiring-
2026-09-14.md` already measured a cell laid at `DEPOSIT` dying in **12 passes
= 144 frames against a 2,200-frame round trip**. So the far end of a real
trail may be gone before anyone can follow it down, and the descent leads to
where the trail died rather than to the food. Step 1 is the cheapest decisive
test of the **diagnosis**; it may not be a sufficient fix.

**Step 2, if step 1 is right but capped — give B a ramp that peaks at the
food.** Mirror the odometer onto **hidden unit 7, which is free** (0–3 are the
gate pairs, 4 the odometer, 5/6 the `Dig` pair, nothing touches 7): charge it
from `FoodAdjacent` instead of `AtNest`, decay it, wire it to `EmitB`. B is
then laid strongest just after leaving food and falls toward the nest — a ramp
peaking at the food, which an ant *ascends*, which is stable because it ends at
a maximum. Same three-weight shape as unit 4, which demonstrably works.

**Step 3 — the depletion signal, which is genuinely missing.** `Carrying` is
**graded, not binary** (`crop_fill = worth / capacity`, and `worth = unit x
cells`, so richer food fills the crop faster) — an earlier draft of this
analysis said binary and was wrong. What saturates is the *trip*: an ant that
fills its crop reads ~1.0 whether the patch has three cells left or three
thousand. So the trail cannot fade as its source empties, which is §Z7's "a
trail that outlives its patch keeps recruiting to nothing". The missing
quantity is *how much is left where I found this*, and nothing senses it.

**Do not re-derive nothing.** Every term in the `Move` row — `(Bias, 2.0)`,
`(Energy, -1.75)`, `(FoodAdjacent, -1.16)`, `(Crowding, -0.3)` — was fitted in
a world where units 2/3 contribute ~0.001. Switching on a term worth up to
+-2.5 in the same sum reallocates the whole budget; `CLAUDE.md` calls this out
and `ant.ron`'s own comments record it biting once already on the
`Bias`/`FoodAdjacent` pair. `Crowding` is the one to watch: it is the
documented anti-ossification term, and -0.3 was sized for a colony that never
follows trails.

---

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

### 4.1 On step one being rejected for not fixing everything — agreed, and this is now demonstrated rather than suspected

This is the most important of the four, and §1c turns it from a worry into a
worked case. The re-gate is **step one of at least two**. Opening the gate
without flipping the sign does not merely fail to help — it gives an empty ant
a working reason to walk home. A measurement of step one alone was therefore
*correct* about step one and *wrong* about the idea, and §Z7's conclusion
("fix what the trail is worth, not what the ant can read") was drawn from
exactly that.

This is epistasis: two changes, each neutral or harmful alone, plausibly
beneficial together. **One-at-a-time A/B testing rejects both, forever, and
will keep doing so however carefully each arm is run.** Nothing in the method
rules guards against it. `CLAUDE.md` has the neighbouring rule — *a correct
mechanism at inherited constants is a regression* — but that one is about
re-deriving constants after a change, not about a change that needs a *second
change* to show any benefit at all.

**Proposed remedy, and I think it earns a line in `CLAUDE.md`:** before
measuring a component of a mechanism, name the **minimal set that could
possibly work**, and measure that set. If a component's benefit is conditional
on another component, its solo arm is not evidence about the mechanism — it is
evidence about the component, and those are different claims. The tell is
available in advance: ask *"if this works, what else has to be true?"* Here the
answer was "the gradient has to point at the food", and nobody had checked
which way it pointed.

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
no fitness gradient, so no lineage can climb toward using channel B, ever. The
current state is the one that forbids evolution. Opening the gate is what
*creates* the axis for it, and the sign flip is then something a lineage could
have discovered on its own — `MUT_ABS_FLOOR` puts a sign change one mutation
away — if only the gate had let the sign matter.

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

Measured in §1c: sequential laying plus real decay produces a clean monotone
ramp, in every arm, at a magnitude the reader can act on. The gradient exists.
It climbs toward the nest, because B is laid **only while laden** and laden
means homeward, so the food end is always the older end. No deposit value and
no decay rate can turn that around — it follows from *when* the cells were
written, not from how much was written.

This is why the question was worth asking and why I had not answered it: I had
argued B was shapeless, and it is not. It is shaped, and pointed the wrong way,
which is a different diagnosis with a different and much cheaper fix.

### 4.4 On the biology

Four things from the real literature bear directly on this, and three of them
are uncomfortable for the current design.

**Trail polarity is a genuine unsolved-by-concentration problem in real ants.**
A scalar concentration trail is direction-ambiguous — an ant standing on one
cannot tell outbound from inbound by smelling harder. This is well known
precisely *because* it does not work. Pharaoh's ants (*Monomorium pharaonis*)
solve it with **trail geometry**: bifurcations are asymmetric Y-forks and the
fork angle encodes direction (Jackson, Holcombe & Ratnieks, *Nature* 2004).
Others solve it with visual landmarks or individual route memory. **Real ants
do not get direction from the concentration gradient**, which means asking an
empty ant here to read direction off B's scalar gradient is asking for
something evolution did not do either — *unless* the trail is given a built-in
spatial ramp, which is what the odometer does.

**The odometer is the biologically real mechanism, and it is already in the
engine.** Desert ants (*Cataglyphis*) barely use trail pheromones — too
volatile in the heat — and navigate by **path integration**: a stride
integrator plus a polarised-light compass. Hidden unit 4 is much closer to
that than to trail-following, and it is the half that works. That is not a
coincidence worth ignoring.

**Quality modulation and, crucially, *cessation*.** Ants lay more pheromone for
richer sources (*Lasius niger* scale deposition with sucrose concentration) and
**stop laying when the source is exhausted**. That negative feedback is what
prevents recruitment to a dead patch — it is the biological form of §2's step
3, and it is the piece genuinely missing here rather than merely mis-signed.

**Real channels differ by physics, not by built-in meaning** — which is the
biology's answer to concern 4.2. Alarm pheromones are small volatile molecules
(fast diffusion, seconds, "here, now"); trail pheromones are heavier and less
volatile ("this route, for a while"); cuticular hydrocarbons are non-volatile
(identity, by contact). The meaning is in which behaviour reads them. The
engine already has the matching knob — per-channel `rho` and `diffuse` — and
that, not semantics in code, is the axis along which species should
differentiate. Honeybees make the point from the other side: distance and
direction are carried by the **waggle dance**, a different modality entirely,
because a scent field is bad at direction.

---

## 5. What would settle it

In order, cheapest first. None of this has been run.

1. **The sign flip, in a bed, as a pair with the gate.** Four weights, no code.
   `nesthome`'s `arm=` and `trailfollow`'s `gate=`/`hidden=` already patch
   genomes at run time, so this needs no rebuild-per-arm. Arms: shipped /
   gate-only / **gate+sign** / sign-only. The third is the one nobody has run,
   and the first two are the ones that produced §Z7's conclusion.
2. **The local gradient a real ant actually reads**, laden and empty
   separately, on the played bed — the measurement §1b could not make, since a
   32-column band total is a mass distribution and the ant reads a 6-cell
   difference where it stands. This is what says whether §1c's single-ant
   derivation survives real traffic.
3. **Re-derive the `Move` row** if step 1 shows anything, per §2's last
   paragraph. A correct mechanism at inherited constants is a regression.
4. **Only then** the unit-7 food odometer, and only if the descent proves
   lifetime-capped.

And one thing not to do: **do not judge any of these on the colony's size
alone.** Foraging range, deliveries and `deepest` are what a trail is supposed
to move; a population count is several steps downstream and is exactly the
readout that produced the reject in the first place.

---

## Instruments

- `examples/onetrail.rs` — `mode=arith` (shipped genome, nothing overridden),
  `mode=walk` (one ant, standing trail, mirrored arms), `mode=timing` (§1c).
- `examples/nesthome.rs` — `channel A` and `channel B` profiles by 32-column
  band, `scene=bed`.
- `examples/trailfollow.rs` — the colony-scale question and the `gate=`
  presets. Note its presets each overwrite **both** gate pairs, so no row of
  its `mode=arith` table is the shipped ant, which is a mix.
