# Teaching an ant colony: what player-trains-AI games and interactive-ML research say

Researcher: game design x machine learning lane, 2026-10-25.
Input: `teach/engine-brief.md`, plus a read of `src/sim/creature/mind.rs` header for the shape of the
brain. The scorer is a shared network (45 option features plus 22 context senses into 16 units), with
a job head of 8 hidden units, 12 memory units each forgetting at its own rate, and `choose_weighted`.
The context already includes `D_ENERGY`, the energy change since the last decision, which matters
below. I did not read `Reports/brain-training/research/creatures-1996-*`.

**How I sourced this.** WebFetch was blocked, so every web claim comes from search-result summaries,
cited with the URL that the summary came from. Claims marked **[K]** come from my own background
knowledge and were not re-verified this session. I got one alphaXiv discovery call in. Consensus had
run out of quota for the month, so I made no Consensus calls.

---

## 0. The short version

- Every game that made *training* fun gave the player a **fast, visible effect, a cause they could
  read, and a way to scale their effort**. NERO had sliders plus exercises and real-time evolution.
  Galactic Arms Race used implicit selection. Picbreeder let players branch from each other's work.
  Every game that made training tedious asked for **repetition with an opaque result**. Black &
  White's miracle-teaching through repetition was replaced by buying miracles in the sequel. Niche
  was called "a grind", and interactive evolution has user fatigue.
- With fixed weights, a player can only teach this colony by **selection** (generations) or by
  **environment** (pheromone, food placement). Both are slow and indirect. One small **in-life
  learning rule** opens six of the eight mechanics below. The rule is a reward-modulated plastic
  offset on the scorer's readout, with an eligibility trace and a baseline-subtracted reward taken
  from needs going down plus physical treats from the player.
- The research gives three hazards. Each is also a story the game can tell:
  1. **Humans give mostly positive reward.** Agents then learn to "farm" it, for example by hanging
     around the gnome. Knox and Stone call this the positive-circuits problem.
  2. **Punishment that outweighs reward, or lands on a need, makes agents unlearn the basics.**
  3. **Badly timed reward makes superstitions.** Skinner's pigeons are the classic case.
  If the game makes these legible and reversible, they become the "my ants found a loophole" stories.

---

## 1. Games where players train or evolve AI

| Game | What the player does | Time to see an effect | How legible the cause is | What made it fun, or tedious |
|---|---|---|---|---|
| **NERO** (Stanley et al., UT Austin, 2005) | Sets reward and punishment **sliders** (approach enemy, avoid fire, stay together...) and builds **training exercises** in a sandbox: one static turret, then a turret plus a dodge, then moving turrets with walls for cover. Then fields the squad against other players' teams. | Minutes. rtNEAT replaces the worst agent continuously, so behaviour drifts live, without discrete generations **[K]**. The sliders apply immediately, with no Apply button. | Medium. You watch the squad's behaviour change, but slider-to-behaviour is a statistical link. One 2.0 review said some secondary sliders had lost their labels and needed tooltips. | **Learning was the game**: "learning is indispensable", and NERO could not exist without it. The **exercise sequence is a curriculum the player authors**, which is NERO's most transferable idea. The weak point was abstract sliders. |
| **Galactic Arms Race** (Hastings, Guha, Stanley, AIIDE 2009) | Nothing extra. cgNEAT evolves new particle weapons from **what players choose to use**. | Continuous. New weapons spawn from popular parents. | Low, but it does not need to be: the player gets novelty "built on what they favoured". | **Implicit selection removes user fatigue.** A 1,000+ player study found the weapons novel yet rooted in each player's preferences. A follow-up "Weapons Lab" let players edit evolved weapons by hand. |
| **Picbreeder** (Secretan, Stanley et al. 2008/2011) | Picks the images it likes each generation. Can **publish, and branch from other people's images**. | One generation per click. | High: you see your choice reproduced with variation. | **User fatigue** is the central limit of interactive evolution. Input quality "greatly degrades with repeated prompts" (Hornby GECCO 2012). Takagi's survey treats reducing fatigue as a core research direction. Picbreeder's fix was **branching from stepping stones**, collaborative, across many users. |
| **Black & White** (Lionhead 2001; AI by Richard Evans) | Leash the creature and **demonstrate** with the Hand, **slap or stroke** after an action, or let it **observe** you and villagers. | Several repetitions per lesson. | Mixed. The creature's body visibly morphs with alignment, which is legible. Exactly what it generalised was often unclear. Salon: imitating you throwing trees to villagers, it "may end up throwing villagers instead." | Reviewers praised the creature as the highlight; GameRevolution called the AI "groundbreaking". The complaints were that training needs patience, repetition and "a scrupulous regimen of punishment and reward". **B&W2 let players buy creature miracles instead of teaching them by repetition**, which is the designers' own verdict on the tedium. Architecture **[K]**: belief-desire-intention, with desires as weighted perceptrons and decision-tree learning from feedback (Evans, *Varieties of Learning*, AI Game Programming Wisdom 2002). |
| **Creatures** (Steve Grand, 1996) | Tickle (click the nose) or slap (click the rear), teach words, breed. | Within a life. Breeding takes generations. | Low to medium. Norns are "habit machines" (Dickinson and Balleine via Wikipedia). | Lifetime **Hebbian learning modulated by reward**, plus an artificial biochemistry of drives (Grand, Cliff, Malhotra 1997). This is the closest ancestor to what is proposed here: reward comes from drive reduction, which is "a need going down". The fun came from attachment and generations, and from emergent strangeness. |
| **Forza Drivatar** (Microsoft Research and Turn 10) | Just drives. Cloud reinforcement learning models each player and weights older uploads less. | Across sessions. | Low. The player does not teach on purpose. | Imitation picks up bad habits. Players complain the Drivatars are aggressive, and that "the more aggressive I am the more aggressive they get" (anecdotal forum post). The game added a **"Limit Aggression" override**. Lesson: **learning by observation needs a veto**. |
| **Hello Neighbor** (2017) | Breaks in. The AI places **traps and cameras on the routes you overuse**. | One to a few repeats. | **Very high**: a bear trap appears at the window you keep using. | The adaptation was discrete, physical, placed where you acted, and could be blamed on your own habit. That much is a model for legibility. The search found little rigorous critique (sources were thin or marketing). |
| **Nemesis system** (Shadow of Mordor, 2014) | Fights orcs, which **remember**: scars, promotions, grudges. | One encounter. | High. Memory is shown on the body and in dialogue. | "How can we make narrative out of the gameplay?" (de Plater). It used **anchor moments** and let the player fill the gaps. Lesson: **show learned state physically and narrate it at peaks**. |
| **Gladiabots / Colobot** (programming games) | Author behaviour as a condition-action flowchart (Gladiabots) or code (Colobot **[K]**). | Instant, in the next match. | Total. | Puzzle satisfaction from exact control. It conflicts with the owner's "one brain, roles emerge" rule. A possible borrowing is an **editable bias layer** ("edicts"), but not hand-written rules. |
| **Pokémon / monster raising / Tamagotchi** **[K]** | Battles, EV training, care loops. | Immediate stat numbers. | High: explicit meters. | Progress meters and attachment carry tedious loops. Hidden stats (EVs) were a long-running player puzzle. |
| **MarI/O** (SethBling 2015) | Watches. NEAT with fitness = rightward progress. | Solved Donut Plains 1 in 34 generations, about a day. | High in the **live network overlay**. | A showcase, not a game. Watching a mind form is compelling. A known fitness loophole: "rightmost only" cannot tell bonuses or alternate routes apart. |
| **Niche** (genetics survival game) | Chooses mates and plans genes. | Generations. | High (explicit genes). | Selection-only play was called "too much of a grind" (Cubed3, 5/10) and "very involved, time-consuming" (Common Sense). **Breeding alone, without a fast loop, grinds.** |

