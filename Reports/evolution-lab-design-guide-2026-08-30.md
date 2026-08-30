# The evolution lab: a design guide

**Status: design guide. Not a plan, not a schedule, and not a decision.**
Downstream of `evolution-lab-feasibility-2026-08-30.md`, which asked whether a
second game could live on this engine and answered yes. This asks *how you
would build it*, and its job is to stop the same questions being re-derived.

**Three sources of authority, kept separate throughout.** Where this says
**measured**, a number in the feasibility report or another named report says
so. Where it says **policy**, `CLAUDE.md` or `design-philosophy.md` or a
recorded owner decision already settles it. Where it says **call**, it is a
judgement made here and open to being overturned — most of §4 and §6 are calls.
Where it says **open**, it needs the owner and §8 lists them together.

---

## 0. The reframe, stated first

**This game already exists as a decision, and nobody noticed.** Owner decision
E8, `creature-evolution-plan.md`, 2026-08-23:

> *"Evolution is a dev tool as well as a mechanic: we can use it to create new
> creatures that get saved and added to the game."*

The lab concept **is E8 with a player in it**. Everything E8 needs — run a
population, select, keep the winner — is what the lab game asks the player to
do, and the export path is already built: `examples/species_export.rs` writes
a genome and its traits out as `assets/species/<name>.ron` and reads them back
through the loader, round-trip verified.

That has two consequences worth taking seriously before any design work:

1. **The lab's core loop has a second customer.** Every hour the owner spends
   playing it produces content for the main game. A creature evolved in the
   lab is a `.ron` file the outdoor world can plant. That is unusually good
   leverage for a side project and it is an argument for building the lab
   *before* the main game needs more species, not after.
2. **The two games should share the species format, not the code.** `.ron`
   in, `.ron` out. Anything tighter couples two games that want to diverge.
   (**Call.**)

---

## 1. The game in one page

A future scientist in a sealed laboratory. A box of soil under grow lights,
a starting plant and a starting animal, and equipment. The player's job is to
keep a population alive and steer what it becomes.

**Two phases, alternating.** This is the owner's own framing and the
measurements support it exactly:

- **Tending** — real time, the player interacts. Plant, install, adjust,
  harvest, cull, move things. The world runs at 1x and everything is
  responsive.
- **Running** — interaction closes and the simulation fast-forwards. This is
  the experiment. The player watches generations turn over, and cannot touch
  anything until it ends.

**Why the split is free rather than expensive** (measured, feasibility §4c):
determinism is required same-build, and `main.rs` already runs a fixed
timestep with a catch-up loop. Raising `MAX_TICKS_PER_FRAME` runs the
*identical* tick sequence in the identical order — a fast-forwarded experiment
and a real-time one are the same simulation, not an approximation. The player
can be told the result is exact, and it will be.

**What the player is optimising** is deliberately left open here — §6 and §8.

---

## 2. What the measurements already decide

The feasibility report's numbers are not just a budget; most of them force a
design choice. This table is the guide's centre of gravity.

| Measured | Design consequence |
|---|---|
| An empty sealed box costs **0.001 ms/frame**; all cost is bought by life | **Unused lab space is free.** Expansion costs nothing until it is planted. The player can be given a large facility from the start without paying for it |
| Under a **moving sun**, the field solves every tile in the world every frame; under a **held light** it does not (§2, §3b) | **The lab has a ceiling, not a sky.** This is simultaneously the fiction, the single largest performance decision, and free — it is not an optimisation to build, it is a thing not to have |
| Cost follows **living biomass**, not world size — a 2048-wide bed measured cheaper than a 512-wide one at fixed founders | **Population is the performance budget, and it is diegetic.** "How many growth beds is your lab running" *is* the frame-time dial. Do not hide it in a settings menu; it is a resource-management quantity already |
| Soil depth: 40 → 240 rows costs **1.9x** for a **byte-identical stand**, because herb's roots never reach past 40 | **Depth is a purchase that only pays once you own something that reaches it.** A genuinely honest upgrade: it costs real frame time and returns nothing until a deep-rooted species exists. Most upgrade trees have to fake this |
| A grow light held at full amplitude: **1,037 seeds against 435** in the same frames, for 12% more cost | **The light schedule is a real strategic lever with a measured payoff.** 24-hour light is ~2.4x the generations per minute against a day/night cycle. Whether that has a downside is **open** — today it does not, and a lever with no cost is not a decision |
| The air simulation runs permanently but has **nothing driving it** in a sealed box: 11–20% of frame, stand comes out *slightly larger* without it | **Equipment switches on a simulation that is otherwise idle.** A fan, a heater, a humidifier, a fire are not set dressing — they are what makes pressure/velocity/advection do work. This is the strongest mechanical argument for the equipment layer |
| The structural scheduler is **16% of the shipped frame** and ~0 in a bed with no rock (§3c) | **Diggable, collapsible rock is a 16% purchase.** If ant tunnels should cave in, that is what it costs, and it should be chosen deliberately rather than inherited. Soil that holds its shape is free |
| Empty sky is **27.4 ns/px** against stone's 6.7 | **Whatever fills the air above the soil must not draw as sky.** A lab interior is cheap to draw; a gradient with a star hash is not |
| `herb` reaches generation 5 in 45,000 frames; `tree` reaches generation 1 in 200,000 | **The starting plant must have a herb's life cycle.** Trees are a late-game unlock and are not a substrate evolution can act on today |
| The ant reaches **generation 0** — richest bank 219 against a birth cost of 1,040 | §3, Gate 0. Nothing else matters first |

