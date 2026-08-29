# Motion is the empty axis: how many verbs a creature should get

**Status: built 2026-08-29 — `BrainOutput::Impulse`, §7's five guards
green, and judged by the owner rather than by them.** All four of §6's calls
are answered; call 4 was the last, and is decision **E11**. §4c reverses this
report's own first recommendation — see there.

**The verdict this had to have, and could not get from a test.** Card
`20260829T154736312Z-2536dc`, a paired frame sequence of the two equal-mass
bodies with the verb off and on: *"With the new jump is great. I choose B"*.
That is the whole of what `CLAUDE.md`'s ethos asks and what §7's counters
cannot answer — whether a hop reads as a hop. The counters say the mechanism
fired (5 launches, 200 airborne frames, 0 falls, against 0 / 0 / 9 in the
control); the owner says it is worth having.

**What shipped, against what §5 promised.** One output, no per-species
locomotion, and the four numbers below are read off the four *shipped* body
plans with nothing authored per species:

| body | cells | launch speed | terminal speed | §5's row |
|---|---|---|---|---|
| `ant` `Chain(2)` | 2 | 2.00 | 1.73 | hops far |
| `ant_long` `Chain(6)` | 6 | 1.15 | 1.37 | shallower |
| `ant_wide` 5x2 | 9 | 0.94 | 2.04 | barely leaves the ground, **glides** |
| `ant_block` 3x3 | 9 | 0.94 | **4.74** | the same launch, drops like a stone |

The last two rows are the design claim: **identical mass, identical launch,
and 2.3x apart on the way down** — the whole difference being the bounding
box. `src/sim/creature.rs`'s `body_drag` is where it is computed and there is
no `match` on species anywhere in the path.

**The float limit E9 asks for is in the same three lines and cost nothing**,
exactly as §2c predicted: `buoyant_share` is `rigid::drag_through_liquid`'s
own `carried`, so a body no denser than what it is in has zero effective
weight and hangs. No creature material is buoyant today, so this is
mechanism rather than behaviour — but authoring one now works.

Written for the owner's question — *"I think motion could be a big
differentiator. Can creatures hop, fly, only swim, different speeds,
different jump/fly heights/distances"* — and for the specific debate he then
asked for: **one impulse verb, or more than one.**

Its governing constraint is his own, stated in the same breath and adopted
here as the test every proposal below has to pass: **everything should have a
cost and a benefit.**

`Reports/creature-appearance-design.md` is the input. It measured what makes a
creature *visible in a still frame* and found the answer is extent, that
palette is exhausted, and that shape at constant extent does nothing. This
report asks the question that one could not: **a still frame cannot show
motion, and nobody has ever measured whether motion differentiates creatures
here.** The owner's own verdict on a creature review card was *"need an
animation to tell"*.

---

## 1. Why cost-and-benefit is the mechanism, not the manners

This is worth stating first because it decides everything downstream.

**If a trait is strictly better, evolution converges and every creature ends
up the same.** That is not a hypothetical: it is the measured outcome of S5.
The survival-versus-`gut_bias` curve came back single-humped with the peak at
the **generalist** (0.911 at gut 0.0 against 0.877 herbivore and 0.614
carnivore), because a gut that eats everything had no compensating
disadvantage. Six months of diet-gene machinery produced one animal.

So variety is a *consequence* of trade-offs rather than of how many knobs
exist. A locomotion mode that is free is a locomotion mode every lineage
acquires, and the world ends up with one creature that hops. **The design
question for each verb below is therefore not "is this cool" but "what does
it cost, and what wins by not paying".**

---

## 2. What the engine already has, measured rather than assumed

Four findings, and three of them are larger than expected.

### 2a. Arbitrary body shapes already exist and are already used

`BodyPlan::Rigid(Vec<(i8,i8)>)` takes arbitrary cell offsets from the head,
authored facing east and mirrored for west. **The beetle already uses it** —
`Rigid([(-1,0),(0,-1),(-1,-1)])`, a 2x2 block — and differs from the ant on
four axes that are all data:

| | ant | beetle |
|---|---|---|
| body | `Chain(2)` | `Rigid` 2x2 |
| `tick_interval` (speed) | 6 | 8 |
| `move_cost` | 0.25 | 0.4 |
| `dig_force` | 1.0 | 0.3 |

