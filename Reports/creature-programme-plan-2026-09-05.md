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

### Phase 1 — the behaviour prospector (Route B) · task #5

**Score every living animal on behaviour descriptors, find the ones in rare
or unoccupied corners of that space, and jar them automatically with their
numbers attached.**

Every piece exists and none of them are connected: `creature_space` already
computes behaviour-space coverage over `travelled` / `commute` / `feeding` /
`depth` (MAP-Elites' measure); the specimen shelf already stores genomes as
jars; `labbatch` already runs racks of chambers headless.

**Why first.** It is the owner's third ask; it pays off whatever the
environment does; and it is *diagnostic* — if it finds nothing interesting,
that is a hard finding about the reachable behaviour space rather than a
failure. Measured baseline to beat: 24 random genomes filled **7 of 16**
descriptor cells and **14 of 24 never moved or ate at all**.

**The design risk, stated up front: novelty is not interest.** A naive
rarity detector mostly finds noise, and the descriptors and the rarity
threshold *are* the design. Two guards: report the descriptor values on every
jar so a person can tell an outlier from a glitch, and include a negative
control — a population with mutation off should produce near-zero jars, or
the detector is finding sampling noise.

### Phase 2 — the two animal-side blockers found today

**2a. Give the ant a sense that a predator exists · task #7.** The shipped
ant authors no `sight_range`, so it defaults to 0 and `sight()` returns
nothing: it is blind. The only sense that fires on a beetle is `Crowding`,
r=2 and unable to tell a beetle from a nestmate. **No environment can select
for fleeing, sheltering, guarding or alarm while the animal cannot know a
predator is there.** The brain reserves slots, so appending a
`ThreatNear`/`ThreatBearing` pair — the mirror of `PreyNear`/`PreyBearing`,
*something that can eat me* rather than *something I can eat* — moves no
existing weight. Verify by ablating the new sense with beetles in the bed.

**2b. Make a tunnel distinguishable from a pit · task #8.** Ants are in the
open on **66.2% of creature ticks**, and exposure priced at **18.2% of total
burn** still does not make digging pay, because `(Bias, Dig)` digs along the
heading and from a surface start that is a **pit, open to the sky**. A pit is
not shelter. Nothing in the genome distinguishes the two, so there is no
variation for a hazard to select on. Check what `SurfaceCurvature` already
reads before adding anything — it may carry half of this.

### Phase 3 — the environment work, which now has something to bite on

**3a. Clumped food · task #3.** A trail only pays when re-finding a patch
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

**Beetle seed sweep · task #6.** Beetles can breed now and a birth is
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

1. **What should the hazard outside actually be?** Exposure exists and is
   switchable but flat. Heat, drought, weather, or a predator that cannot
   follow are different games, and the choice is fiction as much as
   mechanism.
2. **The carry-cue verdict** (card `20260905T062117283Z-6179f0`) decides
   whether the shipped visual default stays brightness, switches to the
   magenta, or is pulled.
3. **How much of the ant should stay authored.** Giving it eyes and a
   tunnel-versus-pit sense is designing the animal, not just the world, and
   there is a legitimate position that says the animal should have to evolve
   those too. That position costs far more generations and I would want it
   stated deliberately rather than arrived at.
