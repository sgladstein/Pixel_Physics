# The queen's lens: how a player-queen teaches a colony (research, 2026-10-25)

Independent design research for the lab game. Lens: **the player IS the queen ant.**
The owner's hard question: *"how is the queen teaching the colony different behaviours
in a way that's interesting and more complex than you walk to some food and your colony
follows you?"*

Source tags used throughout: **[S]** = confirmed from a web-search summary (URL given);
**[K]** = my own knowledge of the game or paper, not re-verified this session;
**[E]** = read from this repository's source or reports.

---

## 0. The answer in one paragraph

A real queen doesn't lead her colony. She shapes it through **who exists** (which eggs, which
father, how many), **what they become** (the nutrition and hormone she and the nurses hand
the brood), **what the brood smells and feels while it grows**, and **where the brood
lies**, which pulls the whole workforce towards it. Those four channels are slower and stranger
than "follow me", and that's the opportunity. The design that answers the owner is a queen
who teaches on **three clocks at once**:

- **seconds:** her body. She lays one egg at a time, carries brood, feeds, bites, plugs doors and leads one pupil on a tandem run;
- **minutes:** the brood cycle. Each cohort comes out shaped by how it was raised, and gets a name and a report card;
- **hours:** the lineage. Her picks of which workers father the next eggs settle into instinct.

The deepest version adds a **queen-scent channel whose meaning is not authored**. The colony
evolves to answer it because the player uses it consistently and breeds the ants that
answer. The player is effectively inventing a language with her own descendants.

---

## 1. Engine facts that decide what is buildable [E]

From the brief, plus three things in the source that matter more than they first look:

1. **No in-life learning.** Weights are fixed at birth. Anything called "in-life teaching" has to
   (a) write to state the ant already has, such as its *last-food memory*, hunger or crop,
   (b) add a small amount of plasticity, or (c) act through the world: scent, structure,
   brood placement. Option (a) is nearly free, and I lean on it.
2. **A developmental channel already exists: `BrainOutput::Provision`.** It is `src/sim/brain.rs:170-205`.
   At budding a parent hands its child one number, stored as `OrganismState::made`. Each
   lineage carries a heritable *developmental block*: one weight per body trait slot,
   saying how far that slot moves with the number. The doc comment reads: *"This is the
   whole of what makes a caste possible without naming one."* That is exactly the real
   biology of nutrition deciding caste (see §3), and the obvious hook for a queen who decides
   what each egg becomes **without hand-made castes**. Today it moves the 14 body slots, not
   the walk slots (`DEV_TRAITS = 14`). Opening it to the walk genes is described there as a
   re-baseline, not a free change.
3. **The walk has named, inherited gains, all switched off.** They live in `how-the-ant-works.md` §6d.
   Six walk-gain slots (home pull, trail hold, route-away, scout, door read, patience) and
   four trail-plane weights are heritable, but they *"ship at allele 0 and mutation width 0
   on every species"*. That is a ready-made, **legible** genetic vocabulary: "this lineage
   holds trails twice as hard". A queen can select on it, and a page can read it out in a
   sentence. The ~2,800 brain weights can't be read out that way.