`ant.ron`'s own comment on the pair: *"No code anywhere knows what a beetle
is."* The differentiation machinery the owner is asking for largely exists;
what is missing is that **exactly one creature was ever authored on it**, and
that creature is inert for unrelated reasons (§13o: `beetles=0` and
`beetles=9` measured bit-identical, because a beetle has no pheromone sense).

`dig_force: 0.3` against soil's `penetration_resistance` of 0.8 is already a
worked cost-and-benefit: the beetle is bigger and cannot dig. Nobody wrote a
rule saying so.

### 2b. The ballistic half of "jump" already exists, twice

There is **no velocity state on an organism** — creatures are relocated one
cell at a time by `step_chain` and have no vx/vy. But the engine carries two
full ballistic systems already:

- `particle.rs`'s `Particle` — `x, y, vx, vy`, plus per-particle `drag` and
  `gravity_scale`.
- `rigid.rs`'s `BodyCell` bodies — `vx, vy`, collision, damping, and a real
  terminal velocity.

### 2c. The float limit E9 asks for is already implemented physics

This is the largest finding in the report. `rigid.rs` gives a falling body a
terminal velocity from **weight-minus-buoyancy against drag**:

```text
    v = sqrt( 2 g d (rho_body/rho_fluid - 1) / Cd )
```

Three consequences, none of which needs new physics:

1. **The owner's float limit is the sign of `(rho_body/rho_fluid - 1)`.** His
   words were *"there should be a mechanical limit to floating as we wouldn't
   want really large creatures floating"* (decision E9). Denser than the fluid
   sinks; less dense floats. It is already density-and-size aware, so it
   scales with a heritable body **for free** and cannot be evolved around.
2. **Glide is already expressible.** `Cd` is the drag coefficient and the
   comment there states the regime was checked rather than assumed: *"Flat
   plates are nearer 2 and spheres nearer 0.5."* Wide-and-flat against
   compact is a difference this model already speaks.
3. **Terminal velocity already goes as the square root of body size** —
   measured on stone in water at 0.71 / 1.16 / 1.84 for 3 / 8 / 20-cell
   pieces. A big creature falls faster than a small one with no new code.

So the "let the body decide what the impulse does" design is not speculative.
The function already exists and is already tuned against playtest.

### 2d. …and the engine currently refuses to leave the ground *on purpose*

This is the countervailing finding and it is the principal risk in this
report. `step_chain` declines any move with no footing, and says why:

> *An ant that can see no footing ahead turns to look somewhere else, which is
> what a real one does and what makes it stop walking off ledges. **The cost
> is that ants cannot cross a gap; ants do not.***

That rule was arrived at after two failed attempts, both recorded at the call
site: an ant heading upward had all three candidates in open air, marched into
the sky, and **falls ran at 59–80% of all moves** through both. A discount on
airborne candidates could not fix it because the discount applied to every
option equally.

**An impulse verb re-opens exactly that failure.** Any design here must carry
a guard on falls-per-move, and §7 sets one.

---

## 3. The slot budget — enlarged 2026-08-29, and no longer the constraint

`brain.rs` reserves growth room. **This section was written when that reserve
was the binding constraint on the whole design; §4c's measurement removed it,
and the reserve was taken to 64/64/64 in the same change that records this.**

| | live | reserved (was) | reserved (now) | free |
|---|---|---|---|---|
| outputs (verbs) | `BRAIN_OUTPUTS` 10 | 12 | **64** | **54** |
| inputs (senses) | `BRAIN_INPUTS` 16 | 24 | **64** | 48 |
| hidden units | `BRAIN_HIDDEN` 4 | 8 | **64** | 60 |

`GENOME_LEN` 584 -> **12,352**. `live_slots()` **268, unchanged** — which is
the whole reason this was affordable.

**The argument in §4b and §4e is unchanged by that.** Slot pressure was never
the real reason to ship one verb rather than two: five of §4's nine candidates
need no verb at all, hover is the same verb held, and the two claimants for a
second slot are each waiting on something else. **A bigger reserve buys room,
not reasons** — a verb still costs live slots (below), thinking, and the risk
of a channel nothing reads.

Appending **within** the reserve is free: *"lights up storage that already
existed and was already zero: not one existing weight moves and `GENOME_LEN`
does not change."*

**Note for anyone reading the dead-end register: its entry saying an output
append is unlawful is superseded.** That entry describes the pre-S1 row-major
layout, where appending an output changed the row stride and renumbered every
weight with input >= 1. The S1–S2 rework reserved every growable dimension and
sized every block from the reserve. The law now holds in all three directions
**until a reserve fills**.

