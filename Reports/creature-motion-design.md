# Motion is the empty axis: how many verbs a creature should get

**Status: design, awaiting the owner's call on §6. Written 2026-08-29.**

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

## 3. The slot budget, which is hard

`brain.rs` reserves growth room, and the reserve is the whole constraint.

| | live | reserved | free |
|---|---|---|---|
| outputs (verbs) | `BRAIN_OUTPUTS` 10 | `OUTPUT_SLOTS` 12 | **2** |
| inputs (senses) | `BRAIN_INPUTS` 16 | `INPUT_SLOTS` 24 | 8 |
| hidden units | `BRAIN_HIDDEN` 4 | `HIDDEN_SLOTS` 8 | 4 |

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

1. **32 genome slots.** Each output lights up `INPUT_SLOTS` (24) + one row of
   `HIDDEN_SLOTS` (8). Two verbs is 64 of 584 — an **11% larger search space**
   for evolution to cross. For a dev tool whose value is throughput, that is a
   real ongoing cost.
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

### 4c. The case for expanding the reserve now (and why I argue against)

There is a real argument that the migration is **cheapest today**: Lane B
shipped the export this morning, so approximately zero genomes have been
persisted. Every saved creature from here makes the migration dearer, and
`genome_manifest` already makes it safe.

**I argue against it anyway**, on the owner's own test. Growing `OUTPUT_SLOTS`
12 -> 16 takes `GENOME_LEN` from 584 to 768 — a **31% larger genome, for
ever**, and genome size is the search space evolution has to cross. That is a
permanent throughput cost paid now against a speculative benefit later. The
principle cuts both ways: **reserving capacity is not free either.**

If a third verb ever earns its place, pay the migration *then*, with evidence
that it is needed, rather than now on a guess about what we will want.

### 4d. Recommendation

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

## 6. The owner's call

1. **One verb or two?** §4d recommends one, with a stated condition for the
   second. The alternative is spending both now.
2. **Expand the reserve while it is cheap?** §4c recommends no — a 31% bigger
   genome for ever, against a speculative need. This is the call I am least
   certain of, because the migration genuinely does get dearer every day.
3. **Is "creatures can cross gaps" wanted at all?** §2d records that the
   current refusal is deliberate and hard-won. An impulse verb reverses a
   decision that cost two attempts, and it is worth being sure the fantasy is
   wanted before re-opening it.

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
