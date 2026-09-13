# The held world — a third game, and why this engine is already most of it

*Concept, 2026-09-13. Not a commitment and nothing is built. Written from a
brainstorm with the owner: a druid gnome in a barren land, seeds you plant, a
founder colony you design, and a bubble of fast time. What follows is that
idea with the engine's own measurements pushed back into it — several of which
change the design rather than merely costing it.*

**Revised 2026-09-13, same day, by the owner from play.** The first draft
said *a rich place is heavy* and priced it off a **stale** measurement (§2a).
The owner's own bed says otherwise — *"I can get 10x full of plants; it really
slows to 1x when I get 1000+ creatures"* — and the code agrees, for a reason
that is better than the one it replaces: **plant cost can be banded and animal
cost cannot.** The corrected law is *a **populous** place is heavy*, and it
does the work §4 was missing. The owner also rejected felling as the economy
(§5) and asked for creatures to be load-bearing rather than resident (§4a).
The inversion itself stands — the owner's word is *"I loved the time inversion
bubble idea"*.

**One line: the land is not slow, it is *held*. Nothing grows, breeds, rots or
weathers anywhere in the world. You carry the only time there is, you spend it
in circles on the ground, and the only way to get more is to leave something
alive behind you.**

---

## 0. The thesis, before the mechanics

This repo has already built this game twice and called it two other things.

- The **outdoor sandbox** is its stage: 8192x2560 of generated rock, weather,
  fire, liquids, a gnome who digs, climbs, swims and fells.
- The **evolution lab** is its interface: rosters, lineage overlays, a
  specimen shelf, plain-speech genomes, and *a hand in the box* — scent,
  alarm, fling, lamp, cull, feed, water, place-a-plant, found-a-colony.

And the owner has already stated the design, in round three of the lab, about
the lab:

> *"Your goals are not tweaking and optimizing evolution now. Give me the
> tools, data, access to the parameters that need to be tweaked and I do that
> testing myself in the game. **That is the game.** If I have access to food,
> water, can cull, can create plants, and creatures, I can figure it out."*

The lab is that sentence with **no price on any of it**. Everything is free,
instant and unlimited, which is correct for an instrument and is why it is not
yet a game. **The missing piece is a currency**, and the time bubble is a
better currency than the usual ones because of what it is made of: see §2.

So the proposal is not "build a third game". It is: **take the lab's hand, put
it on the gnome, drop him in the outdoor world, and charge him for it.**

---

## 1. The inversion: held, not slow

The brainstorm's framing was *world at 1x, bubble at 20x*. Invert it.

**Outside a bubble: physics on, life off.** Rock falls, water flows, sand
piles, the hammer works, fire that is already lit keeps burning. Nothing
*grows*, nothing *breeds*, nothing *rots*, nothing *ages*, the sky does not
move and the weather does not turn.

**Inside a bubble: everything, at a rate you buy.**

Four reasons this beats a slow world, and only the first is the one the
brainstorm was reaching for.

### 1a. It is the engine's own cheap configuration, not a compromise

The shipped world does not fit its frame budget standing still: `App::update`
**18.88 ms ±0.9%** plus ~2.4 ms of render against a **16.6 ms** budget, with
**nobody playing**. The two terms are the field at ~59% and
`plant::step_organisms` at ~27%.

Both of those are *exactly what a held world does not run*. A pinned sky
settles the field and `field::step` skips its whole five-pass solve; no
organism ticks means no `step_organisms`. What is left is the CA sweep over
chunks nothing has disturbed, which sleeps.

The lab measured the same thing from the other end and the ratio is the
headline number of this whole document: **a box costs 0.006 ms empty and 7.03
ms with eight plants in it — a factor of 1,170.** The cost of this engine is
not its size. It is the life in it.

So "the whole world held, one live circle" is not a performance concession
that also happens to be thematic. It is the only configuration in which a
world this size has budget left to spend on anything, and the game spends all
of it in one place the player chose.

### 1b. Barren becomes mechanically true instead of art-directed

A barren land painted brown is a texture. A barren land where the seed bank is
full, the soil has nutrient, the sky is stuck at one hour and *nothing is
running* is a fact about the simulation, and the player can verify it by
standing there and watching nothing happen.

### 1c. The picture is a photograph with one live circle in it