### What a verb actually costs

Not zero, and three of these are easy to miss:

1. **20 live genome slots — and price it in *live* slots, not reserved ones.**
   An output lights up `BRAIN_INPUTS` (16) weights plus one row of
   `BRAIN_HIDDEN` (4), so `live_slots()` goes **268 -> 288, +7.5% per verb**
   (two would be +14.9%). *An earlier version of this line said "32 slots, 64
   of 584, 11%" — that counted the **reserved** width, `INPUT_SLOTS` 24 +
   `HIDDEN_SLOTS` 8, against `GENOME_LEN`. Both halves were the wrong
   denominator: reserved slots are never drawn (§4c), so the search space is
   `live_slots()` and always was.* The corrected figure is smaller but it is
   the one that is real — unlike the reserve, **a live verb genuinely does
   enlarge the space evolution must cross**, which is the whole asymmetry
   between §4c (enlarge freely) and §4d (do not name freely).
2. **Thinking.** `synapse_fraction` charges per *active* synapse per tick.
   More outputs means more reachable synapses, so a richer brain is dearer to
   run. This one is self-limiting by design and is a feature — but it is a
   cost.
3. **A dead channel that looks alive.** `ant_ablation` found **eight of ten
   instincts produced bit-identical behaviour** — the brain was riding along
   while hardcoded locomotion policy did the deciding. Every verb added is
   another chance to ship a channel with a writer and no consequence, which
   `CLAUDE.md` names as the failure this project has hit three times.
4. **The third verb costs a migration.** Growing `OUTPUT_SLOTS` past 12 shifts
   `IO_END`, and every block after it renumbers. `genome_manifest` makes that
   a failing test rather than a silent reinterpretation — so it is safe, but
   it is work, and it invalidates every saved species.

---

## 4. The debate: how many impulses

The owner asked this directly. The decomposition below is the argument, and it
turns on one question asked of each candidate: **is this a decision, or a
property?** A property needs no verb.

| candidate | decision or property? | needs a verb? |
|---|---|---|
| **Speed** | property — `tick_interval` | **No.** Exists, unused by anything but the beetle |
| **Swim** | property — E9's water trait decides whether water is passable to the *existing* `Move` | **No** |
| **Float** | property — sign of `(rho_body/rho_fluid - 1)`, §2c | **No.** Already computed |
| **Glide** | property — `Cd` and size, once airborne | **No** |
| **Sink rate / fall speed** | property — terminal velocity from size | **No.** Already computed |
| **Jump / hop / leap** | **decision** — a discrete commit to leave the ground, distinct from stepping | **Yes** |
| **Sustained lift / hover** | decision per tick — *but see below* | **Contested** |
| **Dash / lunge** | decision — commit a direction at speed, forfeiting the ability to turn | Contested |
| **Grip / anchor** | decision — the inverse of impulse: refuse to be moved | Contested |

**Five of the nine candidates need no verb at all**, and that is the finding
that settles most of the debate. The owner's list — *"hop, fly, only swim,
different speeds, different jump/fly heights/distances"* — is mostly reachable
without spending a single slot, because heights and distances are what a body
does with an impulse rather than separate abilities.

### 4a. The case for ONE verb

**`Impulse`, continuous magnitude, paid per tick while held.**

- **Hover is the same verb held.** A low value is a hop; a sustained high
  value is a flap, paid every tick it is held. They do not need separate
  slots — the difference is duration and what the body does with it, which is
  §2c's physics. Splitting them would be authoring two knobs for one lever.
- **It leaves one slot in reserve**, and §3's fourth cost says the slot after
  that is a migration of every saved genome. Holding one back is itself a
  cost-and-benefit decision.
- **It is the smallest change that can fail visibly.** Given §2d's history —
  two attempts, falls at 59–80% of moves — the first airborne mechanism in
  this engine should be the one with the fewest confounds. Ship one verb, read
  falls-per-move, then decide.
- Fewer verbs, fewer chances of a dead channel (§3 cost 3).

### 4b. The case for TWO

The second slot has two credible claimants.