**One number to design against**: in a grow-lit box, roughly **1–2 µs per
living plant cell per tick**, falling as the stand grows (2.25 µs/cell at 497
cells, 0.91 at 5,684). A 5,000-cell stand is ~3.5 ms/tick. At a 20 Hz display
during Running, that is ~14 simulated ticks per displayed frame — call it
**14x real time**, or a herb generation every 9 seconds. Do not trust this
past a factor of two; it is a sizing rule, not a model.

---

## 3. Build order, as gates

Each gate is a thing that must be **true and measured**, not a task that must
be done. A gate that cannot be measured is not a gate.

### Gate 0 — an ant reaches generation 2

**Nothing downstream matters.** Every gene in `brain.rs` is inert without
heredity, and the shipped ant has never produced a child. The gate is
`creature_probe` reporting non-zero `births` and `deepest generation >= 2` in
a bed a player would recognise.

**It is not a tuning problem** (`ant.ron`, `creature-birth-grant-2026-08-30.md`):
a birth costs the grant plus a 960-unit body stamp, the stamp is invariant to
every knob in the file, and an ant's bank is capped by `hunger_fraction` at
roughly half `start_energy` plus one mouthful. The two routes named at the
source are **a child born at one cell that grows into its plan**, or **a gut
specialised enough to draw a full leaf's 480**. (**Call**: the first is
cheaper and reusable — a one-cell newborn is also what makes heritable body
size in E10 cost nothing extra.)

### Gate 1 — one hand-built box that runs plants and creatures together

**There is no such scene today.** `filmstrip scene=colony` is the closest and
it generates a `wetland` world, grows it 2,400 frames, then founds a colony —
so its plants come from worldgen, which the lab deletes. `PlantScene` builds
beds with no creatures; `creature_probe` builds floors with no plants.

This is small and it is the bed everything else is measured in. It should be
the lab's real geometry from the start — walls, a ceiling, a light, soil of a
settable depth — because a bed that is not the game's bed produces results
that do not transfer. `labbox_cost` already builds most of it.

### Gate 2 — selection has teeth in *that* box

`selection_arena`'s whole finding is that **a null here is a statement about
the world, not the genome**: a bed that does not punish a plant known to be
worse invalidates every evolution result measured in it. A hand-built lab bed
has never been run through it.

Run the `arm=` ladder (`same` / `lethal` / `nobranch` / `norootbranch` /
`early`) in the lab bed and read where discrimination stops. If the lab bed
discriminates less than a generated world does — plausible, since it is
uniform and a generated world is not — **that is a finding about lab design**,
and the answer is heterogeneity: `PlantScene::Relief::Varied` already makes
moisture and depth vary independently across a bed for exactly this reason.

### Gate 3 — the two-phase loop, end to end

Tending at 1x, Running at N x, a stated end condition, and a result the player
can read. **Measure the tick rate at the population the lab actually runs**,
not at a founder cohort — cost follows biomass and a mature box is the
expensive one.

