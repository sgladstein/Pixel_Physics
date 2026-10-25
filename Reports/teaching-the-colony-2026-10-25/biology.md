# How real ant behaviour changes, and how a player could pull on it

*A myrmecologist's catalogue for the Pixel Physics lab, 2026-10-25.*

## How to read this

**Sourcing.** Each claim carries a tag:

- **[S]** means I checked it in this session through web-search summaries of the paper (its abstract, its repository record, or press coverage of it). WebFetch was blocked, so I read no full texts. Numbers marked [S] come from abstracts or press releases. They are not taken from results tables.
- **[M]** means I am working from memory of the literature. The finding is well established, but I did not re-verify the exact figures. Check these before quoting them anywhere public.

**Fields.** Each mechanism has the same fields:

- **Account**: what happens in real insects.
- **Lever**: what the player does.
- **Player sees**: the visible result.
- **Timescale**: seconds, one ant's life, colony months, or generations.
- **Engine**: what the brief says exists today, and what is missing.

**★ marks a mechanism that would make players tell stories**: an emergent outcome nobody scripted.

### Three findings that reframe the owner's question

1. **In real ants, very little "teaching" works on an individual brain.** Most of it works on four other things:
   - the shared environment (scent and structures);
   - the colony's demography (who is alive, how old, how big);
   - development (what the larvae were fed);
   - the genome pool (who gets to breed).

   Individual learning is real and fast in ants. It is narrow, though: odour-to-value, place, route and view, and success at a task. That narrow set is enough. A player can steer a colony through every one of these channels, and only the first needs in-life learning in the brain.

2. **The colony remembers things no ant remembers.** A harvester-ant colony lives 25–30 years, while its workers live about a year [M, Gordon]. Wood-ant trail networks persist across winters and across worker generations [S, Rosengren]. So "teaching a colony" mostly means writing into durable colony state: trails, stores, the dump, the nest shape, the age mix, and the genome mix. It does not mean editing one ant.

3. **Three of the brief's current problems are textbook biology.** Each one shows which mechanism is missing:
   - *A hand-laid trail does not pull workers.* Real trail pheromone is a weak, graded modulator. Even when 20 nestmates had marked a branch, *Lasius niger* chose it only about 70% of the time, and route memory beats pheromone when the two conflict [S, Grüter, Czaczkes & Ratnieks 2011]. A trail works mainly in combination with memory.
   - *Colonies starve when food is far, because ants wander randomly.* The area a random searcher must cover grows as r². Real foragers beat this with path integration (a home vector) and a remembered food vector. Without those, distance kills.
   - *Fed ants idle on the mound.* This is normal: in *Temnothorax*, about 40% of workers are mostly inactive [S, Charbonneau, Sasaki & Dornhaus 2017]. What a real colony adds is an **activation signal**: foragers go out at a rate set by how often successful foragers come back in [S, Gordon lab]. The engine has no such signal.

   A fourth problem, the boom-bust population, is what you would predict when egg-laying is a gene that does not depend on the food store (§D2, §E).

---

## A. Individual learning, within one ant's life

### A1. Odour–food association, often in one trial ★

