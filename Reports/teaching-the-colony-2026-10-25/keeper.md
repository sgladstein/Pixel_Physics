# The Keeper: teaching an ant colony from outside the nest

*Independent research for the teaching-the-colony plan. Lens: the player is a
gardener / gnome who sows, waters and tends a garden, and keeps an ant colony
that lives in it. Written 2026-10-25 from the engine brief, a skim of
`wiki/ants.md` and `wiki/plants.md` for engine facts, web searches, and my own
knowledge of the games and hobbies named. Each source note says which it is:
**[search]** means a web search summary backs it, **[knowledge]** means it is
from memory and is unverified.*

---

## 0. The thesis in one paragraph

A keeper cannot give orders to a colony. Beekeepers, ant keepers, shepherds
and falconers all manage animals that do not take instructions, and they all
use the same three tools. **They shape the place** (space, moisture, warmth,
where the food is). **They control the larder** (hunger is the motive, and
food is the reward). **They pick who breeds** (requeening, splits, selection).
The engine already has the first and third. The one missing piece that would
make a keeper's lessons *legible* is a very small amount of in-life memory: a
per-ant table that ties a smell to good or bad, and that the fixed network
reads as one more feature. Real ants learn an odour–sugar pairing in about
three trials, so this is natural, not a cheat. Once that exists, three of the
best ideas below follow from it:
- **the gnome's own smell is one more odour the colony learns** — trust, earned and lost;
- **plant scents become a vocabulary** — the garden is the teaching language;
- **what a young ant smells in the nursery sets its tastes for life** — cohorts, generations, schooling.

And **whether an ant listens to its memory at all is a gene**, so a colony a
keeper works with becomes domesticated over generations, as Belyaev's foxes
did. That last point is how in-life learning and the owner's
genetics-as-product rule stop competing and start feeding each other.

---

## 1. What "teach" can mean in this engine: four levers

The weights are fixed at birth, so "the colony learned" has to mean one of
these four things. Every verb in §3 is tagged with the lever it pulls.

| Lever | What changes | Speed | Already in the engine? |
|---|---|---|---|
| **W: the world** | scent planes, food placement, plants, soil wetness and hardness, lamps (warmth), stone, water | immediate, gone when the world changes back | yes: trail/alarm painting, lamps, sowing, water. Note that **the brief measured that a hand-laid trail does not pull workers**, so W alone currently fails at the most obvious verb |
| **L: in-life memory** | a small per-ant state the fixed network reads as features | minutes to an ant's lifetime | only partly: the "memory of where it last found food" feature. Nothing a player can write to |
| **G: the bloodline** | which genomes survive, lay and get fed as larvae | generations | yes: laying is a gene, an egg inherits the layer's brain, and the shelf clones and mutates |
| **S: social transmission** | one ant changes another's state: tandem leading, imprinting the young, trail traditions | days to generations | no (trails exist but carry no learned content) |

**The design point that matters most:** an L-lever input only matters if the
evolved network has weight on it. A colony bred offline without the memory
feature will ignore it. So the memory feature has to be present during
offline training. The weight on it then becomes a heritable trait:
**teachability, or biddability, which the keeper breeds for without being
told to.** That is the Baldwin effect, and it is also the plain story of
domestication. Belyaev bred only the tamest 10% of silver foxes each
generation, and tail-wagging appeared within a few generations [search].

---

## 2. What real games and hobbies teach us

### Games

**Black & White (Lionhead, 2001)** [knowledge, with a postmortem located by search]
- *What it did:* you trained a giant creature by slapping and stroking it after it acted, and it also learned by watching what you did. The creature AI is generally described as using desire tables and learned opinions about objects. A leash steered it (compassion, aggression, learning).
- *What worked:* the stories. Players remember the creature that learned to eat villagers, or to throw rocks at its own temple, because they had accidentally rewarded it. A pet that misreads you is memorable.
- *What was boring or frustrating:* **credit assignment was opaque.** Players could not tell which act a slap had been attached to, so the training felt like superstition. Training was also mostly one-on-one micromanagement, which wore thin once the novelty passed.
- *Lesson:* emergent mis-learning is gold, but **only if the player can find out afterwards what was learned and why.** The ant's page should say it plainly, in sentences: "this cohort learned that mint means food (2 rewards, 1 day ago)". The engine already reads genomes back as sentences, so the pattern exists.
- Source: Molyneux's postmortem is at gamedeveloper.com [search located it; the AI section was not read].

**Creatures (1996)** [search, light; kept deliberately brief, see the brief]
- Neural-network creatures, taught words by repeating them, and bred.
- *What lasted:* the community. Players ran genome projects, breeding experiments and adoption agencies. **Sharing bloodlines kept it alive long after the toy part was used up** [search].

**Pikmin** [search + knowledge]
- *What it did:* direct command. A whistle calls, a throw assigns. Each colour is immune to one hazard.
- *What worked:* the satisfaction of a crowd answering you instantly. Pikmin deaths feel bad, so losses carry weight.
- *What it costs:* the creatures are tools. Nothing they do surprises you.
- *Lesson:* this is the opposite pole from what the owner wants ("more complex than you walk to food and your colony follows you"). Use it as a **boundary marker**: if a verb produces Pikmin obedience, it has killed the simulation.

