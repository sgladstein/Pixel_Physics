# Teaching the colony: a game design plan

*2026-10-25. Game design research only: no code and no runs. The owner asked
for "a deep focused session into exploring all the possible ways that a player
could interact with our ant colony to teach them behaviors, and what types of
behaviors, expected and unexpected", from an outside view (a gardener or gnome
who keeps the colony) or an inside one (the player is the queen, which ties in
the genetics), "and how is the queen teaching the colony in a way that's more
complex than you walk to food and your colony follows you".*

*Method: five independent researchers worked separately on the keeper view,
the queen view, other views (guest, colony mind, enemy, lone scout), real ant
biology, and games where players train AI. Their notes (about 250 KB, with
sources) are in [`teaching-the-colony-2026-10-25/`](teaching-the-colony-2026-10-25/).
A first synthesis went to a sixth, independent reviewer, who found that its
keystone would ship doing nothing on today's ants and that it undersold the
queen. This version is rewritten around that review; section 10 lists what it
changed.*

**How the sources were read.** This container blocks most web pages, so the
researchers worked from search summaries and abstracts plus their own
knowledge, and marked which is which. Biology numbers come from abstracts.
Treat them as "true in outline", not as quotable figures.

---

## 1. The short answer

**A real queen does not teach by leading, and neither should ours.** Real ants
change their behaviour through five channels, and only one of them is an
individual brain:

1. **The place**: scent, soil, water, warmth, where the food is.
2. **A little memory in each ant**: a smell that meant food, a place that meant
   danger. Real ants learn a smell paired with sugar in one to three visits.
3. **Ant to ant**: one ant leading another (tandem running), meals that carry a
   smell, young ants imprinting on what surrounds them.
4. **The young**: what larvae are fed and where they lie shapes their bodies and
   tastes.
5. **The bloodline**: who gets to breed.

A teaching game uses all five on three clocks: **seconds** (the player's
body or hand), **minutes** (a brood cycle: each cohort comes out shaped by how
it was raised), and **hours** (the lineage). "Walk to food and they follow"
uses one channel on one clock, and it is a command with a delay, which is why
it feels thin.

**So how does the queen teach?** Through four things a real queen controls,
plus the founding, when she does everything herself:

- **who is born**: which father each egg gets, how mixed a clutch is, and
  which workers' lines she favours (chosen by replaying and blessing their
  deeds, not from a spreadsheet);
- **what they become**: how richly each egg is provisioned, which shapes the
  body through each line's own evolved response, so castes emerge without
  being designed;
- **what the brood lives through**: where she lays it (warm or cool, deep or
  by the door) and what it smells while it grows;
- **where everyone gathers**: moving the brood moves the colony's centre; her
  presence calms the nest and keeps workers home; leaving it makes them bold,
  and some start laying their own eggs.

On top of that, her body has verbs that land *during* an act, so the player
can see what was rewarded: feed a worker from her crop while it is digging,
bite one that is doing the wrong thing, or lead one pupil on a tandem run.

**The seat question is a genuine fork, and it should be settled by playing,
not by argument.** The keeper teaches by *conditioning individuals*; the queen
teaches by *shaping who exists and where they grow up*. The first draft of
this plan recommended the keeper on engine convenience. The review showed that
is backwards: the keeper only becomes a teacher through the riskiest new piece
(ants that learn in life), while most of the queen's levers act through
selection on weights that already exist. **My recommendation now: the queen
as the main seat, with the lab's existing hand tools (plant, lamp, water,
scent) kept as a light "keeper's hand"**, and two faked ten-minute slices,
one per seat, put in front of the owner before anything expensive is built
(section 8).

**The one engine piece both seats eventually need, and the trap in it.** A
small learned layer per ant (a few remembered places and smells with a good or
bad value) is what makes the keeper a teacher and makes tandem runs carry
knowledge. But **a new sense that enters at weight zero does nothing until
selection puts weight on it**, and today's shipped pool has zero weight on the
one memory sense that already exists. The layer must be designed so one
trained reading serves every smell, must be trained on a bed where memory pays
(food that moves), and must ship on only if learning ants beat frozen ones.
Section 7 has the design.

