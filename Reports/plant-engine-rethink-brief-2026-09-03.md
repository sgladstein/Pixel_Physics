# Brief: re-think the plant engine

*Written 2026-09-03 for a session that will run unattended, overnight, on the
evolution lab. The owner is asleep. **This is a direction and a set of
constraints, not a plan** — deliberately. If it told you what to do it would
cap you at what the session that wrote it could imagine, and the whole point
of asking is that that was not enough.*

---

## The thesis, in the owner's words

> *"As much as possible nothing should be hard coded, we don't want to design
> specific behavior but create a flexible system that will allow variety to
> evolve."*

Everything below serves that sentence. Where this brief and that sentence
disagree, the sentence wins.

And, on scope: *"It can consider or recommend a full overhaul of our genome /
plant engine. I want it to think outside the box and not make small
incremental suggestions."*

## What you are authorised to do

- **Reconsider closed decisions.** Anything in `Reports/dead-ends.md`,
  anything a previous session settled, anything this brief asserts. A rejection
  in that file carries the condition it depended on; conditions expire. Say
  which one you are reopening and why the condition no longer holds.
- **Propose a rewrite.** If the answer is "the genome's shape is wrong and
  here is the shape it should have", that is a better outcome than ten tuned
  constants. Cost it honestly and say what it invalidates.
- **Decide, and report.** You do not have a person to ask. Make the calls,
  write down the evidence and the reasoning, and put anything that needs an eye
  into the review queue as a card (`.claude/skills/review/SKILL.md`) — posting
  is fire-and-forget, so post and carry on rather than waiting.
- **Land what you are sure of.** Push, open a PR, merge it if CI is green
  (`CLAUDE.md` carries the owner's standing authorisation for both). Leave the
  rest as a report with its numbers.

**One thing to hold back on**, and it is the only one: do not ship a change
that reallocates a weighted budget without a card in the queue showing both
arms. `Reports/why-changes-cost-so-much-2026-08-27.md` is the scar — reshaping
`phototropism_dir`'s codomain, which is the *repair `dead-ends.md` itself
prescribes*, gave the growth weights a direction they had never had; trees
spread instead of climbing, never reached `seed_maturity`, and **reproduction
went to zero**, with every gate green but one. Proposing that class of change
is wanted. Shipping it unseen is not.

---

## The provocation: where the design currently lives

The owner's sentence says *don't design specific behaviour*. It is worth being
concrete about how much specific behaviour is currently designed, because the
answer is "nearly all of it, and it lives in `assets/species/*.ron`".

- **A genotype is ten scalars that *multiply* authored species constants**
  (`plant::genotype` returns `1 ± variance`). So the species file is the
  design and the genome is a variation dial around it. The consequence found
  2026-09-03: **zero times any genome is zero**, so a species that authors
  `branch_chance: [0.0, 0.0]` has a lineage that can never discover branching.
- **`CellType` is a closed enum of eight**, six of them plant. A lineage cannot
  invent a tissue; `Flower` and `Fruit` had to be added in Rust.
- **`Behavior` is a closed enum of eleven** — Divide, Grow, BudBreak,
  Photosynthesize, Transpire, Absorb, Reproduce, SecondaryThicken, Ripen,
  Germinate, StructuralAnchor. A lineage cannot invent a metabolism.
- **`FateWhen` is a closed enum of five** (Grew, Node, Stale, Flush, Ripe).
- **A growing tip scores its candidates on a fixed set of six terms**
  (continuation, light, wind, upward, crowding, heading inertia). Only the
  weights vary; the *set* is authored.
- **Discrete genes take authored values.** `LOCUS_ALLELES` fixes how many
  alleles each locus has, and `BRANCH_ANGLE_ALLELES`, `INTERNODE_ALLELES`,
  `LEAF_RATE_ALLELES`, `LEAF_TRANSPIRATION_ALLELES` and
  `WOOD_DENSITY_ALLELES` fix what each one *means*. Selection picks among
  authored options; it cannot find a value nobody wrote down.
- **A species names six materials by string** — shoot, root, leaf, flower,
  fruit, windfall — and none of them is heritable. A lineage cannot change
  what it is made of, which is also why it cannot change how its seeds fall.
- **The species id is copied to offspring unchanged**, so **speciation is
  impossible by construction**. `individual_as_species` exists and refuses,
  because it copies the parent *species'* fates rather than the individual's.

The one place the engine already does what the owner is asking for is the
**fate genome** (`organism::FateGenome`): a heritable, mutable production rule
with an `Insert` operator, so a lineage really can acquire a rule its species
never had. It is the existence proof. It is also the only one.

**So the sharpest form of the question this brief exists to ask:** should
`assets/species/*.ron` be an *input* to the simulation at all, or should it be
an output — a starting point that a population immediately leaves? Nothing
here says it must change. It says nobody has asked.

---

## What is already established, as a floor rather than a scope

From `Reports/plant-reseeding-2026-09-03.md`, measured this week. Treat these
as ground you do not have to re-cover, **not** as the list of things to work
on.

**Seed dispersal does not exist as a channel, and that is what started this.**
Not one step of a seed's journey has a heritable dial and two of three have no
dial at all. Wind biases *gases only* — `update_gas`'s own doc calls its bias
"the first thing in this engine that lets the field move *material*", and
nothing else displaces a cell. `roll_along_slope` gives a `seed` a reach of
`1/tan(55°)` = **0.70 cells**. `friction_angle` is a property of the material,
and the material is not heritable. The genome contains nothing about seeds.
The only indirect lever is crown width, and on the lab's own crop it is
multiplying an authored zero. **This is the concrete case the owner's thesis
was provoked by, and it should be in scope whatever else is.**

**Three architectural levers fired and were invisible.** Sympody, tropism and
acrotony are *not* disabled — two are live heritable loci at two alleles each,
and `acrotony` is authored across the full range (conifer 0.8, herb 1.8,
scrambler −1.4). They produced 46–186 sympodial forks per shrub and
1,797–2,750 plagiotropic steps per conifer, and the owner's reading of the
contact sheets was that nothing had changed.
`Reports/plant-appearance-design.md`'s diagnosis: all three change *which cell
gets a label*, and the silhouette was set by two things none of them touch —
every species was ~90% wood and ~5% leaf, and every plant drew from one
four-brown palette and one four-green one.

**That verdict is two rounds old and both of its stated causes have moved
since.** `LOCUS_LEAF_ECONOMY` became a real gene carrying its own foliage
bands, bark bands now derive from wood density, and `cell_scale` doubled the
world's resolution — which the owner rated 5 and called the direction rather
than an experiment. `examples/plant_probe.rs` already prints **foliage share**,
the median percentage of a plant's cells that are leaf. That one number decides
whether the old verdict still stands, and it is a five-minute check. The owner
has explicitly asked for these three to be re-evaluated; the useful framing is
*re-test a stale verdict*, not *enable a disabled feature*.

**Four things real plants do that the engine has no word for**, from the same
report's survey: sex (every seed is one parent's genome plus jitter — no
pollen, no crossing, no recombination anywhere); mineral nutrition (zero
occurrences of nitrogen, nutrient, phosphorus or mineral in the plant code —
income is `min(light, water)` and litter rots into soil *mass*, never
fertility); seasons (the clock has a day, not a year, so no deciduousness, no
dormancy, no stratification, and `noon_equivalent_light` deliberately divides
the day/night phase out of every economic decision, which makes photoperiod
invisible by construction); and movement of tissue that already exists (every
grown cell is immovable — the seed is the only exception in the engine — so
there is no heliotropism, no nyctinasty, no wilting posture, no tendrils).