**SimAnt (Maxis, 1991)** [search]
- *What it did:* you play one yellow ant inside the black colony. It can lay a scent trail, recruit followers, and swap bodies.
- *What worked:* the trail verb, because the simulation was tuned so that trails pull.
- *Lesson:* our engine measured the opposite, so **copying the trail verb will fail as-is.** Idea B1 below turns that failure into progression: a trail pulls in proportion to how much the colony trusts whoever laid it.

**Empires of the Undergrowth** [search]
- *What it did:* an ant RTS. The colony is a unit roster. Documentary-narrator missions, plus a lab "formicarium" home colony that absorbs useful traits from rivals.
- *What worked:* the narrator gives events weight. The formicarium frame is the same fiction as our lab.
- *Lesson:* a narrator or field journal that writes up what your colony did is a cheap way to make a slow change legible. Steal the framing, not the RTS.

**Viva Piñata (Rare, 2006)** [search]
- *The closest existing model of "the garden shapes the animals".*
- *How it worked:* each species has **visit** conditions (it wanders in) and **resident** conditions (it stays). Visit conditions are hidden until you first sight the animal. Some species respond to the garden's makeup ("40% water"). Predators visit because eating a resident is part of their own requirement. Space is limited, so you must choose which species to keep.
- *Lesson:* graded attraction thresholds ("seen at the edge", then "resident") and **discovering the requirement by seeing the animal** are both directly reusable. In our version: a behaviour shows up at the edges of the colony before the whole colony does it.

**Oxygen Not Included, critter ranching** [search]
- *How it works:* grooming at a station tames a critter. A tame critter lays about 15 eggs in its life, against 1 for a wild one. **Diet and environment decide which morph hatches.**
- *Lesson:* "what you feed decides what you get next generation" is exactly lever G, driven through lever W. It reads as husbandry, not genetics homework.

**Niche (Stray Fawn, 2016)** [search]
- *What it did:* explicit genetics breeding with real inheritance rules.
- *Reception:* praised for depth, and called repetitive and predictable by some reviewers.
- *Lesson:* genetics play needs **visible traits and short generations**, or it becomes spreadsheet breeding.

**Equilinox** [search]
- Evolution is a "mutation button" pressed once you have set the right environment. Players noted it is not really an evolution simulation.
- *Lesson:* a button labelled "evolve" reads as shallow. Our selection has to come from survival, not a menu.

**Terra Nil** [search]
- A restoration puzzle in which you rebuild the land and then tear down your own machines. Reviewers liked having no timers.
- *Lesson:* **landscaping as the verb, and watching life respond, is satisfying on its own**, if the response is visible and the player sets the pace.

**Rain World** [search]
- Creatures keep a relationship value per other creature. Lizards learn to recognise and like the player. None of it is scripted.
- *Lesson:* **trust as a number on the creature, raised by feeding**, is the right model for taming. It also proves that a few relationship values produce stories.

**Tamagotchi / Nintendogs** [search]
- *Tamagotchi:* death from neglect gave care weight. Players also quit once care became a thin chore with small rewards and nothing new to look forward to.
- *Nintendogs:* dropped death and made voice-command training the hook.
- *Lesson:* care loops rot into chores. Training that produces **new behaviour** is what keeps people.

**RimWorld / Dwarf Fortress animal training** [knowledge]
- *How it works:* skills are trained (obedience, release, haul, rescue), and they **decay without upkeep**. Tameness reverts. Wildness is a stat.
- *Lesson:* decay is good. A lesson that fades unless renewed is a reason to come back, and it is realistic (memory decays in ants too).

**Stardew Valley** [knowledge]
- Daily watering and animal petting work because each is tied to a visible payoff (produce, hearts) and a seasonal rhythm.
- *Lesson:* the keeper's daily tending should **pay out in behaviour the player can see**, not only in a bar.

**Spore** [knowledge]
- The Sporepedia, where players uploaded and downloaded creations, was the long tail.
- *Lesson:* make colonies shareable as a seed or code.

**Grounded / Grounded 2** [knowledge, uncertain]
- I believe the sequel added rideable "buggies", including an ant.
- *Lesson:* taming as mount-collecting. Wrong pole for us; noted only for contrast.

### Hobbies and real animals

**Ant keeping (formicaria)** [search + knowledge]
- *The keeper's verbs:* founding a queen in a test tube and leaving her alone, feeding protein plus sugar, removing leftovers, adding modules as the colony grows, and keeping it from escaping (fluon, petroleum jelly) [search].
- *A standard trick [knowledge]:* **you move a colony by making the old nest worse** (drying it out, putting it in the light) and the new one better (damp, dark, warm). The colony relocates itself. That is a pure W-lever teaching verb that already works in our engine's terms: wetness, warmth, roofed and headroom are all features.
- *The hobby's pleasure:* slow growth, watching, and the founding being hard. Disturbance is bad: a stressed queen eats her brood.

