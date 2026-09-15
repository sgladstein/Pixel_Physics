# The held world — living design document

**Status: living. Nothing in it is built.** This is the design of record for
the held world (`--bin druid`) *beyond* the founding concept, which is
[`held-world-game-concept-2026-09-13.md`](held-world-game-concept-2026-09-13.md)
and stays the document for the premise, the economy's first draft, the
engineering answer on regional ticking, and the 2026-09-13 rulings. **This one
is not dated in its filename on purpose** — it is meant to be edited in place
as the design moves, the way `design-philosophy.md` and `destruction-plan.md`
are, rather than snapshotted.

**How to use it.** §1 is what may not be re-litigated. §2 is the live design.
§3 is what play feels like. §4 is prior art with its lessons attached. §5 is
what it costs. §6 is what is still open — read that before proposing anything.
§7 is the measurement record. §8 is the append-only decision log; **add to it
rather than rewriting the sections above when a ruling lands**, so the
document keeps its history.

**Where it came from.** A design conversation with the owner on 2026-09-15,
opened by reading the enemy plans and finding that §10a of the concept report
was the only design that existed for them. Everything here either survived
that conversation or was ruled into it.

---

## 1. Fixed points

### 1a. The three mechanics, owner-stated 2026-09-15

> *"The only thing set in stone right now are growing plants founding and
> growing colonies controlling time."*

Everything else in this document is arrangeable around those three. In
particular:

**The character is not fixed.** *"You mentioned ruling out building walls and
things because the character is a druid. That's really just my first thought
of a type of character. It is not set in stone. The vibe can change."* So the
druid, the fiction, and the register of the player's works (grown vs built)
are all open. `--bin druid` is a binary name, not a design commitment.

**Evolution is wanted but not required.** *"Ideally plant and creature
evolution will play a role, but that does not have to be part of this if it
doesn't make sense."* §2g argues it does make sense and is nearly free.

### 1b. Standing rulings that predate this document

From [`lanes/druid-program-coordinator.md`](lanes/druid-program-coordinator.md),
still live:

- **Ant-laid pheromone mechanics are the evolution lab's**, not this
  program's. The gnome-laid trail is this one's.
- **The world starts bare and she plants it.** No seeds at generation.
- **One hue is the default look.**
- **No readout of how much life energy an animal holds** — you learn it from
  how many particles come.
- **It is a separate game**, not a mode of the gnome game.
- **The nest mechanic belongs to the evolution lab** and was handed over in
  full; this program kept only the look.

From the concept report's §10, still live: **the axe stays, narrowed** (leach
is interest, felling is principal); **you drink from animals only**, so a wood
with no colony pays nothing; **the interface starts simple and must not be
shut**; **two dials over one pool** rather than an income that sizes the
circle for you.

---

## 2. The design

### 2a. The enemy is the stillness, given a body

**Ruled 2026-09-15: un-quickening is not the primary enemy.** The owner's
words: *"the un-quickening will not be fun, if it is the primary enemy. It
could be a specific type of enemy or enemy weapon, but I don't think it should
be primary."*

That is the right call on the concept's own terms. Freezing the player's
garden is a *withholding* — the feedback is that something stopped happening —
and `CLAUDE.md`'s second law wants a verb that delivers something and leaves a
mark. §10a's own risk section named it: *"an enemy that attacks your economy
is often the least fun kind."*

**What replaces it keeps §10a's best property and throws away its verb.** The
enemy is the held-ness itself, advancing, as a **substance**: a creeping tide
that flows downhill, pools in valleys and takes ground.

**The lock that makes it affordable and thematically exact at once: it is held
too, and the player is the one who un-holds it.** Outside the player's
influence it is frozen like everything else. It moves only where time has been
made. So:

- It costs nothing where the player is not — which is the engine's own
  premise, not a special case carved for it.
- Every circle lit is a stretch of front handed to it.
- Growing the live world is literally what lets the enemy move.
- §10a's best idea survives intact: **the dial that funds the garden is the
  dial that calls them.**

**This is the one idea in the document with a hard cost attached**, and it
cuts the other way too. See §5b: Creeper World simulates its creeper
everywhere always, and this engine's whole performance model is that settled
things sleep. A world-spanning live fluid is the most expensive thing that
could be added here. The held-tide framing is not a nicety; it is the reason
the idea is admissible at all.