Not fast-vs-slow — **moving-vs-still**. Rain hangs in the air outside as beads
and falls inside. Dust from your hammer flies to the rim and parks. A leaf
blown out of a bubble stops dead mid-flight and is still there an hour later.
That is the engine's chunk-sleep boundary drawn honestly, and it is a
silhouette no other game has. §7 has the craft notes.

### 1d. It gives the rate axis somewhere to stand

`sim::clock`'s five knobs are all **slowdowns** — whole multiples of baseline,
capped at 30, and 1 is as fast as they go. There is no "faster" knob and there
should not be: `frame::step`'s own doc says why.

> *"The tick is the unit of simulated time and it is never scaled. A caller
> that wants the world to run faster calls this more times per displayed
> frame; it does not speed anything up inside."*

with the measurement under it: the same number of organism ticks at
`growth_slowdown: 4` produced a **median 0.61x** final cells across 8 seeds,
range **0.15x–1.34x**. Rate-scaling a subsystem changes what grows. **More
ticks is exact.**

**This is the single hardest architectural constraint on the whole idea and it
is easy to get wrong.** A quickening must be *more ticks over a region*, never
*faster subsystems in a region*. Which means the work is: restrict the tick to
a region.

---

## 2. Everything is time

One meter. It is called something in the fiction — *green*, *spring*, *the
year* — and mechanically it is **seconds of world**.

- A bubble **drains** it while it is up.
- **Living things pay it back**, in proportion to how much genuinely
  self-sustaining biomass is under the circle.
- The whole game is the search for **interest above 1.0**: an arrangement that
  earns more time than it costs to run.

What falls out of that, all of it good and none of it authored:

**You can go bankrupt.** A big fast bubble over dead ground is how you lose.

**Standing gardens are the endgame.** A patch that pays for itself gets a
*permanent* quickening — left running while you walk away, still going when
you come back, filling the map with living circles in a still grey country.
And the cap on how many you can hold is not a designer's number: it is the
frame budget. The game *cannot* let you have unlimited ones, which is the
rarest kind of honest limit.

**Greed self-punishes.** At 8x your ants age 8x and your trees senesce 8x.
Ants have a lifespan; a tree that cannot pay its maintenance is marked
senescent and carried out at the species half-life. Running hot burns a
generation to get a harvest, and the engine already models that without a line
of new code.

**Quality beats quantity, and the game teaches ecology without a tutorial.** A
big grass monoculture pays badly. Grass plus something that eats it plus
something that puts nutrient back pays well, because it *closes*. Soil
nutrient is already Michaelis-Menten with a 45-frame recovery matching exactly
one root's draw — a soil cell supports one root indefinitely and runs down
only where roots crowd. An ecology that recycles is measurably cheaper to run
than one that mines, in the code as it stands today.

### 2a. The best consequence: a **populous** place is heavy

**The first draft got this wrong and the correction is the most useful thing
in the document.** It read the 0.006-vs-7.03 ms split as *life is expensive*
and concluded that a mature wood must run slow. The owner refuted it from
play: **10x with a bed full of plants, down to 1x at 1000+ creatures.**

Both halves are in the tree, and the reason is structural rather than
incidental.

**The 7.03 ms was measured 2026-09-01 and has been overtaken by an entire
optimisation programme.** The full box is now **~1.95 ms** with
`step_organisms` at **0.198 ms — 10% of it**. And it was never the plants'
*code*: of the original 7.03, `active_sites` was 0.28 ms and the other 6.7 was
**the world reacting to them** — 25 awake chunks and 39.3 field solves per
frame. A plant is expensive because it keeps chunks awake, which is a cost
that yields to work, and has.

**Then the owner's own lever, which is the whole answer.**
`PLANT_SIZE_CADENCE` bands a plant by cell count and multiplies its tick
interval — `<50` 1x, `<200` 2x, `<800` 3x, `<3200` 4x, `3200+` 5x. Per-frame
cost is `cells / interval` and cost is flat per cell, so **multiplying the
interval divides the cost exactly.** Measured over ten seeds: the dial median
**2.25x → 5.95x**, per-seed ratio median **2.73x** (range 1.68–3.81), **10 of
10 improved and none worse** — while the box holds **+7% plant cells and 1.60x
the leaf** in a third as many plants. It defaults **off**
(`World::plant_size_cadence`), so this is shipped, measured, and switched off.