---

## 2. What the engine already gives a teacher

Checked against the source; the reviewer corrected two of these.

| Fact | Where | What it means for teaching |
|---|---|---|
| Brain weights are fixed at birth. The mind has **12 recurrent memory units** with their own forgetting rates | `mind.rs` (`MEMORY = 12`) | Ants already hold short-term state within a life. Nothing persists as learning, and the player cannot write to any of it |
| A **last-food memory** sense exists (each option scored on how much it points back to where the ant last picked up food), on by default since today | `options.rs` `f::FOOD_WAY`, PR #884 | **But the shipped lab pool (`assets/minds/std_pool.txt`, layout 4) has every weight on it zeroed when loaded**, by design, until selection wires it. On today's ants, anything that writes this memory (carrying an ant to food, a tandem run) changes nothing |
| Every live brain link costs energy every tick (`PIXEL_PHYSICS_LINK_PRICE`) | `creature.rs` | Selection actively removes weight from a sense that rarely pays. A memory sense survives training only where memory earns its keep |
| The home sense comes from a stored home point, not step-by-step path integration | `f::HOME`, `forage_anchor` | A flung ant always knows home. The biology "flung ant searches where home would be" needs a real integrator |
| The context carries **the change in energy since the last decision** | `mind.rs` `c::D_ENERGY` | A ready-made reward signal ("a need going down") for any learning rule |
| A **provision** number travels from parent to egg and moves body traits through a heritable developmental block | `creature.rs` `made`, `brain.rs` | The hook for "what the queen feeds an egg shapes its body". The brood lay path sets it to 0, so it is unused today |
| Eggs inherit the layer's brain; any ant may lay (a gene) | `brood.rs` | A queen who lays everything lays only her own genome. She needs stored fathers to choose between |
| The returner-contact count (harvester ants: foraging set by contacts with loaded foragers coming home) **was built** as `PIXEL_PHYSICS_RECRUIT` | `creature.rs`; ant reference §12 | **Left off for a measured harm**: adults starved 442 against 298 and 323, on 10 of 12 seeds. Not a free fix for idle ants |
| A hand-laid trail and even a strong positive-control ramp moved nothing in slice 0, and no worker was traced | slice 0 report | The cause is unknown. It must be traced before anything (trust, lessons) is layered on trails |
| Ants carry and drop seeds, which grow | `wiki/ants.md` | "The ants plant the garden" half exists |
| A steerable queen body exists, switched off; she does not die of old age (owner) | `sim::steer` | The queen seat's body is built |
| Player hand tools: plant, lamp, scent drag, alarm, fling, specimen shelf; per-ant life record; graves with cause of death; named lineages | README lab sections | Most readouts teaching needs exist. What is missing is *ambient* legibility (section 6) |

---

## 3. Who is the player?

### 3a. Seven seats

Scores are the researchers' judgement, 1 to 5; the queen row is re-scored
with the verbs the first draft dropped. Build: 5 = cheapest.

| Seat | How the player teaches | Depth | Natural | Genetics | Second to second | Build |
|---|---|---|---|---|---|---|
| **Queen** (owner) | Who is born, what they are fed, where the brood lies; feed or bite in the act; presence and absence; tandem runs; bless deeds | 4 | 4 | 5 | 4 | 4 |
| **Keeper** (owner; in the lab, a formicarium keeper) | Shapes the box, controls the larder, picks who breeds; with the memory layer, conditions ants by reward | 3 (4 with memory) | 4 | 3 | 3 | 5 now, 2 for teaching |
| **Guest / symbiont** (aphid herd, caterpillar) | By what it gives (honeydew) and demands; the colony learns to farm, guard or expel it | 5 | 5 | 5 (arms race) | 3-4 | 3 |
| **Lone scout** | Persuasion only: tandem runs, shared meals, competing for a quorum | 4 | 5 | 3 | 4 | 4 |
| **Colony mind** | Sets the colony's values; the ants find the cheapest way to satisfy them | 4 | 1 | 4 | 2 | 5 |
| **Enemy** (spider, antlion, rival) | By threat; the colony learns to avoid, mob, rescue | 2 | 5 | 3 | 5 | 4 |
| **Ant-plant** (acacia) | By where its rewards appear | 3 | 5 | 4 | 2 | 3 |