**The one thing to get right here**: the display rate during Running is a
design choice, not a constant. At 60 Hz display the render eats a fifth of
the budget; at 20 Hz it eats 7% and the tick multiplier roughly triples.
(**Call**: let the player set it, and show simulated-time-per-real-second on
screen. It is the game's core dial.)

### Gate 4 — verbs (§4)

### Gate 5 — a score for "interesting" (§5)

Gates 4 and 5 are last deliberately: they are the game, and they are the two
that cannot be specified until 0–3 exist to test them against.

---

## 4. What the player actually does

`CLAUDE.md`'s second law: **there must be a verb, and it must deliver
something.** *"If a system can only be changed by the world changing around
it, the player is a spectator of it."* The gnome earned that rule the hard
way — destruction worked for a year and felt inert because nothing could be
*hit*.

So every candidate verb below is listed with **what it produces**. Anything
that produces nothing visible is not on the list. All **calls**.

| Verb | Produces |
|---|---|
| **Plant** a seed / **release** founders | The obvious one. Should show the individual's traits, or planting is a slot machine |
| **Install a light** and set its schedule | Measured 2.4x reproduction at full amplitude — the largest single lever in the game, and it is already real |
| **Install a fan / heater / humidifier** | Switches on the air simulation, which today runs idle. Visible as drift, as heat spreading, as humidity reaching a bed that could not otherwise get it |
| **Deepen a bed** | Costs 1.9x frame time, returns nothing until roots reach it. An upgrade that is honestly priced |
| **Cull** an individual or a lineage | Directed selection — the player *is* the selection pressure. This is the verb the premise most needs and the one with no engine support today |
| **Partition / connect** two beds | Isolation is what makes divergence possible (`plant-evolution-design.md`: asexual isolation is where clusters come from). A door between two beds is a genuine evolutionary operator |
| **Export** an individual to the species library | E8's own verb, already built (`species_export`). This is the game's *keep* |
| **Feed / withhold** | The economy exists; withholding is how a bottleneck is applied |

**The two verbs with no engine support are `cull` and `partition`**, and they
are the two the premise most depends on. Everything else is exposing something
that already runs. (**Call**: that ratio is the strongest evidence the concept
is cheap to reach.)

**A note on the gnome.** The concept deletes him, and with him the whole
interaction vocabulary — a belt, three tools, a left button that swings
whichever one is held (`wiki/the-gnome.md`). The lab needs a *different*
interaction model: placement and settings rather than a body with tools. That
is a real piece of UI work and it is not free just because the physics is.

---

## 5. Scoring "interesting behaviours and relationships"

The hardest problem in the brief, and the one place where the repo is further
ahead than it looks.

**The trap** is scoring an outcome the designer already imagined — "reward
tunnelling, reward farming" — which makes evolution a lookup table for a
checklist. `design-philosophy.md` and the open-endedness assessment both point
away from it.

**What already exists**, `creature-evolution-plan.md` §7, is a definition of
success that does *not* name any specific behaviour:

> behaviour-space coverage should be **smaller** than random sampling's 26/81
> (selection concentrates) while the occupied cells are **separated rather
> than adjacent**, and the three-way reciprocal transplant should be
> asymmetric in all three directions. *Three clusters that transplant
> symmetrically are three settings of one animal.*

That is a novelty score with no content in it. It says: *your population found
distinct strategies that are genuinely specialised to their conditions.*
`creature_space` already measures the coverage half — it answers *"how many
distinguishable ways of being an ant does this system admit"* — and a
reciprocal transplant is running an evolved lineage in a bed it did not evolve
in, which the lab's partitions give you for free.

**(Call.)** So the player's score is not "did you make an ant that digs". It
is:

1. **Separation** — how many distinct strategies your population holds at once.
2. **Specialisation** — how much worse each does in the others' conditions
   (transplant asymmetry).
3. **Persistence** — that they survive their own success, i.e. the clusters
   hold across generations rather than one absorbing the rest.

All three are measurable today or nearly so, none of them names a behaviour,
and all three get *harder* as the lab gets more uniform — which makes
partitioning and equipment into scoring moves rather than decoration.

**What is open** is whether that is *legible*. A number that says "your
population occupies 4 separated behaviour cells" is a good metric and possibly
a terrible readout. §8.

---

## 6. Failure, and the "start all over" question

The brief says: *keep your colonies alive or you have to start all over.*

**This runs into the first law** (`CLAUDE.md`, owner-stated, above
correctness of any individual mechanic): **an outcome is a distribution, not a
binary.** *"A plant that is either thriving or gone, a pool that is either
full or empty, a fire that is either out or total, has the same defect the
rubble did."* A total wipe on a failed experiment is exactly that binary.

The plant line already solved the same problem, and its solution is the
template: a tree that cannot pay its maintenance is **marked senescent and
carried out by `rot_remains` at the species half-life**, so the death is
graded rather than a disappearance — the owner's own ruling.

**(Call.)** The graded version of "start over":

- A failed *experiment* loses its population, not the lab. The beds, the
  equipment and the species library persist.
- A failed *lineage* leaves something behind — a seed bank, corpses that feed
  the next thing, an exported genome banked before it died. The engine already
  has an immortal seed bank and a corpse economy; both are assets here.
- **Total loss should be reachable and rare**, not the default punishment. The
  distribution wants a long middle: a run that limps, a run that survives on
  one lineage, a run that thrives.

The cost of getting this wrong is specific and known: the ethos section says a
mechanic that is right on paper and dull in the hand has failed, and *"the
test passes" is not a defence.* A wipe-on-failure loop is testable and grim.

---

## 7. Traps this codebase has already paid for, that apply here directly

Not general advice — each of these has a recorded incident and each is live
for this concept.

- **A slowed subsystem is not the same subsystem later.** `clock.rs`: the same
  number of organism ticks at 4x `growth_slowdown` produced a median **0.61x**
  final cells across 8 seeds. So the Running phase must run *more ticks*, never
  *faster subsystems*. Raising the tick multiplier is exact; retuning cadences
  is a behaviour change and must be measured as one.
- **A term in a weighted sum is not an independent knob.** Reshaping
  `phototropism_dir` into a real 2D gradient — the correct repair — sent
  reproduction to **zero**, because `light_weight` had been calibrated against
  a codomain that could only point up. Any lab equipment that changes what a
  plant can express reallocates the whole economy. Budget re-deriving the
  constants as part of the work, or the change is not scoped.
- **A coarse-field read is block-nearest.** Four sensors one cell apart land in
  the same field block ~7 times in 8. Any equipment whose effect is read as a
  *gradient* between two nearby points will resolve to a constant direction.
  This has bitten three separate lines and been caught by a reviewer a fourth
  time.
- **A channel needs a writer and a reader.** Three times this project has
  shipped a per-cell channel with one end missing — light with no writer,
  canopy density with an always-zero reader, pressure with no consumer. Every
  new lab channel (nutrient, CO₂, a contaminant) must have both named out loud
  before anything is built on it.
- **The 4,095-organism ceiling.** Herb already runs at **1,812–2,503 live
  organisms**, 44–61% of the cap, with births still outrunning deaths at the
  end of a 45,000-frame run. A lab designed around dense fast-breeding
  populations will hit this. `push_organism`'s range check is a `debug_assert`
  the app never compiles, so it needs a release-mode guard first.
- **Judge it by playing it.** Three separate models that looked correct in
  tests were overturned by the owner's playtest. `scripts/review.py` is the
  channel and it is meant to be used constantly, not saved for milestones.

---

## 8. Open questions for the owner

Real ones — each changes what gets built.

1. **What is the player optimising?** §5 proposes separation + specialisation
   + persistence, which rewards *diversity*. The alternative is a directed
   goal ("evolve something that survives condition X"), which is more legible
   and much less open-ended. These want different games.
2. **Is the grow light's 2.4x free?** Today constant full light strictly beats
   a cycle. A lever with no downside is not a decision. Should darkness buy
   something — rest, a night-active species, lower heat?
3. **Do tunnels collapse?** That is the 16% structural purchase (§2), and it
   decides whether soil is a material or a substrate.
4. **How graded is failure?** §6 proposes the lab persists and the population
   does not. The brief's wording suggests something harsher.
5. **Is the score legible?** A behaviour-space coverage number is a good metric
   and possibly an unreadable readout. What does the player *see*?
6. **One lab or many?** Partitioned beds are the cheapest source of divergence
   and the strongest scoring move — but they are also a UI and a camera
   problem.
7. **Does the lab feed the main game?** §0 says the export path exists. Whether
   lab-evolved species should actually appear in the outdoor world is a
   decision with content implications, not just a plumbing one.

---

## 9. What this guide does not decide

- **Any schedule.** No task list, no sequencing beyond the gates, no estimate.
  The gates are ordered by dependency, not by effort.
- **Whether it is fun.** Nothing here is a judgement about play.
- **The art.** A lab reads nothing like a landscape and the renderer has never
  drawn an interior. §2 says only that it must not draw as sky.
- **Anything about creatures past Gate 0.** Every creature figure in this repo
  is measured on a population that does not breed;
  `creature-evolution-plan.md` §2.6 flags it in as many words — *"creature
  work was measured free at 55 ants and a breeding population is not 55."*
  The lab's real creature cost is unmeasured and unmeasurable until an ant
  buds.
