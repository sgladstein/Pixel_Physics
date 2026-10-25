# What the 1996 game *Creatures* can teach the ant

*2026-10-25. Research only, no runs. Asked by the owner ("explore the old 1996
video game Creatures"). Read as Steve Grand's *Creatures* (Millennium, later
CyberLife; published by Mindscape, 1996) and its Norns, Grendels and Ettins.
Compared against the shipped mind brain (`src/sim/creature/mind.rs`), the
design of record ([`design/general-brain-design-rev3.md`](../design/general-brain-design-rev3.md))
and the Teacher lane's colony selection
([`teacher-training/colony-selection-log-2026-10-24.md`](../teacher-training/colony-selection-log-2026-10-24.md)).*

**How the sources were read.** This container's network policy blocks the
Creatures wiki, Wikipedia and the fan sites, so nothing below comes from a page
read in full. Facts come from web-search summaries of those pages (listed under
*Sources*, with how much each can be trusted) and, where marked *(recalled)*,
from general knowledge of the game that no search confirmed. No number here
should be quoted as measured.

## Short answer

**Creatures is the closest ancestor of our brain design, and the one big idea
we have not taken from it is learning during an ant's life.** The verb-then-
target split, drives read as senses, one machinery for several species, and
genes that switch on with age are all already in the rev 3 design, which cites
Creatures for two of them. What Creatures had that we do not: **every Norn
learned in its own life, from reward and punishment that its own body made
when a need went down or up**, and its genome carried *instincts*, which were
lessons stored as training data and replayed while it slept, not as weights.

Our ant is the opposite. Its weights are fixed at birth, and everything it
knows was put there by selection or by an offline copy. That is why two of our
open problems look the way they do:

- **Far food.** A trained ant cannot know where food is in *this* world, so in
  the owner's 1024-wide lab box it walks at random and starves (8 seeds, fruit
  200 columns out: 28 alive against the hand-built ant's 129 at full link
  price; `far-food-1024` memory, PR #881). An ant that remembered where it last
  found food, or learned which way paid, would not need selection to have
  guessed the distance.
- **Lessons that erode.** A lesson copied into the weights (crater refusal,
  come-home, dig at the foot) is just more weights, so later mutation and
  selection wear it away, and some lessons cannot be held by the wiring at all
  ([`step05-why-copies-fail`](../teacher-training/step05-why-copies-fail-2026-10-21.md)).
  A Creatures instinct is re-learned by every ant in every life, so it cannot
  be worn away by drift in the weights, only by a mutation in the instinct gene
  itself.

**Ranked ideas** (detail and tests in *What to try*):

1. **A last-food memory** (the body remembers where food was last picked up and
   tells each option how much it points there). The cheapest form of "what you
   learn in a life matters". It is already a *must* in rev 3 and was never
   built; its feature slot is now borrowed by the far-from-home sense. Aimed at
   far food.
2. **One small reward-trained unit per ant**, using a cut-down form of the
   Creatures learning rule, with the learning rate as a gene that starts at 0.
   At rate 0 it is bit-identical, so it can ship on and let selection decide
   whether learning pays. Aimed at far food and at worlds that change.
3. **A "bored" drive** that rises while a fed ant does nothing and falls when it
   works, read as a sense. Aimed at the playtest's fed ants idling on the spoil
   mound (30-60% of the colony). Cheap.
4. **Lessons as instincts** (later, needs 2): the genome carries a lesson as a
   situation, a verb and a reward, and each ant learns it while resting.
5. **The player's verb: reward or punish an ant** (later, needs 2), as Norn
   owners did by tickling and slapping. A verb that delivers, in the lab.

**What not to copy**: rewiring by moving connections (we rejected that kind of
growth before), learned weights passed to children, and reward chemistry that
can drift out of balance. That drift is what broke Creatures 2's Norns after
about an hour of life.

## How Creatures worked

### The brain

- **About 900-1,000 neurons in about 10 lobes** (creatures.wiki *Brain*). The
  Creatures 1 sizes found: perception 112, drive 16, source 40, verb 16, noun
  40, general sense 32, concept 640, decision 13, plus an attention lobe whose
  size was not found.