### 3b. The fork, stated honestly

| | Queen | Keeper |
|---|---|---|
| Teaches by | Shaping who exists, what they become, where they grow up; rewarding acts with her own body | Conditioning individuals; shaping the box |
| Works on today's weights? | **Mostly yes.** Selection, provision, brood placement and presence act on the existing 2,800 weights and body genes. Only tandem knowledge and smell-learning need the memory layer | **Mostly no.** Beyond landscaping, every teaching verb needs ants that learn in life |
| What the owner asked about | Both halves of the question: the queen and the genetics | The outside view the owner offered first |
| Biggest risk | The slow clock is too slow to feel; the blessing replay becomes a menu; stored fathers are new code; a queen killed on a trip ends the game | The memory layer ships doing nothing, and the keeper's first lesson lands on ants that cannot hear it |
| Fits the sealed lab box | Yes: a founding queen in a box is the real hobby's starting point | As a formicarium keeper with a test-tube queen, yes; garden commissions, seasons and nuptial flights do not |

**Recommendation: the queen as the main seat, keeping the lab's hand tools.**
Reasons: it is what the owner is drawn to; her levers mostly work without the
riskiest piece; her body gives second-to-second verbs that already exist or
are cheap (carry, lay, feed, bite, plug a door); and the genetics are part of
the fiction, not bolted on. The keeper's tools do not disappear: the lab
already lets the player plant, place lamps, water and lay scent, and those
stay as a light hand over the box. The guest (an aphid herd the colony can
farm) and the enemy (a season as the spider) are good later modes on the same
engine. **But this should be settled by the two faked slices in section 8,
not by this table.**

**What happens to the player's body when the first workers hatch: keep it.**
The first draft never said. Switching genre at the moment the colony first
gets interesting is the worst time to do it. She stays the player's body for
the whole game, moving from doing everything (founding) to shaping (brood,
presence, blessing) as the colony grows.

---

## 4. The teaching verbs