### 2b. Two ranges — the quickening and the waking

**Owner proposal, 2026-09-15**, and it is better than the single radius the
conversation started with:

> *"There might be two ranges. One is where you bring things to normal time
> and you can grow and have creatures but then in a much larger radius it
> awakens the enemy who moves a lot slower."*

| | inner — the **quickening** | outer — the **waking** |
|---|---|---|
| what runs | a whole world: plants, water, fire, weather, creatures | **the enemy, and nothing else** |
| cost | goes as r² — this is the expensive one | the tide's *front*, which is a line, not a disc |
| feel | your garden, at your chosen rate | dread: the only other thing moving, far off, slow |

**The cost argument is the design argument.** The outer ring needs no plants,
no weather, no fire, because nothing out there is the player's. Its price is
the number of tide cells on the advancing front rather than the area of the
ring — so the dread radius can be **several screens** for nearly nothing, and
seeing it coming from a long way off is a thing the game can afford.

**And both scale with the speed dial**, which is what preserves §10a's
property. A fast circle does not only grow the garden faster; it brings the
far thing on faster.

**Open addition, not yet ruled:** *the world remembers being woken.* When the
player leaves, the outer region should settle back to held over time rather
than snapping to it, so that walking away is a real cost rather than a
universal escape. A decaying per-region timer. Filed in §6.

### 2c. Three enemies, one identity

The un-quickening survives the demotion as a weapon, which yields a tiered
antagonist with a single visual language:

1. **The tide** — takes ground, never bites. The **player's** problem, answered
   with terrain, fire and water. The standing pressure.
2. **What walks out of it** — bodies, and these are the **colony's** problem.
   This is the fight the engine already implements (§2d).
3. **The still-bearer** — rare, late, and it carries the un-quickening as its
   weapon: it can put live ground back to sleep. A thing with a body that can
   be killed, rather than a mechanic to be endured. **This is where the
   owner's "a specific type of enemy or enemy weapon" lands.**

Three consequences that fall out of the engine rather than being designed in:

- **The good ground is the dangerous ground.** A tide flows downhill and pools
  in valleys; valleys are where the water and the soil already are. No
  balancing pass required.
- **The enemy's dead feed the colony.** Corpses burn and ants eat corpses, so
  a fight the colony wins is a meal. This loop runs today, and §7 found it is
  currently *stronger* than intended — see the measurement.
- **The tide erases scent trails.** Free, and nasty.

### 2d. How the creatures fight — you site them, you do not command them