4. **Why a hand-laid trail doesn't pull workers** (slice 0, §2) may be partly explained in §6d.
   Under the shipped chooser, `brain_inputs` hands the brain `PheroAAlong`/`PheroBAlong` as
   **0**. Trail B acts only through the chooser's heading multiplier (`1 + TRAIL_GAIN ×
   presence`), and the away term acts only for an empty ant already on a route. Nothing *recruits* an
   empty ant from off the route onto it. The slice-0 report said this was not traced, and it
   still isn't: this is a reading of the docs, not a measurement. It points at the same
   conclusion as the owner's question, though. **Trail-following is the weakest lever a queen
   has, in the engine as in biology.** Real queens don't lay recruitment trails.
5. **There is a queen breeding rule** (`PIXEL_PHYSICS_BREEDING=queen`): only one breeder at a
   time, with succession. The default is `graded`. An egg inherits **the layer's** brain. So
   "the queen lays everything" currently means every egg carries *her* genome, which kills
   variation. **A queen who lays must be given other genomes to lay**, as sperm or patrilines
   (see A1 and A3). Otherwise "choosing which workers to breed from" has no biology under it.
6. Graves, life records (steps, digs, deliveries, young), lineage names and the specimen shelf
   (keep, clone, mutate) already exist. Several mechanisms below are mostly *interface over
   existing data*.

---

## 2. Games where the player shapes rather than commands

### What each did, and what it teaches

| Game | How the player shapes agents | What worked | What was boring or failed | Source |
|---|---|---|---|---|
| **SimAnt** (Maxis, 1991) | You are one yellow ant. You lay trail and recruit 5 or 10 followers. Sliders set caste ratio (worker/soldier/breeder) and behaviour (forage/dig/nurse) | The two levels: a body in the dirt plus colony policy. Rival red colony. Spiders as real danger | The sliders *are* the colony. Behaviour is a ratio you dial, not something that emerges. Recruit-and-follow is exactly the owner's "walk to food and they follow" | [K] |
| **Empires of the Undergrowth** (2017-) | No unit selection. You place pheromone markers that ants go to; queen in a nest you build | Indirect control is "incredibly unique" (PC Gamer, via store). Watching swarms respond | The markers are orders with a delay. Ants are RTS units, nothing is learned, the colony never changes character | [S] https://store.steampowered.com/app/463530/ |
| **Pikmin** (Nintendo) | Captain leads followers, throws them at tasks, they auto-work | Throwing is a satisfying verb. Type differences (fire, water) matter. Followers die visibly | Pure command, no teaching. Pikmin forget you the moment you leave them | [K] |
| **Overlord** | Sweep minions with a stick, gear them up. Minions loot and wear trophies | Personality in minions. Loot changes how they look | Command game. Minion "personality" is cosmetic | [K] |
| **Dungeon Keeper** | Minions are autonomous. You **slap** them (faster work, costs health and happiness), pay wages, design rooms they want | **Rooms as teaching**: building a library *attracts* warlocks and sets their work. The slap is a verb with a visible, costly consequence | Tuning happiness becomes a chore late on | [K] |
| **Black & White** (Lionhead, 2001) | A creature learns from your example, and from **slap/stroke** after an act. Leashes | The fantasy of raising something that becomes *yours*. The creature's look drifts with how it is raised (visible, graded) | Players found learning opaque and unreliable. You slapped it repeatedly without knowing which act was being punished (**credit assignment**). Hours of babysitting | [S] summaries, low-quality, e.g. https://www.itechguides.com/how-black-whites-creature-learning-ai-and-morality-system-work/; details [K] |
| **Creatures** (1996) | Norns with ~1,000-neuron brains plus biochemistry. Tickle (nose) or slap (rear). Teach words. Breed them | Real neural learning plus heredity. A community bred lines for years. "Evolved in ways unpredicted by the makers" | Teaching was slow and noisy. Many players bred rather than taught, because breeding produced visible change and teaching did not | [S] https://en.wikipedia.org/wiki/Creatures_(1996_video_game) |
| **Niche** (Stray Fawn, 2017) | Turn-based breeding over 100+ genes, with real heredity: recessives, inbreeding penalty | Turns make each mating a decision. Genes unlocked by biome experience. Used in classrooms | Short lifespans meant good genes vanished before you could breed them in. Gene micromanagement. Weak long tail (<100 concurrent players reported) | [S] https://www.killscreen.com/niche-survival-game-based-real-science-genetics/ , https://steamcommunity.com/app/440650/discussions/1/5729109343960507982 |
| **Spore** (creature stage) | Pick parts with DNA points | Instant visual result of a choice | Evolution as a shop. Parts are the player's design, not selection | [K] |
| **Kingdom / Two Crowns** | One verb: drop coins. Coins recruit, hire, build | One verb with legible meaning in context. The player "figures out what to do" | Repetitive tasks hurt replay (reviews) | [S] https://de.wikipedia.org/wiki/Benutzer:Calle_Cool/Kingdom_(video_game) |
| **Majesty** (2000) | No direct orders. Bounty flags; heroes decide whether the reward is worth it | Incentive, not command, and *each hero weighs it differently*. Gold comes back through taxes (virtuous circle) | **Majesty 2's heroes "mostly respond to bounty flags with little initiative"**. Once agents only answer incentives, the charm dies | [S] https://www.gamespot.com/articles/majesty-legends-qanda/1100-6026166/ ; https://www.galaxus.de/en/page/gold-instead-of-obedience-crown-of-greed-tested-42293 |
| **Crown of Greed** (2026) | Majesty-like. "Decisions are weightings; traits shift priorities, they don't fix them" | Weighted traits | "Heroes barely develop within a mission" | [S] same Galaxus review |
| **The Sims** (free will) | Agents act on needs. You queue actions or arrange the house | Arranging the house shapes the day | Free will fights the player's queue, which reads as stupidity, not character | [K] |
| **Banished / Songs of Syx / Oxygen Not Included / RimWorld** | Job priorities, zones, policies | Zones and placement shape behaviour cheaply | Priority grids are spreadsheets. RimWorld succeeds through *story* (traits, relationships, graves), not through control | [K] |
| **Rain World** | Autonomous ecology the player is part of, not above | Agents feel alive because they ignore you | No shaping, so not a model for teaching | [K] |
| **Species: Artificial Life, Real Evolution** (2018) | A natural-selection sim. You can kill, feed or move creatures, and watch clades | Watching lineages split. Intervention as selection | Slow. The player is mostly a spectator | [K] |
| **Plague Inc.** | Spend points to buy traits | Responsive. Every buy changes a number | It is a shop (the Spore problem) | [K] |
| **Lemmings** | Assign one skill to one walker at the right moment | Precise single-agent intervention that saves many | Puzzle, not simulation | [K] |

### Lessons pulled out

1. **Breeding beats teaching when teaching is invisible.** Creatures' and Black & White's
   players drifted to breeding because it reliably produced visible change. So in-life teaching
   has to show *on the individual, immediately*, or players will ignore it.
2. **Credit assignment is the killer.** In Black & White the slap arrived after the creature
   had done three things. Any reward verb needs either instant timing (on the act) or an
   interface that says *what* was rewarded.
3. **Indirect control works when the agent weighs the incentive** (Majesty 1), and dies when
   the agent only obeys it (Majesty 2, Empires of the Undergrowth markers). The player must be
   *one input among several* to the ant, never the controller. That is literally what the
   ant's brain is: a scorer over options.
4. **Rooms teach** (Dungeon Keeper). Structure that *attracts* behaviour is cheap and legible,
   and has the owner's "visible consequence" built in.
5. **Sliders turn behaviour into a dial** (SimAnt, ONI). Anything the player sets as a ratio
   stops being emergent.
6. **One verb with rich context-dependent meaning** (Kingdom's coin, Lemmings' skill) is
   better than ten verbs.
7. **Short lives lose genes before you can breed them in** (Niche). The queen must be able to
   *keep* genomes: a sperm store, a shelf, a jar.
8. **Long-term retention comes from story** (RimWorld, Creatures communities): names, graves,
   lineages, rivals. This project already has names, graves and life records.

---

## 3. Real queen biology as levers

**What real queens do NOT do:** lead foraging, give orders, lay recruitment trails (in most
species), or decide individual workers' jobs. After founding, a mature queen is an egg
factory surrounded by a retinue. Colony behaviour is decentralised: workers respond to local
cues, brood, nestmates and their own thresholds [K; standard myrmecology, e.g. Hölldobler &
Wilson, *The Ants*, 1990]. **That is the design opportunity.** A queen who teaches by being
*the source of the colony's composition, chemistry and centre of gravity* is truer than a
leader, and more interesting.

What real queens and colonies do, and the lever each suggests:

| Biology | What it suggests for a player-queen | Source |
|---|---|---|
| **Founding queens do everything.** A claustral queen seals herself in and raises the first brood alone, turning flight muscle and fat into **trophic eggs** and saliva. The first workers are small **nanitics** | **The founding phase is the one time she "leads by doing".** What she does then is the colony's first lesson. Then she hands off. That arc is a natural tutorial-to-midgame structure | [S] https://antwiki.org/wiki/Colony_Foundation |
| **Trophic eggs**: eggs laid only as food | A queen can pay food in eggs: a currency and a choice ("feed them or grow them") | [S] same |
| **Haplodiploidy and sperm control.** Fertilised eggs become females (workers or queens), unfertilised ones become males. A queen stores sperm for life, uses one or two per egg, and can shift sex ratio. Workers sometimes counter by culling males (queen-worker conflict) | A **sperm store of several fathers** (patrilines) that she draws from egg by egg. Males cost food and do no work, but they are the export | [S] https://www.sciencedaily.com/releases/2001/08/010820072111.htm ; https://pmc.ncbi.nlm.nih.gov/articles/PMC1690684 |
| **Polyandry raises diversity, and diversity smooths behaviour.** Honey bee colonies of many fathers keep brood temperature steadier, because workers' response thresholds vary | A **diversity lever** that changes the *spread* of the colony's behaviour, not its average. Inherently graded | [S] https://pubmed.ncbi.nlm.nih.gov/15218093/ |
| **Queen pheromone** (cuticular hydrocarbons in ants, QMP in bees) signals fertility, suppresses worker reproduction, draws a retinue | Her scent field is a *damper and a magnet*. Workers near her behave differently | [S] https://link.springer.com/article/10.1007/s00265-023-03378-8 ; https://www.frontiersin.org/journals/cell-and-developmental-biology/articles/10.3389/fcell.2020.00837/pdf |
| **QMP delays the age workers start foraging** and lowers juvenile hormone | Her presence keeps workers *home* longer (nursing, digging). Her absence makes them forage and explore earlier | [S] https://pherobase.com/literature/data/details/I/47938 (Pankiw et al. 1998, J Insect Physiol 44:685) |
| **Queen loss releases worker reproduction** and emergency queen rearing | Leaving the nest or dying is a lever with consequences: workers start laying their own genomes | [S] Aphaenogaster note in the Frontiers review above |
| **Nutrition decides caste.** In many ants, *workers* (nurses) mostly control what a larva gets, so the queen influences it indirectly. Juvenile hormone is involved | What the brood is fed decides what it becomes. The engine's `Provision` channel is exactly this | [S] Frontiers review above; [K] |
| **Hormones travel in shared food.** Camponotus trophallactic fluid carries juvenile hormone, proteins and microRNAs. Larvae reared on JH-supplemented fluid were twice as likely to complete metamorphosis and grew into **larger workers**. LeBoeuf: ants "casting quantitative ballots" | Feeding is a **message**, and graded: a little or a lot of "royal" substance | [S] https://elifesciences.org/articles/23375 (Insight on LeBoeuf et al. 2016, eLife 5:e20375) |
| **Soldier production is self-limiting.** In Pheidole, adult soldiers inhibit soldier development in larvae by a contact pheromone (Wheeler & Nijhout 1984; Rajakumar et al. 2018). Colonies make more soldiers when rivals are near | The colony's caste mix is a **feedback loop**, not a slider. The queen perturbs it; she doesn't set it | [S] https://experts.arizona.edu/en/publications/soldier-determination-in-pheidole-bicarinata-inhibition-by-adult- ; https://www.mcgill.ca/newsroom/node/86786 |
| **Brood pheromone stimulates foraging.** Adding it raised the share of pollen foragers within an hour, and lowered response thresholds in young bees | **Where and how much brood there is** changes how hard workers forage. Laying more is a way of *telling them to work* | [S] https://doi.org/10.1007%2Fs002650000282 ; https://bioone.org/journals/journal-of-economic-entomology/volume-97/issue-3/ |
| **Learning before adulthood.** Cataglyphis colony-brood recognition is "largely acquired during larval life and persists" through metamorphosis. Formica workers raised among foreign cocoons will tend them. A. senilis larval odour exposure shapes adult nestmate recognition | **Brood imprinting**: what the brood smells sets adult preferences | [S] https://pmc.ncbi.nlm.nih.gov/articles/PMC3843841 and Jaisson 1974 (Behaviour 52:1-37) via the IUSSI 1973 abstract |
| **Rearing temperature shapes adult brains.** Honey bee pupae raised at 36 °C learned and danced well; at 32-34.5 °C they did significantly worse. Survival and appearance were unaffected | **Where the brood lies (warm or cool)** sets adult quality. The engine already simulates heat | [S] https://pmc.ncbi.nlm.nih.gov/articles/PMC165877 (Tautz et al. 2003 PNAS) |
| **Teaching exists in ants.** Temnothorax *tandem running*: the leader goes ~4x slower and waits for the follower's antennal tap. Followers later lead others | **One-to-one, costly, cascading teaching**, not "everyone follow me". It is the biological answer to the owner's question | [S] https://www.bristol.ac.uk/news/2006/879.html ; https://antwiki.org/wiki/Tandem_Running |
| **Colonies can depose queens.** In fire ants, workers of one Gp-9 genotype kill queens lacking the allele (a "green beard"). Matricide is known in other species | The colony's *acceptance of the queen* can be heritable. The colony may evolve to ignore her | [K] Keller & Ross 1998, Nature 394:573, https://www.nature.com/articles/29064 |
| **Fungus-farming queens carry their crop.** A leafcutter (Atta) queen founds with a pellet of the mother colony's fungus in her mouth pocket | **She chooses what living thing the colony farms** | [K] |
| **Slave-making queens.** A Polyergus queen invades a Formica nest, kills its queen and is adopted. Stolen pupae imprint and work for the raiders | Captured foreign brood imprints on *your* colony: importing genes and skills | [K] |

**Design reading.** Real queens act on four things: **composition** (who is born),
**development** (what the born become), **imprint** (what the brood experiences), and
**gravity** (where workers cluster). Leading is the founding phase only. Every mechanism below
acts through one of those, plus the founding phase.

---

## 4. Candidate queen teaching mechanisms

Each entry gives: **Do** (the player's action) · **Changes** (what changes in the ants, and
over what timescale) · **See** (what the player sees) · **Fit** (with the owner's rules) ·
**Cost** (to build).

Fit is checked against four rules: graded, visible consequence, no hand-made castes, and one
brain design for every ant. Cost is relative to this engine: **S** is days, **M** is a week or
two, **L** is a project.

### (a) Genetics as teaching (composition)

**A1. Patriline store: "the fathers in her body."**
- **Do:** The queen carries 3-6 sealed "fathers": genomes stored at founding, from a mating
  flight, or from captured males. Each egg she lays is her genome crossed with one father. She
  can **pin** a father for the next clutch or let it rotate.
- **Changes:** A cohort that is half her and half that line. One brood cycle until it shows.
- **See:** Each patriline gets a colour fleck on eggs, larvae and adults. The colony visibly
  becomes stripier or more uniform. The ant page shows "daughter of the Ember line".
- **Fit:** Graded (mixtures). No castes: patrilines are genome lines, not roles. Real biology.
- **Cost:** M. Needs crossover (not just clone plus mutation), a store, and a fleck render.
  **This is the foundation the rest of the genetics sits on.** Without it, an egg carries
  only her genome and there is nothing to choose between.

**A2. "Take her line": adopting a worker as a father.**
- **Do:** The player walks to a worker whose life record impresses (most deliveries, survived
  the beetle raid) and performs a short mating/grooming act. Biologically this is fiction,
  since real queens don't take sperm from daughters, and it is presented as a *royal adoption*.
  It copies that worker's genome into a father slot, displacing one.
- **Changes:** The next clutch. Selection on observed phenotype, which is how real breeders work.
- **See:** The chosen worker glows briefly, gets a crown mark on its page, and its descendants
  carry its line name.
- **Fit:** A verb with a mark. Graded, because she only weights the next clutch.
- **Cost:** S on top of A1.
- **Risk:** Picking from 300 ants is hard. Pair it with A5 (the deeds ledger) or U3 (the dream).

**A3. Deed-weighted laying ("lay for the diggers").**
- **Do:** Instead of picking genes, the queen picks a *deed* to honour this season: deliveries,
  digs, larvae fed, beetles bitten, longest life. Father slots refill (or crossover weights
  shift) in proportion to each worker's score on that deed.
- **Changes:** Over 3-10 cohorts the colony drifts toward that behaviour.
- **See:** A deed banner over the nest. The page's "who is fathering now" list. Behaviour
  censuses on the colony page.
- **Fit:** Graded. Selection on behaviour, not genes. No castes.
- **Cost:** S-M. The life records exist.
- **Risk:** Goodhart (see U7). Ants evolve to *score* on the deed rather than help. This is a
  feature if surfaced.

**A4. Diversity breath (how many fathers per clutch).**
- **Do:** One gesture. A narrow clutch uses one father; a wide one mixes all of them.
- **Changes:** The *variance* of the cohort's traits. A wide colony is robust to surprises
  (Jones 2004); a narrow colony is efficient and brittle.
- **See:** Colour mix in the brood pile. A spread plot on the colony page: one fat histogram
  per walk gene.
- **Fit:** The purest graded lever in the list.
- **Cost:** S on A1.

**A5. The deeds ledger** (interface, not a mechanism).
- **Do:** A sortable roster by deed, plus "best of this cohort".
- **Changes:** Nothing by itself.
- **See:** It makes A2 and A3 humane at 300 ants.
- **Cost:** S. The data exists.

**A6. Egg-eating / policing.**
- **Do:** The queen (or a worker) eats an egg. She gets energy back; that line loses a member.
- **Changes:** Immediate.
- **See:** The egg disappears, she glows fed, the line counter drops by one.
- **Fit:** Real (worker policing, queen oophagy). A grim, satisfying verb with impulse and a mark.
- **Cost:** S. "Eat" exists; brood needs to count as edible to her.

**A7. Wild eggs (stress mutagenesis).**
- **Do:** In a crisis, she lays a "wild clutch" at high mutation width, which also widens the walk-gene widths that ship at 0.
- **Changes:** One cohort with large variation, mostly worse, occasionally a jewel.
- **See:** Oddly coloured larvae, odd behaviours, and a few shining outliers the ledger flags.
- **Fit:** Graded, visible, a gamble.
- **Cost:** S. The mutation knobs exist.

**A8. The Provision dial: what she hands the egg.** **This is in the engine already.**
- **Do:** Per egg (or per clutch), the queen hands a number: how much royal substance she
  invests, from her reserve. Mechanically this is `Provision`, stored as `made`.
- **Changes:** Each lineage's heritable *developmental block* decides what that number does
  to the child's body: bigger jaw, bigger crop, longer legs, slower metabolism. **The meaning of
  "a well-provisioned egg" is itself evolved**, and differs between patrilines.
- **See:** Big and small siblings in one cohort, a graded body-size spread like real nanitics
  versus majors. The page says "raised rich: jaw +30%".
- **Fit:** *"The whole of what makes a caste possible without naming one"* is a quote from the
  source. No castes are authored. Graded. Real (JH in trophallaxis; nutrition decides caste).
- **Cost:** S-M for the body slots. Extending it to the walk genes (`DEV_TRAITS` → 24) is a
  stated re-baseline.

**A9. The ancestor shelf as the royal memory.**
- **Do:** A dead champion's genome can be re-installed as a father from the specimen shelf.
- **Changes:** The next clutch.
- **See:** Graves become breeding stock.
- **Fit:** Answers Niche's "the good genes died" complaint.
- **Cost:** S. The shelf exists.

### (b) In-life teaching (development, imprint, demonstration)

**B1. Tandem running: teaching one pupil, who teaches the next.** **The most direct answer to "more than follow me".**
- **Do:** The queen taps a worker. It latches onto her, she walks a route, and it follows *only
  while it can still touch her*. Go too fast and it is lost.
- **Changes:** At arrival the pupil's existing **last-food memory** is written with that place,
  and optionally a short-lived "known route" bias toward it. A taught ant that later finds a
  nestmate can tandem-run *it* (a heritable propensity to lead).
- **See:** A chain of pairs spreading through the colony like a rumour. The pupil's page says
  "taught by the queen, day 4". Mistakes are visible: a lost pupil wanders.
- **Fit:** Graded (knowledge spreads to some, decays). A visible verb. Requires no weight
  plasticity, because it writes memory the ant already has. Real (Franks & Richardson 2006).
- **Cost:** M. A follow state, a touch check, a write to the memory slot, and a heritable "lead
  a tandem" output, or reuse of the recruit hooks.
- **Why it beats "follow me":** it is slow, one-to-one, skilful (pace yourself), and cascading. The
  lesson outlives your presence.

**B2. Royal feeding as reward ("the stroke").**
- **Do:** The queen feeds a worker from her crop by trophallaxis, *at the moment* it is doing
  something.
- **Changes:** The worker is fed (immediate), its *favour* tally goes up, and favour adds weight to its line in
  A3. With an opt-in plasticity module, a reward-gated nudge also strengthens the weights that chose its
  current option (the act is still on screen, so credit assignment is clean).
- **See:** A gold sheen on the favoured worker. Its walk changes if plasticity is on.
- **Fit:** The Black & White stroke done right. The reward lands on the act, not after it.
- **Cost:** S (favour only). L (plasticity). The plasticity also breaks "fixed at birth", a big
  decision for the owner.

**B3. The bite ("the slap").**
- **Do:** The queen bites or boxes a worker, as dominance biting does in real queenless ants such as Diacamma.
- **Changes:** The worker's energy drops, it is "chastened" for a while (it avoids the option it
  was taking), and favour drops. Bite too much and it dies, or the colony's alarm rises around you.
- **See:** A flinch and a knockback impulse; the worker scurries off.
- **Fit:** Dungeon Keeper's slap: a costly verb with physical feedback.
- **Cost:** S (the attack verb exists, not yet live).

**B4. Brood imprinting.**
- **Do:** Put something by the brood pile: a seed type, a plant's leaf, a fungus, a corpse of a predator, water.
- **Changes:** Each larva records the dominant smell around it while developing. As an adult it
  carries a small bias toward (or, for predator odour, against) that material. One brood cycle.
- **See:** "Clover-born" cohorts that graze clover. "Beetle-raised" cohorts that bristle at beetles.
- **Fit:** Graded (it depends on exposure). Real (Cataglyphis, Formica, honey bee scented food).
  No castes, and the brain design is unchanged: it is one heritable *receptivity* gene and one
  per-ant offset on an existing target-material feature.
- **Cost:** M. A per-ant offset is new state, but tiny.

**B5. Royal jelly / the nurses' ballot.**
- **Do:** Instead of A8's per-egg number, the queen seeds her substance into the food she gives
  nurses. The nurses then pass it on in proportion to what they have, so the colony's state votes.
- **Changes:** Larvae fed by well-supplied nurses get a higher `made`. A starving colony
  automatically raises small, cheap workers; a rich one raises large ones. *She tilts the ballot, she doesn't decide it.*
- **See:** Body-size distribution tracks the colony's fortunes.
- **Fit:** Very real (LeBoeuf). Graded. Emergent.
- **Cost:** M. `made` must be written at feeding rather than only at lay.

**B6. Presence damping (her pheromone field).**
- **Do:** Simply where she stands. A radius of queen scent.
- **Changes:** Within it, workers are calmer (alarm decays faster), stay nursing and digging
  longer, and workers don't lay. Outside it, or if she leaves, workers explore earlier, forage
  more, and some begin laying their own genomes.
- **See:** A soft halo. Traffic thins near her. When she goes out, the nest "wakes up".
- **Fit:** Real (QMP delays foraging; queen loss releases worker reproduction). Graded with distance.
- **Cost:** S-M. One new scent plane or a computed field, and one new input feature with
  inherited weights. A real cost: a new scent plane is frame cost.

**B7. Queen-scent words (invented language).** See U1. It counts as in-life (immediate
response) and genetic (meaning evolves).

**B8. Social copying** (workers copy the verb of a touched nestmate, and the queen is the most-copied).
- **Do:** Nothing directly. The queen is a high-prestige model.
- **Changes:** Workers sometimes copy the verb of a nestmate they touch.
- **See:** Behaviours spread through contact.
- **Fit:** Plausible.
- **Cost:** L. It needs plasticity, and is probably too opaque. Listed for completeness.

### (c) Structural teaching (gravity)

**C1. Moving the brood: the nest's centre of gravity.**
- **Do:** Carry eggs, larvae and pupae.
- **Changes:** Nurses follow brood, diggers dig near brood (dig company), store food near brood,
  and foragers' "home" effectively shifts. Immediate.
- **See:** The whole colony re-organises around the pile within a minute.
- **Fit:** Real (brood pheromone draws and stimulates). The cheapest, most legible lever.
- **Cost:** S. Carry exists; the brood scent exists.

**C2. Where she lays: warm, cool, deep, by the door.**
- **Do:** Choose the chamber for each clutch.
- **Changes:** Warm brood (near a lamp or sun-warmed soil) develops faster *and* adults come
  out sharper, meaning higher sense range or lower mutation noise. Cool brood develops slower and duller (Tautz 2003).
  Brood by the door stimulates foraging; deep brood is safe from the centipede.
- **See:** Thermal view of the nursery. Cohort quality on the report card.
- **Fit:** Uses the engine's heat field. Graded. Real. Ties the queen to the gardener systems
  (lamps).
- **Cost:** M.

**C3. Laying rate as a work signal.**
- **Do:** Lay faster or slower.
- **Changes:** More larvae means more begging, more foraging pressure and more nurses. Fewer
  means idle workers (the brief's "fed ants idle on the mound" becomes a consequence the player controls).
- **See:** Traffic density at the door rises within minutes.
- **Fit:** Real (brood pheromone). Graded.
- **Cost:** S.

**C4. First cuts are a template.**
- **Do:** In founding, the queen digs the first chamber and gallery by hand.
- **Changes:** The dig-company rule (workers cut only with company) means galleries extend from
  where digging already is. Her first architecture biases the whole nest's shape.
- **See:** The nest grows from her sketch.
- **Fit:** Real-ish. Graded.
- **Cost:** S. It exists as a consequence of the shipped rules; verify by measurement.

**C5. Doors and plugs.**
- **Do:** Pack spoil to close or open entrances (G already packs).
- **Changes:** Traffic re-routes; a new door makes a new foraging direction. Immediate.
- **See:** Streams of ants reflow.
- **Fit:** A visible consequence.
- **Cost:** S.

**C6. Founding middens and stores.**
- **Do:** Where she drops the first corpse, spoil or food.
- **Changes:** Where the colony keeps those things afterwards: the graveyard, the spoil heap, the larder.
- **See:** The colony's "habits" are a fossil of her first choices.
- **Fit:** The nest store and spoil-side rules exist.
- **Cost:** S-M.

**C7. The carried crop.**
- **Do:** At founding she carries a seed, spore or cutting into the nest and plants it in a chamber.
- **Changes:** Imprinting (B4) on that plant from cohort one. The colony comes to tend and graze
  its own garden.
- **See:** A glowing farm chamber; ants carrying water or soil to it.
- **Fit:** Real (Atta's fungus pellet). Ties the ant game to the plant game the owner wants
  ("a garden where ants and plants live together").
- **Cost:** M-L.

### (d) Colony-scale policy (each one a single verb, no sliders)

**D1. Male eggs.** Unfertilised eggs become males: they cost food and do no work, and are the
only way to export the line (mating flight, a new box, U4).

**D2. Daughter-queen permission.** Her scent strength allows or forbids gyne rearing. Weaken it
and the colony raises successors. Graded, risky, and the long arc.

**D3. Trophic eggs as currency.** Pay hungry nurses in eggs during famine (real: trophic eggs).
The choice is "eat the future to save the present".

---

## 5. What the queen might teach

### Expected behaviours (the player will aim for these)
- Forage to far food (the known failure: trained colonies starve when food is far). The levers are B1 tandem runs plus A3 "lay for deliveries".
- Dig a multi-room nest with a nursery, larder and midden (C1, C4, C6).
- Nurse properly: feed larvae, not loaf on the mound (C3, A3 "larvae fed").
- Defend: bite beetles, block doors before a centipede raid (B4 beetle-raised cohorts, A8 big-jaw provisioning, C5).
- Evacuate brood when rain floods the nest (in-life plus genetic; the player first does it by hand, C1).
- Store for the dark: day/night and weather cycles.

### Surprising behaviours that could emerge (the "unexpected" the owner asked for)
- **Gardening.** Clover-imprinted cohorts graze, but also drop spoil near clover. If spoil helps
  growth, the colony accidentally *fertilises its crop*. An unplanned mutualism.
- **Courtiers.** Under B2 favour, lines evolve to hang around the queen because that is where
  favour falls. A useless royal court appears. Real social insects have retinues; here it
  arises from the player's own reward habit. Funny and a lesson.
- **Goodhart's ants.** Under A3 "deliveries", workers evolve to deliver the *same food crumb
  back and forth*, or to carry food out and in. The player discovers their metric is gamed.
- **Brood thieves.** Beetle-raised cohorts with big jaws raid a rival colony's brood pile. Captured pupae imprint on you (U11).
- **Sentinels.** Diverse colonies (A4) produce a few low-threshold ants that bristle at the door all day: guards nobody designed.
- **Self-sealing.** Lines that plug doors every night, because the centipede came at night and the survivors were pluggers.
- **Queen-ignoring colonies.** If following the queen is costly (she leads into danger), lines that ignore her scent outcompete. A colony that needs you less is a *success state* (see below).
- **Mutiny.** If acceptance of the queen is heritable (U9) and she bites too much, workers stop feeding her.
- **Dwarf economies.** A famine colony with B5 on drifts to tiny, cheap, numerous workers. Recovery brings back the large ones. A visible boom-bust in body size.

### When genetics and in-life teaching meet
1. **The Baldwin effect: teaching becomes instinct** (the core progression; see U2). If the
   player teaches a behaviour in-life (tandem routes, imprinting) *and* selects the lines that
   learn it fastest, the learning gets cheaper each generation, until the colony does it
   untaught. **The player's lessons graduate into instinct, freeing her to teach the next
   thing.** That is a curriculum with a felt progression and no skill tree.
2. **Evolving teachability.** Receptivity genes (how strongly an ant responds to imprint,
   tandem or queen scent) are themselves selected. A colony bred under a hands-on queen becomes
   *more* teachable; one left alone becomes self-reliant and ignores her. Both are legitimate play styles.
3. **Meaning drift.** Under U1 the meaning of a scent word lives in the genome. If the player
   changes how she uses a word, the colony's meaning lags by generations, a mis-understanding
   with descendants. That is sad, funny and natural.
4. **Conflict.** Selection for *favour* (B2) and selection for *output* (A3) pull apart. The
   player sees two strategies inside one colony.

---

## 6. Slow breeding against fast play: what the player does minute to minute

Neither clock may wait for the other. The trick is to **make the fast clock produce inputs to
the slow one**, and to make the slow clock **deliver in visible instalments**.

| Clock | Duration | What the queen does | Payoff the player sees |
|---|---|---|---|
| **Body** | seconds | Walk; carry brood (C1); lay *this* egg, choosing its father (A1) and provision (A8); feed (B2) or bite (B3) a worker in the act; tap for a tandem run (B1); plug a door (C5); hide from a beetle | Immediate. Egg colour, worker reaction, traffic reflow, knockback |
| **Brood cycle** | minutes | Decide where the clutch lies (C2), what it smells (B4), how rich it is (B5), how wide (A4) | A **cohort report card** at emergence: name, father, imprint, body-size spread, first-day deeds |
| **Season / lineage** | tens of minutes to hours | Choose the deed to honour (A3); adopt champions (A2); open the dream (U3); export via males (D1) | Census drift, assimilated lessons (U2), new emergent behaviours, rival colonies |

Design notes:
- **Every egg is a micro-decision with a default.** Auto-lay handles them unless the player
  steps in, like Kingdom's coin: you *can* spend on each, and you are never forced to. That
  avoids Niche's micromanagement.
- **Make the generation time short in the lab.** Brood cycles of a few minutes at 1x, with time
  control. One "season" (5-10 cohorts) per 20-40 minutes of play.
- **Danger keeps the body clock honest.** The queen is mortal to predators (four beetles kill
  her every time today). Her body is precious, so every trip out is a decision, and B6 (her
  absence changes the nest) makes the trip matter twice.
- **The founding phase is fast-only.** The player does everything by hand (slice 0's scripted
  queen founds five times faster than her genome does). From about 20 workers on, the slow clock
  opens, which matches real claustral biology.

---

## 7. What makes the queen version boring, and what keeps it fun

### Boring
1. **The spreadsheet queen.** Gene sliders and ratio dials (SimAnt, Spore, Plague). Behaviour becomes a setting.
2. **Invisible teaching.** Any lesson whose effect is a 3% shift in a census (Creatures and Black & White's fate). The bar: the *individual* shows it, on screen, within a minute.
3. **Waiting.** Generations that take an hour before anything changes.
4. **Egg admin.** Choosing 300 fathers by hand.
5. **The colony is fine without you**, so you are a spectator. Or **useless without you**, so you babysit. Both kill agency.
6. **Unattributable outcomes.** The colony got better, but was it my selection, my imprint or the weather? Without paired evidence ("this cohort vs last") the player can't learn the system.
7. **"Follow me" with extra steps.** A queen-scent marker that just summons ants is Empires of the Undergrowth's marker. Avoid any lever that is a command with a delay.

### Fun past hours
1. **Discovery of evolved meaning.** What does *this* line's developmental block do with
   provision? What has the colony decided scent word 2 means? These are answers the designers
   don't know either (A8, U1).
2. **A curriculum you feel.** Lessons assimilated into instinct, so the queen's toolkit shifts
   from doing to shaping to steering (U2).
3. **Story.** Named lines, crowned champions, graves, the clover-born cohort, the mutiny of
   year 3. The repo already has the record-keeping.
4. **Antagonists that test the breeding.** The centipede raid, a rival colony, flood, famine.
   Each reveals whether your lineage has what it needs.
5. **Export and compare.** A mating flight sends your line to a new box. The specimen shelf
   holds colonies the way Creatures players traded Norns.
6. **Mechanism-design puzzles.** Goodhart's ants force the player to choose *what to reward*
   carefully. That is a real skill, and it deepens.

---

## 8. Genuinely unusual ideas

**U1. A queen-scent language whose meaning is not authored.**
- The queen has two or three distinct "royal marks" she can lay or emit. Each is a new scent
  plane, or channels of one plane, with brain inputs whose **initial weights are zero**. The
  marks start with *no meaning*.
- The player uses mark 1 consistently, say on food she wants fetched, and honours (A3/A2/B2)
  the workers who happened to respond.
- Over generations the weights on mark 1 grow. **The colony learns what the player means**, as
  instinct, without any lesson code.
- Different colonies end up with different dialects. If the player is inconsistent, the colony
  ends up with a confused, graded half-meaning.
- **Verdict:** this is the strongest "outside the box" answer, because it is real
  evolution-of-communication, and the owner's "one brain design" rule is what makes it work.
- **Risks:** slow; frame cost of a plane; needs mutation width on those weights.

**U2. The assimilation ladder (Baldwin, made visible).**
- Every in-life lesson (tandem routes, imprint, provision response) has a heritable "needs
  teaching less" gene, a receptivity or bias.
- The colony page shows each lesson as **taught → assisted → instinct**, measured as the share
  of untaught ants doing it.
- When one reaches instinct, the queen's time is free for the next. This is progression with no tech tree.

**U3. The dream: retrospective blessing.**
- When the queen rests (the brood-laying stretch), the player gets a fast **replay of the last
  cycle's notable events** from life records and the event log: the ant that dragged a beetle
  corpse, the one that dug the new chamber, the one that died rescuing larvae from the flood.
- The player blesses moments, and those workers' lines enter the father store.
- This solves the credit-assignment problem (you choose *the act*, not the ant at random) and
  the 300-ant attention problem in one interface.
- **Cost:** S-M for the interface over data that already exists.

**U4. Your own line becomes the rival.**
- At the end of a season the queen's daughters fly (D1/D2). The player chooses to follow one
  into a new box and found again, now with the *mother's* colony as a neighbouring rival
  generated from the same lineage.
- You compete with your own past choices. It is roguelite structure with inheritance, and it
  respects "the player shouldn't die of old age": the old queen lives on as the rival.

**U5. The carried crop / founding dowry** (C7).
- The queen picks what she carries at founding (a seed, a fungus, a beetle's egg?), and the
  colony's whole culture grows around it.
- Different dowries make different games: grazers, farmers, hunters.

**U6. Royal absence as a lever.**
- Leaving the nest *changes the nest*. Without her damping, the colony goes bold, explores and
  forages early, and some workers begin laying, so their genomes leak into the next generation.
- Coming home re-imposes order and *policing*: she can eat worker-laid eggs (A6), or keep them
  if she likes the layer.
- Absence becomes a strategic tool, and a source of genetic surprise.

**U7. Goodhart's colony** (as a designed feature, not a bug).
- A3's honoured deed gets gamed by evolution: back-and-forth deliveries, digging and refilling,
  courtiers. The ledger shows **"deeds that did nothing"** next to deeds that did.
- The player learns to reward outcomes ("food that reached a larva") over activity. This is a
  hidden second-layer skill that keeps a veteran engaged.

**U8. Eggs as messages.**
- She lays trophic eggs to pay or feed (D3). Laid in a chamber, they also mark it, because
  workers come to eat.
- She can **signal with eggs** where she wants work done. The signal is costly and honest.

**U9. Consent: the colony can depose her.**
- Workers' tolerance of the queen is a heritable trait, as in fire ants' Gp-9 green beard.
- A tyrant queen (constant biting, starving workers to feed herself) breeds a colony that stops
  feeding her, and eventually walls her in.
- The stakes are social rather than mechanical, from one inherited number.

**U10. Brood-temperature craft** (C2 pushed further).
- The queen *engineers the nursery*: drags brood toward warmth by day, away at night, and digs
  a chamber under a grow lamp.
- Adult quality tracks the care. It uses the engine's heat field directly and makes "a good
  mother" a physical, spatial skill.

**U11. Kidnapped pupae (dulosis).**
- The queen or her raiders steal a rival colony's pupae. Raised in your nest, they imprint on
  your colony (B4) and work for you, carrying *foreign genes and foreign instincts* in.
- If adopted as fathers (A2), their genes enter your line.
- It is a raid verb with a payoff that lasts generations.

**U12. Provision surprises as a collectible.**
- Each patriline's developmental block responds differently to provision. The ant page logs
  "Ember line, raised rich → long-legged runners; Moss line, raised rich → huge crops".
- Discovering them is a bestiary the player fills in, and the designers did not write it.

---

## 9. Ranked top 8, with build order

1. **B1 tandem running.** The direct answer to "more than follow me". Real biology. Writes
   the existing last-food memory, so no plasticity is needed. Cascades. M.
2. **A1 patriline store, plus A4 diversity.** The foundation: without it a laying queen has
   nothing to choose between. Visible colour flecks. M.
3. **A8/B5 provision: what she feeds the egg.** Already in the engine as `Provision` and `made`.
   Castes emerge without authoring, as a graded body-size spread. S-M.
4. **C1/C2/C3 brood as gravity, temperature and work signal.** The cheapest, most legible
   levers. Instant feedback, and ties to the heat and lamp systems. S-M.
5. **U3 the dream, with A3 deed-weighted laying.** Selection on behaviour at 300-ant scale.
   Solves credit assignment. S-M over existing life records.
6. **B4 brood imprinting, with C7 the carried crop.** Cohorts with character. Bridges the ant
   game and the garden. M.
7. **U1 the un-authored scent language.** The long-game showpiece. Slow, so it needs short
   generations and patience. M-L.
8. **U6 royal absence, with B6 presence damping and A6 egg-eating.** Makes her body's position a
   strategic, graded lever with genetic consequences. S-M.

Through-line: **U2 assimilation ladder** as the progression frame over 1, 6 and 7.

## 10. Biggest risks to fun

1. **The slow clock is too slow to feel.** If a lesson takes an hour of real time to show,
   players won't connect cause and effect.
   - Mitigation: short lab generations, cohort report cards, paired "this cohort vs last" readouts.
   - Every lever above lists an *individual-level* immediate sign as well.
2. **Teaching that doesn't visibly change the individual** (the Creatures and Black & White
   trap, and today's measured fact that a hand-laid trail moves nothing).
   - Each in-life lever must be **measured as a worker-level trace** before it ships.
   - Slice 0 showed the trail lever looked plausible and did nothing.
3. **Admin replaces play.** Choosing fathers, deeds and provisions for hundreds of eggs becomes
   a spreadsheet.
   - Mitigation: defaults everywhere; one verb per decision; selection by *blessing moments*
     (U3) rather than by stats; no sliders.
   - Close behind: **Goodhart's ants** gaming a deed metric in a way that looks like bugs
     rather than evolution, unless the interface shows it.

---

## Sources

Confirmed through search summaries (WebFetch was not used; claims are as summarised):
- Franks & Richardson 2006, Teaching in tandem-running ants, Nature 439:153: https://www.bristol.ac.uk/news/2006/879.html ; https://www.nature.com/articles/439153a ; https://antwiki.org/wiki/Tandem_Running
- Jones et al. 2004, Honey bee nest thermoregulation: diversity promotes stability, Science 305:402: https://pubmed.ncbi.nlm.nih.gov/15218093/
- LeBoeuf et al. 2016 eLife 5:e20375, and Insight e23375: https://elifesciences.org/articles/23375
- Queen pheromone reviews: https://link.springer.com/article/10.1007/s00265-023-03378-8 ; https://www.frontiersin.org/journals/cell-and-developmental-biology/articles/10.3389/fcell.2020.00837/pdf
- Pheidole soldier inhibition: https://experts.arizona.edu/en/publications/soldier-determination-in-pheidole-bicarinata-inhibition-by-adult- ; https://www.mcgill.ca/newsroom/node/86786
- Brood pheromone and foraging (Pankiw): https://doi.org/10.1007%2Fs002650000282 ; https://bioone.org/journals/journal-of-economic-entomology/volume-97/issue-3/
- QMP delays foraging onset (Pankiw et al. 1998): https://pherobase.com/literature/data/details/I/47938
- Preimaginal learning in ants: https://pmc.ncbi.nlm.nih.gov/articles/PMC3843841 ; https://www.dictionnaire-amoureux-des-fourmis.fr/UIEIS-IUSSI/Actes-SF-UIEIS/IUSSI-Londres-1973/IUSSI-Londres-1973-Jaisson.pdf
- Tautz et al. 2003 PNAS, pupal temperature and adult performance: https://pmc.ncbi.nlm.nih.gov/articles/PMC165877
- Claustral founding, trophic eggs, nanitics: https://antwiki.org/wiki/Colony_Foundation
- Sex ratio conflict and sperm use: https://www.sciencedaily.com/releases/2001/08/010820072111.htm ; https://pmc.ncbi.nlm.nih.gov/articles/PMC1690684
- Creatures: https://en.wikipedia.org/wiki/Creatures_(1996_video_game)
- Niche: https://www.killscreen.com/niche-survival-game-based-real-science-genetics/ ; https://steamcommunity.com/app/440650/discussions/1/5729109343960507982
- Majesty / Crown of Greed: https://www.gamespot.com/articles/majesty-legends-qanda/1100-6026166/ ; https://www.galaxus.de/en/page/gold-instead-of-obedience-crown-of-greed-tested-42293
- Kingdom: https://de.wikipedia.org/wiki/Benutzer:Calle_Cool/Kingdom_(video_game)
- Empires of the Undergrowth: https://store.steampowered.com/app/463530/
- Black & White (low-quality secondary): https://www.itechguides.com/how-black-whites-creature-learning-ai-and-morality-system-work/

From my own knowledge [K], not re-verified: SimAnt, Pikmin, Overlord, Dungeon Keeper, The Sims,
Banished, Songs of Syx, ONI, RimWorld, Rain World, Species ALRE, Plague Inc, Lemmings mechanics;
Keller & Ross 1998 (Gp-9 green beard, https://www.nature.com/articles/29064); Atta fungal pellet;
Polyergus dulosis; Diacamma dominance biting; Hölldobler & Wilson 1990.

Repository [E]: `src/sim/brain.rs:170-205` (Provision, developmental block, `DEV_TRAITS`);
`Reports/how-the-ant-works.md` §6d (walk-gain slots at width 0; trail inputs zeroed to the brain
under the chooser), §9/§12 (breeding rules); `Reports/play-as-ant-slice0-2026-10-11.md`.