- **Perception was symbolic.** A Norn did not see pixels: each kind of object
  had a neuron that fired when one was in front of it (Wikipedia, series page).
- **Attention picked the object, the decision lobe picked the verb applied to
  it.** Attention was winner-takes-all, because a Norn attends to one thing at
  a time (discoveralbia). Early Norns had no way to choose what to look at by
  what they needed; a later fix by Chris Double focused attention on things
  that mattered (creatures.wiki *OHSS*).
- **The concept lobe was the memory.** 640 neurons, each the AND of 1-3 inputs,
  so each neuron stood for one combination of situation features. Their links
  to the decision lobe could be positive or negative, which is how a Norn
  stored "do this here" and "never do that here" (creatures.wiki *Concept
  lobe*).

### How it learned

- **Reward strengthens and punishment weakens** the links that were active
  (creatures.wiki *Concept lobe*).
- **Susceptibility is an eligibility trace.** Each link had a susceptibility
  that rose (by 4 per update in Creatures 1) while the link took part in what
  the Norn was doing, and decayed at a rate set by a gene. Only links with
  susceptibility above 0 were changed, by the reinforcement signal times their
  susceptibility (creatures.wiki *Brain*). So reward reached the links that
  produced the act, even if it came a little later.
- **Two weights per link, on two time scales.** A short-term weight did the
  work and moved quickly; a long-term weight was its resting point, moved
  slowly towards it, and the short-term weight relaxed back towards the
  long-term one (creatures.wiki *Brain*). One lucky or unlucky event moves
  behaviour for a while; only repeated experience changes it for good.
- **Unused links moved.** A link whose strength stayed near 0 could let go and
  reattach to a different active input, so the brain kept sampling new
  combinations (creatures.wiki *Concept lobe*).
- **Language too.** The same machinery learned a small verb-object vocabulary
  from the player (Grand and Cliff 1998, abstract).

### The body: needs, chemistry and reward

- **Every need was a chemical**: hunger, sleepiness, loneliness, sex drive and
  others, each its own concentration (a player's account; the Times Higher
  Education piece).
