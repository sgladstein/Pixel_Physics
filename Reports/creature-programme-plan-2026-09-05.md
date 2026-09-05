# The creature programme from here — plan, 2026-09-05

*Written at the end of a day that started with the owner asking why the
creatures are not interesting, and ended with three separate findings that the
bottleneck was not where I said it was. This is the plan that falls out of
those, in the order the evidence supports rather than the order the asks
arrived.*

---

## 1. The through-line, and it is a correction to my own framing

I began by telling the owner the bottleneck was **population size and run
length**. The owner corrected it to **the environment**. Both were partly
wrong, and the day's measurements say so three times over:

| what I expected to be blocking | what actually was |
|---|---|
| not enough generations to evolve away the dig drive | the world could not **see** digging — every verb was free |
| the bed does not punish an ant that ignores trails | the trail pathway is **95% saturated off** except at ~60% laden |
| shelter needs a hazard outside | the ant is **blind** and cannot tell a tunnel from a pit |

**Only the first was an environment problem, and fixing it worked
immediately** — pricing the verbs took the dig ablation from a 50.0% null to
63.6% on 4 seeds of 4. The other two are the *animal*: a behaviour the genome
cannot express or the senses cannot trigger is not selectable at any
population size, in any world.

**The claim needs stating narrowly, and the first draft did not.** *"A
behaviour the genome cannot express is not selectable at any population
size"* is too strong, and this report's own evidence refutes it:
trail-following is not a dedicated pathway, it is four weights into two
differential hidden pairs whose gate is an accident — an emergent combination
with no purpose-built route. The defensible version is about **senses**
rather than the genome: *a behaviour whose triggering quantity is not in the
input vector is not selectable.* Senses are species fields, not heritable
slots, which is why they are the thing to check.

**So the standing rule for this programme is: audit the animal before
building the environment.** The instrument is `creature_arena --
arm=ablate`, and the check is two questions — *can this animal perceive the
thing?* and *can it express the response?* Both were skipped today, twice,
and both cost a wrong claim.

## 2. Two routes, and they need different things

**Route A — the world selects.** Needs all three of: the animal can express
the behaviour, the environment pays for it, and the population is large and
long-lived enough to climb the gradient. Today it is blocked on the *first*,
which is the one nobody was looking at.

**Route B — the player selects.** Find the individual doing something odd,
jar it, breed it. Needs only that variation exists and that a person can see
it. It sidesteps the environment question entirely and is much faster at
producing a *specific* trait — dog-breeding rather than waiting for wolves.
It is also the owner's own stated game: *"give me the tools, data, access to
the parameters... I do that testing myself."*

**Route B is not a substitute for Route A** — a trait the player breeds by
hand will not persist once selection stops paying for it — but it is the
faster way to find out **what this genome can actually do**, which is the
question underneath everything else.

## 3. The plan

### Phase 1 — split in two after review · task #5, the behaviour prospector

**1a, and it goes first: measure whether an ant's behaviour is heritable at
all.** `clone_variance` measured broad-sense heritability on plant size at
**H² = 0.013 / 0.054 / 0.000** against a 0.75–0.82 positive control, and
`clone_identity` found two clones one column apart ending at 464 against
1,057 cells. **For creatures the number is unmeasured.** If an ant's
`travelled`/`feeding` is mostly *where it stood*, every jar the prospector
pulls is a lucky ant, breeding from it regresses to the mean, and 1b is dead
before it is built. `specimen::drift`/`release` already build the clone arm
and `clone_variance`'s design transfers wholesale. One harness, a yes/no, and
it gates everything after it.

**1b, gated on 1a: the rarity detector, auto-jarring and the UI.**

**Three corrections the review forced, each of which would have produced a
confident wrong number:**

- **`creature_space`'s descriptors are population statistics, not per-animal
  ones.** `travelled` is a p90 over ids, `commute` a median, `feeding` the
  *fraction of ids that ever carried*, and `depth` a global mean with no
  per-id key at all. A prospector needs a descriptor per living animal, so
  this is a new accumulator, not a wiring job. "Every piece exists and none
  are connected" overstated it.
- **The "7 of 16 cells" bar is the wrong baseline.** It is coverage over 24
  *independently random* genomes; a live colony is one founder plus mutation
  at generation 1–7. That is a different space and an upper bound the
  prospector structurally cannot reach — *ask what your number counts*, in
  its exact form. Take the baseline from a census on a live colony.
- **`Specimen`/`Provenance` have no field for behaviour numbers**, so "jar it
  with its numbers attached" is a schema addition needing `#[serde(default)]`
  for old jars.

**And the falsifier, which the first draft lacked.** "If it finds nothing
interesting, that is a hard finding" was unfalsifiable: a null has four
readings — poor space, wrong descriptors, wrong threshold, or a population at
generation 1 that has not diverged. So: **plant a genome whose oddity is
known** (the zeroed brain at survival 0.420, or `creature_space`'s `r016`,
which travelled 218 cells and ate nothing) and require the prospector to jar
it. The mutation-off arm becomes the **null distribution the threshold is
calibrated against**, not a pass/fail control — a clonal colony is *not*
behaviourally clonal, so it will produce jars, driven by position.

**Precondition:** `genome_drift` refuses to report below generation 2, for
exactly this reason. Check it before prospecting at all.

### Phase 2 — the two animal-side blockers found today