**`Grip` — refuse to be moved.** The genuine inverse of impulse, and they pair:
a world where things launch is a world where things get launched. Costs energy
and forfeits movement while held; buys survival on a ledge, in a current, in a
collapse. **The argument against is that nothing currently knocks a creature
about** except its own falls, so the benefit has no supplier yet. It becomes
compelling *after* impulse ships, not with it — which is an argument for
sequencing, not for spending the slot now.

**`Strike` — a committed lunge.** Predation (E7) is deferred to a probe, and
the register's re-test condition is *"predation cannot be a selective pressure
until a predator can find prey."* A strike verb does not solve finding; it
solves catching, which is not the blocker. **Premature.**

### 4c. Expanding the reserve — this section argued the wrong way and is corrected

**The first version of this section recommended against enlarging the reserve,
on the grounds that "genome size is the search space evolution has to cross".
That is false, and the code says so.**

`is_live_slot` gates on `BRAIN_OUTPUTS` / `BRAIN_INPUTS` / `BRAIN_HIDDEN` —
the **live** counts — not on the `*_SLOTS` reserves. Everything that walks a
genome walks `live_slots()`: `random_genome` draws only into live slots and
`debug_assert!(reserve_is_zero(&g))` afterwards, and the doc on `live_slots`
requires the mutation operator to do the same. So:

| | now | at `OUTPUT_SLOTS = 16` |
|---|---|---|
| `GENOME_LEN` (storage) | 584 | **712** |
| **`live_slots()` — the actual search space** | **268** | **268, unchanged** |

**The reserve is inert.** It is never drawn, never mutated, never read, and
never evaluated — `eval_brain` loops the live counts. Enlarging it costs:

- **+128 f32 per individual = 512 bytes.** At the 4095-organism ceiling that
  is **2.1 MB, worst case.** Against a world that allocates ~84 MB for
  pheromone planes alone.
- **No runtime cost.** Growing `OUTPUT_SLOTS` adds rows at the end that
  nothing visits; it does not change `INPUT_SLOTS`, which is the stride
  inside a row and the only part `eval_brain`'s locality depends on.
- **No growth in saved species files.** Lane B's export writes a sparse named
  wiring list, not raw floats, so a file lists what is wired and nothing else.

**So the timing argument, which was the weaker one, is now the only one — and
it points the other way.** The migration renumbers every stored genome and
invalidates every saved species. Today that set is approximately empty,
because the export shipped hours ago. Every creature saved from here makes it
dearer, and `genome_manifest` already turns a stale genome into a failing test
rather than a silent misread.

**Measured 2026-08-29, because the argument had been wrong twice and a third
round of reasoning was not worth having.** Two `forage_probe` binaries
differing only in the three reserve constants — 12/24/8 against **64/64/64**,
`GENOME_LEN` 584 against 12,352 — run paired and alternating, 6 reps, 4 seeds
x 6,000 frames, 55 ants:

| | median | range | spread |
|---|---|---|---|
| base 12/24/8 | 49.09 s | 42.67–50.40 | 16% |
| **64/64/64** | 45.20 s | 42.04–51.66 | 21% |

**Not measurable.** The bigger genome was faster in **3 of 6** pairs — a coin
flip — and the median says it is 7.9% *faster*, which cannot be real. The
same-binary spread (16%) exceeds the between-binary difference, so the effect
is below the floor. Read as *no detectable cost*, never as a speed-up.

**The control that makes it a comparison at all:** both binaries produced
**byte-identical output** — same moves, blocked, trips, depths, on every seed.
One simulation, two memory layouts.

**What it does not cover**, so it is not over-read later: one machine, ~55
creatures, no breeding population (S6 does not exist to make one), and no test
of memory pressure at the 4095-organism ceiling, where 64/64/64 is **202 MB**.
That figure is the only one with weight left, and the ceiling is shared with
plants so it is not a population this engine will reach soon.

**Three predicted costs that all dissolved on contact with arithmetic**, kept
because the pattern is more useful than the conclusion: a "31% larger search
space" (it is 0% — `live_slots()` is 268 either way); an L1 cache penalty (the
span `eval_brain` walks is 1.3 KB against 3.5 KB, both comfortable); and a
"20x birth cost" (~0.5 s spread across an overnight run). Each was a
*proportion* of a quantity nobody had converted to an absolute. **A proportion
of a tiny number is tiny**, and that is the whole lesson.

**Corrected recommendation: enlarge the reserve now.** It is close to free,
it is cheapest at this exact moment, and the thing it was argued to cost does
not exist.