- **Reward was a need going down; punishment was a need going up** (Wikipedia,
  series page: "drives which give punishment when they are raised, and reward
  when they are lowered"). In Creatures 1's hunger this was literal chemistry:
  `Hunger+ -> Hunger + Punishment` and `Hunger- + Hunger -> Reward`, each a
  reaction with a half-life (discoveralbia's breakdown of the genome).
- **Chemistry was genes**: emitters released chemicals when conditions held,
  receptors watched levels and acted on the brain or body, reactions turned
  chemicals into others, and one gene held every chemical's half-life
  (creatures.wiki *Biochemistry*). A 1996 review lists digestion, an energy
  metabolism, an immune system and hormones.
- **What an event meant was a gene too.** *Stimulus* genes set which chemicals a
  Norn received when something happened to it, from being patted to sneezing
  (discoveralbia). So evolution could change what a Norn found rewarding, not
  only how it acted.

### Instincts: lessons stored in the genome

- **An instinct gene named a situation (up to three input neurons), and the
  Norn replayed it while asleep**: the gene lit those neurons, the brain picked
  a response, and a reinforcement signal followed, as in a dream. Players were
  told not to wake a deeply sleeping Norn, because its dreams were part of its
  mental development (Creatures Caves forum; creatures.wiki *Tips.txt*; the
  official genetics tutorial).
- **Instincts were nudges, not rules**: they made an act more likely than a
  random one when the Norn did not otherwise know what to do (official
  genetics tutorial).
- **How instincts worked changed between games** (Creatures 1-2 against
  Creatures 3 / Docking Station), so details vary by version.

### The genome

- **About 810 genes in a typical Norn**, of about 16-19 types in four groups
  (Genetics Kit manual; an older Creatures 2 page).
- **Every gene had a header**: an id, a generation number, a **switch-on age**
  (one of seven life stages, embryo to senile), and **flags: mutable,
  duplicable, deletable**, male-only, female-only, not expressed. A gene with
  no mutation flag set was compulsory and never mutated. Later versions added
  a per-gene mutation weighting (creatures.wiki *GEN files*; Genetics Kit
  manual).
- **Genes could be duplicated when two genomes crossed, and a copy that then
  mutated became a new gene** (discoveralbia), so a genome could grow.
- **The genome was variable-length and specified both the network's layout and
  the chemistry**, so evolution could reach both (Grand, Cliff and Malhotra's
  1996 report, as quoted in search results).
- **Species were genomes.** Grendels and Ettins ran on the same machinery with
  different genes; a Norn-Grendel hybrid carried genes from both (fan sources;
  weakly supported).

### What went wrong

- **"One Hour Stupidity Syndrome"** (Creatures 2): Norns' intelligence all but
  stopped working after about an hour of life. Lis Morris traced the root cause
  to an imbalance of the reward and punishment chemicals: rewards ended up
  handed out almost at random, so learning was noise (creatures.wiki *OHSS*).
- **Norns starved beside food.** Players report Norns that forgot how to eat
  later in life, ignored food when other Norns were near, or starved "while it
  is obvious to us that the cheese is only a few inches away". Players
  suspected the punishment from very high drives, and noted that anything that
  dropped a drive fast paid more reward (alt.games.creatures; Steam
  discussions).
- **The fixes were more brain**: community breeds (Canny Norns, three extra
  lobes; Nova Subterra, four) ate better and forgot less, and CyberLife's later
  eggs changed the reward and punishment genes. The extra-lobe breeds could not
  safely cross with other Norns, because their brain layouts no longer lined up
  (alt.games.creatures FAQ).
- **Evolution inside the game was weak.** Players ran "wolfling runs" (a
  population left alone with no player input), but populations were a dozen or
  so, runs often ended with one or two survivors, and one 1999 post called it
  a good breeding sim but not an evolution sim. The most common mutation
  players met was fast ageing (alt.games.creatures; creatures.wiki).

## Creatures against our ant, side by side

| | Creatures (Norn) | Our ant (mind brain, `mind.rs`) | Gap worth closing? |
|---|---|---|---|
| Choosing | attention picks the object, then the decision lobe picks the verb | the job head picks the verb, then one shared scorer picks the target (rev 3, citing Creatures) | no, already taken |
| Perception | one neuron per object kind in front of it | ~45 features per option, 22 context senses | no |
| Needs | chemicals with genetic half-lives; reward = need down | energy, crop and their change since the last decision are senses; no reward signal | **yes**, if learning comes in |
| Learning within a life | yes: reward and punishment, eligibility, two-speed weights | **none**: weights fixed at birth | **yes, the main lesson** |
| Memory | concept lobe of 640 AND-cells, learned | 12 memory units with genetic forgetting rates; body home vector | partly: no last-food memory |
| Innate behaviour | instinct genes replayed in sleep, plus initial wiring | the genome's weights, from selection and offline copies | **yes**, as a later step |
| Genome | ~810 typed genes; switch-on age; mutable / duplicable / deletable flags | one weight vector with named slots; new slots enter at 0; `noprune` and `protect` lists live in the harness, not the genome | small: protected lesson slots, as the Fable review recommended, are the same idea as gene flags |
| Selection | players breed what they like; small wolfling runs | in-run selection of parents plus colony selection, 24-36 ants a colony, 32-64 colonies a batch, a common garden as judge | ours is far stronger; do not lean on in-world evolution the way Creatures did |
| Species | one machinery, different genomes | one brain design, bodies declare senses and verbs (rev 3) | no, already taken |
| The player's verb | tickle (reward) and slap (punish) *(recalled)*; teach words | paint food and soil, drop colonies, alarm, fling, lamps (all logged) | **yes**, once ants can learn |

## What to try

Each idea says what it is for, what it costs, how to know it worked, and how it
could fail. Numbers to beat are the Teacher lane's current controls on the same
build, paired by seed, never a remembered figure.

### 1. Remember where the food was (last-food memory)

**What it does.** The body keeps one more vector, like the home vector: the
offset to where this ant last picked up food, updated as it walks and reset at
each pickup. Every walk option gets a feature for how much it points there,
scaled by distance on a log scale (graded, no cap, as the owner prefers for the
far sense). This is the rev 3 design's *must* "last-food vector" (*Memory*,
*Body memory*), never built: `options::f::LAST_FOOD` (feature 40) is listed as
missing, and `PP_HOMEVEC` has since borrowed it for the far-from-home sense, so
it needs a new feature slot.

**Why Creatures says so.** A Norn's most useful learning was "this place or
thing paid". Real ants learn feeder locations and routes, the one thing rev 3
already says they learn (*Learning during life*). It is the smallest step into
in-life learning: no plastic weights, just a memory the brain can choose to
trust, and a new slot enters at weight 0, so nothing changes until selection
puts weight on it.

**Cost.** Two numbers per ant, one feature per option. Negligible frame cost.
One more slot for the link price to bill if selection wires it.

**How to know.** First a positive control: one fixed food patch far from the
nest, and the time from leaving the nest to reaching the food on an ant's
second trip against its first. A brain that uses the memory must be faster on
trip 2; a brain with the feature cut must not. Then the decision: the lab case
with `lab_far200_fruit` and the distance ramp, ctl against memory-on from the
current pool, paired by seed, plus the common garden so digging is checked too.

**How it fails.** A memory of one patch sends every ant back to an emptied
patch. Mitigation is in the brain, not a rule: the ant also senses scent B and
how long ago the memory was set (age of the memory as a second feature), and
can learn to ignore a stale one.

### 2. One reward-trained unit per ant (Creatures' rule, cut down)

**What it does.** Each ant gets one extra scorer unit whose input weights it
learns during its own life, starting from 0 at birth. The rule is the Creatures
one with the chemistry stripped out:

- **Eligibility (Creatures' susceptibility).** When the ant picks an option,
  each input weight's trace moves towards that option's feature value; traces
  decay each decision at a genetic rate.
- **Reward is a need going down.** Energy gained by eating, and food put down
  at home, pay +; a fall in energy below a level, or damage, pays -. How much
  each source pays is a gene (Creatures' stimulus genes), so evolution decides
  what is worth learning from.
- **Two-speed weights.** Each learned weight moves by `rate x reward x trace`,
  is clamped, and relaxes back towards 0 at a genetic rate, so one meal moves
  behaviour for a while and only a habit lasts.
- **Not inherited.** Children start from 0, as in Creatures. Selection keeps
  the genes that make learning pay, never the learned weights (the Baldwin
  effect), which fits the owner's "the pool of genomes is the product".

**Why it can ship on.** The founder's learning-rate gene is 0, so the unit
stays at 0 and every choice is bit-identical to today. Mutation can turn the
rate up, and selection then says whether learning pays. Rev 3's growth rule
("new units enter at zero outgoing weight") already covers it, and the project
rule is features on unless there is a measured harm.

**Why Creatures says so.** It is the one part of Creatures our design left out
("Non-goals: learning during an ant's life (left for later)"). Evolved
reward-driven plasticity is the standard answer when the right choice changes
between lives: in simulated bees, evolved learning rules responded quickly to
changing rewards and produced real bees' risk aversion (Niv et al. 2002), and
evolved plasticity let foragers follow reward locations that moved (Soltoggio
2008). Our worlds vary exactly this way: food distance in training against the
lab box, plants against piles, painted food.

**Cost.** About 45 learned weights and 45 traces per ant, about 90
multiply-adds per decision on top of the scorer. Small against the ~1,900 links
billed today. Whether learned weights are billed by the link price is a design
choice to make and say; billing them as one link is the simple start.
Deterministic as long as it uses only add, multiply and the existing squash.

**How to know.** The positive control has to come first, because without it a
null result says nothing: a **moving-food arena**, where food sits on one side
for half the run and then the other, the side drawn per world. A fixed brain
cannot know which side comes second; a learner should switch. Score deliveries
in the second half, rate gene forced on against forced 0, paired by world.
Only then the real cases: the lab far-food case and the garden.

**How it fails, and the Creatures warnings to test for.**

- *One Hour Stupidity*: learning that works at first and degrades with age.
  Run at full game length (200k frames or more), and plot learned-weight size
  and success rate by ant age. A rising weight size with falling success is
  the signature.
- *Starving beside food*: learning that unlearns eating. The eat and pick-up
  paths stay in the genetic weights, and the learned unit is an offset on top,
  never a replacement. Check the eat rate by age.
- *Reward from nothing*: a gene that pays reward for an event the ant can
  repeat at will (Creatures' "anything that drops a drive fast pays more").
  Pay reward only on real energy or delivered food, and keep selection on
  colony outcomes, which a self-rewarding ant cannot game.

### 3. A "bored" drive for fed ants that do nothing

**What it does.** A body-kept number that rises while the ant is fed and not
working (no dig, carry, feed or pick-up) and falls each time it works, with its
rise and fall rates as genes, read as one context sense. It enters at weight 0.

**Why Creatures says so.** Norns had boredom as a drive, and acting on things
lowered it. The playtest found 30-60% of the colony sitting fed and idle on the
spoil mound ([`far64` playtest](../playtest/far64-lab-2026-10-24/README.md)).
Today nothing tells an ant how long it has been idle: the mind's context has
`SINCE_NEST` and `AGE` but no stillness, and a memory unit could in principle
build one, but none has.

**Cost.** One number per ant, one context slot. Negligible.

**How to know.** The share of fed ants idle above ground at fixed times, and
the nest size, in the garden and the lab box, ctl against sense-on from the
current pool. **A null result is informative**: if selection does not use the
sense, idling is not costing the score anything, and the fix belongs in the
score (the nest gate), not the senses. Without learning (idea 2) a drive is
just a sense; it helps only if selection pays for leaving the mound.

### 4. Lessons as instincts (later; needs idea 2)

**What it does.** A lesson is stored in the genome as a short list of
instincts, each a situation (2-3 senses and their values), a verb and a reward
sign and strength. When the ant rests (or at birth), it replays each instinct
through the idea-2 learning rule, as a Norn dreamed. The instinct genes carry
their own flags, as Creatures' gene headers did: whether mutation may change
them and whether it may delete them.

**Why.** Copied lessons today are weights, and they erode: crater refusal,
the come-home copy and the nest score all wear down over rounds (colony
selection log, "nest erosion"). An instinct is re-learned by every ant each
life, so drift in the scorer weights cannot erase it; only a mutation of the
instinct gene can, and its flag can forbid that. The Fable review's preferred
answer was protected lesson slots; Creatures' gene flags are a working example
of the same idea, carried in the genome rather than the harness.

**Risk.** Creatures' instincts were only nudges. Ours need to be strong enough
to carry a dig lesson, and the copy studies showed the scorer's wiring is
sometimes the limit, which replaying through a learning rule does not fix.
Research step, not the next batch.

### 5. The player's verb: reward or punish an ant (later; needs idea 2)

Norn owners taught by reward and punishment by hand. With idea 2 in place, the
lab could offer the same: click an ant to reward it (it learns towards what it
just did) or punish it. That is a verb that delivers something visible, in the
sense `CLAUDE.md` asks for, and a way for the player to shape a colony without
painting the world. Like every lab verb it must log itself
(`world.log_player_action`). This belongs to the lab interface, after idea 2
shows ants can learn at all.

## Already taken from Creatures, so not to redo

- **The verb, then its target**: rev 3 *How it decides* cites Creatures.
- **Drives as senses read by the job head**: rev 3 *Jobs and switching* cites
  Creatures, Polyworld and the Bibites.
- **One machinery, many species**: the body/brain contract.
- **Genes that switch on with age**: the mind reads `AGE`, and rev 3's life
  stages are bodies; a switch-on age per gene is not needed while one stage
  exists.
- **Growth by adding**: new senses, verbs and units enter at zero, which is
  Creatures' "a duplicated gene becomes a new gene" without the crossover
  hazards that broke the extra-lobe breeds.

## Not to copy

- **Links that let go and reattach elsewhere.** It is rewiring, which this
  project rejected for the brain (`creature-direction.md` D4: unreadable, slow
  to start), and rev 3 keeps a fixed layout that grows only by zero-entry.
- **Learned weights passed to children.** Creatures did not, and neither
  should we: it would turn the pool into whatever one lucky ant learned.
- **Free-running reward chemistry.** Creatures 2's collapse came from reward
  and punishment drifting out of balance. Any reward in idea 2 is tied to real
  energy and delivered food, both signs bounded, and tested at full game
  length.
- **Leaning on evolution inside the world.** Creatures' populations were too
  small for it, and its most common visible mutation was fast ageing. Our
  colony selection is the strong part; in-life learning is a supplement to it.

## Sources

Read through search summaries only (the pages are blocked from this
container). Trust: **paper** = the developers' own publication; **wiki** =
the community wiki, generally careful but not official; **fan** = a forum or
blog post.

- Grand, S., Cliff, D. and Malhotra, A. (1997). *Creatures: Artificial Life
  Autonomous Software Agents for Home Entertainment.* Agents '97, 22-29.
  [dblp](https://dblp.uni-trier.de/pid/84/3426.html). **paper** (abstract only)
- Grand, S. and Cliff, D. (1998). *Creatures: Entertainment Software Agents
  with Artificial Life.* Autonomous Agents and Multi-Agent Systems 1(1),
  39-57. [record](https://www.datalearner.com/academic/journal-papers/1387-2532/volumes-and-issues/584/paper-detail/4211). **paper** (abstract only)
- [Creatures (video game series), Wikipedia](https://en.wikipedia.org/wiki/Creatures_(video_game_series)): drives, reward and punishment, object neurons, wolfling runs.
- creatures.wiki: [Brain](https://creatures.wiki/Brain) (susceptibility, short- and long-term weights, lobe sizes), [Concept lobe](https://creatures.wiki/Concept_lobe), [Decision lobe](https://creatures.wiki/Decision_lobe), [Biochemistry](https://creatures.wiki/Biochemistry), [GEN files](https://creatures.wiki/GEN_files), [Tips.txt (C2)](https://creatures.wiki/Tips.txt_(C2)), [OHSS](https://creatures.fandom.com/wiki/OHSS). **wiki**
- [Creatures Genetics Kit manual (Steam)](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1838430/manuals/Genetics_Kit_Manual.pdf): gene types, switch-on stages, compulsory genes. **official**
- discoveralbia: [How hunger works in Creatures 1](https://discoveralbia.com/2015/08/how-hunger-works-in-creatures-1.html), [Visualizing the Creatures 1 brain](https://discoveralbia.com/2016/07/visualizing-the-creatures-1-brain.html), [Duplicating genes](https://discoveralbia.com/2016/05/genetics-lesson-duplicating-genes.html), [Unexpected Creatures biochemistry](https://discoveralbia.com/2011/07/unexpected-creatures-biochemistry.html). **fan**, careful genome readings
- [Creatures Caves forum thread on instincts](http://creaturescaves.com/forum.php?thread=3664&view=12) and the [official genetics tutorial](http://www.creaturesvillage.com/creatures1/library/science/gen_tut/sci_genkit_tut5.htm): instincts in sleep. **fan** / **official**
- alt.games.creatures: [Getting Norns to eat and sleep (FAQ)](https://groups.google.com/g/alt.games.creatures/c/qsQfTy-x_K0), [The problem with Norns](https://groups.google.com/g/alt.games.creatures/c/RGqQ5gdPPG4/m/Cy3VNFXIbisJ); [Steam: teaching Norns to feed themselves](https://steamcommunity.com/app/1818340/discussions/0/6324828960567351459/). **fan**
- [Review of Creatures (1996)](https://humanist.de/erik/Review_of_Creatures/creature.htm); [Nine hours to live in Eden, THE](https://www.timeshighereducation.com/news/nine-hours-to-live-in-eden/109259.article). Press.
- Niv, Y., Joel, D., Meilijson, I. and Ruppin, E. (2002). *Evolution of
  reinforcement learning in uncertain environments: a simple explanation for
  complex foraging behaviors.* Adaptive Behavior.
  [page](https://www.princeton.edu/~yael/AdaptiveBehavior2002.htm).
- Soltoggio, A. (2008). *Neural plasticity and minimal topologies for
  reward-based learning.* [repository](https://dspace.lboro.ac.uk/2134/17042).