**2a. Open the ant's eye — it does not need a new sense · task #7, the ant's eye.**
**Revised after review, which refuted the first version of this section.** I
proposed appending a `ThreatNear`/`ThreatBearing` pair. That is unnecessary:
**a beetle is already visible prey to an ant the moment the eye opens.** The
ant's gut is `traits: (0.0, …)` and beetle flesh is `food_class: 1.0`, so
`diet_quality` is `(1 − 1/2)² = 0.25` and `diet_yield` is **50 against
`EAT_YIELD_THRESHOLD` of 12** — `is_visible_prey` returns true. The only
thing suppressing it is `creature.rs`'s `if def.sight_range > 0`, and the ant
authors no range. `brain.rs` records that `PreyBearing`'s sign was left free
*precisely so a weight can mean approach or retreat*.

So: **author `ant.ron` a `sight_range` and measure whether the existing prey
pair separates a beetle from a nestmate in the bed.** One asset line instead
of an engine change, a manifest bump and a new cast filter. The price is
already authored — `sight_fraction` is on the ant, put there so that "the day
`sight_range` becomes heritable the gene arrives into a world that already
charges for it". A dedicated `Threat` pair earns its slot only if that comes
back null, and then on evidence rather than assertion.

Two things this carries either way: `creature.rs`'s `priced_but_blind >= 1`
control fires on the ant and **will go red the moment it gets a range**; and
a cast is 328–1,186 `World::get` per ant per creature tick, which is a frame
cost to state before proposing, not after.

**And `Reports/creature-genome-flexibility-2026-09-02.md` already carries a
written, never-executed "Stage 2b — switch the eye on, and price it", with
its own guard.** Execute that rather than reinventing it.

**2b. Let the ant perceive the roof it is already under · task #8, perceiving the roof.**
**Also revised: the first version was wrong twice.** It said nothing in the
genome distinguishes a tunnel from a pit. In fact `is_sheltered` exists in
`creature.rs` — I wrote it today — it distinguishes a horizontal gallery from
a vertical shaft at any depth, and the exposure charge **already calls it
every creature tick**. It simply is not a `BrainInput`. And
`SurfaceCurvature` is *live* in the ant (`curvature_radius: 2`, wired to
`Drop` and `DropSpoil`), reading enclosure isotropically.

So the missing piece is one weight from an enclosure signal to `Dig`/`Move`,
and CLAUDE.md's own reachability condition says anything on a single weight
is reachable. **Append one input reading `is_sheltered`** — which makes the
animal perceive *exactly the quantity the exposure tax charges*, the ideal
case for selection.

**But resolve the diagnosis before building it.** My "the hole is the wrong
shape" reading is one of two, and the engine's own census leans the other
way: `burrow_probe arms=colony` records **roofed void 89–139** on the shipped
behaviour, so colonies demonstrably build roofed space. The competing reading
— *the shelter is fine and the ant walks back out of it, because nothing
tells it it is indoors* — fits the 66.2%-exposed figure equally well, and is
the one this section's fix addresses. The separator is cheap and already
built: roofed-void census against `exposed_ticks / creature ticks`. Plenty of
roof plus ants outside means behaviour, not shape. **Measure that first**;
CLAUDE.md's *resolve an ambiguous complaint before building anything* is
exactly this case.

### Phase 3 — the environment work, which now has something to bite on

**3a. Clumped food · task #3, clumped food.** A trail only pays when re-finding a patch
beats re-searching for it, and the lab bed scatters food within a few body
lengths of everything. A `LabBox` knob plus the parameters page. **Do the
saturation check first**: the trail mechanism is worth 18% of the movement
decision at ~60% laden and under 1% empty or full, so how much trails matter
depends on where the bed puts the crop-fill distribution — which is a lever
nobody knew they had.

**3b. Re-run the exposure experiment** once 2b lands. It is already built
(`creature_arena -- arm=ablate input=Bias output=Dig exposure=0.05`) and its
null is on record.

### Phase 4 — measurement debt

**Beetle seed sweep · task #6, the beetle seed sweep.** Beetles can breed now and a birth is
verified; whether it changes the ecology is unknown because two seeds
disagreed in direction. 8–12 paired seeds read at a direction statistic. Pure
measurement, no design decisions, and it closes something half-open.

## 4. What I am not proposing, and why

- **Tuning the economy further.** The owner's standing direction is *stop
  balancing, start exposing*. The verb prices shipped with their derivation;
  the next move on them is the owner's.
- **Food sharing between adults**, which division of labour needs. Real, and
  much larger than anything above — an ant that specialises away from feeding
  currently just starves. Not until Phases 1–3 have paid.
- **More generations as a first move.** It is real (generation 7 per 60,000
  frames) and it is second-order until a behaviour is both expressible and
  paid for.

## 5. Open decisions that are the owner's, not mine

1. **What should the hazard outside actually be?** Exposure exists, is
   switchable, and is now on the parameters page — but it is flat. Heat,
   drought, weather, or a predator that cannot follow are different games,
   and *what the fiction is* is the owner's. *Whether the hazard that already
   exists bites* is not a question: it is measured, and its null is on
   record.
2. **The carry-cue verdict** (card `20260905T062117283Z-6179f0`) decides
   whether the shipped visual default stays brightness, switches to the
   magenta, or is pulled.
3. ~~**How much of the ant should stay authored.**~~ **Withdrawn — it was
   two measurements wearing a question, and punting a measurable thing to the
   owner is a failure mode.** For a *sense*: `CREATURE_TRAITS` is 3, being
   gut bias, birth grant and reproduce-at; `sight_range` and
   `curvature_radius` are plain species fields and **are not heritable at
   all**, so "let the animal evolve its own eye" is not a position that can
   be adopted — the mechanism does not exist. For a *weight*: at
   `mutation_rate` 0.0058456 a given slot is touched about once per 171
   births against 45–88 births a run, so an unauthored weight moves ~0.3–0.5
   times per run and cannot be found. **The real question is whether you want
   the mutation rate raised or the runs lengthened, and both are dials on the
   parameters page.**