**Account.** *Lasius niger* foragers learn a new odour–sucrose pairing in a single visit (Czaczkes & Kumar 2020, as cited in a 2024 *Behav Ecol Sociobiol* paper [S]). The learning is strongly order-dependent. Ants trained on two odours prefer **the one they met first**, and one unscented familiarisation visit beforehand weakens or reverses that preference. The authors call this a "fragile primacy effect" [S, Oberhauser, Bogenberger & Czaczkes 2022, J Exp Biol, https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10088526/]. Trail pheromone did not change what the ants learned in that study [S]. Carpenter ants (*Camponotus fellah*, *C. mus*) form odour memories under laboratory conditioning that last days [S, Josens et al. 2009 J Exp Biol; Dupuy et al. 2006]. In honeybees, one pairing gives a short-to-mid-term memory and about three spaced pairings consolidate a long-term memory [M, Menzel's proboscis-extension work].

**Lever.** Food carries a smell, and the player chooses it: berries smell of mint, seeds of resin. The player decides which flavour the colony meets first and how it is paired: with sweet reward, with nothing, or with harm (§A6).

**Player sees.** After one feeding, foragers walk straight past unscented food and veer toward anything that smells like the first reward. The first lesson sticks hardest, so a colony that started on mint keeps preferring mint even when a better food is on offer. Every lineage acquires a "taste" that depends on its history.

**Timescale.** One visit to learn, days to keep.

**Engine.** Nothing yet. There is no in-life learning and no odour identity: food is one material, and the planes are A, B and alarm. The cheap version is a few odour channels on food cells, and per ant a small table of K odour valences that is updated on reward or harm and fed to the network as an input feature. That costs bytes per ant, not plastic weights.

### A2. Value is relative: incentive contrast ★

**Account.** Ants trained to expect low-quality food accept medium food more readily than ants trained to expect medium food. Ants that expect high quality accept medium food *less*. The effect is cognitive, not sensory, and it also changes how much pheromone they lay. Food received *from nestmates* sets the same reference point [S, Wendt, Strunk, Heinze, Roider & Czaczkes 2019, eLife, https://elifesciences.org/articles/45450].

**Lever.** What you fed them yesterday sets what they will work for today.

**Player sees.** Spoilt colonies. Feed rich syrup for a week and then offer ordinary seeds: foragers sniff, refuse, and trails fade. Start a colony on poor food and it works hard on anything. This is "teaching" through expectation. The effect is graded, and it spreads by feeding nestmates.

**Timescale.** Visits to days.

**Engine.** Missing. It needs a per-ant running average of reward. One float per ant is enough.

### A3. Colour and visual pattern learning

**Account.** Honeybees learn colours, violet fastest, sometimes in a single trial, and also learn shapes and patterns [M, Menzel 1967; von Frisch]. Many ants learn visual cues as well, and desert ants rely on them heavily.

**Lever.** Coloured lamps or markers at food. This only matters for **sighted lineages**, and sight is heritable in the engine.

**Player sees.** A sighted sub-lineage that uses the red lamp as a beacon while blind sisters smell their way. This is a visible reason for sight to spread under selection.

**Timescale.** Visits.

**Engine.** Sight exists as a heritable trait. Visual learning is missing.

### A4. Landmark, route and view learning; learning walks ★

**Account.** Desert ants (*Cataglyphis*, *Melophorus*) learn routes as sequences of remembered views. Before their first foraging trip, naive ants do structured **learning walks** around the nest entrance, turning back to "look" at it [S, review in J Comp Physiol A 2022, https://link.springer.com/10.1007/s00359-022-01600-y]. When the views on a test route differ from the training views, ants meander more [S]. Wood ants keep individual routes for weeks. The classic demonstration is Tinbergen's pine-cone ring around a digger-wasp burrow: move the ring and the wasp lands in the middle of the moved ring, not at its burrow [M, Tinbergen & Kruyt 1938]. The equivalent experiments have been done on ants.

**Lever.** The player places stones, plants and lamps, which become landmarks. Moving a landmark is the experiment.

**Player sees.** New foragers circling the nest mouth before their first trip. Later, the player moves the boulder beside the nest, and returning foragers search confidently around the **boulder**, not the hole. That is the moment the player realises the ants were using the boulder.

**Timescale.** One walk to learn, weeks to keep.

**Engine.** Missing. Ants are blind by default. A scent-only analogue is a "familiar-smell" memory: a hashed snapshot of nearby material and scent near the route. A sighted lineage could use the real thing.

### A5. Path integration: the home vector ★ (buildable now)

**Account.** Desert ants keep a running vector home. They measure direction with a sky compass and distance by **counting steps**. Ants given stilts overshoot the nest and ants with shortened legs stop short [S, Wittlinger, Wehner & Wolf 2006, Science 312:1965]. An ant picked up and moved runs its full home vector into empty ground and then starts a systematic spiral search [M, Wehner; Müller & Wehner 1988]. The vector memory decays within a day or two; landmark memory lasts much longer, on the order of weeks [M, Ziegler & Wehner 1997].

**Lever.** **Fling an ant**, a verb the player already has.

**Player sees.** The flung ant gets up, runs a confident straight line to the patch of ground where home *would* be, stops, and spirals outward until it hits the trail. It is the most famous experiment in insect navigation, available as a toy.

**Timescale.** Seconds to minutes.

**Engine.** There is a "direction home" feature. If it is computed from true nest geometry, it is omniscient and cannot be fooled by a fling. Replacing it with a per-ant integrator (two floats, updated each step, reset on entering the nest) gives a real vector, the fling story, **and** the main fix for far-food starvation: a remembered food vector lets the ant go back.

### A6. Aversive learning, one-trial and retrospective ★

**Account.** *Melophorus bagoti* and *Cataglyphis fortis* running a familiar route fell into a hidden pit trap. On the very next run, some ants stopped, scanned, and detoured. The views seen **a few seconds before** the fall were associated retrospectively with it ("trace conditioning") [S, Wystrach, Buehlmann, Schwarz, Cheng & Graham 2020, Curr Biol 30:1927, https://www.sussex.ac.uk/lifesci/insectnavigation/documents/wystrach-et-al-curr-biol-2020.pdf]. Leaf-cutting ants learn to avoid a plant that harmed their fungus: delayed rejection develops within about 24 h. Foragers still avoided the plant after 9 weeks (*Acromyrmex*, Herz et al. 2008) and **16 weeks** (*Atta colombica*, Saverschek et al. 2010), longer than many foragers live [S, via Falibene, Roces & Rössler 2015, https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4389540/]. Colonies can also learn this avoidance **entirely from waste at the dump** [S, Arenas & Roces 2017, PLOS One].

**Lever.** Hazards the player places or causes: a water pit, a hot lamp, a spider ambush, fire, a plant made bitter.

**Player sees.** Ants skirt the place where nestmates drowned. The colony stops harvesting the plant the player sprayed, and keeps refusing it for months, after every ant that was there has died.

**Timescale.** One trial to learn, months to keep at colony level.

**Engine.** Alarm scent exists as a short-lived spatial signal. Learned aversion is missing, and so is any colony-level carrier (the dump, the store) that would let it outlive individuals.

### A7. Memory duration, consolidation, extinction and forgetting

**Account** [mostly M, honeybee and ant conditioning literature]:

- **Consolidation.** Spaced repetition consolidates memory; massed repetition is weaker.
- **Extinction is new learning, not erasure.** An association left unrewarded fades, then partly returns after a pause ("spontaneous recovery").
- **Collective forgetting.** Temnothorax colonies get faster over successive emigrations. They **forget** if the gap between emigrations is long, and keep the gain longer if it was repeated several times [S, Langridge, Franks & Sendova-Franks 2004, Behav Ecol Sociobiol 56:523].
- **Different memories decay at different rates.** Desert ants' vector memory fades in about a day, while landmark memory lasts weeks [M].

**Lever.** The rhythm of the player's interventions. One-off lessons fade; spaced repeats stick. Stop baiting a spot and the habit extinguishes, then flickers back.

**Player sees.** A fading trail of habit, and a "they remembered!" moment when the player returns to an old feeding spot after a break.

**Timescale.** Hours to weeks.

**Engine.** Missing. Any plastic per-ant value in A1, A2, A6 or A8 needs a decay constant and a reinforcement rule. Two constants per value are enough.

### A8. Success makes a forager and failure makes a nurse ★ (central for "roles emerge")

**Account.** In the clonal raider ant, workers that were **repeatedly allowed to find prey** became strongly exploratory foragers. Workers that **repeatedly failed** reduced foraging. A month later the successful ants were still foraging and the failures had moved to brood care. Experience alone generated a lasting division of labour among genetically identical ants [S, Ravary, Lecoutey, Kaminski, Châline & Jaisson 2007, Curr Biol 17:1308]. Response-threshold models with self-reinforcement predict this: performing a task lowers your threshold for it [M, Theraulaz, Bonabeau & Deneubourg 1998].

**Lever.** **Rig success.** Put food in front of particular ants (a "favourite"), or make certain ants fail.

**Player sees.** A specialist the player made. The ant fed at the first feeder becomes a lifelong scout, while the timid one that never found anything spends its life on the brood pile. With one shared brain design, this produces castes the owner did not hand-make: the roles come from history. The life record would show the divergence.

**Timescale.** One ant's life, with lasting effects.

**Engine.** Missing, and this is the highest-value addition. Per ant, add a few task-bias scalars (forage, dig, nurse) that rise with success at that verb class and decay slowly, fed to the network as context senses. Evolution then decides how much to listen to them.

### A9. Time–place learning: the colony knows dinner time ★

**Account.** *Ectatomma ruidum* ants learn the time of day and the place where food appears, and turn up there before it arrives [M, Schatz, Beugnon & Lachaud 1994]. Honeybees learn feeding times; this was Beling's "Zeitgedächtnis", 1929 [M].

**Lever.** Feed at the same hour of the lab's day/night cycle.

**Player sees.** Ants gathering at the feeding spot *before* the player drops the food. This is the clearest possible proof to a player that the colony learned something.

**Timescale.** Days to learn.

**Engine.** The day/night cycle and lamps exist. Learning is missing. The cheap version is a per-ant phase-of-day estimate of when food was last found, given to the network as a feature.

### A10. Tool use: soil grains and sponges as cups ★

**Account.** *Aphaenogaster* ants drop absorbent debris into liquid food and carry it home soaked. They choose materials by how easy they are to carry and how well they absorb, and they **learn to prefer** unfamiliar artificial materials that work better, such as sponge and paper over native soil grains [S, Maák et al. 2017, Animal Behaviour].

**Lever.** Offer liquid food (spilt nectar or syrup) next to powders of different absorbency.

**Player sees.** Ants ferrying soaked grains of sand back to the nest. A falling-sand engine is nearly unique in being able to show this honestly.

**Timescale.** Minutes to see; learning the best material takes hours.

**Engine.** Liquids, powders and wetness all exist, and so do the pick-up and drop verbs. The missing parts are a "soaked grain carries food" rule and a reward signal for it. **Very buildable, very visible.**

---

## B. Social learning and information transfer

### B1. Tandem running as teaching ★

**Account.** In *Temnothorax albipennis*, a knowledgeable ant leads a naive one to food or a new nest. The follower taps the leader's legs and abdomen with its antennae. The leader waits if the taps stop and goes on when they resume. A hair tapping the leader at 2 Hz or more is enough to keep it going [S]. The leader pays a cost, reaching the goal about **4 times slower** than alone, and the pupil benefits, finding food in about **two-thirds** of the time it would take alone. This was the first claimed case of teaching with two-way feedback in a non-human animal [S, Franks & Richardson 2006, Nature 439:153, https://www.nature.com/articles/439153a]. Followers often become leaders, so the knowledge cascades. "Reverse" tandem runs also exist, leading ants back to the old nest [M].

**Lever.** The player-queen (or a favoured ant) leads one ant to a site, slowly. That ant leads another.

**Player sees.** Pairs of ants walking nose-to-tail, stop-start. Leadership chains spread out from the player's first lesson like a family tree, and the per-ant life record can show who taught whom.

**Timescale.** Minutes per run. The cascade takes hours.

**Engine.** The player-queen body exists but is switched off. There is no "follow the nestmate ahead" option and no "wait for follower" behaviour. Adding them needs a target-option type (a nestmate) and a sense for "a nestmate is touching me from behind".

### B2. Trail recruitment, its weaknesses and its negative signal ★

**Account.**

- **Shortest-path selection.** Mass-recruiting ants lay graded trails. Argentine ants pick the shorter branch of a double bridge because ants on it return sooner and reinforce it first [M, Goss et al. 1989]. *Lasius niger* foragers lay about 43% more marks for rich food than for poor food, which alone is enough to select the richer source [S, Beckers et al. 1990].
- **Lock-in.** If a poor source is found first, *Lasius niger* **stays locked on it** even after a rich one appears [S, secondhand account of Beckers, Deneubourg, Goss & Pasteels 1990].
- **Memory beats pheromone.** Naive followers obey a marked branch only 62–70% of the time, and route memory beats pheromone when they conflict [S, Grüter et al. 2011]. Memory and pheromone otherwise work together [S, Czaczkes et al. 2011 Biol Lett].
- **A "no entry" signal.** Pharaoh's ants lay a **repellent** pheromone at the start of unrewarding branches. It lasts 78 min, against 33 min for the attractive trail [S, Robinson, Jackson, Holcombe & Ratnieks 2005, Nature 438:442; Robinson et al. 2008].

**Lever.** Lay trail. The biology says this works best on ants that already have memory of the place. A "no entry" stroke that blocks a branch would be a second verb.

**Player sees.** Trails that strengthen to rich food and fade from poor food. A colony stubbornly loyal to the first mediocre feeder (a story). A "no entry" mark that sends a column zigzagging away.

**Timescale.** Minutes.

**Engine.** Planes A and B exist; the measured fact that a hand-laid trail does not pull is consistent with biology, where pheromone without memory is weak. A negative plane and **reward-graded deposition** are missing. If deposition is not graded by food value, colonies cannot pick the better source.

### B3. Feeding nestmates passes on information ★

**Account.** Carpenter ants that received scented sugar **only by mouth-to-mouth feeding** later chose that odour in a Y-maze. They learned a food they had never visited [S, Provecho & Josens 2009, J Exp Biol 212:3221]. Honeybees learn nectar scent the same way [M, Farina et al.]. The fluid passed between ants also carries hormones and proteins, including juvenile hormone, so it can shift development and roles [M, LeBoeuf et al. 2016 eLife].

**Lever.** Flavour one food source. Ants that stayed home learn its scent through the crop, then search for it when they go out.

**Player sees.** Ants that never went to the mint patch heading for mint. Knowledge spreading through feeding, so the colony "knows" more than any forager visited.

**Timescale.** Minutes to hours.

**Engine.** The feed verb and the crop exist. This needs only an odour tag on crop contents plus A1. **Cheap once A1 exists.**

### B4. Imitation and social learning beyond trails (bees, with care for ants)

**Account.**

- **Social learning beyond individual reach.** Bumblebees learned a two-step puzzle box **only from trained demonstrators**. About a third of observers succeeded, while bees alone failed for up to 24 days, so the behaviour was "too complex to innovate alone" [S, Bridges et al. 2024, Nature 627:572].
- **A signal that must be watched to be learned.** Honeybees that never followed a dance danced with more angle and distance errors. Angle errors improved with practice, but **distance coding stayed wrong for life** [S, Dong, Lin, Nieh & Tan 2023, Science].
- **Other bumblebee social learning.** Bumblebees also learn string-pulling socially (Alem et al. 2016) and learned "ball-rolling" by watching (Loukola et al. 2017) [M].
- **Ants.** True observational imitation is weakly evidenced; their social learning runs through tandem runs, trails and feeding.

**Lever.** A demonstrator: the player trains or picks one specimen (the shelf already clones individuals) and introduces it into another colony.

**Player sees.** A technique, such as tool use (§A10), spreading outward from one introduced ant.

**Timescale.** Hours to days.

**Engine.** Missing. A cheap proxy is "bias toward the verb a nearby successful nestmate just performed" as a feature.

### B5. Patrollers choose the colony's doorway ★

**Account.** Each morning, red harvester-ant patrollers mark a sector of the mound of about 20 cm with Dufour's gland secretion, and foragers leave along trails from that sector. Blocking patrollers reduces use of that trail, and painting Dufour's extract back on restores it. **With no patrollers, foragers use yesterday's direction** [S, Greene & Gordon 2007, Am Nat]. Glass beads coated with the cuticular hydrocarbons of task groups, dropped into the nest entrance, are enough to start foraging [S, Greene & Gordon, press coverage; M, Greene & Gordon 2003 Nature].

**Lever.** Mark which side of the mound the colony leaves by. Drop a "scented bead" (a token cell) into the nest entrance to wake the foragers.

**Player sees.** The whole day's traffic pivoting to the side the player marked, and a dead-calm mound erupting when the token drops in.

**Timescale.** Seconds to a day, with day-to-day memory.

**Engine.** Scent can be laid, but there is no "task-group smell" and no activation by contact. See C3.

---

## C. Task allocation: who does what

### C1. Response thresholds

**Account.** Each worker has a threshold for each task stimulus, such as larval hunger or spoil in a tunnel. Variation between workers, partly genetic and partly experiential, gives a smooth, robust division of labour. Removing the workers that respond pulls in the next-lowest threshold [M, Robinson 1992; Bonabeau, Theraulaz & Deneubourg 1996]. A self-reinforcing version (thresholds fall with practice) is A8.

**Lever.** Create a stimulus, such as a spoil pile, a crowd of hungry larvae or a dead beetle, and watch who answers it.

**Player sees.** Graded mobilisation. A small mess recruits two ants; a disaster recruits fifty.

**Timescale.** Seconds.

**Engine.** Implicit: the network scores stimuli, and thresholds are genetic weights. Graded mobilisation should already emerge if genomes vary. The reinforcement in A8 is missing.

### C2. Age polyethism and spatial fidelity

**Account.** In *Camponotus fellah*, tracking about 9 million interactions showed workers falling into three spatial and social groups: nurses near brood, cleaners in between and near the refuse, and foragers near the entrance. Workers move through them **as they age** [S, Mersch, Crespi & Keller 2013, Science]. In honeybees, older foragers inhibit young bees from maturing. Remove the foragers and young bees forage precociously; remove the nurses and some foragers revert [M, Huang & Robinson 1992; Robinson 1992].

**Lever.** Demography. Fling away the foragers, or add a pile of young.

**Player sees.** After a predator raid on the foragers, very young ants appear outside early. Bands of the nest (the brood pile, the middle, the door) have their own population.

**Timescale.** Days, or a lifetime.

**Engine.** Ants age. It is unclear whether age is a sense the network sees; it should be. Social inhibition, where the forager count in contact slows maturation, is missing.

### C3. Encounter rate: returning foragers set the pace ★ (fixes "idle on the mound")

**Account.** In red harvester ants, the rate of brief antennal contacts between returning successful foragers and waiting foragers, in the entrance tunnel, sets how many go out. Both activation (seconds) and availability (minutes) rise with return rate [S, Pinter-Wollman et al. 2013, https://pmc.ncbi.nlm.nih.gov/articles/PMC3767282; Prabhakar, Dektar & Gordon 2012 PLoS Comp Biol, https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3426560/]. No spatial information is needed: a Poisson return process plus contact counting reproduces the data [S]. The same logic, an "unloading wait", regulates honeybee foraging: if receivers are slow, foragers do a tremble dance and recruitment drops [M, Seeley 1992].

**Lever.** Put food close to the entrance so returns come fast, and the colony ramps up foraging. Put it far away and the colony throttles back.

**Player sees.** Foraging that breathes with success: a trickle on a bad day, a flood on a good one, and an idle mound that wakes up when the first forager comes home loaded.

**Timescale.** Seconds to minutes.

**Engine.** "Crowding" and "nestmate" exist. Missing is a per-ant decaying **count of recent contacts with laden returners**, given as a context sense. It is cheap and likely the single most direct fix for idle fed ants.

### C4. Lazy ants are a reserve workforce

**Account.** About 40% of *Temnothorax* workers are mostly inactive. Remove the 20% most active and the inactive ones replace them within about a week. Remove the inactive ones and **nobody replaces them** [S, Charbonneau, Sasaki & Dornhaus 2017, PLOS One, https://www.ncbi.nlm.nih.gov/pmc/articles/PMC5587300/]. Models suggest that reserves protect long-term persistence [M, Hasegawa et al. 2016].

**Lever.** Remove workers.

**Player sees.** Loungers springing into action after a disaster.

**Timescale.** Days.

**Engine.** Idling already exists. What is missing is that idle ants respond when need rises, which C3 and C1 supply.

---

## D. Nutrition and development: the body is taught before it hatches

### D1. Larval feeding decides size and caste ★

**Account.** In most ants, whether a female larva becomes a queen, a minor worker or a major worker is decided by **nutrition and hormones**, not genes.

- In *Pheidole*, juvenile hormone during a late larval window switches a larva to the soldier path [S, Wheeler & Nijhout 1981 Science].
- **Adult soldiers inhibit** soldier development through a contact pheromone. Fewer soldiers means more soldiers get made: a thermostat [S, Wheeler & Nijhout 1984, J Insect Physiol 30:127].
- *Pheidole pallidula* colonies exposed through a mesh to **foreign conspecific colonies** make more soldiers. This was the first experimental adaptive caste-ratio shift [S, Passera, Roncin, Kaufmann & Keller 1996, Nature 379:630].
- Juvenile-hormone treatment of species that have no supersoldier caste produces **supersoldiers**. The potential is ancestral and dormant [S, Rajakumar et al. 2012, Science].
- Honeybee queens come from royal-jelly feeding, and knocking down DNA methylation produces queens [M, Kucharski et al. 2008].

**Lever.** What and how much larvae are fed: the player feeds them as queen, or provisions the store as gnome. Exposure to enemies makes more soldiers.

**Player sees.** Big-headed workers appearing after the colony is fed protein-rich prey, or after a neighbour colony or the centipede has been pressing the border. Remove the big ones and the colony makes more. These are graded bodies, not a binary caste switch.

**Timescale.** One brood cycle, from days to weeks.

**Engine.** Larvae are fed, and body genes (size and others) are fixed by the genome. Missing: body size = genome + integral of larval food (+ an alarm-exposure term). This respects "one brain design, no hand-made castes" because the body varies, the brain is the same, and the brain's inputs include its own size. That is the real mechanism by which ant castes emerge.

### D2. Colony nutrition and the protein trap ★

**Account.** Colonies regulate protein and carbohydrate intake as a unit. In many ants the larvae digest protein and feed workers back (Dussutour & Simpson 2009 Curr Biol [M]). Forcing *Lasius niger* workers onto high-protein, low-carbohydrate food **cut worker lifespan up to tenfold**, and a single day of it shrank colonies by more than 20% [S, Dussutour & Simpson 2012, Proc R Soc B 279:2402, https://pmc.ncbi.nlm.nih.gov/articles/PMC3350685]. In fire ants, workers feed larvae in proportion to larval hunger signals, and brood production follows intake [M, Cassill & Tschinkel 1995].

**Lever.** Diet choice: sugar plants against prey and insect carcasses.

**Player sees.** A colony that balances its diet, and one the player overfeeds on meat that collapses, its foragers dying first. A cautionary tale players will repeat.

**Timescale.** Days to weeks.

**Engine.** There is apparently one food material. Two nutrients would be needed, and the engine has carcasses (corpses) and plants. **Coupling egg-laying to the store is likely the boom-bust fix**: in real colonies, brood output tracks intake.

### D3. Early imprinting: who your family is ★

**Account.** Newly emerged workers learn the colony odour template that defines "nestmate". Pupae moved into another colony, even another species, grow up as loyal workers of the colony that raised them [M, Jaisson 1975; Carlin & Hölldobler 1983, mixed-species colonies]. Slave-making exploits exactly this (§I3).

**Lever.** Cross-foster pupae between colonies with the specimen shelf.

**Player sees.** A colony of two lineages living as one, with different genomes and one colony smell.

**Timescale.** Imprinting takes the first days of adult life; the effect lasts a lifetime.

**Engine.** "Nestmate or stranger" exists. If it is a genome comparison, cross-fostering cannot work. If it is a learned colony-odour tag, it can.

### D4. Living larders: repletes

**Account.** In honeypot ants (*Myrmecocystus*), some large workers hang from the ceiling as living storage, swollen with crop fluid fed to them by others, and give it back in lean times [M]. They are raided and stolen in wars (§I1).

**Lever.** A glut of sugar.

**Player sees.** Grape-sized ants hanging in a room.

**Timescale.** Weeks.

**Engine.** A crop and a colony store exist. A physical replete, an ant as a storage cell, is missing.

---

## E. The queen: what she does and does not do

**Account.**

- **She signals; she does not command.** Queens signal fertility with cuticular hydrocarbons that **suppress worker ovary development**. In *Lasius niger* the compound is 3-methylhentriacontane. Its synthetic form inhibits queenless workers' ovaries, it falls when the queen's immune system is challenged, and it coats her eggs [S, Holman, Jørgensen, Nielsen & d'Ettorre 2010, Proc R Soc B 277:3793]. Related hydrocarbons are conserved queen signals across ants, wasps and bees [M, Van Oystaeyen et al. 2014 Science].
- **Policing.** When a worker does lay (haplodiploid workers can lay male eggs), nestmates often eat the eggs [M, Ratnieks & Visscher 1989, honeybees].
- **She does not direct work.** "The queen is not in charge" is Gordon's standard line [M]. She sets the brood supply, and through brood she sets the demand that workers respond to.
- **Workers can kill her.** In the polygyne fire ant, a "green-beard" supergene makes workers carrying the *b* allele kill BB queens, which they recognise by a transferable odour [S, Keller & Ross 1998, Nature 394:573].
- **Queenless systems.** In some ponerine ants there is no queen caste. Workers duel to become mated egg-laying "gamergates" (*Harpegnathos*), and their brains and ovaries change reversibly with status [M, Penick et al. 2021].

**What this means for the engine.** Today every ant may lay (a gene), which makes the game biologically closest to **gamergate ants**. That is legitimate. A queen-player design changes the lever:

1. The queen's presence or pheromone lowers laying in others, giving one reproductive and a sterile workforce.
2. She **chooses which eggs to lay**, which is selection (§L).
3. Through brood amount she sets demand.

**Lever.** As queen: presence, pheromone and laying. As gnome: remove or add a reproductive.

**Player sees.** When the queen dies or is moved away, workers start laying and fighting over it. When she returns, it settles. A queen that "goes sick" (her signal drops) and is executed by her own workers.

**Timescale.** Hours (the signal) to generations (the laying).

**Engine.** Laying is genetic and per-ant; larvae inherit the layer's brain. Queen pheromone, policing and suppression are missing.

---

## F. Collective decisions and colony personality

### F1. House-hunting by quorum ★

**Account.**

- **Quorum.** *Temnothorax* scouts assess candidate nests and recruit nestmates by tandem run after a delay that is shorter for better sites. When an ant's **rate of encounters** at the site passes a quorum, it switches to carrying nestmates directly, which is about three times faster. Ants prevented from touching nestmates recruited as if the site were empty [S, Pratt 2005; Pratt, Mallon, Sumpter & Franks 2002].
- **Area by Buffon's needle.** Scouts measure nest area by laying an individual trail on one visit and counting how often their path crosses it on the next [S, Mallon & Franks 2000, Proc R Soc B 267:765].
- **Groups are better at hard choices.** Colonies are more "rational" than individual ants on hard choices [M, Sasaki & Pratt 2011, 2012].
- **Practice.** Colonies get faster with practice (§A7).
- **Search effort tracks the current nest.** Colonies in poor nests scout more than colonies in good ones [S, Doran & Franks 2013, Royal Society news, https://royalsociety.org/news/2013/ants-investigate-housing-market].

**Lever.** Build a better home (dry, dark, the right size) nearby, or wreck the current nest.

**Player sees.** The colony **moving house**: tandem pairs, then a rush of carried nestmates and brood, a quorum tipping point, and sometimes a split. Repeated moves get faster.

**Timescale.** Hours.

**Engine.** Digging and rooms exist. Emigration does not, and carrying an adult nestmate is not a verb. "Nest dug" being a fitness term means the colony is never asked to choose.

### F2. Colony personality and heritable collective behaviour ★

**Account.** Colonies differ consistently in aggression, exploration and foraging restraint [M, review by Bengston & Jandt 2014; Wray, Mattila & Seeley 2011 on honeybees, linked to fitness]. Red harvester colonies that **hold back foraging on dry days** have more offspring colonies, and that collective restraint is **heritable from parent colony to daughter colony** [S, Gordon 2013, Nature 498:91, https://www.nature.com/articles/nature12137]. (Caution: some widely cited colony-personality papers by J. Pruitt were retracted. Do not use them.)

**Lever.** Breeding colonies: choose which colony founds the next one, or as queen, which lineage to lay.

**Player sees.** Each lineage has a temperament visible in a minute of watching: bold or cautious, explorers or homebodies. The generated lineage names would carry a reputation.

**Timescale.** Generations.

**Engine.** A colony is a genome pool, and colony selection already happens offline. Exposing it to the player is the lever.

---

## G. Traditions: colony memory that outlives every ant

**Account.**

- **Wood-ant route traditions.** *Formica* foragers keep individual site fidelity to trails and areas, and keep it **across winter dormancy** [S, Rosengren et al., https://zenodo.org/record/17379]. Popular accounts of Rosengren describe older foragers walking the habitual route with younger ones in spring, after which the young keep the route [S, Smithsonian, https://www.smithsonianmag.com/science-nature/ant-colonies-retain-memories-outlast-lifespans-individuals-180971022/].
- **Harvester-ant direction memory.** Harvester foragers default to yesterday's direction [S, §B5]. Colonies keep their foraging areas for decades while workers turn over yearly [M, Gordon].
- **Bees.** The bumblebee puzzle box and the honeybee dance both show socially transmitted behaviour (§B4).

**Lever.** **Early investment.** The trails and feeding spots the player establishes in a colony's youth are still in use long after the founding generation has died.

**Player sees.** A route around a boulder that is no longer there, still walked. A detour around a pit filled in long ago (§A6). This is a "why do they go that way?" story with a real answer.

**Timescale.** Generations of workers.

**Engine.** Scent planes decay and the food memory is per-ant, so nothing carries over. It needs tandem/follow (B1), plus trail substrates that persist (worn paths, i.e. compacted soil, are a physical memory the engine could represent as a material).

---

## H. Farming and husbandry: the colony works the player's garden

### H1. Fungus gardens and learned crop choice ★

**Account.** Leaf-cutting ants (*Atta*, *Acromyrmex*) cut leaves to feed a cultivated fungus. They learn to reject plants that harm the fungus (§A6). Waste workers are segregated, and contaminated ants are attacked (§J3). They carry antibiotic-producing bacteria to defend the crop [M, Currie et al. 1999]. Termites (*Macrotermes*) farm *Termitomyces* the same way [M].

**Lever.** The player's plants are the crop. Which species the gnome grows decides what the colony can farm.

**Player sees.** Leaf fragments marching underground, and a fungus chamber swelling. Spraying one plant species makes the colony abandon it for months.

**Timescale.** Days to months.

**Engine.** Plants exist, and so do rot and decay. A fungus material growing on cut leaves in a dark, damp room would be one new material; whether it fits the plant/organism system is a design question.

### H2. Herding aphids ★

**Account.**

- *Lasius niger* tends aphids for honeydew and defends them from predators.
- Ant chemical footprints **slow aphid walking** and reduce dispersal [S, Oliver et al. 2007, Proc R Soc B 274:3127].
- *Lasius flavus* keeps root aphids underground and carries their eggs through winter [M].
- Ants move aphids to better plants [M].

**Lever.** Put sap-sucking bugs on the player's plants.

**Player sees.** Ants guarding a herd on a stem and fighting off the beetles that come for it. Herds kept where ants walk, and colonies whose wealth depends on the garden.

**Timescale.** Days.

**Engine.** Bug predators (beetles, spiders, centipede) exist. A honeydew bug would be a new creature.

### H3. Harvesting seeds, granaries and planting by accident ★

**Account.**

- Harvester ants (*Pogonomyrmex*, *Messor*) store seeds in dry chambers and remove seeds that start to germinate [M].
- Many plants pay ants to disperse their seeds with a fatty appendage, the elaiosome. Ants eat it and discard the seed in the midden or tunnels, where it sprouts in rich soil [M, the myrmecochory literature].
- Leaf-cutter refuse heaps are nutrient hotspots for plants [M, Farji-Brener].
- *Myrmelachista* ants kill rival plants with formic acid, creating "devil's gardens" of their host tree [M, Frederickson et al. 2005 Nature].

**Lever.** Plant seeds with a "fatty reward", and let ants clear weeds.

**Player sees.** **The ants plant the garden.** Seedlings come up on the colony's dump, in lines along the trails, and the garden reshapes itself around the nest.

**Timescale.** Weeks.

**Engine.** Plants seed and ants carry things, and the dump is spoil. Ants carrying seeds would cost almost nothing. **Very buildable, and it ties the two systems the game is about.**

---

## I. Warfare, raids, slavery and care of the wounded

### I1. Tournaments, assessment and raids ★

**Account.** Honeypot-ant colonies meet in **ritual tournaments**: hundreds of ants stilt-walking and displaying, with almost no fighting. When one colony is much weaker, its brood and repletes are **raided and absorbed**, and the raided brood grows up as workers of the winner [S, Hölldobler 1976/1981; Lumsden & Hölldobler 1983; Kronauer et al. 2003]. Fights between ants follow Lanchester-type laws, where numbers beat individual strength in open melee [M, Franks & Partridge 1993; Plowes & Adams 2005].

**Lever.** Two colonies in one box.

**Player sees.** A border war that is a show of force, and sometimes a lopsided raid in which the loser's young become the winner's workforce (D3).

**Timescale.** Hours.

**Engine.** The attack verb exists but is inactive; there is a stranger sense.

### I2. Enemy-specific defence ★

**Account.** In *Pheidole dentata*, a minor worker that meets a **single fire-ant scout** lays a trail that recruits soldiers, which destroy the scout before it can recruit its own colony. The response is narrowly specific to *Solenopsis*. Against a large invasion, the minors lay fewer trails and defend closer to the nest [S, Wilson 1975, Science 190:798].

**Lever.** Introduce a specific enemy repeatedly. Combined with §D1, exposure raises soldier production.

**Player sees.** A colony that has "learned" the centipede: big-heads rushing out whenever its scent appears, and indifference to harmless bugs.

**Timescale.** Seconds per response; generations for the response to become innate.

**Engine.** The alarm plane exists, and so do predators. An enemy-identity scent is missing.

### I3. Slave-making and slave rebellion ★

**Account.** Slave-making ants (*Polyergus*, *Temnothorax americanus*) raid other colonies for pupae, which imprint on the raiders and work for them (D3). Enslaved *Temnothorax* workers rear the slavemakers' larvae normally until **pupation, then kill them**: two-thirds of slavemaker pupae died, 83% of queen pupae against 3% of males, about 30% torn apart and 53% neglected outside the nest chamber [S, Achenbach & Foitzik 2009, Evolution 63(4)].

**Lever.** Cross-fostering or raiding between lineages.

**Player sees.** A captured workforce, and a revolt.

**Timescale.** Brood cycles.

**Engine.** Missing. It requires D3 and attack.

### I4. Rescue, wound care and amputation ★

**Account.**

- **Rescue.** *Megaponera analis* carry injured nestmates home after termite raids, triggered by mandibular-gland compounds. Untreated injured ants die in 32% of cases [S, Frank et al. 2017, Science Advances].
- **Wound treatment.** Nestmates treat the wounds [S, Frank et al. 2018, Proc R Soc B].
- **Amputation.** Florida carpenter ants **amputate a nestmate's leg** when the wound is high on the femur, which significantly raises survival. Lower tibia wounds are cleaned only [S, Frank et al. 2024, Curr Biol].

**Lever.** Battles and predator attacks.

**Player sees.** An ant carrying a wounded sister home from the centipede fight, and a three-legged veteran later in the life record.

**Timescale.** Minutes.

**Engine.** Injury is missing; the engine knows death but not wounds.

### I5. The dead: necrophoresis and cemeteries ★

**Account.**

- **Oleic acid.** Dab a *living* ant with oleic acid and nestmates **carry it to the refuse pile, repeatedly**, while it walks back each time [S, Wilson, Durlach & Roth 1958, Psyche 65:108].
- **The life signal.** In Argentine ants, recognition is triggered by the **loss of "life" chemicals** (dolichodial, iridomyrmecin), which fall below half within about 10 minutes of death. Coating a corpse with them prevents its removal [S, Choe, Millar & Rust 2009, PNAS 106:8251].

**Lever.** Scent things as "dead".

**Player sees.** A perfectly healthy ant being hauled, protesting, to the cemetery again and again. Players will love it.

**Timescale.** Seconds.

**Engine.** Corpses exist as a sensed feature. A "death odour" decoupled from actual death is missing; adding it gives this joke for free.

---

## J. Structures built from bodies, soil and waste

### J1. Bridges, rafts, chains and bivouacs ★

**Account.**

- **Bridges.** Army ants (*Eciton*) link bodies into living bridges that lengthen, widen and **migrate to create shortcuts**, trading the saving against the number of ants locked up [S, Reid, Lutz et al. 2015, PNAS 112:15113].
- **Rafts.** Fire ants form waterproof rafts in floods [M, Mlot, Tovey & Hu 2011 PNAS].
- **Weaving.** Weaver ants chain bodies to pull leaves together and glue them with **larval silk**, waving larvae like glue guns [M, Hölldobler & Wilson].

**Lever.** Rain and flooding (both exist), gaps, overhangs.

**Player sees.** Ants turning into a solid object, which a cellular physics engine can genuinely represent, perhaps as a temporary "linked-ant" solid cell.

**Timescale.** Seconds to minutes.

**Engine.** Missing. The physics exists.

### J2. Building by stigmergy

**Account.** Termites and ants build by responding to the work itself. Fresh, pheromone-laden deposits attract more deposits, which grows pillars and arches without a plan [M, Grassé 1959; Khuong et al. 2016 PNAS, ant nest building set by body size and pheromone decay].

**Lever.** The materials offered: clay, sand, gravel.

**Player sees.** Different architectures from different soils. Tall mounds in clay, sprawling ones in sand.

**Timescale.** Hours to days.

**Engine.** Digging, dropping and spoil exist. A "freshly dropped" scent is missing, and adding it is cheap.

### J3. Waste dumps, quarantine and cemeteries

**Account.** Leaf-cutter waste passes from garden workers through an intermediate cache to heap workers who **rarely leave the dump**, and garbage-contaminated ants are attacked. Together these isolate pathogens [S, Hart & Ratnieks 2001, Behav Ecol Sociobiol 49:387].

**Lever.** Where the dump goes, and what grows on it (H3).

**Player sees.** A dedicated waste crew, a mound of refuse that sprouts plants, and outcasts.

**Timescale.** Days.

**Engine.** Spoil, corpses and rot exist; dump workers would emerge from C1 and A8.

### J4. Climate control: moving brood

**Account.** Ants move brood up and down the nest through the day to track the best temperature and humidity [M, Roces & Núñez and others]. Wood-ant workers bask in the sun and carry heat inside [M]. Honeybees fan and spread water [M].

**Lever.** Lamps, which the player already moves, and watering.

**Player sees.** The brood pile migrating toward the warm wall each morning and away at noon. A visible daily rhythm the player caused.

**Timescale.** Hours.

**Engine.** Warmth is a feature and brood can be carried. **This may emerge from the existing kit with only a fitness term.**

---

## K. Failure modes: the stories players tell most

### K1. The ant mill ★

**Account.** Army ants separated from the trail lock onto each other's pheromone and **circle until they die of exhaustion**. Beebe described a mill about 370 m around in 1921; Schneirla studied it [S, Wikipedia and Delsuc 2003 PLoS Biol, https://en.wikipedia.org/wiki/Ant_mill]. Couzin & Franks reproduced lanes and mills in models [M].

**Lever.** Lay a closed **loop** of trail.

**Player sees.** A whirlpool of ants. Death by tradition, and a moral.

**Timescale.** Minutes to hours.

**Engine.** Blind trail-followers with scent planes exist, so **this may be reachable today**. Worth a test: lay a ring of plane-A trail.

### K2. Lock-in and over-exploitation

**Account.** Lock-in to the first poor source [S, Beckers et al. 1990]. Over-harvesting of aphid herds and seed banks; boom-bust in colonies whose reproduction outruns intake [M].

**Player sees.** A colony stubbornly working a dry well, and a famine after a feast.

**Engine.** Boom-bust already happens (§D2).

### K3. Parasites that hack the rules ★

**Account** [M throughout]:

- **Butterfly caterpillars as brood.** *Phengaris* (*Maculinea*) caterpillars are adopted as brood. Some mimic queen sounds and are fed ahead of the colony's own larvae [M, Barbero et al. 2009 Science].
- **Beetles as brood.** *Lomechusa* beetles are fed in preference to the ants' own brood [M].
- **Zombie ants.** *Ophiocordyceps* fungus makes "zombie" ants climb and bite before dying [M].
- **Lures and traps.** Pitcher plants exploit foraging, and antlion pits are deadly ground.

**Lever.** Hazards and parasites in the box.

**Player sees.** The colony feeding a cuckoo, and a stem studded with dead ants.

**Engine.** Each needs a new creature. The *principle* applies directly, though: anything that matches an ant's sensed features (larva, food, nestmate) **will be treated as that thing**, which makes the engine's feature list an attack surface the player can play with.

---

## L. Genetics: what selection and breeding can and cannot do

### L1. Heritable colony behaviour, and how fast breeding works ★

**Account.** Colony-level foraging restraint is heritable [S, Gordon 2013]. A foraging gene (*for*, a PKG) expression differs between harvester-ant task groups [M, Ingram, Oefner & Gordon 2005]. Honeybee **high and low pollen-hoarding strains** were selected apart in a few generations [M, Page & Fondrk 1995].

**Lever.** **The queen-player chooses whose offspring to lay**, or the gnome chooses which colony to split. This is artificial selection, the oldest teaching method.

**Player sees.** Within a few generations, lineages visibly drift toward what the player rewards. The genome page records the change in plain sentences.

**Timescale.** Generations.

**Engine.** Eggs inherit a brain, the shelf clones and mutates, and offline colony selection exists. The main lever is already in the engine and only needs to be handed to the player.

### L2. Genetic diversity within the colony ★

**Account.** Honeybee queens inseminated by **15 drones rather than 1** produced colonies that built about 20% more comb, stored up to 78% more food and built 39% larger reserves within two weeks. **All single-father colonies died over winter; a quarter of the diverse colonies survived** [S, Mattila & Seeley 2007, Science; Cornell coverage]. Diverse colonies also hold brood temperature more stably, because patrilines differ in their thresholds [M, Jones et al. 2004 Science].

**Lever.** The player-queen chooses **one elite father, or many**. The obvious "breed only from the best" strategy loses to diversity, a counter-intuitive lesson the game can teach.

**Player sees.** A monoculture colony of clones of the champion that is brilliant at one thing and collapses at the first change.

**Timescale.** Generations.

**Engine.** Possible now. The engine's genome pool *is* this experiment.

### L3. Supergenes and selfish genes

**Account.** In fire ants, the green-beard Gp-9 region (§E), carried on a "social chromosome", decides whether colonies have one queen or many [S, Ross & Keller 1998 PNAS; M, Wang et al. 2013 Nature]. In some harvester-ant lineages, caste is **genetically** determined [M, Julian et al. 2002; Helms Cahan et al. 2002].

**Lever.** None directly. This is a caution that the genome can evolve to police itself.

### L4. Baldwin effect and genetic assimilation: learned behaviour becomes innate ★

**Account.**

- **Baldwin effect** (Baldwin 1896 [M]). The ability to learn a behaviour lets a lineage survive long enough for mutations that produce it without learning to be favoured.
- **Genetic assimilation** (Waddington 1953 [M]). An environmentally induced trait becomes constitutive under selection.
- **In silico.** Hinton & Nowlan 1987 showed that learning smooths a needle-in-a-haystack fitness landscape and speeds evolution [M].
- **Experimental evolution.** In *Drosophila*, the chance to learn **sped up** the evolution of an innate pineapple preference but **slowed** an orange one. The effect is asymmetric and not guaranteed [S, Mery & Kawecki 2004, Evolution 58(4)].
- **In ants.** Dormant ancestral potentials (supersoldiers) that the environment unlocks are raw material for genetic accommodation [S, Rajakumar et al. 2012].

**Lever.** **Teach, then breed.** The player teaches a behaviour in-life (A1, A6, A8 or tool use). Ants that learn it fastest thrive. Over generations, newborns do it untaught.

**Player sees.** The "my colony was born knowing" moment: generation 1 needed lessons, generation 10 does it at birth. The genome page could even say "is born preferring mint".

**Timescale.** Tens of generations.

**Engine.** This requires in-life learning that feeds the network as an input (A1, A2, A8). Then the genome can evolve to pre-set, ignore or amplify it. Without in-life learning, a Baldwin effect cannot happen, which is a strong argument for adding a small plastic layer.

---

## What the biology says to build first

The biology suggests a **small plastic layer of per-ant scalars that the network reads as inputs**, not plastic network weights. It is cheap at hundreds of ants, legible on the ant's page, and it lets evolution decide how much each scalar matters (§L4).

| Per-ant state | Bytes | Real mechanism | Unlocks |
|---|---|---|---|
| Home vector (path integrator) + one food vector | 16 | A5 | Far-food foraging, the fling story, tandem leaders |
| Task-success biases (forage, dig, nurse, …) | ~8 | A8, C1 | Roles that emerge from history; specialists the player made |
| Recent laden-returner contact rate | 2 | C3 | Activation; fixes idle fed ants; foraging that breathes |
| Odour-valence table (K≈4 odours) | ~8 | A1, A6, B3 | Taste, aversion, learning through feeding, primacy |
| Expected reward (running mean) | 2 | A2 | Spoilt or hardy colonies |
| Food phase-of-day | 2 | A9 | The colony knows dinner time |

Colony-level couplings to add: egg-laying tied to the store and to queen presence (D2, E); body size from larval food (D1); a negative scent plane and trail deposition graded by reward (B2); a "death" odour separate from death itself (I5).

### Ten levers that are real, visible and buildable

1. **Fling an ant: the home vector** (A5). Needs a real path integrator. The textbook experiment as a toy, and the far-food fix.
2. **Rig success to make specialists** (A8). Feeding an ant at the right moment turns it into a lifelong forager; repeated failure makes a nurse. Roles emerge from history with one brain.
3. **Fast returns wake the colony** (C3). Food placement throttles colony activity through contact rate. Fixes idle ants.
4. **Flavour food and choose the first lesson** (A1 and B3). One-trial, primacy-biased taste, spread by mouth-to-mouth feeding to ants that never visited.
5. **Hazards teach detours and lasting refusals** (A6). Pits, fire and sprayed plants. Avoidance outlives the ants that learned it (16 weeks in *Atta*).
6. **What you feed the larvae shapes the colony** (D1 and D2). Protein raises big workers, enemy pressure raises more soldiers, and too much protein collapses the colony.
7. **Feed at the same hour** (A9). Ants gather before the food appears.
8. **The queen picks the father: one champion or many** (L1 and L2). Selection works, and diversity beats the elite.
9. **Ants plant the garden** (H3). Seeds carried, elaiosomes eaten, seedlings sprouting on the dump and along trails.
10. **Trail plus "no entry"** (B2). Graded, reward-weighted trails plus a repellent stroke. Lock-in to the first source is the story.

Runners-up: tandem-run teaching chains (B1); moving brood after the lamps (J4); house-hunting by quorum (F1); patrollers choosing the doorway (B5).

### Five behaviours that are astonishing but true

1. **A healthy ant carried to the graveyard, again and again.** A dab of oleic acid makes nestmates haul a living ant to the refuse pile repeatedly [Wilson 1958]. Ants recognise death by the *loss* of life chemicals [Choe 2009].
2. **Ants that use sand grains as sponges** to carry home liquid food, and learn to prefer better materials [Maák 2017]. A falling-sand engine can show this honestly.
3. **The death spiral.** Trail-followers that close a loop circle until they die; one army-ant mill was about 370 m around. One stroke of player trail could cause it.
4. **Ants teach.** A leader slows itself fourfold and waits for its pupil's antennal taps [Franks & Richardson 2006]. Ants also amputate a wounded sister's leg to save her [Frank 2024], and carry the injured home from battle.
5. **Slave rebellion.** Captured workers raise their captors' young faithfully, then kill about two-thirds of the slavemaker pupae [Achenbach & Foitzik 2009]. Close behind: a colony that knows dinner time, and avoidance of a poisoned plant that lasts longer than the lives of the ants that learned it.

---

## Sources checked in this session [S]

- Franks & Richardson 2006, *Teaching in tandem-running ants*, Nature 439:153. https://www.nature.com/articles/439153a ; https://www.bristol.ac.uk/news/2006/879.html
- Prabhakar, Dektar & Gordon 2012, PLoS Comp Biol. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3426560/ ; Pinter-Wollman et al. 2013. https://pmc.ncbi.nlm.nih.gov/articles/PMC3767282
- Gordon 2013, *The rewards of restraint…*, Nature 498:91. https://www.nature.com/articles/nature12137
- Greene & Gordon 2007, *How patrollers set foraging direction…*, Am Nat. https://ideas.repec.org/a/oup/beheco/v18y2007i2p451-455.html ; https://news.stanford.edu/pr/03/antchat57.html
- Oberhauser, Bogenberger & Czaczkes 2022, J Exp Biol. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10088526/
- Wendt et al. 2019, eLife. https://elifesciences.org/articles/45450
- Grüter, Czaczkes & Ratnieks 2011 (secondary accounts). https://pmc.ncbi.nlm.nih.gov/articles/PMC3291321/
- Robinson et al. 2005, *"No entry" signal*, Nature 438:442. https://sro.sussex.ac.uk/id/eprint/22787/
- Beckers et al. 1990, Insectes Sociaux 37:258. https://link.springer.com/article/10.1007/BF01201674
- Provecho & Josens 2009, J Exp Biol 212:3221. https://bibliotecadigital.exactas.uba.ar/collection/paper/document/paper_00220949_v212_n20_p3221_Provecho
- Wystrach et al. 2020, Curr Biol 30:1927. https://www.sussex.ac.uk/lifesci/insectnavigation/documents/wystrach-et-al-curr-biol-2020.pdf
- Learning walks review, J Comp Physiol A 2022. https://link.springer.com/10.1007/s00359-022-01600-y
- Wittlinger, Wehner & Wolf 2006, Science 312:1965. https://www.staff.uni-mainz.de/cduch/Softskills_SS2013/3-4_Ants%20on%20Stilts%20Pedometers/12_ants%20on%20stilts_paper.pdf
- Saverschek et al. 2010 / Herz et al. 2008, via Falibene et al. 2015. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4389540/
- Ravary et al. 2007, Curr Biol 17:1308. https://antkey.myspecies.info/en/content/individual-experience-alone-can-generate-lasting-division-labor-ants
- Langridge, Franks & Sendova-Franks 2004. https://uwe-repository.worktribe.com/output/1057547
- Charbonneau, Sasaki & Dornhaus 2017, PLOS One. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC5587300/
- Mersch, Crespi & Keller 2013, Science. https://infoscience.epfl.ch/handle/20.500.14299/92751
- Pratt et al. 2002, 2005 (quorum). https://link.springer.com/article/10.1007/s00265-002-0487-x ; https://catalog.comses.net/publications/1208
- Mallon & Franks 2000, Proc R Soc B 267:765. https://pmc.ncbi.nlm.nih.gov/articles/PMC1690598
- Doran & Franks 2013 (Royal Society news). https://royalsociety.org/news/2013/ants-investigate-housing-market
- Wheeler & Nijhout 1984. https://experts.arizona.edu/en/publications/soldier-determination-in-pheidole-bicarinata-inhibition-by-adult-/
- Passera et al. 1996, Nature 379:630. https://www.doi.org/10.1038/379630A0
- Rajakumar et al. 2012, Science. https://mcgill.ca/channels/news/back-future-supersoldier-ants-illuminate-evolution-213059
- Dussutour & Simpson 2012, Proc R Soc B. https://pmc.ncbi.nlm.nih.gov/articles/PMC3350685
- Holman et al. 2010, Proc R Soc B 277:3793. https://pmc.ncbi.nlm.nih.gov/articles/PMC2992706
- Keller & Ross 1998, Nature 394:573. https://link.springer.com/article/10.1038/29064
- Rosengren et al. (wood-ant overwinter memory). https://zenodo.org/record/17379 ; https://www.smithsonianmag.com/science-nature/ant-colonies-retain-memories-outlast-lifespans-individuals-180971022/
- Oliver et al. 2007, Proc R Soc B 274:3127. https://pmc.ncbi.nlm.nih.gov/articles/PMC2293950
- Wilson 1975, Science 190:798 (enemy specification). https://antcat.org/references/130379 ; https://keyapa.com/pheidole/2024/05/29/pheidole-dentata-coping-with-fire-ants/
- Hölldobler 1981; Lumsden & Hölldobler 1983. https://experts.azregents.edu/en/publications/tournaments-and-slavery-in-a-desert-ant/
- Achenbach & Foitzik 2009, Evolution 63(4). https://www.nationalgeographic.com/science/article/the-rebellion-of-the-ant-slaves
- Frank et al. 2017 (rescue), Science Advances. https://pmc.ncbi.nlm.nih.gov/articles/PMC5829198 ; Frank et al. 2024 (amputation), Curr Biol. https://news.mongabay.com/2024/08/ants-perform-life-saving-amputations-on-injured-nestmates-study-finds/
- Wilson, Durlach & Roth 1958, Psyche 65:108. https://www.doi.org/10.1155/1958/69391
- Choe, Millar & Rust 2009, PNAS 106:8251. https://pmc.ncbi.nlm.nih.gov/articles/PMC2688878
- Reid et al. 2015, PNAS 112:15113. https://digitalcommons.njit.edu/fac_pubs/6629
- Hart & Ratnieks 2001, Behav Ecol Sociobiol 49:387. https://sro.sussex.ac.uk/id/eprint/28597/
- Ant mill (Beebe, Schneirla, Delsuc 2003). https://en.wikipedia.org/wiki/Ant_mill
- Mattila & Seeley 2007, Science. https://news.cornell.edu/node/275605 ; https://pmc.ncbi.nlm.nih.gov/articles/PMC2596908
- Mery & Kawecki 2004, Evolution 58(4). https://bioone.org/journals/Evolution/volume-58/issue-4/03-540/THE-EFFECT-OF-LEARNING-ON-EXPERIMENTAL-EVOLUTION-OF-RESOURCE-PREFERENCE/10.1554/03-540.full
- Bridges et al. 2024, Nature 627:572. https://chittkalab.sbcs.qmul.ac.uk/2024/s41586-024-07126-4.pdf
- Dong, Lin, Nieh & Tan 2023, Science. https://today.ucsd.edu/story/complex-learned-social-behavior-discovered-in-bees-waggle-dance
- Maák et al. 2017, Animal Behaviour (tool selection). https://whyevolutionistrue.com/2017/01/09/selective-tool-use-in-ants/

Everything tagged [M] above is from memory and should be verified before it is used in public-facing text.