**And the sentence that decides the game's shape**, from that same section:

> *"Creatures are not banded — an ant's tick is **its brain** rather than an
> economy that can run slower."*

A tree's economy is a rate and a rate can be run on a slower cadence. **A
thought cannot.** Every animal has to decide every tick it is alive, so
creature cost is linear in population with no lever under it.

So the corrected law, and it is a better one:

- **A grown place is light.** A mature wood runs fast, and the more mature it
  is the cheaper per cell it gets. Growing a wood stays a fast, watchable,
  satisfying thing — which is what the moment-to-moment loop needs.
- **A populous place is heavy.** Your colony is what makes time expensive, and
  it is the one thing whose size you chose.
- **An empty place is free.** An empty lab runs at **1024x**, which is the
  measurement that licenses the held world at all.

**This is a better mechanic than the one it replaces on every axis.** The
weight now sits on the thing the player deliberately made and cares about
rather than on passive scenery; the cost curve is a decision instead of a
tax; and it is what makes §4a possible — creatures are structurally central
because **population is the price of time**, which is not a conceit but the
literal shape of the engine's cost.

And the fiction is exactly true, which is rare: **time is cheap over things
that merely grow, and expensive over things that think.** A druid can run a
century through a forest in an afternoon. He cannot do it to a mind.

### 2b. What the dial can honestly promise

The lab's speed dial is the closest existing thing and it is a **reality
check**: on a full box the whole display-rate ladder moves the achieved
multiplier **2.03x → 2.47x**; with soil water off as a control it reaches
**6.9x**. A tick costs 7.3 ms and a drawn frame 4.7 ms.

**Revised against §2a's correction**, and the shape is now a function of
*population* rather than of biomass. From the owner's own bed:

| bubble | what is in it | rate |
|---|---|---|
| any | held / empty | ~free (an empty lab runs at 1024x) |
| medium | a wood, no colony | **~10x** — owner's figure, from play |
| medium | a wood + a working colony | falls with head-count |
| medium | a wood + 1000 animals | **~1x** — owner's figure, from play |

So the dial is not a mystery the player has to feel out: **it is a
population counter.** That is legible, controllable, and it puts the throttle
on the exact quantity §4a wants the player thinking about.

A rate you have to husband is a decision; a rate you always have is a loading
screen. And note the lever still on the shelf — `PLANT_SIZE_CADENCE` is worth
a further **2.73x median** and is default-off, so the plant half has headroom
already measured and not yet spent.

---

## 3. Seeds: three scarcities, and not one of them is a count

The brainstorm named the real tension: **the player needs seeds to be rare;
the ecology needs each plant to make hundreds.** Counting seeds in an
inventory fights the simulation. Three scarcities that run *with* it instead.

### 3a. The ground is already full of seed, and that is the exploration loop

The seed bank exists and is measured: seed that lands somewhere it cannot
germinate **waits**, viability decaying gradually, and a sealed bed settles at
roughly **four hundred waiting seeds under about fifty standing plants** —
nine in the ground for every one you can see. Grass seed outlasts tree seed by
about double, which is why grass is what comes back first.

In a held world **nothing germinates anywhere**, so the whole bank is waiting.
The land is not empty. It is *full and stopped*.

Which means the core verb is not *plant a seed*. It is: **quicken ground and
find out what was in it.** A valley remembers what grew there. You walk a dead
country reading it, spend time on the patch you think is worth it, and watch
what comes up. That is archaeology, and it is a far better loop than
inventory-management — and it is a UI over data the engine already keeps.

Give the druid a cheap divining verb that shows the bank as ghosts before he
spends: informed, but not certain, because the bank tells you what *seed* is
there and not whether the *soil* will hold it.

### 3b. What you collect is a lineage, not an item

You never hold three acorns. You hold **the oak** — and specifically *an* oak:
a genome, an individual's, with its own history.

The engine is already built for this: species are `.ron` genomes, the specimen
shelf makes an individual's genetics outlive the box, there is a plain-speech
readout of a genome as sentences, a side-by-side view with the differences
marked, and a lineage overlay.