**Beekeeping** [search]
- The beekeeper never commands bees. They manage **space** (adding boxes), **timing**, and **the queen**.
- *Swarm control:* splits, which mimic swarming without losing the bees, and annual requeening, which reduces swarming.
- *Temperament is changed by requeening with gentler stock* [search]. Genetics is the steering wheel and the queen is the handle.
- *Lesson:* the **split** is a ready-made G-lever verb: take part of the colony with its brood to a new box.

**Shepherding and dog work** [search]
- You move animals by **pressure and release** at the edge of their flight zone. You do not chase stragglers, and stepping back is the reward.
- *Lesson:* a gnome's **body as pressure** (stomp, shadow, presence) that pushes ants away, with release as the reward. You steer a crowd without touching an individual.

**Falconry** [search]
- **Manning:** habituating the bird to the handler, and linking the glove with food.
- **Weight management:** a slightly hungry bird is motivated; too hungry and it is weak.
- **The lure:** a paired food reward.
- **The creance:** a tether stage before free flight.
- *Lesson:* hunger is the keeper's dial. A colony is most teachable when it is a bit hungry, and **the larder is the control.**

**Clicker training** [knowledge]
- A neutral cue (the click) is paired with food until the cue alone rewards. Behaviour is then shaped by rewarding successive approximations.
- *Lesson:* ants can be conditioned to a gnome cue such as a stomp, a chime or a scent.

**Real ant learning**
- Formica fusca workers learned to tie a cancer-cell odour to sugar in about three trials (minutes) and remembered it for days (Piqueret et al., 2022) [search].
- **Callow workers imprint on colony odour in their first days**, and larvae raised in an adoptive colony later prefer that colony's brood. This is how slave-making ants' captives come to work for them [search].
- **Tandem running in Temnothorax is the first demonstrated animal teaching:** the leader slows to about a quarter of its normal speed, and the follower taps the leader when ready to continue (Franks & Richardson 2006) [search].
- **The double bridge:** colonies converge on the shorter of two routes through trail reinforcement (Goss, Deneubourg et al., c. 1989–90) [search].
- **Colonies have personalities:** some Temnothorax colonies are consistently risk-prone and others risk-averse [search]. *Caution:* the social-spider "bold individual drives the colony" work that also came up is largely Pruitt's, several of whose papers were retracted in 2020–21 [knowledge]. Do not lean on it.

**Ant–plant mutualism** [search]
- Acacias house Pseudomyrmex ants in hollow thorns and pay them with nectar and protein (Beltian) bodies. The ants defend the tree. Acacias reward better defenders more and **invest more in rewards during drought**. Cheater ants take the nectar without guarding.
- **Elaiosome seeds** pay ants to carry them home.
- *The engine already has the second:* a seed's provision rides home and the seed is set down (`wiki/ants.md`, "a harvester became a sower").

---

## 3. Teaching verbs for a keeper

Key: **Lever** W/L/G/S as in §1. **Fit** = graded outcome, visible consequence,
natural feel, no hand-made castes, each judged ✓ / ~ / ✗. **Cost** = S
(existing systems), M (new per-ant state or one new interaction), L (new
subsystem).

### A. The larder: food as reward and hunger as motive