The owner named sex, clonal spread, seasons, defence and tissue movement as the
ones he liked, and flagged tissue movement as *"maybe too complicated"*.
**Clonal spread is the one worth knowing about before you start**: a `RootTip`
fate could set `child: GrowingTip` and put a shoot up from underground, nothing
stops it, and every shipped species gives roots `plastochron: [0]`, which
disables nodes outright. Rhizomes, runners, suckers and root-sprouting after
fire are *one authored number* away and no species has taken it. That is a
different class of gap from the others and it is worth looking for more of it.

**Seasons are more natural in the lab than outdoors, which is not obvious.**
The box holds the sun at a fixed frame and declares itself sunless for light,
but **sky temperature is untouched** — a sealed room still feels the day. So
the temperature cycle is already running and only the light is frozen. In a
grow room a season is *the schedule you set*: photoperiod on the fixtures,
temperature, watering. `set_sky_hold(None)` restores the cycle in one call, and
the design guide already measured the light schedule as the game's largest
lever at 2.4x reproduction. A fixture schedule also keeps the performance
property the ceiling was built to buy — a moving sun re-solves every field tile
every frame, where lamps switching on and off are two states with a re-solve
only at the transitions.

---

## Questions worth answering — not steps, and not a complete list

Pick the ones the evidence supports. Add ones nobody thought of. Answering two
of these well beats touching all of them.

**On the genome's shape.**
Is a multiplier on an authored constant the right meaning for a genome slot at
all? What would an additive or a codomain-carrying slot cost, term by term?
Which slots are dead *by measurement* rather than by inspection — at 0x and at
10x, on a case whose answer you know? Should a lineage be able to leave the box
its species defines, and if so is that speciation, and does anything downstream
survive a species id that changes? **The owner has explicitly delegated this
decision to you.**

**On what the genome should contain.**
Ten scalars and six loci is a very small genome for a thing that is supposed to
produce variety. Is the right move more slots, different slots, or a different
representation entirely? What does the fate genome — which already does the
thing the owner is asking for — suggest about the rest?