So a seed taken from a scoured gorge is *that gorge's drought line*, not a
generic species. Rarity lives in **kinds**; abundance lives in **copies**; the
ecology gets its hundreds of seeds per plant and the player still has a
collection worth crossing the map for. Two lines of one species crossed in a
bubble give a third — the breeding loop, free, from apparatus that exists.

### 3c. Ground is the per-unit cost

Nutrient is already zero-at-zero by construction: Michaelis-Menten, **exactly
zero at status 0 for any `Km`**, so no soil is structurally fatal rather than
merely expensive. Barren ground reads near zero, so *sowing is free and
nothing lives*.

Making soil is the work: litter, rot, middens, and a first generation that
dies to feed the second. **Your first bubble on a patch is supposed to fail**,
and it fails *gradedly* — a stand that half-takes, a species that survives
only in the bank — which is `CLAUDE.md`'s first law at the scale of the whole
game rather than one mechanic.

---

## 4. Creatures: you write three commandments and roll the rest

The brainstorm asked for founding a colony to be rare, costly, and **part
random, part player design**. The engine's genome is already that shape and
almost nothing needs inventing.

A creature is a `.ron` file holding: a **body** (`Chain(2)`, cell types, with
articulated bodies on a branch), a **tick interval** (its metabolism), and an
**instinct list** — `(sense, verb, weight)` triples over **29 inputs** and
**13 outputs**. That is 377 possible connections. The shipped ant authors
about ten.

So the founding ritual is:

1. **Choose a body.** Segments and shape. Visible, costly, and it decides
   everything downstream.
2. **Write three or four instincts, as sentences.** *When you smell your own
   kind, turn toward them. When you are carrying and you are home, put it
   down. When you are hungry, dig.* The plain-speech genome readout already
   renders exactly this; this is that view made writable.
3. **Buy a sense, or don't.** An eye is priced: the ant was paying **4% of its
   life** for a sense it never used, measured across 12 of 12 seeds. A real
   trade with a real number behind it, on the founding screen.
4. **The world rolls everything else.** The remaining weights come out of a
   band, and mutation runs from generation one. Two founders written
   identically diverge.

**The stakes are real and already measured.** `creature_arena arm=lethal` puts
a zeroed brain at **0.0% of animals on 12 of 12 seed-runs** against the
shipped one. Selection has teeth in this bed. A badly written founder colony
genuinely dies; a slightly-off one limps at six ants; a good one takes the
valley. Graded, per the first law, without anyone tuning a failure curve.

**And the loop this creates is the best thing in the concept.** You write the
commandments *before* you know the land. So: read the valley, guess what it
needs, write an animal for it, be wrong — then read the corpses. The life
record already says what an individual did and what killed it. Write a better
one. Nobody else can build that loop because nobody else has a real ecology
under it.

**One caution the lab paid for.** The founding moment matters enormously:
dropped on seedlings a colony collapses at once; founded on grown plants it
seats fewer and holds — 5 against 39 at frame 6,000 on the same seed and bed.
So "when may you found" is a real decision the player should be making, not a
button that is always available. *Found it on a stand you grew* is the rule the
measurement already suggests.

## 4a. Making the colony load-bearing, not resident

**The owner's objection to the first draft, and it was correct:** *"this
doesn't intimately integrate creatures into the game."* They were a founding
minigame and then scenery. Three roles fix that, and all three are mechanisms
that already exist.

### The colony is your mouth

**You cannot draw time from a plant.** A plant turns light into tissue; only
*animal* metabolism produces the thing a druid can drink. So **a wood with no
colony pays you nothing**, and founding one stops being flavour.

That makes the food match the puzzle rather than a checkbox: your animals have
to be able to eat what you grew. The engine already prices that — diet lists,
`nectar_only` (*a plant specialist's mouth eats the plant and no gut setting
avoids it*), the whole forage economy.

**And it makes the control problem the right shape.** You leach the colony;
the colony leaches the wood. Three levels, and your *economic* lever only
touches the top one — which is exactly what governing an ecosystem feels
like, and is a far more interesting instrument than a slider on a forest.
It does not fall foul of the second law, because the hand-verbs in §5 reach
every level directly: route them with scent, thin them with a cull, feed
them, fling them, break a drought over them.

### The colony is your hands beyond the rim