Grouped by channel, each with what the player does, what changes and how fast,
what the player sees, and what it needs. **Needs** codes: **now** = works on
today's engine and today's pool; **mem** = needs the memory layer (section 7)
and a pool trained to use it; **new** = a new behaviour, creature or material.
**Seat** says who has the verb (Q queen, K keeper's hand).

### 4a. The queen's body: verbs that land during an act

| Verb | What happens | What you see | Needs | Seat |
|---|---|---|---|---|
| **Feed in the act** | She feeds a worker from her crop while it is doing something. It gains energy at once and gains *favour*, which weights its line in her next clutches | A gold sheen on the worker; its line counter rises | now (energy), new (favour) | Q |
| **Bite** | A dominance bite (real in queenless ants): the worker loses energy, recoils, and loses favour. Too much and it dies, or alarm rises around her | A flinch and knockback; the worker scurries off | now (attack verb, not yet live) | Q |
| **Carry the brood** | Moving eggs and larvae moves the colony's centre: nurses follow, diggers dig near it, food is stored by it | The colony reorganises around the pile within a minute | now (to verify) | Q |
| **Where she lays** | Warm, cool, deep or by the door; brood by the door stimulates foraging, deep brood is safe from the centipede; warm brood could come out sharper (true in honeybees) | A thermal view of the nursery; cohort quality on its report card | now (place), new (quality) | Q |
| **Lay faster or slower** | More larvae means more begging, more foraging; fewer means idle workers | Traffic at the door rises within minutes | now | Q |
| **Presence and absence** | Near her, workers are calmer and stay home longer; when she leaves, the nest wakes, workers forage early, and some lay their own eggs. Back home, she can eat those eggs or keep them | A soft halo; the nest stirring when she goes out | new (one field and one sense) | Q |
| **Plug or open a door** | Packing spoil re-routes traffic; a new door opens a new foraging direction | Streams of ants reflowing | now | Q |
| **Tandem run** | She taps a worker; it follows only while touching her; go too fast and it is lost. On arrival its food memory is written, and it can lead the next | Nose-to-tail pairs in stop-start rhythm; a chain spreading like a family tree | new (follow and lead), mem (for the knowledge to stick) | Q |

### 4b. The bloodline and the young

| Verb | What happens | What you see | Needs | Seat |
|---|---|---|---|---|
| **Fathers in her body** | Each egg is her genome crossed with one of three to six stored fathers; one gesture sets how mixed a clutch is | Colour flecks per father line; the colony gets stripier or more uniform | new (crossover, store) | Q |
| **The dream: bless a deed** | While she rests, a fast replay of the last cycle's notable acts from the life records: the ant that dragged a beetle corpse, the one that dug the new chamber. Bless a moment and that line is favoured in later clutches. (A game liberty: real workers can only add sons) | Blessed workers marked on their page; their line name spreading | new (interface over existing data) | Q |
| **What the egg is fed** | The provision number shapes each body through that line's evolved response | Big and small siblings; body size tracking the colony's fortunes | now (hook exists, unused) | Q |
| **One champion father or many** | A design hypothesis from bees (diverse colonies did better in one small study): mixed clutches are robust, narrow ones efficient and brittle. To be tested in this engine, not assumed | A monoculture brilliant at one thing, collapsing at the first change | new | Q |
| **The old colony becomes the rival** | At a season's end her daughters fly; the player follows one into a new box, with the mother colony as a neighbouring rival from the same lineage | You compete with your own past choices | new | Q |
| **The founding dowry** | She founds carrying a seed (leafcutter queens carry fungus); the colony's culture grows around it | Grazers, farmers or hunters depending on what she brought | new | Q |
| **Split and requeen** | Beekeeping's tools: lift a chamber with brood into a new box; introduce a shelved bloodline | Two colonies drifting apart | new | K |

### 4c. The place (the keeper's hand; works today)

| Verb | What happens | What you see | Needs | Seat |
|---|---|---|---|---|
| **The watering can as architect** | Wet soil digs easier, so the nest grows toward water; a flooded gallery is abandoned | The nest follows your watering | now (to verify) | K |
| **Warm the nursery** | A lamp at a chosen depth draws the brood | The brood pile migrating under the lamp | now | K, Q |
| **Training grounds** | Stepping-stone food at growing distance, a channel to cross: a course the player builds | A success count per stage | now | K |
| **The double bridge** | Two routes to food; the colony converges on the shorter | Traffic shifting to one bridge | now (to verify) | K |
| **Sow seeds the ants carry** | Seeds go home and are dropped at the dump or along routes | Trails becoming hedgerows | now | K |
| **Evict by light or drought** (the ant-keeper's trick) | Make the old nest worse and a new spot better, and the colony moves itself | A migration column carrying brood | new (emigration) | K |

### 4d. Memory in each ant (needs the memory layer and a pool trained to use it)

| Verb | What happens | What you see | Seat |
|---|---|---|---|
| **Flavour the food** | Food carries one of a few smells; ants learn it in one visit, and the first lesson sticks hardest | Foragers walking past unscented food toward the first reward's smell | K, Q |
| **Meals carry the smell** | An ant fed mouth to mouth learns the food's smell and goes looking for it, though it never visited | Ants that never went to the mint patch heading for mint | both |
| **Treat and puff** | A sugar bead on an ant just after it acts (real food, so a real reward); bitter dust as the negative; an area version | The ant turns back to the spot; a mark on it | K |
| **Hazards teach refusals** | A pit or a sprayed plant: ants learn the place in one trial; the colony's refusal can outlast the ants that learned it, through smells at the dump | Ants skirting the spot where nestmates drowned | both |
| **Rig success** | An ant that keeps finding food forages more; one that keeps failing drifts toward the nest (took weeks of repeated success in the real study) | A forager you made, in its life record | both |
| **Dinner time** | Feed at the same hour of the lab's day | Ants gathering *before* the food arrives | K |
| **Becoming a nestmate** | Instead of a trust number: a keeper who smells of the nest (rubbed with nest material) is accepted, as real guests are | Ants that stop alarming at your hand | K |

### 4e. Relationships

| Verb | What happens | What you see | Needs |
|---|---|---|---|
| **Keep a herd** | An aphid herd on a plant pays honeydew; ants tend, guard and carry it | A ring of ants facing outward when a beetle comes | new (creature) |
| **Pay the guard** | Plants with nectar get patrolled and defended | Ants clustered on the trees you planted | new (nectar organ) |
| **A season as the spider** | The player hunts; the colony learns to avoid, mob and rescue | Nestmates digging a victim out of your pit | new |

---

## 5. Behaviours: what the player aims for, and what surprises them

### Expected

- Forage far (the known failure), including by tandem chains.
- Dig a multi-room nest with a nursery, larder and midden, shaped by where
  she lays and plugs.
- Nurse properly instead of idling.
- Defend: plug doors before a raid, big-bodied lines from rich provisioning.
- Evacuate brood from a flood (first by her hand, later on their own).
- Avoid a hazard; prefer a food smell.

### Surprising (each with its cause)

| What happens | Why |
|---|---|
| **Courtiers**: lines that hang around the queen because that is where favour falls | The player's own reward habit, selected for |
| **Goodhart's ants**: bless "deliveries" and ants carry the same crumb out and in; bless digging and they dig and refill (training round 5 already found pellets going back into the hole) | Any rewarded measure gets gamed; the dream should show "deeds that did nothing" |
| **The colony that needs her less**: if following her is costly, lines that ignore her win | Teachability evolves both ways; a self-reliant colony is a success state |
| **Mutiny**: a queen who bites too much breeds a colony that stops feeding her | Tolerance of the queen as a gene (real in fire ants) |
| **The bold nest**: she leaves for a trip and comes back to worker-laid eggs from a line she never chose | Her absence as a lever |
| **Dwarf economies**: a lean colony raises tiny cheap workers; recovery brings back big ones | Provision through each line's evolved response |
| **Vintages**: a mint-raised cohort that forages mint all its life | Nursery imprinting (mem) |
| **Dialects**: change what a smell means and old foragers keep the old meaning | Memory differs by age (mem) |
| **Haunted ground**: a patch where a raiding party died stays empty for generations | Learned avoidance kept by the colony (mem) |
| **Superstition**: a crumb fell by a pebble once, and an ant now circles that pebble | Badly timed reward (mem) |
| **Begging**: ants loiter where treats come from | Reward that does not depend on the act (mem) |
| **The cheater line**: drinks your nectar without guarding and outbreeds the guards | Real acacia biology |
| **The ant mill**: followers on a closed trail loop circle until they die | Real army-ant behaviour; worth testing by laying a ring of trail |
| **A living ant carried to the graveyard**, again and again | Ants recognise death by smell |
| **Sand grains as sponges**: soaked grains carried home from spilt nectar | Real (Maák 2017); a falling-sand engine can show it honestly |
| **Slave rebellion**: captured workers raise their captors' young, then kill most of the pupae | Real; needs raids and imprinting |
| **Lessons slowly become instinct** | The Baldwin effect: real, but slow and partial (section 7) |

**Not a delightful surprise: the spoilt colony.** A colony fed on a timetable
that stops foraging and starves when the player stops is *Creatures 2*'s worst
failure. It is a failure to guard against: graded decline, a visible reason,
and a way to recover.

---

## 6. Playing it: the first ten minutes, the loop, and the fifth hour

### The first ten minutes (queen seat)

- **Minutes 0 to 3: found.** Land, dig the first chamber, lay. Digging is
  physical, with debris and a hole. This already works.
- **Minutes 3 to 8: the founding, without a dead wait.** A real founding queen
  seals herself in and waits. Instead, give the player short foraging trips out
  of the chamber with graded risk: a beetle wounds rather than kills (wounds
  heal since 2026-10-03), and a wounded queen limps home. Keep beetles out of
  the first minutes, as slice 0 advised. While she is home, she tends the brood
  (move it to the warm side, feed the larvae).
- **About minute 8: first workers.** She keeps her body. The first instant
  teaching verbs open: feed a worker in the act and see the sheen; carry the
  brood and watch the first workers follow it.
- **Minutes 8 to 10: the first cohort report.** The first workers' card:
  father, how they were raised, their first deeds.

### The second-to-second loop

**Act on one ant, see the response on that ant, see the colony shift.** Feed or
bite in the act, carry brood, lead a pupil. The payoff must be on the ant and
in the colony, not text behind a click. That needs **ambient legibility**:

- tint ants by father line, by favour, or (later) by what they have learned;
- an overlay of the paths taught ants take;
- pin and follow one ant;
- an on-ant mark when a lesson or favour changes a decision;
- time control to fast-forward a brood cycle.

### The fifth hour

- **Good**: the field notebook of behaviours the colony has been seen doing;
  the assimilation ladder (each lesson as *taught → assisted → instinct*);
  dialects and haunted ground; the old colony as rival; Goodhart puzzles in what
  to bless.
- **The risk**: per-ant lessons die with their ants (about 40,000 frames of
  life against a 12,000-frame generation). Unless lessons cascade or the
  bloodline carries them, the fifth hour is renewing lessons, which is
  Tamagotchi upkeep with a teaching skin. The queen seat is less exposed,
  because her main levers are genetic.
- **What the colony does for the player**: the missing payback loop. Ants that
  plant the seeds she carried, clear pests off plants, and feed back into the
  box's economy make teaching pay off, not just maintain.

### What keeps it fun, and what kills it

1. **The effect shows on one ant within a minute.** Players of both *Creatures*
   and Black & White found teaching opaque or tedious, and Black & White 2's own
   designers let players buy creature miracles instead of teaching them. That
   is the genre's own warning.
2. **A dog, not a robot and not a goldfish.** Too obedient is Pikmin; too weak
   is a spectator sport. This needs a mechanism, not a target: learned values
   are capped below instinct, and the chooser never fully excludes an option.
3. **Every per-ant verb spreads**: an area version, a cascade, or inheritance.
4. **Rewards are real things**: food eaten, harm suffered; never a score. Learn
   from contrast, so a steady drizzle teaches nothing.
5. **Loopholes are stories only if legible and reversible.**
6. **Defaults everywhere, no sliders.** Choose by blessing moments, not stats.
7. **Graded failure.** A colony shrinks, shows why, and can recover.
8. **Decide about cruelty.** Every verb that can hurt will be used to hurt
   (*Creatures* had "Norn torture" sites). Record it (graves), keep it legible,
   and keep it from poisoning a shared colony.

---

## 7. The memory layer, redesigned so it cannot ship dead

**The trap.** A sense entering at weight zero changes nothing until selection
puts weight on it. The training bed does not reward memory (food sits a fixed
distance out), and every live link is billed, so selection removes weight from
a rarely useful sense. A smell that never appears in training (the keeper's)
can never be weighted at all. A sweep of such a feature reads "neutral"
because nothing moves, and under "features default on" it would ship doing
nothing.

**The design that avoids it:**

1. **Identity-free inputs.** The brain reads a few generic option features:
   "this step climbs a smell I have found good" (the sum over a fixed palette
   of 3 to 4 smell channels of each channel's learned value times its gradient),
   its negative twin for "bad", and "this step heads toward a place I remember
   as good / bad" (generalising the last-food memory to two or three
   remembered places with a value). One trained set of weights then serves
   every smell, including ones training never saw.
2. **Heritable starting values per smell channel**, so there is something for a
   lesson to become instinct *in*: "born knowing mint is food" means that gene
   has drifted up.
3. **Make it work on day one.** Either hand-seed the founder's weight on the
   generic inputs at a modest value (`enter_rung` already writes weights by
   hand), or route the learned value through a body-level pull with a heritable
   gain, as many existing pulls in the walk work. Evolution then tunes how much
   to listen.
4. **Train where memory pays.** The moving-food arena (`foodflip=`, built today)
   must be a *training* bed, not only a test, so selection keeps the weight
   against the link bill.
5. **Ship on only if learners beat frozen ants** on paired seeds, never because
   a sweep reads neutral.
6. **Price the smells.** Each smell channel is a scent plane, a per-cell cost
   every frame. Fix the palette at 3 to 4 and quote worst-frame cost from
   `examples/ascii.rs`.

**The collapse to guard against** (*Creatures 2*'s "One Hour Stupidity
Syndrome"): punishment that outweighs reward, or lands on eating, makes agents
unlearn their needs. Guards: learn from contrast, not raw reward; cap learned
values below instinct; no learning on the hunger→eat path; decay; the chooser
never excludes an option. Prove each guard by removing it and watching a test
fail. Judge at full game length, by ant age.

**Two ways to learn, and which first.** The ML researcher proposed small
reward-driven changes to the brain's readout weights (an eligibility trace,
reward compared with expectation). The biology and keeper researchers proposed
learned values read as senses. Build the senses first: cheaper, legible as
sentences, unable to erase an instinct, and they let evolution decide how much
to listen. Keep the weight-change rule for later.

**Be honest about the Baldwin effect.** Learning can guide evolution, but it is
slow and partial, and learning can also shield genes from selection. In a
single colony where selection is weak, "born knowing" may take far longer than
a play session. The player seat that actively selects (the queen) is the one
where it can show.

---

## 8. What to build, in order

| Step | What | How we know it worked | Cost |
|---|---|---|---|
| **0. Look and trace first** | (a) Worker-level trace of whether trail B is read at all (slice 0's open question). (b) Print the pool's weight on the last-food sense, then hand-seed it as a positive control and check that "carry an ant to food" changes that ant's next trip. (c) Ring-of-trail test for an ant mill. (d) Does watering steer the nest? Does carrying brood move the colony? | Each is a scene with a counter, shown as a picture | small |
| **1. Two faked slices** | Ten minutes of each seat with a *scripted* learned response (hand-set biases on ants that were fed or treated) and the ambient overlays, so the teaching loop can be played before it is built | The owner plays or watches both and picks; posted as GIFs on an Artifact page | small to medium |
| **2. The queen's no-learning verbs** | Feed and bite in the act (energy and favour), brood placement as the colony's centre, presence and absence, the dream over life records, provision on the brood path | Favoured lines grow over cohorts; the colony visibly follows the brood; cohort cards differ | medium |
| **3. The memory layer** (section 7) | Generic smell and place values with heritable priors, hand-seeded or as a pull, trained on the moving-food bed | Learners beat frozen ants on paired seeds; a new trail pulls after it has led to food a few times; a treat dose-response (0, 3, 10, 30) is graded; no loss on a 24+ seed sweep's worst case; worst-frame cost | medium-large |
| **4. Tandem runs** | Follow and lead as options every ant has | A taught chain spreads; far-food deliveries rise against no-tandem on paired seeds | medium |
| **5. Fathers** | Stored fathers and crossover | Mixed versus narrow clutches under a changing box | medium-large |
| **6. Relationships** | An aphid herd, nectar plants, a season as the spider | Guarding where honeydew is; cheater lines appear | large |

The memory layer also bears on today's two big problems, but carefully: route
memory and tandem runs are the biological answer to far-food starvation; the
returner-contact count was already tried for idle ants and harmed survival, so
it needs a stated reason it would come out differently before it is tried
again.

---

## 9. Decisions for the owner

1. **Which seat is primary?** Recommended: the queen, keeping the lab's hand
   tools. Settled best by the two faked slices.
2. **May ants learn during their lives?** The memory layer turns "weights fixed
   at birth" into "weights fixed at birth, plus a few learned values read as
   senses". It ships on only if learners measurably beat frozen ants.
3. **How much game liberty with the biology?** Blessing a worker's deed to
   favour its line, and a queen who leaves the nest on trips, are liberties
   (real workers add only sons; real founding queens mostly seal in). Fine for
   the game, but worth choosing on purpose.

---

## 10. What the independent review changed

The reviewer read the first draft cold. Its full review is
[`critique.md`](teaching-the-colony-2026-10-25/critique.md). What it changed:

- **The memory layer would have shipped dead.** The draft's "now" verbs on the
  last-food memory do nothing on the shipped pool (its weights on that sense
  are zeroed on load). The layer is redesigned (section 7): identity-free
  inputs, heritable priors, a day-one route, a training bed where memory pays,
  and a "learners beat frozen" gate.
- **The seat recommendation flipped** from keeper to queen, and is now framed as
  a fork to be settled by play. The draft dropped the queen's best verbs (feed
  and bite in the act, presence and absence, brood as the colony's centre, the
  old colony as rival); they are back.
- **The first ten minutes and the second-to-second loop are designed**
  (section 6), including keeping the queen's body after the first hatch and
  replacing sudden beetle death with wounds.
- **The keeper is fitted to the sealed lab box**, not an outdoor garden.
- **Corrected claims**: the dead trail is not "realistic" (62 to 70% in a
  two-way choice is a strong effect, so our null is unexplained); tandem
  running was the first *claimed* case of non-human teaching and was contested;
  Black & White has no breeding (its sequel let players buy miracles instead of
  teaching, which is the stronger warning); the Baldwin effect is slow; Belyaev's
  foxes were human selection; the bee diversity result is a small study, so a
  hypothesis here; the returner-contact count already exists and was left off
  for a measured harm; the spoilt colony is a failure to guard, not a feature.
- **Added**: ambient legibility, a payback loop, graded failure and recovery,
  becoming a nestmate instead of a trust number, a decision about cruelty, and
  a "look and trace first" step 0.

The review's other point stands as a warning on everything above: **this is a
fourth model**, and the owner's playtests have overturned three before it.
Step 1 exists so play can overturn this one cheaply.

---

## Sources

The research notes, each with its own source list and the search-summary versus
memory marking:

- [`keeper.md`](teaching-the-colony-2026-10-25/keeper.md): the keeper view; about 15 games, 6 hobbies (ant keeping, beekeeping, shepherding, falconry, clicker training, fox domestication), 36 verbs.
- [`queen.md`](teaching-the-colony-2026-10-25/queen.md): the queen view; 17 games, queen biology as levers, about 30 mechanisms, 12 unusual ideas.
- [`perspectives.md`](teaching-the-colony-2026-10-25/perspectives.md): guest, colony mind, enemy, scout, ant-plant, parasite fungus, researcher, weather, multiplayer.
- [`biology.md`](teaching-the-colony-2026-10-25/biology.md): about 40 real mechanisms of ant behaviour change, with a teaching lever for each.
- [`ml-teaching.md`](teaching-the-colony-2026-10-25/ml-teaching.md): games where players train AI (NERO, Black & White, Picbreeder, Galactic Arms Race and others), human-feedback learning, specification gaming, a learning rule with tests.
- [`critique.md`](teaching-the-colony-2026-10-25/critique.md): the independent review of the first draft.
- [`engine-brief.md`](teaching-the-colony-2026-10-25/engine-brief.md): what the researchers were told about the engine.

Related: [`brain-training/research/creatures-1996-game-design-2026-10-25.md`](brain-training/research/creatures-1996-game-design-2026-10-25.md) (what made *Creatures* fun and boring), [`play-as-ant-slice0-2026-10-11.md`](play-as-ant-slice0-2026-10-11.md) (the queen body; the hand-laid trail measurement), [`evolution-lab-design-guide-2026-08-30.md`](evolution-lab-design-guide-2026-08-30.md) §4 (the lab's verbs).