**The ignition already ships and needs no new wiring.** A non-kin body in
reach is not excluded by `adjacent_food`'s kin filter, so an ant *eats* it;
the swallow calls `cry_alarm`; and `ant.ron`'s authored `(Alarm, Attack, 2.0)`
turns the bite into a brawl. Predation is the initiator the combat layer was
said to lack. This is [`why-colonies-do-not-fight-2026-09-14.md`](why-colonies-do-not-fight-2026-09-14.md)'s
finding, and it corrects §10a, which blamed the blind ant and the
retaliation-only `Alarm` wire — *both true of the code and neither the binding
constraint.* The binding constraint was `scent_spread: 0`, and it shipped at
**2.0** on 2026-09-14 (PR #423): cross-colony killings went from **0 of 12
seeds to 11 of 12**.

**Verified against the tree on 2026-09-15**, because the coordinator note's
own rule is to check the code does what the comment says:

- `ThreatNear` / `ThreatBearing` appear in species files **only in comments** —
  zero authored weight anywhere. A threat wire would be a *second* initiator,
  not the missing one.
- `sight_range` is **0 on every ant variant**.
- `TRAIT_REACH_DEFAULT = TRAIT_REACH_MAX = 8.0`, so the plate spans 0.1–9.0
  and §10a's first draft quoted a pre-fix armour ceiling.

**So the design does not build a "defend the nest" behaviour.** It makes the
fight happen where the colony already is. Soldiers are a **chitin body and a
longer chain** — a species file, no code, since `ant_long.ron` and
`longant.ron` exist and `chitin_pale` / `chitin_mid` are an armour ladder.

**The one thing genuinely on the critical path** is the scent-tool bug: the
shipped ant cannot read the trail the player lays (the laden gate parks the
unit where the squash curve's slope is one in a thousand). If "tell them where
to go" is the signature verb, that is not backlog.

### 2e. Hearths, and what the player is actually growing

**The open question this answers.** A *carried* circle — the concept's
model — means the world has no memory of the player's work and there is no
home. "How do the creatures defend our home" presupposes a home a carried
circle does not have.

**Proposed: she carries a small fast circle, and can spend to make a place
self-sustaining.** A hearth keeps running without her, slowly.

What that buys:

- **The game is about reclamation.** The score is how much of a dead world is
  permanently alive again, read off a map — a better fantasy than the radius
  of one circle.
- **The enemy gets a target that is not the player**, and loss is legible: a
  hearth goes out, the disc greys, the ants stop mid-stride, the trees stand
  held. Recoverable, so a setback rather than a game over.
- **Siting a hearth is a place-decision**, which is §2f's rule applied to the
  biggest choice in the game.
- **It is the tower** (§4a), which resolves the building question.

**This is the assumption doing the most work in this document**, and it is not
ruled. See §6.

### 2f. The rule against the resource-management trap

**Owner concern, 2026-09-15:** *"we need to be wary of this turning into a
mostly complex resource management game (that is obviously part of the game
but we need to think well about how to make it fun or not fun)."*

The concern is well founded against the concept as written, which is *already*
mostly an economy: two dials over one pool, `cost = speed x radius`, leach as
interest, felling as principal, founders priced in power. That is a lot of
arithmetic and very few hands.

> **The rule: if a decision can be a number in a HUD, make it a place on the
> ground instead.**

This is Creeper World's actual lesson (§4a) and it is also what this engine is
for. Worked through:

| instead of | make it |
|---|---|
| a radius slider | *where you stand* |
| "allocate 30% to soldiers" | dig the trench here, put the nest behind it |
| an enemy with 400 hit points | a thing coming up that valley, with an overhang above it |

The concept's §11 already lists *"the economy is fiddly rather than legible"*
as one of four things that could kill the game. This rule is the antidote, and
§2h is the same rule applied to the cost model.

### 2g. Evolution is what you buy with time

The arms race is **already built**: `TRAIT_ARMOUR` and `TRAIT_DIG_FORCE` are
heritable, priced, and paired in `ARMS_RACE_SLOTS`. A colony that fights for
many generations breeds tougher ants. Generations cost time; time is the
currency. So **evolution is a thing the player buys, and its price is the
resource the whole game is denominated in.**

What that buys the design, past flavour:

- **Old hearths are worth more than new ones.** A hearth running quietly for
  an hour holds a colony several generations deeper than one lit five minutes
  ago. That gives slow, cheap, dormant hearths a strategic payoff and makes
  "leave it running" a plan rather than an overhead.
- **The answer to the enemy is grown, not unlocked** — and can be lost.
- **Plants too**: species in the tide's path get selected, and the lab's
  machinery for all of it exists.

Note the precondition: a colony must survive long enough to *have*
generations, which is what §7 measures.

### 2h. Make the frame budget the economy

**Owner concern, 2026-09-15:** *"if we keep awakening areas and keep them
awake long-term as we move on to a new part of the game, the processor won't
be able to handle that, especially when there are areas we want to speed up to
high speed."*

Correct, and the proposed remedy — *"the hearth keeps areas alive while you're
gone but they run at a quarter speed or a tenth of the speed so they draw way
less processor power"* — is **already the design of record**, in
[`regional-time-scope-2026-09-13.md`](regional-time-scope-2026-09-13.md) §3d:
a fine clock with a **per-region stride**.

- Every region gets a stride. Stride 1 runs every pass; stride 8 runs one pass
  in eight.
- **Chunks overlapping no live region are forced asleep for that pass**, so
  the CA sweep and the field do no work there.
- Cost shape, quoted: roughly `8 x (fast region) + 1 x (everything else)`.
- **A slow region is bit-identical to today's world, merely slower** — which
  matters for §2g, because a colony breeding in a dormant hearth must breed
  *correctly*, not approximately.

Two constraints to carry:

- **Rates must be divisors of the maximum** (1, 2, 4, 8), so *"a tenth"* is
  not on the ladder as it stands, and hearths slower than normal world speed
  need strides *above* the maximum. That extension is arithmetic on the same
  counter and looks cheap, but it is unbuilt and unmeasured.
- That report's own recommendation is **ship nothing per-circle yet**; do the
  two pieces of groundwork every option needs first.

**And the prize beyond the fix.** The concept's §10 already prefers, of three
costing models, *"charge what is actually awake inside — it is what the engine
truly costs."* Run that all the way and **the frame budget and the game's
currency become one number**: a fast small circle and a slow wide one cost the
same, the price of every hearth kept lit is visible, and there is no way for
the game's arithmetic and the frame rate to disagree. The processor stops
being a constraint to work around and becomes the thing the game is about.

---

## 3. What ten minutes of play looks like

Nothing in the concept report describes this, and it is the real test of §2f.

> You come over a ridge into a dead valley — grey, still, a photograph. You
> pick a spot with water and soil and light your circle: colour and motion
> bloom in a disc around you, and the sound comes back. You plant. Then you
> *wait*, except waiting is a verb here — you push the dial and watch a season
> pass in twenty seconds.
>
> Which is also what starts the trouble. Over the ridge the tide begins to
> move, because you made time for it. It comes down the draw, slow, pooling.
> Your choices are all things your hands already do: cut a trench and let it
> pool where it cannot reach you, burn the dry grass in its path, drop an
> overhang on it, or pull your circle in and let the valley freeze again —
> which costs you the season you just bought.
>
> Your colony founds and ants stream out. Something walks out of the tide, an
> ant bites it, the alarm goes up, and forty of them swarm it. You did not
> order that; you sited it.
>
> You leach. The meter fills. You spend it on a hearth, so this valley keeps
> living when you leave. Then you walk on, and behind you there is one green
> valley in a grey world.

**Count the numbers in that.** One meter. Everything else is a place.

### 3a. There is no night — there is only the tempo you dare

The held premise hands the game a rhythm nothing else can have. Kingdom
imposes night on a metronome; Factorio triggers attacks off pollution. Here
the player *is* the clock: speed up and the garden grows, the tide advances,
the things in the ground wake; slow down and everything quiets, income
included.

**The failure mode to design against is the player who never speeds up.** The
economy already answers it: growth is the only source of time, so standing
still is a slow death. The player chooses *when* to take the risk, never
whether.

**And this is the premise solving Creeper World's worst flaw for free** — see
§4a. There is no mop-up, because a decided outcome can be fast-forwarded.

---

## 4. Prior art

### 4a. Creeper World — mine it, do not avoid it

**Owner correction, 2026-09-15:** *"you mention trying to stay away from tower
defense games but I actually think we could learn a lot from them and there
could be tower defense aspects that are fun for this as we are trying to
defend an area against an encroaching enemy."* Taken; the first draft of this
argument over-steered. Creeper World is itself a tower defence at heart.

**Take:**

- **The enemy as a substance rather than units.** Nearly free in a falling-sand
  engine, and it makes the hammer, the pick, fire and water into weapons with
  no combat code.
- **Terrain as the primary weapon** — the thing players actually praise:
  redirect the creep with embankments, take height.
- **A continuous front** rather than waves.
- **Simplify the model, do not simulate it.** CW's creeper is values on a grid
  with wave behaviour, not a real fluid. Our tide probably wants to be a
  **field** — we already have fields and a debug overlay path — rather than
  particles.

**Refuse, and these are the measured complaints:**

- **The mop-up.** Once you are winning you still execute the position for
  twenty minutes. §3a says the premise removes this by construction.
- **Micromanagement without depth.**
- **Attrition as the difficulty** — a resource-management game wearing a
  fight's clothes, which is exactly §2f's trap.
- **Oversized maps trading intensity for tedium**, worth remembering against a
  2560x960 world.

**What tower defence does well that this design wants:** placement as the
whole decision; reading a map for chokepoints before anything happens; and
**anticipation then payoff** — prepare, then watch it work. That third one we
have the best version of, because **speeding up time is the "start wave"
button, and it is the same button that grows the garden.**

**What tower defence does badly, and why this would not inherit it:** solved
maps and a static optimum die against destructible terrain and a fluid enemy
(no fixed lanes); the optimum decays because the defences are **alive** and
have to be re-grown; and the player has hands during the fight.

**The resolution of the building question: the hearth is the tower**, and the
player's works split into two registers that stay separate —

| register | what | examples |
|---|---|---|
| **built** | few, permanent, expensive, important | hearths and what upgrades them |
| **grown** | many, cheap, expendable, alive | plants, thickets, ants, soldiers |

A defence that is alive is what keeps this off the tower-defence rails: a wall
that is a thicket has a middle state, which is the first law for free.

### 4b. The other three worth stealing from

- **Factorio's pollution cloud** — the strongest validation of §10a's surviving
  idea. The factory's emissions summon the biters, so the dial that grows you
  is the dial that aggros them, and it is one of the most-loved loops in the
  genre.
- **Kingdom (Two Crowns)** — the monarch has *no* means of self-defence and the
  fighting is the subjects'; the creator's stated aim was an experience centred
  on defending structures. Evidence that "you do not fight" can be great — and
  a warning, because it pays for it with constant immediate agency and a hard
  day/night rhythm.
- **Frostpunk** — the closest existing thing to the bubble: a circle of life
  around a generator, the city built in rings, upgrades extending it by exactly
  one ring. Note what makes it dramatic: the circle is **permanent and
  expandable**, and always *barely* enough.

### 4c. On building, the owner already ruled once

[`building-rethink.md`](building-rethink.md) carries the steer verbatim: *"I
want to be able to build and destroy environments, but right now the building
is very hard. I don't want my constructions to just immediately fall down or to
have to work at all to make sure they are structurally stable, but I do want it
to break realistically. Also right now using a paint brush type tool to build
is not satisfying."*

And [`design-props-and-shoring.md`](design-props-and-shoring.md) records that
the engine already pays 600 frames of deliberate collapse delay *so the player
can get supports in* — and **no support verb exists**. If building becomes a
pillar here, that is the designed-and-unbuilt piece to cash in.

---

## 5. What it costs

### 5a. Free or nearly

Terrain weapons (hammer, pick, fire, water all ship). The whole fight:
`Attack`, swarm damage banking on the victim, severing long bodies, the chitin
armour ladder, and the mouth-ignition of §2d. Soldiers as a caste — a species
file, no code. Held-ness itself. Plants, growth, the seed bank. A debug overlay
path for a new field.

### 5b. Genuinely new

- **The tide**: a field, a flow rule, and a takes-ground rule.
- **The gate that moves it only where time runs** — probably the same regional
  gate, and the cheap part.
- **Hearths are the real engineering item.** The regional tick was scoped for
  *a* region; several at once is unmeasured, and §2h's stride extension is
  unbuilt.
- **The still-bearer's verb** and a brain slot, on append-only enums.
- **The scent-tool fix**, which is on the critical path whatever gets built.

### 5c. The standing performance trap

`CLAUDE.md`: frame cost is a hard constraint, not a tiebreaker; a visual
improvement that costs the dirty-rect render skip or keeps chunks awake is not
automatically worth it. **A hearth is by definition a region that keeps chunks
awake**, so every hearth proposal must say what it costs, measured with
`examples/ascii.rs`'s worst-frame figure and the pinning test (mean x frames
≈ worst) before it is quoted.

---

## 6. Open questions

**Ranked by how much they change.**

1. **Carried circle, or plantable hearths?** (§2e.) The biggest one — it
   decides whether the game is a journey or a reclamation, and most of §2
   assumes hearths.
2. **Does the tide ever recede?** If it is monotone, the game is a fighting
   retreat and the best outcome is a slower loss. If ground can be reclaimed,
   the map fills in. This document assumes the second.
3. **If hearths run without the player, do they still pay?** Yes makes the
   game idle-accumulation and numerical again; no makes hearths pure cost and
   nobody lights one. Working instinct: they pay *slowly* and only while
   alive, so the map is the income and defending it is the game.
4. **Does the woken world settle back to held, or snap?** (§2b.) A decay stops
   "walk away" being a universal escape.
5. **Is the tide the only enemy, or one of several?**
6. **What is the character?** Not fixed (§1a). Note when picking: the engine's
   material palette — soil, wood, chitin, water, fire, rot — is warm and
   biological and will fight a cold science-fiction frame.
7. **Do grown walls appeal, or are they a detour?**

---

## 7. Measurement record

### 7a. One correction to the standing plan

`lanes/druid-program-coordinator.md` gates the enemy on a **`creature_arena`**
sweep. Reading both harnesses on 2026-09-15, that is the wrong instrument for
the gate it describes. `creature_arena` races two genomes and reports share of
animals — *does this bed punish a worse animal*. The gate's actual question,
from §10a, is *does an armoured predator still collapse the colony*, and that
is **`labstats`**, which is where the original figure came from and which is
the only one carrying `predators=`, `beetlearmour=` and `antbite=`.

### 7b. The gate's question has to change, because "inedible" no longer exists

That is the graded bite working as the owner ruled: *"nothing should be binary
edible or inedible."* `beetlearmour=` sets an allele on a reciprocal axis —
`0` is exactly what the species was authored with, `+1` is double plate, and at
the shipped `trait_reach` of 8 the ceiling is nine times plate. **The
configuration the original finding named cannot be built any more.** The useful
question is *where on the armour ladder the collapse starts*, which is a better
measurement and is the number that says how tough the tide's bodies may be.

### 7c. The population column is junk on this bed — read death causes

Probe, one seed, 24,000 frames, `RAYON_NUM_THREADS=4`:

| arm | ants alive | starved | killed by beetle | beetles alive |
|---|---|---|---|---|
| no predators | **0** | 51 | — | — |
| 6 beetles, authored plate | **3** | 40 | 8 | 2 |
| 6 beetles, double plate | **2** | 21 | **25** | 4 |

**What this licenses.** The instrument separates the arms on the right column:
killed-by-beetle moves **0 → 8 → 25**. That is the positive control passing,
so the sweep is worth its half hour. And the alive column is **0, 3, 2** —
the floor, not a signal. `instruments.md` already warns that at one-to-seventeen
survivors these beds cannot see a small effect; reading population the way the
original finding did would have measured noise.

**What it does not license.** It is **one seed**, and this file's own rules say
six is not a sweep. The hint that the colony died out *without* a predator and
survived *with* one is not a finding until the twelve-seed sweep says so.

**And the bed is not the game.** This is the lab bed, which starves its colony
to near-extinction in every arm. It is the right bed for a *relative* question
and is not evidence about a well-fed valley in the held world.

### 7d. If it holds, it is a design lever rather than a defect

An edible enemy is a **harvest**: dangerous, but a wave survived leaves the
colony fatter. An armoured one is a real threat at the cost of that subsidy.
**It is a dial per enemy type**, which is how §2c's three tiers can differ
without inventing a mechanic — early bodies edible and feeding you, late ones
plated and not. It also gives the still-bearer a clean identity: *the one that
is not food.*

---

## 8. Decision log

Append here; do not rewrite the sections above.

| date | ruling / finding | where it landed |
|---|---|---|
| 2026-09-13 | The enemy is third in order, after pheromone and shrink-and-walk | §1b |
| 2026-09-14 | `scent_spread` ships at 2.0 — colonies are strangers, the fight ignites | §2d |
| 2026-09-15 | **Un-quickening is not the primary enemy**; may be one enemy's weapon | §2a, §2c |
| 2026-09-15 | **Two ranges** — a quickening radius and a larger, slower waking radius | §2b |
| 2026-09-15 | **Tower defence is to be mined, not avoided** | §4a |
| 2026-09-15 | **Processor budget is a hard constraint**; hearths should run slow | §2h |
| 2026-09-15 | **The character is not fixed**; only the three mechanics are | §1a |
| 2026-09-15 | Coordinator note names the wrong instrument for the enemy gate | §7a |
| 2026-09-15 | "Inedible" is unbuildable since the graded bite; use an armour ladder | §7b |
| 2026-09-15 | On the lab bed, read death causes, not population | §7c |