**Ants already carry seed and set it down somewhere else** — shipped
2026-09-12. A seed dropped outside the bubble does not die: it goes into the
bank and **waits**, because outside is held.

So your colony sows ground you have not quickened, and you find out what it
did **when you later expand the circle and a wood comes up that your ants
planted while you were not looking.** That is an emergent long-game payoff
out of two shipped mechanisms, and it is the concrete answer to the owner's
*"it is outpacing you"* — the way a garden outgrows your draw is that
**something else is planting it faster than you can.**

### The colony is the weight

Per §2a: population is the frame cost and **cannot be banded**, because a
brain has to run every tick. So head-count is a dial with a real price, and
there is an *optimum* — big enough to harvest and to sow, small enough to run
time fast over. It is different in every valley, and it is a live decision
every session rather than a one-off.

**Which finally gives "founding is rare and costly" a reason that is not
arbitrary.** A colony is the most expensive thing you can put inside a bubble
and you are committing to running it **forever**. The cost is not a gate on
the founding screen; it is the standing bill.

**Mouth, hands, weight.** How you eat, how you expand, and what you pay.

---

## 5. The druid's hands — and the axe

**This is where the concept is most at risk.** A bubble is an *indirect* verb:
place it and wait. `CLAUDE.md`'s second law is precisely about that failure —
*"if a system can only be changed by the world changing around it, the player
is a spectator of it."* A game whose only verb is "wait, but faster" has the
defect the law names.

**The hands already exist. They are in the lab.**

| verb | today | outdoors |
|---|---|---|
| **scent** | drag to lay channel A/B | route your colony to the water you found |
| **alarm** | drop alarm at the cursor | scatter them off something |
| **fling** | click an animal, it launches | the cheapest satisfying verb there is |
| **lamp** | place/move/remove a fixture | light, fire, a hole cut in a canopy |

Plus the three he already carries — **pick, hammer, axe** — and the shake, and
climbing, and swimming.

**One of these is currently a lie and the game would depend on it.** The scent
tool's own positive control counted **1,903 ant-ticks, byte for byte, both
arms, twice**: the shipped ant *cannot read the trail you lay*. The cause is
diagnosed — the laden gate is `-45 + 75·Carrying`, which parks the unit at 30
on the squash curve where the slope is one in a thousand, so the ±6 trail term
moves a step by about ±0.003. If a druid's signature verb is "tell them where
to go", that bug is on this game's critical path rather than in the backlog.

### The trade the game is actually about — interest, not principal

**The first draft made felling the economy and the owner rejected it, rightly:**
*"I don't think it would be fun to have to grow a bunch of plants to then chop
them all down and the world is always barren."* A loop whose steady state is
bare ground throws away the only thing the game is for.

**The owner's model instead**, and it is better: *"you seed an area, let it
grow, and you are leaching on it, slowing it down, but it is outpacing you (if
you are winning the game)."* Conceded as the economy. It is continuous rather
than punctuated, the garden **persists**, and the win condition is legible at
a glance — is it outgrowing your draw or not?

Three things that make it work, each using a mechanism that exists:

**Leach taxes income, never biomass.** If a draw removes cells it is slow
logging and we are back to barren. Tax the *energy budget* the plant economy
already runs, and an over-drawn stand stops **growing** and stops **seeding**
while still standing. That is the right failure: your garden goes static and
stops spreading, and it is recoverable. Push far enough and the engine kills
it anyway, gradedly, through the mechanism it already has — a plant that
cannot pay its maintenance is marked senescent and carried out at the species
half-life.

**Leach is visible on the plants.** Colour is already a readout rather than
decoration, so a stand you are drawing too hard from **pales**. Your draw rate
is legible by looking at the wood, with no HUD at all, and it is graded rather
than binary — the first law, for free.

**Winning and losing are both one image: the radius.** If income funds the
circle, a garden that outpaces your draw **visibly expands the live circle**,
and your score is its radius with nothing on screen to read. The failure image
is as good and as free: an over-drawn circle *contracts*, and the outer ring of
your own wood freezes mid-life as the rim passes back over it — trees you grew,
now held, standing dead still in the grey.

### Where the axe survives, narrowed

Not as the economy. As a **bank withdrawal**.

> **Leach is interest. Felling is principal.**