**On dispersal specifically.**
It has no channel, and any answer has to say what carries a seed and what makes
that heritable. The three shapes visible from here are a launch offset in
`bear_seed_at` (cheap, one draw, but a seed appearing eight cells away with
nothing in between reads as magic), wind on light powders (reuses a field
channel that already exists and already biases gases, gives the player a verb
to turn on, and costs frame time that must be measured because it keeps chunks
awake), and animal dispersal (most satisfying, furthest away, and note that
*today* the colony measures as a seed predator that cuts the stand 2.6x). None
of those is a recommendation.

**On the three invisible levers.**
Does foliage share still read ~5%? If it has moved, what do sympody, tropism
and acrotony look like at the current resolution and palette? If it has not,
is composition itself something a genome should be able to move?

**On what "flexible" costs.**
The appearance lesson cuts against naive generality: three general levers fired
and produced nothing, because the axis that reached the screen was not
parameterised. More degrees of freedom is not automatically more variety. The
question to keep asking is `CLAUDE.md`'s — *which pixels does this lever move* —
before ranking anything by how general it sounds.

---

## The discipline that still binds

Read `CLAUDE.md` properly; it is not decoration and most of it was paid for. The
parts that will bite this work specifically:

- **The ethos.** An outcome is a distribution, not a binary; and there must be
  a verb that delivers something. Both were arrived at independently on the
  plant line.
- **Ask what your number counts when nothing is wrong** — and run the positive
  control, the case whose answer you already know is non-zero. The single
  worst-recurring failure in this repo. This session's own instrument tripped
  it twice: two independent counts of "how many seeds were made" shared one
  blind spot and corroborated each other into a wrong answer, which is why
  `World::seeds_borne` now exists.
- **A cost that vanishes may be work that vanished** — and its inverse, which
  this week's lighting change ran into: **a cost that appears may be biomass
  that appeared.** The control is an arm with nothing living in it.
- **Frame cost is a hard constraint, not a tiebreaker.** Say what a proposal
  costs. `examples/labperf` and `examples/reseed_probe`'s `COST` line are the
  instruments; `founders=0` is the arm that prices a mechanism rather than the
  biosphere it grows.
- **Do not break the outdoor game.** It shares this engine and is not in scope.
  A change to a material, to `update.rs`, or to the field reaches every forest;
  a change to `src/lab/` does not. Say which side of that line a proposal sits
  on. (This is why `seed.ron` gaining `falls_through_organisms` — measured to
  double germination — is still uncommitted.)
- **Editing an asset `.ron` does nothing until the next build**, and a stale
  example binary prints plausible numbers with a newer mtime than the source.
  Identical output across a change that must have moved something is the tell.

## Working unattended

- **Post cards, do not block.** `review.py post` is fire-and-forget and the
  owner may read it on a phone; a card must stand on its own — a title, one
  answerable question, and **the discrete event count in `meta`**. Prefer a
  paired comparison to a single run against a remembered impression; outcomes
  here have enormous spread.
- **Land what is settled, report what is not.** A branch nobody can see is a
  branch nobody merges. Open the PR.
- **Leave the next session a note.** `Reports/lanes/` for the protocol-level
  material, a report in `Reports/` for the findings, and its line in
  `Reports/README.md` in the same commit — `docscheck.sh` gates that.
- **Record what you tried and rejected**, with the condition the rejection
  depends on, in `dead-ends.md`. That is the file that makes the next session
  cheaper, and it is the only reason this one could move as fast as it did.

## Reading map — and what not to open

Context is the binding constraint on a long unattended run. Four documents here
are 60k–97k tokens each and **none of them should be read whole**.

| want | read |
|---|---|
| the method, and the ethos | `CLAUDE.md`, whole. It is the one thing worth its cost |
| what the lab is, and every decision already taken | `Reports/lanes/evolution-lab-coordinator.md`, whole |
| this week's findings, and the seed case | `Reports/plant-reseeding-2026-09-03.md`, whole |
| why the genome is shaped as it is | `Reports/plant-genome-design.md` |
| why the last three architectural levers were invisible | `Reports/plant-appearance-design.md` |
| why plant changes cost so much | `Reports/why-changes-cost-so-much-2026-08-27.md` |
| "has this been tried?" | **grep `dead-ends.md` for the mechanism, never the area.** Measured: `thicken` returns ~2,460 tokens, `rot_remains` zero — a real answer, cheaply. Grepping an area costs 12k–31k, more than the file |
| "is this broken?" | `Reports/open-bugs-handoff.md` — read its generated index table first, then only the sections it names |
| "does an instrument exist?" | `Reports/instruments.md`. Twenty-seven do, and their names do not say what they answer |
| what a plant should look like when it is right | `wiki/plants.md` — the written form of the bar |

`README.md`'s **By topic** table maps subsystem to section with line numbers,
and tags each `engine` / `outdoor` / `lab`. You can skip every `outdoor` row.