**The general rule this yields, which is the transferable part:** when asked
whether to over-provision capacity, ask **does the unused capacity get walked,
drawn, or computed?** If it does, it is a real ongoing cost and should be
sized to need. If it is inert storage behind a liveness gate, over-provision —
the only real price is the migration you pay by *not* doing it early.

### 4d. Naming a reserved slot without implementing it — don't

Asked directly by the owner: is there harm in naming the second verb now and
changing it later, rather than leaving it empty?

**Yes, and it is a specific failure this project has already hit three times.**
Naming a slot is what makes it live. Add `Grip` to `BrainOutput` and
`BRAIN_OUTPUTS` becomes 11, at which point `is_live_slot` returns true for its
24 input weights and 8 hidden weights, and from that moment:

- `random_genome` draws into them, so every random genome carries a Grip
  wiring;
- the mutation operator perturbs them, so evolution spends effort on them;
- `synapse_fraction` charges for them as **active synapses**, so every
  creature pays to think about a verb;
- and `eval_brain` computes an output value that **nothing reads**.

That is exactly `CLAUDE.md`'s standing check — *"a channel needs a writer and
a reader, and the compiler checks neither… one that is read and never written
is worse, because every consumer of it is dead code that looks alive"* — with
the polarity reversed: written, costed, evolved against, and consumed by
nothing.

`live_slots`'s own doc states the sharper version of the same danger: *"a
perturbed reserved slot is invisible for exactly as long as its slot is
unnamed, and then springs to life as a connection nobody authored, in every
individual descended from the one that was perturbed."*

**The resolution is that a name costs nothing in a document and everything in
the enum.** Write down what slot 12 is intended for — §4b does — and leave
`BRAIN_OUTPUTS` at 11 until the day the verb has a reader.

### 4e. Recommendation

**One verb — `Impulse` — and hold the second slot deliberately.**

Not as a compromise: five of the nine candidates need no verb, hover is the
same verb held, and the two claimants for the second slot are both waiting on
something else (Grip on there being forces to resist; Strike on predation
finding prey at all). The recommendation is **"one now, one held, and a stated
condition for spending it"** rather than "one, because budget".

The condition for spending slot 12: **something in the world moves a creature
against its will** — a current, a collapse, a predator's impact. On that day
`Grip` has a supplier for its benefit and should be built. Until then it is a
verb whose cost is real and whose benefit is hypothetical.

---

## 5. What the one verb buys, per body

The point of a single verb is that the *body* decides what it does, using
§2c's existing physics rather than a table of creature types.

| body | mass (cells) | drag `Cd` | what one impulse does | what it pays |
|---|---|---|---|---|
| short chain | low | low | hops far, lands hard | nothing much — this is the cheap generalist |
| long chain | higher | low | shallower hop, more ink on screen | more metabolism per tick |
| wide flat rigid | high | **high** | barely leaves ground, but **glides** on the way down | measured **25–43% blocked movement**; cannot follow prey into a one-cell tunnel |
| compact dense rigid | high | low | almost nothing; drops like a stone | but is hard to shift, and digs |
| buoyant (low density) | any | any | **floats** rather than sinks | cannot follow anything underwater |

**Every row's benefit is another row's cost, and none of it is a table of
creature types** — it all falls out of cell count, bounding box and material
density, which are properties a body already has. That is the same
"state the difference as data" rule that `CLAUDE.md` records four failed
support models learning the hard way.

The heritable-body route E10 chose (chain length as one integer) plugs
straight into this: **a longer chain is a different animal in motion, not just
a longer one on screen.**

---

## 6. The owner's calls — answered 2026-08-29

1. **One verb or two? — ONE.** *"I agree with impulse now. I agree with
   holding an extra slot for another movement."* Slot 12 stays reserved with
   §4b's stated condition for spending it: the day something in the world
   moves a creature against its will.
2. **Name the held slot now? — NO**, per §4d. The owner asked whether naming
   it and changing it later does harm; it does, and the harm is mechanical
   rather than stylistic. The intent is written down in §4b instead.