| # | Verb | What the player does | How behaviour changes | Feedback the player sees | Fit | Cost |
|---|---|---|---|---|---|---|
| A1 | **Feeding station** | Put food at the same spot repeatedly (a bird table) | W now; L if memory exists ("last found food here" already exists, so it would become a decaying place-preference); G as foragers that use it survive | Traffic line to the station thickens over days and fades after you stop | ✓✓✓✓ | S |
| A2 | **Reward on arrival (clicker)** | Drop a crumb right where an ant just did something you liked (dug at the right spot, crossed a gap) | L: an odour/place valence update for that ant, and for nestmates fed from its crop by trophallaxis (they get the smell of the reward) | That ant's page: "rewarded for digging here"; its subsequent digs cluster | ✓✓✓✓ | M |
| A3 | **Hunger dial (falconer's weight)** | Feed lean or rich | W/G: hungry colonies explore and learn faster; overfed colonies idle (**the known "fed ants idle on the mound" problem, recast as the cost of spoiling them**) | Activity on the mound; lessons "take" faster when lean | ✓✓✓✓ | S |
| A4 | **Diet steering (ONI morphs)** | Offer seeds vs fruit vs protein | G: lineages that handle that food breed more, so the colony drifts toward harvester or hunter habits | Lineage share chart shifts over generations; the colony's routes change | ✓✓✓✓ | S |
| A5 | **Paired cue (stomp, chime or scent then food)** | Make a cue, then feed | L: cue gains positive valence, then the cue alone draws ants: recall | Ants converge on a stomp. **Too often without food and it extinguishes** | ✓✓✓✓ | M |
| A6 | **Feeding the nursery** | Leave food in a chosen chamber | G: larvae in that chamber (often from particular layers) are fed better and survive more. Soft selection without killing anyone | That chamber's lineage grows | ✓✓✓✓ | S |

### B. The garden as vocabulary: plants, soil, water, light

| # | Verb | What the player does | How behaviour changes | Feedback | Fit | Cost |
|---|---|---|---|---|---|---|
| B1 | **Plant a signpost** | Sow a species with a distinctive scent (each species gets an odour tag) | L: ants rewarded near mint learn mint = good; mint now attracts them wherever you plant it | Ants beeline to new mint you sow on the far side of the bed | ✓✓✓✓ | M (odour tag per species plus the memory table) |
| B2 | **Plant a fence** | Sow unpalatable or thorny species ("plants that are worse to eat" exist) | W/L: bad experience, so negative valence; a hedge they avoid | A traffic gap where the hedge stands | ✓✓✓✓ | S–M |
| B3 | **Pay a bodyguard** | Grow a plant with nectaries (acacia model) | G/L: foragers that loiter there are fed; colonies that patrol it thrive and defend it against beetles and the centipede | Ants clustering on your fruit trees; beetle attacks there drop | ✓✓✓✓ | M (nectary feature plus attack verb) |
| B4 | **Plant for sowing** | Choose plants whose seeds carry ant provisions | W/G: the colony carries those seeds home and drops them at the midden or along routes | **Trails become hedgerows.** The garden's layout records the colony's roads | ✓✓✓✓ | S (exists) |
| B5 | **Water where you want them to dig** | Use the watering can on a patch | W: wet soil is easier and preferred (the wetness and hardness features exist); the nest grows toward it. Drown a gallery and they abandon it | The nest map grows toward wet patches; a flooded room empties | ✓✓✓✓ | S |
| B6 | **Eviction by light or drought (the hobbyist's move)** | Put a lamp over the nest, dry it out; make a dark, damp, warm spot elsewhere | W: brood carried to the better spot, so the colony relocates | A visible migration column carrying brood | ✓✓✓✓ | S–M (needs brood-carrying to a site choice) |
| B7 | **Stone and sand landscaping** | Place stone (no dig) and sand (fast, collapsing) | W: the nest's shape follows your geology | Nest silhouette | ✓✓✓✓ | S |
| B8 | **Warm room** | Put a lamp at a chosen depth | W: brood kept where it is warm | Nursery moves under the lamp | ✓✓✓✓ | S |

### C. The gnome's body and his smell

| # | Verb | What the player does | How behaviour changes | Feedback | Fit | Cost |
|---|---|---|---|---|---|---|
| C1 | **Trust (your own scent)** | Everything the gnome lays (trails, food, carried ants) carries "gnome" odour | L: gnome-scent valence rises when a gnome trail led to food, and falls when it led nowhere or when he flung or flooded ants. **A trail pulls in proportion to trust** | Early on ants ignore your trails (as measured today); later they follow. Cry wolf and they stop | ✓✓✓✓ | M |
| C2 | **Pressure and release (shepherd)** | Stand or stomp near ants: they move away. Step back when they go the right way | W: alarm or vibration repels; L if release is rewarded | Herding a column around an obstacle | ✓✓~✓ | M (vibration sense) |
| C3 | **Carry an ant (social carry)** | Pick up an ant and set it down at food | L: it gets the place memory; with S it may lead others | That ant returns repeatedly and sometimes brings a follower | ✓✓✓✓ | S (fling exists) to M |
| C4 | **Shade** | Gnome's shadow cools and darkens a patch | W: ants avoid or seek it depending on warmth | Small, subtle | ~✓✓✓ | S |
| C5 | **Manning** | Spend time near the mound without harming them | L: gnome-scent habituation, so less alarm when he is near | Alarm flares on approach shrink over days | ✓✓✓✓ | M (with C1) |

### D. The bloodline: lever G, the beekeeper's tools

| # | Verb | What the player does | How behaviour changes | Feedback | Fit | Cost |
|---|---|---|---|---|---|---|
| D1 | **Split (beekeeper)** | Lift a chamber with its brood and workers into a new box or another corner | G: the daughter colony carries that chamber's genomes. Choose which chamber | Two colonies with diverging habits | ✓✓✓✓ | M |
| D2 | **Requeen from the shelf** | Introduce a shelved specimen or its eggs | G: inject a known-good bloodline. **Acceptance graded by odour** (callows are accepted more readily, as in real ants) | Watch it be accepted or attacked | ✓✓✓✓ | M |
| D3 | **Nuptial net** | At a flight, choose which winged founders to catch and let found nearby | G with founder choice | New colonies spring up in the garden | ✓✓✓✓ | L |
| D4 | **Show bench (pedigree)** | Pick the best forager from the life records and put it on the shelf | G: clone or mutate it; the shelf as a stud book | Lineage names and records | ✓✓~✓ | S (exists) |
| D5 | **Thinning (cull)** | Squash or remove ants doing the wrong thing (raiding your seedlings) | G: direct selection | Harsh. Natural for a gardener controlling a pest, but it reads as punishment | ~✓✓✓ | S |

### E. Social transmission (lever S): the colony teaching itself

| # | Verb | What the player does | How behaviour changes | Feedback | Fit | Cost |
|---|---|---|---|---|---|---|
| E1 | **Teach a teacher** | Heavily train one ant (C3, A2), then return it | S: a laden, confident ant tandem-leads naive nestmates (follower copies the place or odour memory). Lesson spreads like an epidemic | Lesson-spread map: the share of the colony that "knows" mint, over time | ✓✓✓✓ | L |
| E2 | **Nursery imprinting** | Put a scent, food type or plant in the brood chamber | S/L: ants eclosing in the first days imprint that odour as positive (callow imprinting) | **Cohorts:** a "mint vintage" of workers that forage mint all their lives | ✓✓✓✓ | M (initialise memory at eclosion from local odours) |
| E3 | **Trail traditions** | None directly; arises from A1/B1 | S: a well-used road persists because new ants are born into it | Roads outlive the reason for them | ✓✓✓✓ | S |

### F. Hardship as a teacher (drills)

| # | Verb | What the player does | How behaviour changes | Feedback | Fit | Cost |
|---|---|---|---|---|---|---|
| F1 | **Staged predator** | Release a caged or weakened beetle near the mound | G and L: alarm and defence; survivors breed; alarm-place memory | Defence speed improves across drills | ✓✓✓✓ | M |
| F2 | **Rain drill** | Water the surface heavily | G: colonies that dig high chambers or berms survive | Spoil berms around entrances | ✓✓✓✓ | S |
| F3 | **Lean season** | Withhold food | G: frugal lineages survive | Boom-bust smoothing | ✓✓✓✓ | S |
| F4 | **The double bridge** | Build two routes to food | W/S: the colony converges on the shorter one | A visible route choice over time; a puzzle | ✓✓✓✓ | S |

---

## 4. Behaviours a player might train

### Expected

- Forage at a chosen place: A1, B1, C1.
- Avoid a bed or a hazard: B2, F1, and death sites.
- Dig where wanted, and not under the vegetables: B5, B7.
- Relocate the nest: B6.
- Defend chosen plants against beetles or the centipede: B3.
- Sow or disperse chosen seeds: B4.
- Clear the midden and corpses away from where you want them.
- Come when called (stomp or chime): A5.
- Harvest a chosen crop, and leave another alone.
- Prune, or graze back, a weed you want gone (leaf-grazing exists, and some lineages can use standing leaves).

### Surprising and emergent (the story generators)

1. **The gnome gets mobbed.** A colony that trusts gnome-scent swarms the gnome's feet every time he walks by. It is a pet, and it is in the way.
2. **They learn your sowing.** Every time you sow, there is food, so the colony learns gnome-scent plus fresh seed means feed. Your new beds are harvested before they sprout. You must teach them that your seed is not theirs, or sow where they do not trust.
3. **Spoilt colony.** Fed on a timetable, foragers stop foraging and wait at the station. When you go on holiday (stop playing), the colony starves. This is the brief's "fed ants idle" problem, written as a moral.
4. **Crying wolf.** Lay trails to nothing three times and your trails stop working for a generation. Young ants imprinted on a distrustful nest never trust you.
5. **Dialects.** You change mint from good to bad (counter-conditioning). Old foragers keep visiting mint, the new cohort avoids it, and for a while the colony runs two traditions. If memory is shared by trophallaxis, the old guard may re-teach the young.
6. **The cheater bloodline.** Your bodyguard plant pays nectar. One lineage learns to drink without patrolling (as the real Camponotus planatus does), and outbreeds the guards because guarding is costly. Your trees lose their guard while still looking busy.
7. **Hedgerow roads.** Seed-carrying along trails leaves lines of shrubs. Over a long game the garden's plant map is a fossil of the colony's history.
8. **Haunted ground.** The spot where four beetles slaughtered a raiding party carries alarm memory, and nursery-imprinted cohorts inherit the avoidance. A perfectly good patch stays empty for generations. The player can exploit it: graves as fences.
9. **Superstition.** A crumb fell once beside a pebble while an ant circled it. Random reward schedules produce rituals (Skinner's pigeons). The ant's page reports "believes the stone at the east wall is lucky". Cheap, funny and true to learning.
10. **Levee builders.** Colonies drilled by repeated floods pile spoil into berms around entrances. Placed right, the berms protect your seedlings from runoff too, or dam water onto them.
11. **The trusted-scent exploit.** If gnome-scent is positive, the player can lay it on hazards and lure ants to death (or to a rival colony). It is a betrayal with consequences, because trust collapses.
12. **Raiding the rival.** With two colonies in one garden, a lineage learns that stranger-scent plus a brood chamber means food, and starts robbing instead of foraging. You taught them to forage near the border; they found a shortcut.
13. **The lost taste.** A cohort imprinted on a plant you since pulled up wanders searching for it, a whole generation slightly maladapted. It is a visible echo of your earlier garden.
14. **Undermining.** You taught them to dig where wet. Your watering of the tomatoes now pulls the nest under the tomato roots, and the bed collapses into the galleries (falling sand physics).
15. **Following the can.** Because water makes soft digging, the colony learns the watering can's smell or sound means good ground, and trails follow the gnome's watering route.
16. **Reward hacking.** If any reward is a player-side score rather than an ant-side meal, ants game it (shuttling one crumb back and forth across a "delivery line"). **Design rule: every reward must be something the ant itself consumes**, so the natural economy prevents hacking.

---

## 5. What would make it boring, and what keeps it fun

### Boring, in order of danger

1. **The lesson is invisible.** G-lever change takes generations, and nothing on screen says what changed. Niche got called repetitive; Black & White training felt like superstition. *Fix:* every verb needs a same-session sign (traffic thickening, an ant's page line, a cohort colour) plus a long-term one (lineage share).
2. **Obedience collapse.** If lessons become commands, ants become Pikmin and the surprise dies. If lessons are too weak (as the hand-laid trail measures today) the player is a spectator. **The target is a dog, not a robot and not a goldfish:** about 60–80% compliance with visible exceptions.
3. **Chores.** Daily feed and water as upkeep with no new behaviour is the Tamagotchi rot. Tending must produce discovery.
4. **The first verb fails.** If the first thing a player tries (a trail) does nothing, they conclude nothing works. Trust (C1) makes "it doesn't work yet" a stage, not a bug. But the game must *say* that the colony does not trust you yet: a visible sniff-and-turn-away animation, and a line on the page.
5. **One optimal colony.** If one recipe wins, every keeper converges. Keep it varied with competing commissions (guard versus sow versus harvest trade off) and a garden that changes (seasons, plants aging out).
6. **Opaque mis-learning.** Mis-learning is fun only when explained afterwards (see §2, Black & White).

### What keeps it fun past the first hours

- **An ethogram to fill (the field notebook).** A notebook of behaviours your colony has been seen doing, detected by event counters: first tandem run, first berm, first superstition, first cheater lineage, first hedgerow. Discovery rather than unlocks, and it pushes players to set up the conditions for the next entry. This is the Viva Piñata "first sighting reveals the requirement" loop, applied to behaviours.
- **Garden commissions.** Pollinate N flowers via the flitter while keeping ants off them; keep beetles off the vegetable bed for a season; get a seedling of species X established on the far side; move the colony out from under the patio. These are graded (how well), never pass/fail.
- **Seasons and aging plants.** The garden changes, so lessons go stale, and decay plus renewal gives a reason to come back.
- **Multiple colonies with temperaments.** Two or three colonies with different upbringings in one garden, which are compared, conflict and trade territory.
- **The stud book and sharing.** Lineage names, the shelf, and exportable colony seeds. A "Formic Trials" course (see B5 in §6) where shared colonies are run on the same garden. Creatures and Spore both show that sharing outlives the toy.
- **A narrator or journal.** An Empires-style voice, or a dated journal that writes up "Bramblewort colony learned mint means food (day 12)". It turns slow change into a story.

---

## 6. Bold ideas, explained

**B1. Trust is a smell (the gnome's credibility).** The gnome has an odour, and every trail, food drop, carry, fling and flood is stamped with it.
- Each ant keeps a valence for gnome-scent; trophallaxis spreads it, and nursery imprinting passes it to the young.
- A gnome trail's pull is scaled by trust.
- *Why bold:* it converts the engine's measured failure (trails do not pull) into the core progression of taming. It is honest to real ants (odour learning in three trials) and to Rain World's relationship model.
- *Graded:* trust is a continuum, it varies by cohort, and it can be lost.
- *Cost:* M (one odour tag, one valence per ant, one feature on each option).

**B2. The perfumer's garden (plants as words).**
- Each plant species carries an odour tag, and ants learn what each smell means from experience. The player's vocabulary is what they have planted.
- To say "go here", plant mint there. To say "keep out", plant rue.
- Words can be rewritten, which creates dialects between cohorts.
- *Why bold:* it fuses the two halves of the game (gardening and keeping) into one verb set, so teaching is gardening.
- *Cost:* M.

**B3. Schooling by imprinting (vintages).**
- What a callow ant smells in its first days sets its lifelong preferences, as in real ants. The keeper "schools" each generation by what is in the nursery.
- The colony visibly carries cohorts: "the drought vintage", "the mint vintage". It is a whole generational layer without touching the genome.
- When the vintage dies out, the habit dies with it unless renewed.
- *Graded and natural.*
- *Cost:* M (initialise the memory at eclosion).

**B4. Teach a teacher (lessons as epidemics).**
- Train one ant heavily, return it, and watch it tandem-run others. Tandem running is the first documented animal teaching.
- A lesson-spread map shows the share of ants that "know" over time, with the original teacher named.
- If the teacher dies early, the lesson dies.
- *Cost:* L (follow-the-leader behaviour plus memory copying).

**B5. The Formic Trials (sheepdog trials for colonies).**
- A standard garden course: a double bridge, a flood plain, a beetle pen, a scented maze. Shared colony seeds are run on it.
- Scored on graded outcomes (food home, losses, time to converge) and on style (did they find the short bridge?).
- *Why bold:* it gives keepers a public goal and an audience. It is also a free test harness that matches the offline lessons the project already runs.
- *Cost:* L (mode plus sharing format).

**B6. Pay the guard (contracts with plants).**
- Grow nectary plants; the colony that patrols them gets paid and defends your garden against beetles and the centipede.
- The surprising part is cheater lineages exploiting the contract.
- *Why bold:* it is real ecology, it uses predators already being built, and it turns the colony into a garden service with a moral hazard.
- *Cost:* M.

**B7. Haunted ground and superstition.** Two learning side effects, made into features:
- **avoidance of death sites**, which persists through imprinting;
- **superstition**, where random rewards create ritual places.
- Both are reported in plain sentences on the page.
- *Why bold:* it makes the colony feel like it has a past and beliefs, a cheap source of story.
- *Cost:* M (falls out of the valence table, plus a page text).

**B8. The watering can as architect.** Soil wetness steers where the nest grows; a lamp moves the nursery; drying evicts. The nest becomes a garden bed you grow by tending, with no commands at all.
- *Why bold:* it is the most natural verb in the list, it already works on engine features, and every action leaves a mark (galleries, migrations, collapses).
- *Cost:* S.

**B9. Domestication as an emergent gene (biddability).**
- The weight the network puts on its learned-memory features is heritable. A colony the keeper works with drifts toward listening; a neglected or wild one drifts toward ignoring.
- Show it on the colony page as "biddable / wild", measured, not assigned.
- *Why bold:* it unites the owner's two framings (taming from outside, genetics from inside) in one mechanism, the way Belyaev's foxes did.
- *Cost:* S–M once the memory exists (it is just which weights evolve).

---

## 7. Ranked recommendation

1. **Trust is a smell (C1/B1).** Fixes the measured trail failure, gives taming a progression, and produces stories (mobbing, crying wolf, betrayal).
2. **The watering can as architect (B5–B8).** Cheapest, most natural, works on existing features; every action leaves a mark.
3. **Plants as words (B1/B2 in §3).** Teaching through gardening, with dialects.
4. **Nursery imprinting and vintages (E2).** Generational schooling without touching the genome; visible cohorts.
5. **Larder and hunger dial with feeding stations (A1, A3, A6).** The falconer's lever; recasts idling as spoiling.
6. **Pay the guard (B3).** Defence and ecology, plus cheaters.
7. **Ethogram notebook and commissions.** The long-term fun loop; counter-driven, so it also verifies that things fired.
8. **Split and requeen (D1, D2).** The beekeeper's genetic handle, under natural framing.

Also notable: teach a teacher (expensive but magical), Formic Trials (sharing),
and biddability as a gene, which is not a verb but the mechanism that makes
items 1, 3 and 4 heritable.

### The 3 biggest risks to fun

1. **Invisible or slow feedback.** If a lesson's result shows only after generations, or only in a chart, players will not connect cause and effect. Every verb needs a same-minute visible sign *and* a plain-sentence explanation on the page.
2. **The compliance band.** Too obedient and it is Pikmin with no surprise; too weak (as trails measure now) and it is a screensaver. A lesson that does nothing at first is fine only if the game shows it was *noticed but not trusted*.
3. **Chores and lock-in.** Care loops that only maintain (feed, water, repeat) rot into Tamagotchi upkeep, and a single dominant recipe flattens variety. Decay, seasons, competing commissions and multiple colonies are the countermeasures.

### Engine notes for whoever builds it

- **Minimal L-lever form:** per ant, a small table of (odour tag → valence in −1..1), about 6–8 slots.
  - Update on consumption (eat or deliver near tag k: valence up) and on harm or alarm (down).
  - Slow decay; copied in part by trophallaxis; initialised at eclosion from local odours.
  - One new per-option feature: the valence of the strongest learned odour at the target.
  - Cost: tens of bytes per ant, one table read per option. Frame cost should be negligible, but measure it.
- **It must be present in offline training,** or the founder will not listen to it.
- **Odour tags** for plant species, the gnome, stranger colonies and death sites. Death sites partly exist already via corpses and alarm.
- **Rewards must be ant-internal** (food eaten or stored, harm taken), never a player score, so that ants cannot game them.
- **Counters for every emergent behaviour in §4,** so the ethogram can fire and so the owner's "did it fire at all" rule is met.

---

## Sources

Games
- Black & White postmortem (Molyneux, 2001): https://gamedeveloper.com/design/postmortem-lionhead-studios-i-black-white-i- [located by search; AI details from knowledge]
- Salon review of Black & White (2001): https://salon.com/2001/04/10/black_and_white_2
- Creatures series: https://en.wikipedia.org/wiki/Creatures_(video_game_series) ; https://www.technologyreview.com/2014/09/18/171315/a-grand-quest-to-create-virtual-life/ ; https://fanlore.org/wiki/Creatures
- Pikmin: https://www.wikipedia.org/wiki/Pikmin ; https://www.pikminwiki.com/Pikminology
- SimAnt: https://en.wikipedia.org/wiki/SimAnt
- Empires of the Undergrowth: https://gamerofpassion.com/empires-of-the-undergrowth-review/ ; https://wiki.hoodedhorse.com/Empires_of_the_Undergrowth/Quote_-_Formicarium_Challenge
- Viva Piñata: https://en.wikipedia.org/wiki/Viva_Pi%C3%B1ata_(video_game) ; https://djcube.co.uk/rare-review-viva-pinata/ ; https://www.trueachievements.com/game/Viva-Pinata-Trouble-In-Paradise/walkthrough/3
- Oxygen Not Included ranching: https://oxygennotincluded.wiki.gg/wiki/Ranching ; https://oxygennotincluded.wiki.gg/wiki/Grooming_Station ; https://oxygennotincluded.wiki.gg/wiki/Critters
- Niche: https://www.keengamer.com/articles/reviews/niche-a-genetics-survival-game-review/ ; https://www.commonsense.org/education/game/niche-a-genetics-survival-game
- Equilinox: https://www.gamingonlinux.com/2018/11/relaxing-nature-sim-equilinox-is-out-seems-to-be-a-hit-with-users/page=1/
- Terra Nil: https://shacknews.com/article/134777/terra-nil-review-nature-appreciation ; https://gameluster.com/?p=78815
- Rain World: https://en.wikipedia.org/wiki/Rain_World ; https://rainworld.miraheze.org/wiki/Behavior ; https://blog.playstation.com/?p=187776
- Tamagotchi / Nintendogs: https://itch.io/devlog/73412/2-on-virtual-pets-part-1-of-2.amp ; https://www.cubed3.com/games/reviews/nintendo-ds/nintendogs
- RimWorld, Dwarf Fortress, Stardew Valley, Spore, Grounded 2: knowledge only.

Hobbies and animal handling
- Ant keeping: https://www.ant-shack.com/blogs/ant-keeping-beginner-guides/ant-keeping-faqs-for-beginners ; https://www.lesswrong.com/posts/NBqhWBdagH8QfeEfo/keeping-ants-and-spotting-queens ; https://www.formiculture.com/topic/10035-how-to-feed-a-small-colony-in-a-founding-formicarium/
- Beekeeping: https://sfyl.ifas.ufl.edu/agriculture/requeening-a-beehive ; https://biobees.com/library/general_beekeeping/swarming/Swarm_Prev_Control_PM.pdf ; https://easternapiculture.org/wp-content/uploads/2023/01/Weird-Techniques-for-Swarm-Control.pdf
- Flight zone / herding: https://douglas.ksu.edu/agriculture/agent-articles/livestockflightzone.html ; https://www.fbg.org.au/event-wrap-ups/2018/12/4/low-stress-stock-handling-traing
- Falconry: https://ohiofalconry.org/basic-training/ ; https://www.kswo.com/2020/03/31/makenzies-outdoor-adventures-falconry/
- Belyaev foxes: https://www.scientificamerican.com/blog/guest-blog/mans-new-best-friend-a-forgotten-russian-experiment-in-fox-domestication ; https://evolution-outreach.biomedcentral.com/track/pdf/10.1186/s12052-018-0090-x

Ant biology
- Odour learning (Piqueret et al.): https://thesciencebreaker.org/en/breaks/health-physiology/using-ants-to-sniff-out-cancer ; https://www.sciencefocus.com/news/sniffer-ants-detect-smell-cancer
- Tandem running as teaching: https://www.nature.com/articles/439153a ; https://www.bristol.ac.uk/news/2006/879.html ; https://www.scientificamerican.com/article/teacher-ants-show-student/
- Callow imprinting and brood recognition: https://pmc.ncbi.nlm.nih.gov/articles/PMC1199612 ; https://doaj.org/article/6f694e83f7b643d7b6c1f96f3a4f7930 ; https://www.frontiersin.org/articles/10.3389/fnbeh.2018.00191/pdf
- Double bridge: https://arxiv.org/pdf/1908.08007 ; https://staff.washington.edu/paymana/swarm/dorigo99-al.pdf
- Colony personality: https://experts.arizona.edu/en/publications/be-meek-or-be-bold-a-colony-level-behavioural-syndrome-in-ants-2/
- Acacia–ant mutualism: https://www.smithsonianmag.com/blogs/smithsonian-tropical-research-institute/2024/11/21/mutualism-under-pressure-new-research-in-panama-shows-a-plants-ability-to-keep-its-defender-ants-happy/ ; https://repository.royalholloway.ac.uk/items/2195c11d-f1a4-6ef0-edf7-d9a03ea4eac6/1
- Seed dispersal by ants: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4587374/ ; https://en.wikipedia.org/wiki/Harvester_ant

Engine facts: the engine brief; `wiki/ants.md` (seed carrying, midden, the flitter, leaf grazing); `wiki/plants.md` (plants worse to eat, seed provisions).