Leaching is a *rate*, and a rate cannot save you from a crisis. Felling
converts standing capital into time **immediately**, at the cost of that
plant's yield for ever. So it is what you do when you are about to go bankrupt,
or to fund one push you cannot otherwise afford.

That turns stripping the world from a *strategy* into a **death spiral**:
behind, so you fell; less income, so further behind. The barren ending is still
reachable and it is now reachable only by losing, which is exactly right — and
the axe stays in the game, where it belongs, because felling already works and
already feels good.

---

## 6. The weather is a spell list you already own

`weather::Pin` is nine named skies, each chosen to cross a threshold something
downstream actually reads, with the effects measured over 600 frames at a
pinned noon: BREEZE 23 gusts at 230 delivered against GALE's 23 at 486; FROST
3,844 freezes; BLIZZARD 29,248; RAIN deliberately below the lightning
threshold against STORM's 1 bolt. CLEAR is the control and moves nothing.

**Pinning the sky inside a bubble** is therefore a full druid spell list with
real physics under it and near-zero new simulation:

- **call rain** on a bank that needs wetting to break the seed bank's dormancy
- **call frost** to kill back something that is winning too hard
- **call a gale** to disperse seed further than it would go
- **hold noon** over a young stand
- **light a fire** and control the wind that carries it — a burn releases
  nutrient, which is the ecologically correct move and the most satisfying
  verb this engine has

Each of those is a *direct verb with a visible consequence*, which is the
second law satisfied four more times.

---

## 7. The look, and the one thing to build first

The picture is the reason to build this, so it is the thing to check before
anything else is designed.

- **Outside is a photograph.** Desaturated toward grey-blue and *still*. The
  dirty-rect skip means it draws for free. The five zoom-in styles —
  particularly painted+ink — already give the range to make "held" read as a
  deliberate look rather than a missing feature.
- **The rim is where the game reads.** Motion crossing it stops. Rain outside
  hangs as beads; inside it falls.
- **The rim must not be a clean circle.** `dead-ends.md` already settled this
  from the other direction: thresholding a radial kernel field at a constant
  level was **rejected on sight** by the owner — *"the smooth circular
  shape/edges look fake"* — and it is an artifact of the method that cannot be
  tuned out, because a sum of radial kernels cut at one level can only produce
  circular arcs. Soap bubbles. The fix found there applies here unchanged:
  perturb the level with coherent value noise **keyed to world position**, or
  it crawls when the camera moves. A quickening wants a ragged, breathing edge
  like frost spreading on a window.
- **Inside runs a day.** A fast bubble should visibly cycle sun and dark while
  the outside sky holds at one dead hour. Sun strobing over one circle of a
  still grey country is the trailer shot.
- **Ending a bubble should set, not pop.** Colour drains from the rim inward
  over a second or two and the last leaf freezes mid-fall. The field's own
  settle would very nearly do this for free.

### Build this first, before designing another line

A held-world render with one ragged live rim, over a world that already
exists. No economy, no seeds, no founding — a preset that holds everything and
draws one circle where it does not.

It is a render change, it is cheap, and it is the judge-by-eye claim the whole
concept rests on. Post a `filmstrip` strip — and a `gif=1`, because the
question is whether the *stillness* reads, which a grid of stills structurally
cannot answer — and get the verdict before a single mechanic is specified.
`CLAUDE.md`'s method section says this in three separate places; this is the
case it is describing.

---

## 8. What it would cost — what exists, what is new

| piece | state |
|---|---|
| world, rock, weather, fire, liquids, worldgen | shipped |
| gnome: run, jump, climb, swim, dig, fell, three tools | shipped |
| plants: genome, growth, senescence, seeds, seed bank, rot | shipped |
| creatures: body, brain, 29 senses / 13 verbs, breeding, lifespan, kin | shipped |
| soil nutrient, moisture, litter, middens | shipped |
| hand-verbs: scent, alarm, fling, lamp | shipped **in the lab binary** |
| rosters, specimen shelf, plain-speech genome, life record, lineage overlay | shipped **in the lab binary** |
| sky pin, weather pin, five time axes, multi-tick catch-up loop | shipped |
| **regional tick — the bubble** | **new, and the whole engineering risk** |
| **held-world render + rim** | **new, cheap, and the thing to build first** |
| **the time economy** | **new, and mostly numbers** |
| **the founding screen** | **new UI over an existing data model** |