3. **Expand the reserve? — YES**, per the corrected §4c, and this reverses
   what the first version of this report recommended. The owner's instinct
   (*"even if we don't use it for this we will want it for something in the
   near future"*) was right and the argument against it was wrong: the
   reserve is inert, `live_slots()` stays at 268, and the cost is 2.1 MB at
   the population ceiling.
4. **Is "creatures can cross gaps" wanted at all? — YES.** Asked directly,
   given that the refusal in §2d is deliberate and cost two attempts, the
   owner chose *"Yes, they should cross"* over both *"No, refusing is fine"*
   and an offer to render both readings first. Recorded as decision **E11**
   in `Reports/creature-evolution-plan.md` §0; answered in session, so there
   is no review card to cite, unlike E9 and E10.
   **The condition travelled with the authorisation**: the verb ships with
   §7's guards, and especially falls-per-move, or it does not ship. It has —
   see the status line at the top of this report for the readings.

---

## 7. Guards this must ship with

Non-negotiable given §2d, and each named with its known-good reading:

- **Falls per move.** The failure mode of both previous attempts, at 59–80%.
  Measured here on the foraging scene at 12,000 frames, **`moves 11,031 /
  falls 1,629` = 14.8%** — but that reading predates `d007c156` and
  `4c95233`, and lane C's post-merge run reports `moves 8812` without
  quoting falls. **Re-take the ratio on the current tree before building**;
  do not gate against 14.8% without doing so. A rise here is the mechanism
  failing, not a tuning problem.
- **Blocked moves.** `forage_probe` already reports it; the climb-over work
  moved it 0.311 -> 0.033. A rigid glider should *raise* it, and by roughly
  the 25–43% Lane A measured. If it does not, the body is not being read.
- **`ascii` counters byte-identical for species that do not use the verb.**
  The ant is `Chain(2)` with no impulse authored; if its counters move, the
  verb is not gated where it claims to be.
- **The verb must be ablatable.** Per lane C's finding that `-Bias->Move`
  reproduces the zero control exactly, `ant_ablation` is the instrument that
  says whether a new verb does anything. **Run it against the new verb before
  claiming the verb works** — an output with no measurable ablation effect is
  §3's cost 3 arriving.
- **Frame cost.** Airborne creatures keep chunks awake. Quote the whole-frame
  figure from `frame_profile`, paired and alternating, never a sub-phase.

---

## 7b. What the guards actually read, 2026-08-29

Every figure here was taken on this branch merged up to `main` at `3c464c2`,
on a container shared with three other agents. **Everything gated is a
counter**; the one wall-clock figure is quoted with its caveat and carries no
conclusion.

### 1. Falls per move — unchanged, and the bar now refuses a wrong budget

`forage_probe seeds=12 frames=12000 spacing=4`:

| | min | median | max |
|---|---|---|---|
| falls / move | 0.208 | **0.225** | 0.334 |
| blocked / move | 0.031 | 0.034 | 0.065 |
| tumbles / move | 0.683 | 0.791 | 0.857 |

The same three figures lane C measured before this verb existed
(`creature-motion-baselines-2026-08-29.md`), which is what "the shipped ant
takes the same draws in the same order" predicts.

**The bar ships as `forage_probe gate=1`, set at 0.40** — above the *worst*
seed rather than on the median, because which seed is worst reshuffles on any
legitimate change and a bar on the median gets rubber-stamped. It **refuses to
run** at any other frame budget: the statistic does not settle (0.239 / 0.225
/ 0.215 at 6k / 12k / 24k), so a bar quoted without its budget is not
reproducible. Checked: at `seeds=2 frames=6000` it exits 2 and says why.

### 2. Blocked moves — the body is read, and by 8–10x

Measured with the appearance lane's own instrument on its own preset, so the
figures are comparable to `creature-appearance-design.md` §5 rather than to a
number invented here. `creature_look mode=live count=40 frames=600`:

| body | cells | moves blocked |
|---|---|---|
| `ant` `Chain(2)` | 2 | **5%** |
| `ant_long` `Chain(6)` | 6 | **4%** |
| `ant_wide` 5x2 | 9 | **41%** |
| `ant_block` 3x3 | 9 | **43%** |

That reproduces §5's table exactly, so a rigid body is still blocked 8–10x as
often as a chain and still sits inside the 25–43% band §7 names. **What it
does *not* show is anything about this verb**, and saying so matters: blocked
movement is a property of `BodyPlan` that lane A shipped and this branch does
not touch. The claim *this* branch has to support is that the same bounding
box now also decides the descent, and that has its own reading — the 5x2 slab
and the 3x3 block, identical mass and identical launch, differ **2.3x** in
terminal speed and **1.64x** in time aloft (113 frames against 69 on one
plinth over one drop, same seed and same organism id).

### 3. `ascii` counters — byte-identical

Two binaries, this branch against `origin/main`, same scenes:
**1,109 counter lines match byte for byte.** The 146 lines that differ all
carry a millisecond figure, which is the machine rather than the simulation.

### 4. Ablatable — and it is the largest single output in the table

`ant_ablation seeds=3 frames=6000`, 22 arms:

| arm | travelled | coverage | foraged | first-pickup | pickups | deliveries |
|---|---|---|---|---|---|---|
| `zero` (control) | 0.0 | 46 | 0.00 | never | 0.0 | 0.0 |
| `authored` | 71.2 | 1,267 | 0.06 | 1,208 | 2.7 | 0.0 |
| `Impulse=lo` | **71.2** | **1,267** | **0.06** | **1,208** | **2.7** | 0.0 |
| `Impulse=hi` | **331.4** | **3,989** | **0.50** | **118** | **29.3** | **8.3** |
| `-Bias->Move` | 0.0 | 46 | 0.00 | never | 0.0 | 0.0 |

Three readings, and they are three different claims:

- **`Impulse=lo` reproduces `authored` to every printed digit.** At a
  saturated-negative weight the `> 0.0` gate never opens, so the run is
  bit-identical to the ant that ships. The gate is where it claims to be.
- **`-Bias->Move` still reproduces `zero`**, which is the harness's own
  positive control and the thing that makes every other row readable. If it
  ever stops matching, nothing in this table means anything.
- **`Impulse=hi` moves every column**: 4.7x travelled, 3.1x coverage, 8.3x the
  fraction that ever forages, first pickup ten times sooner, 11x pickups —
  and deliveries **0.0 → 8.3**, a colony completing nest → food → nest, which
  the authored ant does not do at all in this scene. It is not a dead channel;
  it is the largest single output in the table by a wide margin.

**And that last row is a warning, not only a result.** §1's whole argument is
that a strictly better trait makes every lineage converge on it — measured, in
S5's diet genes producing one animal. On these columns hopping *is* strictly
better, and the reason is that **this scene cannot see what it costs**:
`ant.ron` is `start_energy` 900 at `idle_cost` 0.10 and `tick_interval` 6, an
idle life of ~54,000 frames against a 6,000-frame run, so nothing starves and
an energy price is invisible in every harness we have. That is exactly the
arithmetic decision **E14** (the horizon change) is about. Until it lands,
*"is the impulse priced correctly"* is not a question any instrument here can
answer, and this report does not claim it is.

### 5. Frame cost — a counter first, because the box was not quiet

The shipped ant never launches, so the only cost the verb adds to today's
game is one `f32` clamp and compare per creature tick, plus 20 bytes of
`Option<Flight>` on every `OrganismState`. Guard 3 is the evidence that the
first does no work: the counters are byte-identical.

**What a *hopping* creature costs is scheduler slots, and that is countable
rather than timed.** An airborne creature is rescheduled every frame instead
of every `tick_interval`, so its footprint is **6x while it is in the air** —
and it carries no `eval_brain` on any of those frames, because
`creature_tick` returns before `sense`. Against
`scheduler::MAX_CREATURE_SITES_PER_FRAME` = **256**:

| | sites per frame |
|---|---|
| 52 ants walking (`tick_interval` 6) | ~9 |
| 52 ants *all* airborne at once | 52 |
| the reserve | 256 |

So the budget binds at **256 concurrently-airborne creatures** against ~1,536
walking ones. That is the number to re-take when S6 breeding raises the
population past 55, and it is a graceful ceiling rather than a cliff: #118's
own comment records that the budget bounds work and never gates whether a
creature ticks, so exhausting it stretches an arc rather than dropping one.

---

## 8. What this report does not propose

- **No new senses.** Eight input slots are free and it is tempting; nothing
  here needs one. A creature that can jump does not need a new sense to decide
  when — `Crowding`, `Ahead` and the pheromone channels already exist. Adding
  a sense is a separate proposal with its own cost.
- **No per-species locomotion list.** No `can_fly: true`. That is the table of
  creature types §5 exists to avoid.
- **No change to `tick_interval`.** Speed already varies per species and
  nothing but the beetle uses it. That is an *authoring* gap, not a mechanism
  gap, and it is free to close today.
- **Nothing about predation.** E7 stands: predation cannot be a selective
  pressure until a predator can find prey, and no verb in this report changes
  that.