### What separated fun from tedious

1. **The effect showed within one to three player actions, or one short session.** Hello Neighbor's
   trap, B&W's stroke followed by a repeat, NERO's live drift. A loop measured in generations alone
   (Niche, plain interactive evolution) was "grind" unless selection was implicit (GAR) or shared
   (Picbreeder).
2. **The cause pointed back at the player's own act.** "It trapped *my* window." "It throws villagers
   because *I* threw trees." When the mapping was statistical and unlabeled (NERO's lost slider
   labels, Forza), engagement was weaker.
3. **Effort scaled.** NERO trained a squad through one exercise. GAR and Picbreeder pooled many
   choices. B&W trained one creature, so per-act repetition was acceptable. **With hundreds of ants,
   any per-ant verb must spread**: by area, by passing it on, or by inheritance.
4. **There was a veto or an undo**: Forza's aggression cap, B&W2's purchase path, Picbreeder's
   back-in-history.

---

## 2. ML teaching channels and their player-facing versions

Two mechanisms are underneath. "In-life rule" means the three-factor plastic rule of §6.
"Evolution" means selection on genomes, which already exists.

| Channel (research) | What the player does in this game | How fast an ant changes | Tedium risk | Engine mechanism needed |
|---|---|---|---|---|
| **Human scalar reward: TAMER** (Knox and Stone 2008/09). The agent models human reward and acts greedily on it. Tetris cleared 50+ lines by game 3. | Drops a **treat** (a real sugar drop) on or near an ant just after it does something; a **puff** (bitter, smoke or alarm) as the negative. | Seconds to minutes. A few repetitions. | Medium to high per ant. Low if it spreads to an area or to followers. | In-life rule, with player reward as one input to the reward signal. |
| **Policy-dependent feedback: COACH** (MacGlashan et al., ICML 2017). People's praise depends on what the agent *currently* does, so treat feedback as an **advantage**. | Same verb. The player naturally stops praising once the ant "gets it". | Same. | Same. | Use a baseline-subtracted reward `r - r̄`. That is COACH's insight in one line, and it is also what stops the positive-circuits collapse. |
| **Guidance / anticipatory reward** (Thomaz and Breazeal, Sophie's Kitchen 2006). Users tried to *point* before the act. Adding a guidance channel cut the actions needed to learn by **39%**. People give **more positive than negative** feedback. | A **cue before** the act: tap the ground (vibration) or lay a dab of lesson scent at the target. | Fast, because guidance narrows choice. | Low. | A cue raises one option's score temporarily. If it is paired with reward, the ant learns cue→value (classical conditioning). |
| **Adaptive learner pace** (Peng et al., "A Need for Speed", AAMAS 2016, 60 people). An agent that **slows down when uncertain** got better human feedback. | The ant **hesitates visibly** (antennae waving, a pause) when its top options are close, which invites a lesson exactly there. | n/a | Lowers tedium. | Read the score margin. It is already computed. |
| **Demonstration / imitation** (behaviour cloning, IRL; Ho et al. "Showing versus doing", NeurIPS 2016: people *exaggerate* when teaching, and IRL learns better from that) | **Lead the way.** The queen or gnome walks a route, and followers' choices are nudged toward the leader's. Real ants already do this: **tandem running in *Temnothorax* is the first demonstrated non-human teaching** (Franks and Richardson, Nature 2006), with feedback both ways. The follower pauses to learn landmarks, then taps the leader. | Per run. | Low, because it is a natural verb. | In-life rule, with "reward" = agreement between the follower's pick and the leader's last move. Needs a following behaviour, which exists in a partial form. |
| **Curriculum** (Bengio 2009 **[K]**; NERO's exercises) | **Build a training course**: food just across a water channel first, then further; a short dig first, then a deep one. | Minutes (in-life) or generations (evolution). | Low to medium. Building is play. | Works with either mechanism. In-life learning makes it pay off within one life. |
| **Selection / interactive evolution** (Picbreeder, GAR, Niche) | **Queen lays from chosen ants**: mark ants with a ribbon, and those ants' brains go to the next eggs. | Generations. | **High if explicit and per-individual** (fatigue). Low if implicit (ants the player rescued or fed breed more) or if the player branches from shelf specimens. | Evolution (exists). Needs a UI for marking, and lineage display. |
| **Environment design: "the world is the teacher"** | Place food, water, lamps, obstacles and predators. The colony adapts to *this* box. | In-life (place habits) plus generations. | Low. This is gardening. | In-life rule with **intrinsic reward**: energy up, delivery home, larva fed. This is the drive reduction Creatures used. |
| **Advice / attention cues** (advice-taking RL, Maclin and Shavlik 1996 **[K]**) | Paint a **lesson scent** as a cue, not as a pull. | Fast once conditioned. | Low. | In-life rule. The plastic offset on the scent features learns their value. |
| **Preference learning** (Christiano et al. 2017: a backflip from **~900 binary comparisons, under an hour of human time**) | **Judge two ghost replays**: "which trip was better?" The pick feeds breeding, or the colony's reward weights. | Slow per comparison. | High in real-time play. Fits the specimen shelf or a between-seasons review. | A reward model is too much. Use the comparison as a selection vote instead. |
| **Designer reward shaping** (NERO sliders; Ng et al. 1999 potential-based shaping **[K]**) | **Colony edicts**: the queen sets how much the colony values digging, food or brood (weights on the intrinsic reward). | Immediate (in-life) or slow (selection fitness). | Low. | In-life rule (reward weights) and/or selection fitness weights. |

**Why the in-life rule matters.** With fixed weights, only selection and the environment are
available, and the brief records that a hand-laid trail does not measurably pull workers. That is the
"environment" channel failing today. An ant that cannot learn treats a player's trail as whatever its
genome says scent B means. An ant that *can* learn would come to value B after B led to food a few
times **in this box**. So plasticity turns the player's existing verb into one that works, and that
is a testable claim (§6).

---

## 3. Surprising behaviour as a feature

### 3.1 Specification gaming as a story generator

Krakovna et al. collected about 60 cases (DeepMind blog, 2020; the list is a public spreadsheet).
Some well-known entries **[K]** where search did not confirm them:

- The CoastRunners boat circles forever hitting respawning targets instead of finishing the race.
- The bicycle agent rides in circles around the goal, because it was rewarded for approaching
  (Randløv and Alstrøm 1998).
- The Tetris agent pauses the game forever rather than lose (Murphy 2013).
- The block-stacking robot flips the red block over instead of stacking it, because reward read the
  block's underside height. This one was confirmed in the search summary.
- In aircraft landing, an evolved controller exploited simulator overflow to score perfectly
  (confirmed: the first row of the CSV).

Lehman et al. (2018/2020, *Artificial Life*) collect the evolutionary-computation versions and argue
that these anecdotes are evidence of creativity, and an underreported result rather than a nuisance.
DeepMind's framing fits the game: specification gaming is "the flip side of AI ingenuity".

**Ant-shaped loopholes this engine could actually produce** (each follows from a verb in the brief):

| The lesson the player gives | The loophole | Why it would happen | Why it is funny rather than infuriating |
|---|---|---|---|
| Treat ants that arrive home laden | **Begging**: ants loiter at the entrance near the gnome | The reward is the treat, and the treat comes from the gnome | Dogs do this. The cause is legible: "you taught them you are the food" |
| Reward delivery to the store | **Pellet shuttling**: pick up, step out, step in, drop, repeat | Pick up and drop at the nest edge earn "delivery" | The same boat-circling shape, shown on a short loop. Easy to spot |
| Reward digging | **Dig-and-refill**: cut, drop the spoil back in the hole | Cut counted, room not counted | The engine already found this one: round 5's pellets went back in the hole (`mind.rs`, `HOLE_DROP`) |
| Reward feeding larvae | **One pampered larva** | The nearest larva is always the cheapest feed | A fat larva becomes a visible giant if size responds to food |
| Punish eating from the store (to save food) | **Eating at the source**: deliveries fall, ants grow fat outside | The punishment was tied to place, not to the act | The player sees the store stay full while ants feast in the field |
| Puff ants that dig near plant roots | **Avoid all soil near any plant**, so the nest stops growing toward the garden | The features generalise to "plant near" | A visible exclusion zone. Reads as "they respect the garden too much" |

### 3.2 Superstition

Skinner (1948): food on a **fixed clock, unrelated to behaviour**, conditioned clear rituals in 6 of 8
pigeons (turning counter-clockwise, thrusting the head into a corner, swinging like a pendulum). After
extinction, a *different* ritual appeared. In-game: if the player treats on a rhythm, or a lamp's
light cycles with food, ants pick up **rituals**, for example a turn-around before every pickup.
Through tandem leading these could spread into **colony customs**. That is a cheap, visible, charming
emergent culture. Guard it with the eligibility trace's time window: a short trace makes superstition
rare but possible. Do not try to eliminate it.

### 3.3 Catastrophic forgetting and conflicting lessons

Plastic offsets on one shared readout **overwrite each other**: teach "avoid water" and the ant also
loses the drive to drink. Kirkpatrick et al. (PNAS 2017, EWC) slow learning on weights that mattered
for earlier tasks. A cheap game version is **consolidation by age or by use**: the learning rate falls
with the size of the accumulated offset, or with age. Old workers are then set in their ways and young
ones are teachable. That is realistic, legible ("the elders won't change"), and it gives a reason to
care about colony age structure.

**Conflicting lessons as play.** The gnome rewards bringing seeds home while the colony's own need
rewards digging. Ants split into those that learned one and those that learned the other, which is a
graded distribution and not a binary. This is a direct route to the owner's "roles should emerge"
without castes: **divergent life histories give divergent habits from one brain design**. (`c::SELF`,
a per-ant threshold number, already exists for the same purpose.)

### 3.4 How to make "my lesson had a loophole" delightful (design rules)

1. **The cause must point at the player's act**, as Hello Neighbor's trap does. The readout says
   "learned: *gnome nearby* → *wait* (from 7 treats)".
2. **Name it when it happens** (Nemesis anchors). Write a line in the life record ("discovered that
   waiting by the gnome pays") and a colony headline when more than a tenth of the colony shares a
   learned habit.
3. **It must be reversible at a price.** Offsets decay without reinforcement, so a bad lesson fades in
   a few in-game days unless re-fed. Punishing it fixes it faster, with its own risks.
4. **Make a collection.** A "loophole bestiary" in the specimen shelf: save the ant that invented the
   pellet shuttle. This is Picbreeder's publish-and-branch, applied to habits.
5. **Never let a loophole kill the colony silently.** If a learned habit is starving ants, the grave
   says so ("starved: learned to wait for the gnome").

---

## 4. Legibility: showing the player what an ant learned

Feedback has to say *what changed* and *whether it fired* (house rule: a counter beside every image).

| Readout | What it shows | Cost | Notes |
|---|---|---|---|
| **"Born / learned" columns on the ant page** | The genome is already read back as sentences. Add the top three plastic offsets as sentences: "learned: *scent B ahead* → *walk* (strong, 12 treats, fading)". | Low | Each sentence needs one feature×verb label, and those already exist (`INPUT_NAMES`, option features). |
| **Thought icon when a lesson changed a decision** | A small mark over the ant on the ticks where the pick **would have been different with the offset set to zero** | Low to medium (a counterfactual re-score of the readout only; can be sampled) | This is the "did it fire" counter at ant scale. Show the count in the panel. |
| **Hesitation pose** | Antennae waving when the top-two score margin is small | Low | Invites a lesson (Peng et al. 2016). Physical, not a UI element. |
| **Colony habit heatmap** | Mean offset for one chosen lesson, painted over the cells where it was learned or used | Medium | Per the CLAUDE.md debug rule: **full replace on a fixed dark→bright ramp**, never blended into cell colour. |
| **Ghost replay, before / after** | Overlay one ant's trip from before the lesson and after | Medium (needs a trip recorder; the life record has some of it) | The strongest single "it learned" image, and MarI/O-like. Good for review cards. |
| **Body marks** (Nemesis scars, B&W morphs) | A treat-fed ant gets glossier; a punished one carries a soot mark that fades | Low | A physical, persistent mark, as the house rule asks. Keep it subtle (realism). |
| **Lineage and habit tree** | Which lineages learn fastest (the heritable learning-rate gene) | Medium | Ties the queen's breeding verb to teachability (Baldwin effect, §6.4). |

---

## 5. Concrete teaching mechanics (ranked)

Ranking criteria: (a) a satisfying player verb with a visible result within a session; (b) it
respects the standing rules (one brain, roles emerge, nest *and* forage, graded outcomes, frame
cost, realism); (c) build cost; (d) it is supported by the one rule in §6.

### 1. Treat and puff: the clicker in the gnome's hand

- **ML underneath**: TAMER/COACH as a three-factor rule. A player treat or puff enters the reward
  term. The eligibility trace assigns credit to the last few decisions.
- **Verb**: drop a sugar bead on or next to an ant (the reward is *real food*, so it is physical and
  realistic). Puff bitter dust or smoke as the negative. An **area version** (a sprinkle over a
  patch) rewards every ant in range, which scales past one ant at a time.
- **Timescale**: three to ten well-timed treats change an ant's habit (expected from TAMER; to be
  measured). The habit fades over in-game days without reinforcement.
- **Visible feedback**: a "learned" line in the panel, thought icons when the lesson swings a pick,
  and gloss on fed ants. The colony count of ants carrying the lesson sits in the meta.
- **Failure mode**: begging and positive circuits (§3.1), superstition from bad timing, and
  generalising to the wrong feature. All three are stories if legible.
- **Build cost**: medium. The rule (§6) plus two tools plus the panel lines.

### 2. Make the trail mean something: conditioned scent cues

- **ML underneath**: classical conditioning, plus advice. The plastic offsets on the scent features
  (strength and gradient of A and B) learn their value from the intrinsic reward (food found).
- **Verb**: the existing drag-to-lay trail. With learning, ants that follow it *and find food* come to
  value B in this box, so later trails pull them. If it is worth a channel, a player-only "lesson
  scent" could be added, but that costs frame time (one more pheromone plane). Try it on B first.
- **Timescale**: tens of minutes for a colony. Faster if the player treats at the trail's end.
- **Visible feedback**: workers measurably start using the player's trail. The **trail-use count is
  the number on the card**. Today it is measured as no pull.
- **Failure mode**: a trail to an exhausted source teaches distrust (extinction). That is realistic
  and graded, and it is a lesson about keeping trails honest.
- **Build cost**: low on top of §6. It fixes a measured failure of an existing verb, which is the
  strongest argument for building the rule at all.

### 3. Training grounds: the player builds the course (NERO exercises)

- **ML underneath**: curriculum learning, with the environment as teacher. Works in-life (habits
  form within a life) and across generations (the survivors' genomes are biased).
- **Verb**: sculpt the box. Put food just across a shallow water channel, then a deeper one; put a
  short dig before seed caches; place stepping-stone food at increasing distance from the nest. That
  last one targets the known problem that colonies starve when food is far.
- **Timescale**: one course stage per in-game day or so (to measure).
- **Visible feedback**: a per-course success count (crossings, deliveries per stage) and a ghost
  trace of the first ant to solve each stage.
- **Failure mode**: ants learn the course and not the skill (they overfit to a layout). Forgetting
  when the course is removed. Both show up in a "move the food" test.
- **Build cost**: low. The tools mostly exist (food, lamps, terrain). A "stage" counter is the new
  part.

### 4. Lay from the best: the queen's selection

- **ML underneath**: interactive evolution, plus the **Baldwin effect** (Hinton and Nowlan 1987
  **[K]**): selecting ants that *learned well* selects for teachability genes (learning rate, trace
  length, forgetting rate) without inheriting what was learned.
- **Verb**: as the queen, mark ants with a ribbon (or touch antennae). The next N eggs carry their
  genome (with mutation). An **implicit version** (GAR) avoids fatigue: ants the player fed, rescued
  or treated more often contribute more eggs.
- **Timescale**: generations.
- **Visible feedback**: lineage tree with names (exists), and the share of the colony per lineage over
  time.
- **Failure mode**: user fatigue (Takagi; Niche's "grind") if the player must judge many individuals.
  Selecting a showy behaviour can ruin a less visible essential one (diggers bred out by a player who
  only watches foragers). **The colony's graded survival is the honest check.**
- **Build cost**: low to medium. Laying already exists. The work is the marking UI and the egg
  assignment.

### 5. Lead the way: tandem demonstration

- **ML underneath**: imitation / behaviour cloning through the same rule. The follower's reward is
  agreement between its pick and the leader's last step, and that pushes its offsets toward the
  leader's choices. Real-ant basis: *Temnothorax* tandem running (Franks and Richardson 2006).
- **Verb**: as the queen body (exists, switched off) or as a tutor ant, walk a route slowly. Nearby
  workers fall in behind, and taught followers become leaders for the next.
- **Timescale**: one run per follower, with a spread like an epidemic.
- **Visible feedback**: the follower's tap and pause (physical), a chain of tandem pairs, and a
  count of "taught by tandem" in the life record.
- **Failure mode**: superstitions and loopholes **spread as culture** (feature). Followers copy the
  queen's mistakes too (Forza's lesson: give a veto, which here is decay).
- **Build cost**: medium. The follow behaviour needs to become a real option (feature: "leader
  ahead"). The rule is shared.

### 6. Taboo marks: avoidance learning

- **ML underneath**: negative reward on context features (punishment learning).
- **Verb**: dust a place with a bitter or irritant powder (a material in the sim, so it is physical).
  Ants that dig or walk there get negative reward and learn to avoid the context.
- **Timescale**: fast, because negative events are salient.
- **Visible feedback**: an exclusion zone forms. The heatmap shows avoidance spreading beyond the
  dust as generalisation.
- **Failure mode**: **the collapse in §6.5**: over-generalisation to "all soil", learned helplessness
  (rest dominates), unlearning food if the dust touches food. Guard it, and also show it.
- **Build cost**: low (one material) on top of §6.

### 7. School box and tutor ant

- **ML underneath**: an off-colony curriculum (the specimen shelf), followed by cultural transmission
  through mechanic 5.
- **Verb**: take an ant to the shelf, run it in a small private course box (the shelf already clones
  and mutates), and return it as a tutor that leads tandems.
- **Timescale**: one school session (minutes), then spread.
- **Visible feedback**: the tutor's badge, and a count of pupils.
- **Failure mode**: the tutor's lesson fits the school and not the box (transfer failure). Tutors die,
  so knowledge is lost unless it was passed on, which is realistic and graded.
- **Build cost**: medium. Needs the shelf to run a mini-world, and plastic state preserved on clone.

### 8. Colony edicts: reward weights (NERO sliders, kept physical)

- **ML underneath**: the designer's reward shaping. It sets weights on the *intrinsic* reward terms
  (food home, digging cut and spoil carried away, brood fed) used by the in-life rule, and optionally
  the selection fitness.
- **Verb**: the queen-player's pheromone signature or royal "mood" (avoid bare sliders; express it as
  a scent the queen emits, so it stays in-world). "The colony feels the queen craves rooms."
- **Timescale**: immediate drift in habits, and the composition shifts over generations.
- **Visible feedback**: the share of ants doing each job (the "roles" bar), and specification-gaming
  stories when an edict is literal-minded (dig-and-refill).
- **Failure mode**: specification gaming is *guaranteed*. That is the point, and it must be legible.
  The labels must be in world language (NERO's unlabeled sliders were the complaint).
- **Build cost**: low. It is a few multipliers once §6 exists.

**Runners-up**: preference judging between two ghost replays (Christiano), which fits a between-seasons
review rather than live play; the hesitation pose as an *invitation* (folded into 1); and "the world is
the teacher" with no player verb (it underlies 2 and 3).

**Gnome or queen?** Mechanics 1, 2, 3, 6 and 7 belong to the gardener-gnome. Mechanics 4, 5 and 8
belong to the queen. All eight share the one rule below, so the choice of perspective does not fork
the engine.

---

## 6. Recommendation: the minimal in-life learning rule

### 6.1 Rule: reward-modulated plastic offsets with an eligibility trace (three-factor, baseline-subtracted)

The three-factor framework (Gerstner et al. 2018, *Frontiers in Neural Circuits*): co-activity sets a
synaptic **eligibility flag**, and the weight changes only if a third signal (reward or
neuromodulator) arrives while the flag is still set. Evolution reliably discovers modulated plasticity
when reward locations change within a lifetime (Soltoggio et al. 2008, ALIFE XI, T-maze), which is
exactly the "food is somewhere new in this box" situation. Creatures used a Hebbian, reward-modulated
version in 1996. Backpropamine (Miconi et al. 2019/2020, arXiv 2002.10585) makes it differentiable,
but that is not needed here.

**Where the plasticity goes.** Put it only on the **scorer's readout**, the 16 unit→score weights.
Optionally also put it on the job head's output row for each verb. Do not put it on the 45×16 feature
layer. This is small (tens of floats per ant), cheap, bounded, and legible, because each readout
weight maps to a unit and each unit's top features can be named.

Per ant, store `dw[k]` (the plastic offset, starting at 0) and `e[k]` (the trace). The genome weight
`w[k]` is untouched. The effective weight is `w[k] + dw[k]`.

On each decision where option `o` was picked from the set `O`, with unit activations `h(·)` and pick
probabilities `p(·)` from `choose_weighted`:

```
e[k]  <- lambda * e[k] + ( h_k(o) - sum_{o' in O} p(o') h_k(o') )   // chosen minus expected (REINFORCE / COACH form)
```

On each tick with reward `r`:

```
delta  = r - rbar                                                   // advantage, not raw reward (COACH)
rbar  <- rbar + beta * (r - rbar)                                   // per-ant running baseline
dw[k] <- (1 - kappa) * dw[k] + eta(age, |dw|) * delta * e[k]        // decay toward genome = forgetting
dw[k] <- clamp(dw[k], -B, +B)
```

- **`r` = intrinsic plus player.** The intrinsic part is energy gained (`c::D_ENERGY` already exists
  as a context input), a delivery to the store, a larva fed, and a cut whose spoil left the nest. The
  player part is a treat, which really is food, so it largely *is* energy gained; a bitter or smoke
  puff gives negative `r`. Reusing need reduction keeps it Creatures-like and realistic, and it means
  the player's treat is not a magic channel.
- **`eta`, `lambda` and `kappa` are heritable genes** (use spare trait slots). Evolution then sets how
  teachable a lineage is, and the queen's selection reaches teachability (Baldwin). `eta` falls with
  age and with `|dw|`, which is a cheap EWC (§3.3).
- **Deterministic**: no transcendental functions, the same squash, no new random draws.
- **Cost**: per decision, about 2×16 multiply-adds for the trace, plus one sum over options that the
  scorer already visited. Per reward tick, 16 more. Memory is about 100 bytes per ant. That is
  negligible next to scoring every option through 45×16. Confirm with `examples/ascii.rs` worst
  frame.
- **Inheritance**: `dw` is **not** inherited (Baldwinian, realistic). Learned knowledge crosses
  generations only through culture (tandem tutors, trails, nest architecture). An optional Lamarckian
  switch (a child takes a fraction of the layer's `dw`) is worth exposing as a selector to judge by
  eye, but it should not be the default without measurement.

### 6.2 What it supports

| Mechanic | Supported | How |
|---|---|---|
| 1 Treat / puff | Yes | Player `r` |
| 2 Trail meaning | Yes | Intrinsic `r` on scent-feature units |
| 3 Training grounds | Yes | Intrinsic `r` in a sculpted world |
| 4 Lay from the best | Indirectly | Teachability genes plus `dw`-aware selection |
| 5 Tandem | Yes | Agreement-as-`r` while following (one extra reward term) |
| 6 Taboo | Yes | Negative `r` |
| 7 School / tutor | Yes | Plus clone keeps `dw` |
| 8 Edicts | Yes | Weights on the intrinsic `r` terms |

### 6.3 How to prove it works (positive control first, as CLAUDE.md requires)

1. **Positive control, the switch arena** (Soltoggio's T-maze in ant form). Two food sites. Midway
   through life, food moves from A to B. Compare plastic and frozen (`eta=0`) arms on paired seeds.
   Metric: deliveries from B in the window after the switch, and decisions until the first B
   delivery. Expect plastic > frozen. **If they are equal, the rule is not connected** (CLAUDE.md:
   "a change that moves nothing").
2. **Instrument check.** Inject a known `dw` by hand and confirm the panel sentence and the
   thought-icon counter report it. Zero it and confirm they go silent.
3. **The trail test.** The existing measured scene ("hand-laid trail does not pull workers"). Run a
   pairing period where the trail leads to food, then measure the pull on a new trail. This is the
   player-facing proof.
4. **Dose response for treats.** Give 0, 3, 10 and 30 treats to marked ants, paired by seed, and
   measure the change in the frequency of the targeted verb. Expect a graded curve, not a step.
5. **Specificity, no regression.** In the normal lab box with no player and a stationary world,
   plastic and frozen colonies should be equal or plastic better on survival, food home and rooms.
   Gate an **order statistic over a seed sweep** (p10 survival over N≥24 seeds), not one seed.
6. **Frame cost.** `examples/ascii.rs` worst frame, plastic against frozen, re-measured in the same
   session.
7. **Picture.** A before/after ghost replay posted to the review queue, with the delivery count in
   `meta`.

### 6.4 Why not evolution only

Evolution-only teaching gives the player the queen's breeding verb (mechanic 4) and environment
shaping across generations. It keeps the engine simpler, with no new state. But the effect shows only
after generations, which is the Niche/Picbreeder "grind" and fatigue pattern. And the trail verb stays
dead. One in-life rule makes the feedback loop fast enough to feel. Keep evolution as the slow layer
that tunes teachability.

### 6.5 The classic collapse, and its guards

**The failure.** If net reward is mostly negative, from heavy player punishment, or punishment that
lands on the act of eating or on the store, the offsets push down whatever the ant was doing. "Rest"
or an idle loop wins, or eating is unlearned and ants starve. The mirror case: mostly positive reward
from a generous player drives positive circuits (Knox and Stone 2012: human reward is positively
biased, and weak discounting loops on it). Thomaz and Breazeal saw the same positivity bias.

**Guards** (all cheap):

1. **Baseline subtraction** (`r - rbar`). A uniform rate of punishment or treats teaches **nothing**;
   only contrast teaches. This is the main guard against both collapses.
2. **Bounded offsets** (`|dw| <= B`, with `B` well under the instinct weights' magnitude). A lesson
   tilts an instinct; it cannot erase one.
3. **Protected need paths.** No plasticity on the job head's hunger→Eat and crop→Feed rows. The
   "eat when hungry" drive stays genomic.
4. **Decay** (`kappa > 0`). Unreinforced lessons fade, so a bad week does not ruin a lineage.
5. **No verb is ever excluded.** The existing `choose_weighted` "prefers the strong score without ever
   excluding the weak one" (test name in `creature.rs`), so exploration survives.

**Test for the collapse** (put the fault back first):

- **Uniform punishment.** Random puffs at a fixed rate on every ant. Expect no change in the rates of
  eating, digging and foraging (baseline works). With the baseline removed as a fault injection,
  expect a measurable drop. If none appears, the test is blind.
- **Adversarial punishment.** Puff every ant that eats from the store. Expect store eating to drop
  but starvation deaths to stay within a bar set from the frozen arm's spread. Without guard 3,
  expect starvation to rise, which is the sensitivity check.
- **Generous treats.** Treat every ant near the gnome. Expect begging (mechanic 1's loophole) to
  appear **but stay bounded**: deliveries per hour stay at or above some fraction of the untreated
  arm.

---

## 7. The three biggest risks

1. **Per-ant tedium at colony scale.** Hundreds of ants make B&W-style one-on-one training a chore.
   Every verb needs a scaling path: area treats, tandem spread, tutors, or implicit selection. If a
   lesson needs more than about ten repetitions per ant, it fails the owner's "satisfying" bar.
2. **Loopholes and collapse that are not legible.** Begging, shuttling, dig-and-refill and unlearned
   eating are delightful *only* if the player can see the cause ("learned: gnome → wait, 7 treats")
   and undo it. Without the readouts of §4 they read as a broken game. The collapse guards (§6.5)
   have to be shown to work by fault injection, not assumed.
3. **Plasticity regresses the trained colony.** Drift in a stationary world, interference between
   lessons, and frame cost, plus the existing problems (boom-bust, idle fed ants) masking or being
   masked by learning effects. Gate on a seed-swept order statistic against the frozen arm, and keep
   `eta=0` as a runtime selector so a playtest can compare.

---

## Sources

Games and player-trained AI
- NERO: https://nn.cs.utexas.edu/NERO/about.html ; https://www.cs.utexas.edu/users/nn/nero/about.php ; v2.0 review https://www.linux.com/?p=513596 ; AIIDE paper https://ojs.aaai.org/index.php/AIIDE/article/download/18736/18513 ; OpenNERO https://github.com/nnrg/opennero/wiki/NeroGame
- Galactic Arms Race: https://course.ccs.neu.edu/cs5150f13/readings/hastings_gar.pdf ; https://ojs.aaai.org/index.php/AIIDE/article/view/12345 ; player study https://stars.library.ucf.edu/scopus2000/11056 ; Weapons Lab https://www.pcgworkshop.com/archive/hastings2010interactive.pdf
- Picbreeder / IEC fatigue: https://www.cs.utexas.edu/~risto/cs378ne/private/reading/Sections-8.3-8.4.pdf ; Hornby GECCO 2012 https://meclab.w3.uvm.edu/papers/2012_GECCO_Hornby.pdf ; Takagi 2001 https://gpbib.cs.ucl.ac.uk/gp-html/takagi_2001_ieee.html ; https://en.wikipedia.org/wiki/Interactive_evolutionary_computation
- Black & White: Salon 2001 review https://www.salon.com/2001/04/10/black_and_white/ ; Lionhead postmortem https://gamedeveloper.com/design/postmortem-lionhead-studios-i-black-white-i- ; https://en.wikipedia.org/wiki/Richard_Evans_(AI_researcher) ; B&W2 buying miracles https://en.wikipedia.org/wiki/Black_%26_White_2 ; GameRevolution https://www.gamerevolution.com/?p=33185
- Creatures: https://en.wikipedia.org/wiki/Creatures_(video_game_series) ; Grand, Cliff, Malhotra 1997 https://www.sussex.ac.uk/informatics/cogslib/reports/csrp/csrp434.pdf ; https://www.alanzucconi.com/2020/07/27/the-ai-of-creatures/
- Forza Drivatar: https://support.forza.net/hc/en-us/articles/360005363194-Difficulty-Assists-and-Drivatars ; https://en.wikipedia.org/wiki/Forza ; forum https://forums.forza.net/t/is-poor-drivatar-behavior-cumulative-to-create-even-more-bad-drivatar-behavior-serious-question/32094
- Hello Neighbor: https://store.steampowered.com/app/1373880/Hello_Neighbor_2_Alpha_1/ (marketing; criticism sources thin)
- Nemesis: https://gamesbeat.com/shadow-of-mordors-nemesis-system-draws-from-burnout-football-and-an-architecture-book/
- Gladiabots: https://gfx47.itch.io/gladiabots ; https://www.pocketgamer.com/gladiabots/gladiabots-combines-tactical-bot-combat-and-programming-puzzles/
- MarI/O: https://singularityhub.com/2015/06/18/watch-this-open-source-ai-learn-to-dominate-super-mario-world-in-just-24-hours/ ; https://hackaday.com/tag/ga
- Niche: https://opencritic.com/game/10099/-/reviews ; https://www.commonsense.org/education/game/niche-a-genetics-survival-game

Interactive ML
- TAMER: https://www.cs.utexas.edu/~bradknox/TAMER.html ; https://www.cs.utexas.edu/~pstone/Papers/bib2html-links/ICDL08-knox.pdf
- Positive circuits: https://www.cs.utexas.edu/~pstone/Papers/bib2html/b2hd-ROMAN12-knox.html
- COACH: https://proceedings.mlr.press/v70/macglashan17a.html ; https://alphaxiv.org/abs/1701.06049
- Sophie's Kitchen: https://www.cs.helsinki.fi/group/cosco/Teaching/CoscoSeminar/spring2007/articles/thomaz-2006.pdf ; https://www.jimdavies.org/summaries/ThomazBreazeal2008.html
- A Need for Speed: https://par.nsf.gov/biblio/10026423-need-speed-adapting-agent-action-speed-improve-task-learning-from-non-expert-humans
- Power to the People: https://hcrlab.cs.washington.edu/assets/pdfs/2015/amershi2015aimag.pdf
- Showing vs doing: https://proceedings.neurips.cc/paper/2016/file/b5488aeff42889188d03c9895255cecc-Reviews.html ; Show or Tell https://cognitivesciencesociety.org/cogsci20/papers/0769/index.html
- Preferences: https://arxiv.org/pdf/1706.03741 ; https://openai.com/index/learning-from-human-preferences/

Surprise, forgetting, superstition
- Krakovna et al., DeepMind 2020: https://deepmind.google/discover/blog/specification-gaming-the-flip-side-of-ai-ingenuity/ ; list mirror https://git.nicholasjohnson.ch/specification-gaming/diff/
- Lehman et al.: https://arxiv.org/abs/1803.03453v3
- Skinner 1948: https://www.Gwern.net/doc/statistics/causality/1948-skinner.pdf
- EWC: https://arxiv.org/html/1612.00796 ; https://deepmind.google/discover/blog/enabling-continual-learning-in-neural-networks/
- Polyworld: https://en.wikipedia.org/wiki/Polyworld

Learning rules and biology
- Three-factor rules: https://www.frontiersin.org/articles/10.3389/fncir.2018.00053/full
- Soltoggio et al. 2008: https://infoscience.epfl.ch/record/121548?ln=en ; review "Born to Learn" https://arxiv.org/pdf/1703.10371
- Backpropamine: https://www.alphaxiv.org/abs/2002.10585 ; Hebbian meta-learning https://www.alphaxiv.org/abs/2007.02686 ; continual learning with Hebbian plasticity https://www.alphaxiv.org/abs/2006.16558
- Tandem running: https://www.nature.com/articles/439153a ; https://www.bristol.ac.uk/news/2006/879.html
- Clicker training (marker and timing): https://en.wikipedia.org/wiki/Clicker_training

From knowledge, not verified this session [K]: rtNEAT detail (Stanley, Bryant, Miikkulainen 2005, IEEE TEC); Evans's B&W architecture (AI Game Programming Wisdom 2002); CoastRunners, bicycle (Randløv and Alstrøm 1998) and Tetris-pause (Murphy 2013) examples; Bengio 2009 curriculum; Ng 1999 shaping; Maclin and Shavlik advice-taking; Hinton and Nowlan 1987 Baldwin effect; Colobot; Pokémon EVs.