### The one genuinely hard part

Restricting a tick to a region, phase by phase:

- **CA sweep** — `parallel::step` already iterates *active* chunks. Force
  everything outside the bubble asleep and this falls out. Tractable.
- **Organisms and creatures** — schedules, entries due at `frame + interval`.
  Tick only entities inside the circle. Tractable.
- **The field** — a global five-pass solve and ~59% of the frame. This is the
  hard one. It already sleeps per-tile (`FieldTile::sky_drifted`,
  `is_converged`), so a per-tile gate is plausible rather than speculative,
  but it is the piece that decides whether the whole idea is affordable.
- **The player** — always 1x. He walks through his own bubble and watches it
  race around him, which is *correct*, and `gnome_slowdown` being a separate
  axis already says the engine agrees.

**Prove the field can be gated regionally before building anything else
mechanical.** If it cannot, the concept still works but every bubble costs a
full field solve, which caps the rate ladder hard and should be known on day
one rather than found in month two.

---

## 9. Naming

The gnome stays — M9 is built and he is good. What wants a name is the world
and the verb.

- The world is **held**. *This ground is held.* It implies somebody is holding
  it, which is a story hook that never has to be paid off.
- The verb is **quickening** — the old sense, the moment something starts to
  live.
- The player is a **quickener**, or a **warden**.

Working titles: **Quickening**, **The Held World**, **Still Country**,
**Greenwake**.

---

## 10. Open questions for the owner

**Two of the first draft's five are now settled** and are recorded here rather
than asked again. **The inversion is right** — *"I loved the time inversion
bubble idea"*. **The axe stays, narrowed**: felling is principal, leaching is
interest (§5), and the barren ending is reachable only by losing.

Live, in the order they change the most work:

1. **Is a standing garden a base, or is the game a walk?** A permanent
   quickening you return to makes this a colony-sim with a map of holdings; a
   forward walk through a dead country, leaving circles you will never see
   again, is a very different and possibly better game. Everything about
   progression, saving and the map depends on the answer.
2. **Do you drink from animals, or from the whole system?** §4a's strongest
   claim is that a plant pays nothing and only animal metabolism produces what
   a druid can take — which is what makes the colony structural rather than
   decorative. The cost is that a wood alone is worthless, which may be too
   harsh.
3. **How much of the lab's instrument comes across?** Rosters, specimen shelf,
   plain-speech genome, life record, lineage overlay. Enormous assets that
   would take this well past *watch a garden grow* — and a lot of screen, in a
   game whose UI has been kept thin on purpose.
4. **Is the circle's radius the score?** Income funds the rim, so winning is
   the circle growing and losing is it closing over your own wood. It is a
   HUD-free readout of the whole economy; it also commits the game to circles
   as the permanent shape of everything.
5. **Does founding a colony need a ritual, or is it a menu?** The measured
   founding cliff — 5 against 39 at frame 6,000 on the same bed — says *when*
   you found matters as much as *what*, which argues for a moment with a cost
   rather than a button that is always lit.

Standing question, not yet argued: **is there anything in the grey?** A held
world with something in it that does not need time would be a strong
antagonist, and the engine has `ThreatNear`, `Attack` and the alarm plane
already. Not pushed — it is a whole second design.

## 11. What could kill this

- **The field cannot be gated regionally** (§8). Survivable, but it caps the
  rate ladder and should be measured first.
- **The bubble is a spectator verb** (§5). The lab hand-verbs are the answer
  and they exist; the risk is shipping the bubble without them and discovering
  the second law the expensive way, which this repo has done before.
- **The still world reads as broken rather than held** (§7). One `filmstrip`
  GIF and one review card settles it, and it costs a day.
- **The economy is fiddly rather than legible.** "Interest above 1.0" is a
  clean idea and a horrible HUD. The number the player watches has to be one
  number, and finding it is real design work not attempted here.

---

*Sibling documents: [`two-games-one-repo-2026-08-30.md`](two-games-one-repo-2026-08-30.md)
for what is scoped rather than shared between the existing two games, and
[`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md) for
the owner rulings this concept leans on.*
