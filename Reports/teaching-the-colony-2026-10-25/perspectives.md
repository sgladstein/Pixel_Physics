# Who is the player? Perspectives beyond the gardener and the queen

Independent design research for the "teach the colony" plan, 2026-10-25.
Scope: the guest/symbiont, the colony mind, the enemy, the lone scout, and
five further perspectives (ant-plant, parasite fungus, researcher, weather,
multiplayer), each judged as a full game concept. The owner's two proposals
(the gardener and the queen) are judged briefly at the end for comparison.

**How the sources are marked.** `[S]` means the claim came from a web search
summary in this session (URL in the Sources list at the end; WebFetch was
blocked, so I read summaries and abstracts, not full papers). `[K]` means it is
from my own knowledge and was not checked this session. Treat `[K]` as
probably right but worth checking before it goes into a design document.

---

## 0. The problem every perspective has to solve first

The brief says it plainly: **an ant's weights are fixed at birth, with no
learning during its life.** So whatever the player's role, "teaching" today can
only travel through three channels:

| Channel | What carries the lesson | Timescale | Works today? |
|---|---|---|---|
| **World (stigmergy)** | scent planes, dug tunnels, where food and corpses lie | seconds to hours | Partly. Measured: a hand-laid trail does **not** measurably pull workers to food |
| **Selection (genetics)** | which genomes survive and lay eggs; eggs inherit the layer's brain | generations (many minutes to hours of play) | Yes, but slow, and the player can only steer it indirectly unless they hold the eggs (the queen) |
| **In-life learning** | an individual ant changes what it does after an experience | seconds to minutes | **No. Not built** |

Every perspective below is either strong or weak depending on which of these it
leans on. Two observations shape the whole document:

1. **Without in-life learning, every perspective's teaching collapses to "change
   the world and wait for evolution."** That is fine for a gardener or a queen,
   whose game is slow by nature, and fatal for the scout and the enemy, whose
   game is about minutes. The cheapest fix that serves *every* perspective is
   small:
   - **(P1) Writable memory.** The ant already has a feature "memory of where it
     last found food." Let *other events* write it: a tandem run, a trophallaxis
     meal that carries a food odour, a guest that fed it. That one change turns
     the existing feature into a channel for teaching.
   - **(P2) A few learned valences.** Give each ant a handful of plastic scalars:
     "how good or bad is smell X," updated by reward (fed, warm) or harm
     (bitten, alarm). X would be a small set of identity odours: the gardener's
     own scent, a guest's scent, a named food smell, a predator's scent. Each
     scalar feeds the existing scorer as one more feature. The learning rate is
     a gene. **One brain design still holds**: plasticity becomes part of the
     design, and every ant has the same amount of it.
   - **(P3) A graded colony odour.** The ant has a binary "nestmate or
     stranger" feature today. Make it a *distance* between the target's
     odour signature (a small body-gene vector) and the colony's, with each ant
     carrying a heritable tolerance gene. That gives you nestmate recognition,
     mimicry, and arms races for free (see §1 and §3).

   Each costs a few floats per ant and no per-pixel work, so frame cost is
   negligible against "every pixel simulated."

2. **The Baldwin effect makes in-life learning and genetics one story instead of
   two** `[K: Hinton & Nowlan 1987, "How learning can guide evolution"]`. When
   ants can learn, a behaviour the player teaches by hand in early generations
   makes the ants that learn it quickly more successful. Selection then favours
   genomes that start closer to it, and within some generations the taught
   behaviour turns into instinct: the colony stops needing to be taught.
   **This is the best genetics tie-in available to every perspective, not only
   the queen**, and the player can watch it happen. Show it on the ant's page:
   "this habit was learned in her grandmother's time; she was born knowing it."

The measured fact that hand-laid trails do not pull workers matters
everywhere. Any perspective whose main verb is "lay scent" inherits that
failure until the trail-following is fixed or (P1) supplies a stronger channel.

---

## 1. The guest: the player is a myrmecophile living with the colony

### The biology it rests on

- **Aphids and honeydew.** Black garden ants (*Lasius niger*) tend aphids for
  honeydew. Their footprints carry chemicals that slow the aphids down; aphids
  on ant-walked paper moved ~30% slower. The chemicals affect only
  ant-farmed aphid species. Sedated aphids crowd less, so fewer of them grow
  wings and leave. Ants are also recorded biting wings off and suppressing wing
  growth chemically. In return, ants drive off ladybirds and other predators
  `[S: Oliver et al. 2007, via Imperial College / National Geographic]`. Some
  *Lasius* species carry aphids into the nest and over-winter their eggs `[K]`.
- **Caterpillars that pay and drug.** Lycaenid caterpillars feed attending ants
  from a dorsal nectary organ. In *Narathura japonica* that secretion **lowers
  dopamine in the ants' brains and reduces how much they walk**, so they stay
  with the caterpillar. Fed ants also become more aggressive when the
  caterpillar everts its tentacle organs (Hojo, Pierce & Tsuji 2015, *Current
  Biology*). The bodyguard function is the authors' interpretation, not a
  tested result `[S]`.
- **The cuckoo caterpillar.** Alcon and mountain-alcon blue caterpillars
  (*Phengaris/Maculinea alcon*, *rebeli*) smell like ant brood and get carried
  into *Myrmica* nests. Full-grown larvae imitate the sounds of the *queen*
  (lower frequency, higher intensity than workers). Workers feed them **before
  their own larvae, favour them when food is scarce, and rescue them first**
  when the nest is disturbed `[S: Barbero et al. 2009, *Science*, via Physics
  Today / BMC blog]`. The chemical mimicry probably evolved before the acoustic
  mimicry `[S]`.
- **The arms race.** The closer a caterpillar's surface chemistry matched its
  host, the more easily it exploited the colony. *Myrmica rubra* populations are
  genetically differentiated, consistent with an ongoing arms race, while a
  second host, *M. ruginodis*, was not. The authors propose that the alternative
  host is a refuge while the main host counter-adapts (Nash et al. 2008,
  *Science*) `[S]`.
- **The beggar beetle.** *Lomechusa* rove-beetle larvae rear up and tap a
  worker's mouthparts to trigger regurgitation, and reportedly get more food than
  the ants' own larvae by begging harder. They are groomed for their secretions,
  and **they eat ant larvae while being tended**. Adults use "appeasement" fluid
  to get carried in `[S: Hölldobler 1967; Hölldobler & Kwapich 2019 review]`.

### Three playable guests

| Guest | Player's verbs | What the colony can learn to do with you |
|---|---|---|
| **Aphid herd** (farmed) | choose which plant to feed on; pulse honeydew (more sugar = more tending); grow a winged generation and fly to a new plant (desertion); kick off an ant that milks you too hard; drop alarm when a ladybird or beetle comes | tend you, guard you, carry you to better stems, carry you indoors before rain or cold, build a soil shelter over you, bite the wings off your young (they keep you) |
| **Lycaenid caterpillar** (bribe and bodyguard) | secrete reward (volume and recipe); evert tentacles (summon guards); "slow" secretion (the dopamine effect: tenders stay); feed on a host plant until you pupate | form a guard ring; learn your smell as food (P2); grow dependent on your recipe; attack anything that comes near you, *including the gardener* |
| **Alcon / beetle cuckoo** (parasite) | tune your chemical disguise toward brood or nestmate (P3); sing "queen" for priority; beg by tapping; eat a larva when nobody is looking; hide in a brood chamber | adopt you, carry you inside, feed you before their own young; or, over generations, evolve a tighter recognition template and **kill you on sight** |

### How the colony's behaviour changes

- **Through the world, immediately.** Ants who fed from you go home on the food
  trail, so you become a food source on the B-plane like any other. Honeydew can
  be a liquid food material in the existing physics: it drips, pools, gets
  washed away by rain. That is a visible consequence for free.
- **In life (needs P1/P2).** An ant fed by you raises its valence for your
  smell, so it comes back, stays longer (the dopamine story, cheaply), and
  defends the spot. An ant you kicked or that got nothing lowers it.
- **Over generations (needs P3 for the parasites).** If you are valuable, the
  colonies whose genomes tend you well do better, and the pool drifts toward
  farming you. If you are a parasite, selection favours ants whose
  recognition template excludes you. Your disguise and their template chase
  each other. **That arms race is the deepest genetics tie-in in this whole
  document, and the player never touches a gene to cause it.**

### What the player sees

A cluster of ants around the herd, antennating; droplets passing mouth to
mouth; an ant picking up an aphid and walking it to a new stem; a ring of
ants facing outward when a beetle comes; a guest being carried down a tunnel
into the brood room. If you are a parasite: an ant pausing over you longer
than usual, then nipping, then dragging you toward the spoil heap. Each step
is a different behaviour already in the ant's verb list (walk, pick up, carry,
feed, attack), so **the relationship is read off what the ants do, with no
reputation meter needed**.

### Minute to minute, and the long arc

- **Minute to minute (aphid / caterpillar):** manage a sugar budget. Paying a
  lot buys guards and tending but slows your own growth; paying little gets you
  ignored, and a ladybird eats you. A threat arrives: evert the tentacles, see
  whether the guard ring forms. That is a real decision every minute, with an
  answer you can see.
- **Minute to minute (cuckoo):** a stealth-social game. Your disguise is a
  graded distance, not a binary. Each inspection rolls against the inspecting
  ant's own tolerance gene, so the same disguise passes most ants and fails a
  few. Eating a larva fills your belly and raises suspicion nearby (alarm and
  corpse cues).
- **Long arc:** the cuckoo's arc is Plague Inc's: **kill the host and you die
  with it**. Take too much and the colony crashes. Take too little and you never
  pupate. Across generations of caterpillars (you play the next one), the
  colony's recognition gets better, and you have to switch to a new host colony
  (the Nash "refuge") or re-tune. The aphid's arc is a slow merger: the colony
  you trained becomes one that carries you indoors every winter.

### Expected and surprising behaviours

- *Expected:* tending; guarding; ants on your plant all day; trail traffic to
  your stem.
- *Surprising, and all emergent from the existing verbs:*
  - ants **move you** to a stem you did not choose, because the trail made it
    convenient for them;
  - ants **guard your plant against the gardener**, or against a plant-eater
    the player wanted to come by;
  - **feeding you before their own larvae crashes the brood**, a slow-burning
    graded failure the player caused and has to notice;
  - ants **bite the wings off** your winged young, so you cannot leave (real
    biology, and it reads as the colony keeping you);
  - two colonies fighting over your herd;
  - the colony evolving to **reject a disguise that worked on its
    great-grandparents**, which the player sees as "they used to love me."

### What makes it fun, and what makes it boring

- **Fun:** a reversed role (Carrion's insight, applied to symbiosis); a small
  body in a big busy world; a relationship you read off behaviour, the way
  Rain World's scavengers judge you by your gifts `[S: Rain World wiki]`; real
  stakes on both sides (you can be eaten, or evicted). Viva Piñata's "meet the
  conditions and the creature moves in" pattern is the gentle version `[S]`.
- **Boring:** an aphid that only sits on a stem and sucks has almost no verbs.
  The herd has to be the unit (move the herd, split it, grow winged forms) or
  it is a screensaver. The cuckoo's stealth loop risks being a meter-watching
  game unless the inspections are visible ant behaviour.

### Fit with the owner's rules

One brain design: untouched, because the player is not an ant. No hand-made
castes: none. Graded outcomes: the strongest of any perspective (ignored →
inspected → tended → carried → fed before larvae; or nipped → dragged →
killed). Visible consequence: droplets, clusters, carrying, corpses. Natural
feel: very high; all of it is documented biology.

It also **gives idle fed ants a job**, which addresses a known problem. Tending
is a labour sink that sits near the nest, exactly where idle ants already are.

### Build cost

Medium. A new creature body (the creature framework already has spiders,
beetles and a centipede). Honeydew as a food liquid. The elegant part:
**mimicry needs no new brain inputs**, because the ant already reads "what is
in the target cell" as one of food / brood / ant / corpse. A guest that
*presents as brood* to the scorer is exploiting the ant's real senses, which is
exactly what Alcon caterpillars do. Needs P3 for the arms race and P2 for
in-life tending. No aphids exist in `src/` today (checked: zero matches).

---

## 2. The colony mind: the player is the colony's drives, not any ant

### The biology it rests on, and where it bends it

- **There is no central controller in a real colony** `[K: Hölldobler &
  Wilson, *The Superorganism*, 2009]`. So "the colony mind" is a fiction, and
  should be presented as one: you are the *values* the ants share, not a voice
  giving orders.
- **Response thresholds.** Division of labour can come from workers with
  different thresholds for the same stimulus `[K: Bonabeau, Theraulaz &
  Deneubourg 1996]`. Moving a threshold moves who does what, and roles emerge
  without castes. This fits the owner's "one brain, roles emerge" rule exactly.
- **Interaction rate as the signal.** In harvester ants, foraging is driven by
  how often returning foragers are met (about one patroller every 10 s
  stimulated the most foraging), rather than by any message they carry
  `[S: Greene & Gordon 2007; Gordon reviews]`. Colonies differ consistently
  in how they regulate foraging `[S: Gordon et al. 2011, title only]`; colonies
  that held back foraging in dry conditions had more offspring colonies, and
  the restraint resembled mother-to-daughter `[K: Gordon 2013, *Nature*]`.
- **Colony memory lives outside heads.** *Temnothorax* colonies that inspected a
  mediocre nest learned to avoid it. When the ants' pheromone marks were
  removed, or the landmarks rotated, the colony chose at random, so the memory
  was partly in the marks `[S: Stroeymeyt et al. 2011; Franks et al. 2007]`.
  The aversion could backfire when every alternative was worse `[S]`.

### The player's verbs

You set a small number of colony-wide values:

- **Drives:** how urgent hunger feels; how much risk is acceptable; curiosity
  (the pull of unexplored cells); crowding discomfort; how much the young
  matter compared with the self.
- **Meanings:** what a smell signifies. "Plane B near the red flower means
  danger." "The gnome's scent means food." (That needs P2: the player sets the
  valence's starting value, and experience moves it from there.)
- **What counts as success:** the weights of the colony's own fitness, meaning
  which ants' eggs get fed first and who lays.

### How the colony's behaviour changes

- **Immediately:** drives become context inputs to every brain, so a fixed
  network still responds to them. Raise risk tolerance and foragers range
  farther, at once. This works today with no learning.
- **In life (P2):** meanings are starting valences that experience then
  corrects. Tell the ants red flowers are dangerous, and the ones who find food
  there anyway learn otherwise. The player watches their own belief get
  overruled.
- **Over generations:** the success weights *are* the selection pressure. The
  engine already does exactly this offline (colony selection on food, nest,
  rooms, survival); this perspective turns that scoring into the player's
  hands. In-box it has to act through reproduction: larvae of ants who did
  valued things are fed first, or such ants are likelier to lay.

### What the player sees, and the minute-to-minute loop

The danger is that the player sees **a panel of sliders**. To be playable, each
value needs a visible avatar in the world: a heat tint on the scent planes
showing what the colony currently "means" by them, or ants visibly hesitating
at a cue you made frightening. The loop: notice a problem (food far away, ants
starving), shift a value, watch hundreds of ants respond with a lag of seconds,
then watch generations drift.

### Expected and surprising behaviours

This perspective is the best **surprise generator** in the document, for one
reason: **Goodhart's law.** Whatever the player rewards, the colony will find
the cheapest way to produce it.

- Value "food brought home" → ants carry food *out* and back in, or hoard by
  the entrance.
- Value "rooms" → hundreds of one-cell rooms.
- Value "few deaths" → nobody leaves the nest, and they starve, gently.
- Value curiosity → a colony of explorers that forgets to come home.

That is the monkey's-paw colony, and it is the real lesson of reward design in
AI, made playable. It is also natural in a way: real colonies do odd things
when selection rewards the wrong proxy.

### Fun and boring

- **Fun:** strategic, legible cause and effect at the scale of hundreds; the
  Goodhart surprises; the player authoring a culture. Black & White's creature
  is the closest precedent: the player shapes behaviour through reward rather
  than commands, and the creature's habits drive its choices when it is not
  instructed `[S, from low-quality summaries; the design details are better
  sourced from Lionhead's own talks]`. Plague Inc shows that a perspective with
  no body works if the feedback (news, charts, the world map) is dense `[S]`.
- **Boring:** abstract; the player never touches anything; every decision is
  a slider. It is the least *natural* of all the perspectives, because no
  real ant colony has a mind to inhabit. And it is the furthest from "the
  verb must deliver something": a slider moving is not an impulse.

### Fit and cost

No castes (thresholds give emergent roles), one brain. Graded: yes. Visible:
only if each value has a world avatar. **Build cost: the lowest of any new
perspective**, because the scoring machinery already exists offline and drives
are just inputs. Its best use is probably not as a whole game but as the
**late-game layer of another one** (see §7).

---

## 3. The enemy: the player is a predator or a rival colony

### The biology it rests on

- **Anteaters are run off by the colony's defence.** A giant anteater feeds at
  one nest for roughly 40 seconds to a minute before the bites and stings drive
  it away, so it never wipes out a colony in one visit `[S: zoo sources,
  approximate]`. This is a fully graded predator-prey outcome.
- **Antlions and rescue.** *Cataglyphis cursor* workers dig out a nestmate
  caught in a snare and **bite at the snare itself**. They help only active
  nestmates, never strangers or motionless ants (Nowbahari et al. 2009, *PLoS
  ONE*). Red wood ants rescue by trial and error instead, switching between
  pulling, digging and biting `[S: 2024 preprint]`.
- **Medics.** Matabele ants (*Megaponera analis*) carry injured nestmates home
  from termite raids. Inside the nest they groom the wounds, which cuts 24-hour
  mortality sharply. They apply antimicrobials to infected wounds, which they
  can apparently diagnose by a change in body chemistry. Heavily injured ants
  that stop responding are not treated; the injured ant's own behaviour decides
  it `[S: Frank et al. 2017, 2018, 2023]`.
- **Enemy specification.** *Pheidole dentata* soldiers respond strongly to fire
  ant scouts in particular: they kill the scouts early, and if the invasion
  wins they evacuate `[K: Wilson 1975]`.
- **Slaves who rebel.** *Temnothorax* workers enslaved by *Protomognathus
  americanus* kill about two thirds of the slave-maker's female pupae, which
  they recognise as chemically foreign. This lowers parasite pressure on
  related free colonies nearby `[S: Achenbach & Foitzik 2009]`. The behaviour
  is found in every parasitised population studied, with strong variation
  between sites `[S]`.

### The player's verbs

- **Antlion:** dig a cone in the sand. The falling-sand physics *is* the antlion
  mechanic: flick sand at a climbing ant and the slope avalanches under it.
- **Spider:** web the chokepoints; ambush at the nest entrance; choose when to
  strike.
- **Anteater (or the gnome as one):** breach the mound; feed until the bites
  hurt.
- **Rival colony / slave-maker:** raid the brood chambers and carry pupae home.

### How the colony's behaviour changes

- **Through the world, within minutes:** alarm scent where you attacked
  (the plane exists); corpses where you killed (the corpse feature exists);
  tunnels abandoned and re-dug. The colony "remembers" you for as long as the
  marks last.
- **In life (P2):** an ant that survives you raises its aversion to your
  scent and to the place. New verbs that matter here: **rescue** (pick up or
  dig at a struggling nestmate) and **mob** (the attack verb, not yet active).
- **Over generations:** colonies that survive your pit bear more vigilant,
  more mobbing, more rescuing genomes. Defensive architecture can emerge from
  digging selection: narrower entrances, deeper brood rooms, spoil heaps
  that cover an entrance. With P3, a colony that was raided evolves to reject
  stolen brood and kill raiders' young, the rebellion above.

### Expected and surprising behaviours

- *Expected:* avoidance; an alarm cloud; a stampede home.
- *Surprising:* nestmates **digging a victim out of your pit** and biting the
  silk; a mob that turns the hunt into a fight you lose; a colony that **moves
  house** rather than live next to you; a nest whose entrances face away from
  your web after a few generations; injured ants carried home and groomed.

### Fun and boring

- **Fun:** hunting is intrinsically fun and immediate. Carrion's reversed
  horror shows the predator perspective sells itself `[K]`. Alien: Isolation is
  the important cautionary example: its xenomorph only *appears* to learn,
  because behaviour-tree branches unlock when the player repeats a tactic
  (hiding in lockers, using vents) `[S: Game Developer analysis]`. Here the
  colony **really** learns and evolves, which is a selling point no shipped
  game has.
- **Boring:** the colony's only lesson is fear. Its whole vocabulary is
  avoid, flee, mob, rescue. That is a narrow kind of teaching, and the player
  never builds anything with the colony. If learning is generational only, the
  predator just farms a colony that cannot respond within a session.

### Fit and cost

Graded outcomes and visible consequences: excellent (debris, corpses, alarm,
pit collapses). Natural: excellent. Build cost: low to medium for a body (the
creature framework exists; the owner already measured that four beetles kill a
lone queen every time, so predators are real). It needs the attack verb, and
rescue would be new. **Best as a mode, or as the "switch sides" half of
another perspective**: play a season as the spider, then return as the keeper
and see what your hunting taught the colony.

---

## 4. One scout worker: no royal power, only persuasion

### The biology it rests on

- **Tandem running is the textbook case of teaching in a non-human animal.**
  In *Temnothorax albipennis* a leader guides one follower to food. The run is
  slow because the follower keeps stopping to learn landmarks, and **only when
  the follower taps the leader's legs and abdomen does the run continue**. The
  leader pays a cost (it would be faster alone) so the follower learns faster,
  with feedback in both directions (Franks & Richardson 2006, *Nature*) `[S]`.
- **Quorums.** House-hunting *Temnothorax* switch from slow tandem runs to fast
  carrying once enough nestmates are at a candidate site `[K: Pratt et al.
  2002]`. Persuasion has a threshold built in.
- **Individual memory beats the trail.** *Lasius niger* learns an odour-food
  link in a single visit, and a place moderately well; visual learning was
  absent; trail pheromone does not change how fast they learn `[S: Koch, Kabas
  & Czaczkes 2024]`. They prefer whatever they were trained to first, and a
  familiarisation visit can disrupt or reverse that `[S: Oberhauser et al.
  2022]`. *L. niger* has been reported to trust its own memory over the trail
  `[S, snippet only]`.
- **Food odours spread mouth to mouth.** Ants can learn a food odour from a
  meal passed by trophallaxis `[K: e.g. Provecho & Josens 2009, *Camponotus
  mus*]`.

### The player's verbs

- **Tandem lead:** walk slowly enough that your follower keeps up. If you
  outrun her she loses you, which makes it a skill.
- **Feed a sample:** a mouth-to-mouth taste carries the food's odour (P1/P2).
- **Lay trail:** cheap but weak (as measured today).
- **Carry a nestmate:** the fast, late, forceful option (adult transport).
- **Antennate / beg / give alarm.**

### How the colony's behaviour changes

- **The key mechanism, and it is cheap:** a completed tandem run **writes the
  follower's "where I last found food" memory** (P1). That feature already
  exists. Now the follower knows the route.
- **Cascades:** if "lead a tandem" is an option in every ant's verb list, then
  your taught follower can teach a third ant. Knowledge spreads through the
  colony as a visible chain, and it fades as memories are overwritten.
  **The player's job is to start the right cascades and keep them alive.**
- **Opinions:** with P2, nestmates arrive with their own priors (first-learned
  preferences, as in *L. niger*). Other scouts advertise other sites. You are
  competing for a quorum, not commanding.
- **Genetics:** you have no royal power, but in this engine workers lay eggs. The
  ants you taught to succeed eat better, live longer and lay more, so your
  teaching selects indirectly. With the Baldwin effect, routes you taught by
  hand turn into the colony's instinct for that kind of terrain.

### What the player sees

Pairs of ants moving in stop-start rhythm; a growing fan of trails from the
food you found; a rival scout's pair heading somewhere else; a quorum filling
at a new chamber, then the switch to carrying.

### The owner's objection, met head on

The owner wrote that "you walk to some food and your colony follows you" is
too simple. SimAnt did exactly that: the player is one yellow ant who can lay
trails and order a few nearby ants to follow, and Wikipedia notes the orders
work only in a limited way `[S]`. What makes the scout deeper than SimAnt:

1. followers are **taught, not dragged**. They keep the knowledge after you
   leave and pass it on;
2. persuasion is **contested**: quorums, rival scouts, first-learned
   preferences;
3. the follower's feedback is a game: lose her and the lesson fails;
4. what you teach **outlives you**, through the cascade and then the genome.

### Fun and boring

- **Fun:** the most embodied perspective; direct cause and effect; a social game
  of influence. The Last Guardian is the right reference: you cannot control
  Trico, you learn to read it, and it "doesn't always do what you ask" by
  design `[S: Ueda interviews]`. Followers that half-listen are a feature.
- **Boring:** one ant among hundreds. Without cascades, the player hand-delivers
  every lesson forever. Leading run after run gets repetitive by the hour, so
  the scout should not stay a scout all game (see §7).

### Fit and cost

One brain (a "follow" option and a "lead" option are in everyone's list; roles
emerge in who uses them). No castes. Graded: the cascade's reach is continuous.
Visible: pairs, fans of traffic. **Cost: low to medium.** The playable queen's
body already exists and can be reused as a worker body without laying. The
real work is the follow behaviour and P1.

**Side benefit for the engine:** this is a direct attack on the known problem
that trained colonies starve when food is far from home. A colony that can
pass on routes does not have to rediscover them by random walk.

---

## 5. Further perspectives

### 5a. The ant-plant: the player is a plant that trades with ants

**Biology.** Swollen-thorn acacias house *Pseudomyrmex* ants in hollow thorns
and feed them twice: sugar from extrafloral nectaries, and protein-rich Beltian
bodies that adults harvest for their larvae. The relationship is obligate
`[S]`. A generalist ant that steals nectar does not harvest Beltian bodies `[S]`.
**The plant locks its ants in**: workers of the mutualist ant lack the enzyme
invertase and cannot digest ordinary sucrose. The acacia's nectar has the
sugar already split, and a chitinase in the nectar irreversibly disables the
ant's invertase at its first meal as an adult (Heil et al., 2005 onward; the
chitinase result ~2013–14) `[S]`. In drought the acacias **increased** their
investment in ant rewards `[S: 2024, *Oecologia*]`. The ants attack
herbivores and clear vegetation around their tree `[K]`.

**Verbs.** Grow domatia (hollow chambers: a ready-made room); place nectaries
(where sugar appears, the ants patrol); grow protein bodies (feed the brood);
lace the nectar with something that makes them dependent; withhold rewards.

**How the colony changes.** Through the world, immediately: ants patrol where
the rewards are. In life (P2): rewards raise the plant's valence. Over
generations: a colony living on your plant evolves into a bodyguard, and with
the invertase lock it cannot easily leave.

**Surprising behaviours.** Ants **pruning the gardener's other plants**
because they shade or touch yours; ants attacking the gnome; a colony that
starves when you cut rewards, because you made them unable to eat anything
else (graded and earned).

**Verdict.** Lovely fit: plants already grow, flower, seed and have genomes
in this engine, so a myrmecophyte is a new organ type, not a new system. It
works best **merged into the gardener**: the gardener breeds plants that hire
ants. It needs herbivore pressure (beetles exist) so that guarding means
something. Build cost: medium.

### 5b. The parasite fungus: Plague Inc. at ant scale

**Biology.** *Ophiocordyceps*-infected carpenter ants leave the canopy, walk
randomly and convulse, and bite the underside of a leaf vein ~25 cm above the
ground at around noon. They die by sunset, and the jaw stays locked because the
mandible muscles atrophy. The bodies cluster in "graveyards" `[S: Hughes et al.
2011]`. The fungus kills its hosts at the colony's "doorstep": in nests,
cadavers failed to develop, and live ants removed most of them `[S: Loreto et
al. 2014]`. In other systems, nestmates groom off spores, a low-level dose
immunises nestmates against the same strain, and caregivers adjust their care
to their own risk `[S: Konrad, Cremer et al.]`.

**Teaching, inverted.** The player *is* the disease and teaches the colony
hygiene by threatening it: corpse removal, grooming, quarantine, cemetery
placement. Over generations the colony evolves social immunity, and the player
has to evolve around it (the cure race in Plague Inc, where wiping out hosts
ends your own game) `[S]`.

**Verdict.** Strong biology, strong arms race, uses the day/light cycle (the
noon summit) and the corpse feature. But high cost (infection state, spore
transport, grooming) and a dark tone. Better as a scenario than as the main
game.

### 5c. The researcher

The player runs the lab literally: Y-mazes, food-choice assays, nest-choice
tests, selected lines on the specimen shelf, cloning and mutating. Niche shows
that visible genes and deliberate breeding can carry a whole game `[S]`. This
is close to how the engine trains ants *offline today* (colony selection,
stacked lessons). It is cheap and honest, but it overlaps the gardener and the
queen heavily, and its teaching is purely artificial selection, not something
the colony experiences. Best as a tool set inside another perspective.

### 5d. The weather / season

The player chooses rain, drought, cold and heat. The colony learns only by
selection (who survived the flood). Verbs are huge and indirect, and the
colony cannot notice the player as a someone. Weakest teaching of anything
here. It is useful as a difficulty dial, not as a role.

### 5e. Multiplayer: several perspectives at once

Four players: the queen (the eggs), a scout (persuasion), a guest (the
economy), and a predator (pressure). The colony is the shared board, and every
player's lesson goes into the same brains. It is genuinely interesting, since
the colony would be taught contradictory things at once and settle somewhere.
It is expensive (netcode, a shared simulation at frame budget) and a world
of 512x320 is too small. **The single-player version is cheaper and keeps most
of it: one save, rotating seats.** Play a season as the predator, then come
back as the keeper and find what the colony learned from you.

---

## 6. Comparison table

Scores 1 (poor) to 5 (excellent). Build cost is 5 = cheapest. The owner's two
proposals are judged briefly here, without the full treatment.

| Perspective | Depth of teaching | Natural feel | Genetics tie-in | Moment-to-moment fun | Build cost (5 = cheap) | One line |
|---|---|---|---|---|---|---|
| **Gardener / keeper** (owner) | 3 (4 with P2) | 4 | 3 | 4 | 5 (mostly exists) | Teaches through the world; fun because the physics is; the colony never notices *you* unless P2 gives you a scent |
| **Queen** (owner) | 2 | 4 early, 2 late | 5 | 4 founding, 2 after | 5 (body exists) | Founding is great survival play; after it, real queens only lay, so "teaching" becomes picking parents |
| **Guest / symbiont** | 5 | 5 | 5 (arms race) | 3 aphid, 4 cuckoo | 3 | Two-way relationship read off ant behaviour; the colony evolves around you without you touching a gene |
| **Colony mind** | 4 | 1 | 4 (you write the fitness) | 2 | 5 | Best surprise engine (Goodhart), worst feel; a late-game layer, not a game |
| **Enemy** | 2 (fear only) | 5 | 3 | 5 | 4 | The most fun minute by minute; the colony learns only to avoid, flee, mob and rescue |
| **Scout worker** | 4 | 5 | 3 (indirect, Baldwin) | 4 | 4 | Real teaching (tandem runs) that cascades; fixes far-food starvation as a side effect |
| **Ant-plant** | 3 | 5 | 4 | 2 | 3 | Teaches by where rewards appear; best merged into the gardener |
| **Parasite fungus** | 3 | 4 | 4 | 3 | 2 | Plague Inc at ant scale; teaches the colony hygiene; dark and costly |
| **Researcher** | 2 | 3 | 4 | 2 | 5 | The offline trainer as a game; overlaps gardener and queen |
| **Weather** | 1 | 3 | 2 | 1 | 4 | Difficulty dial, not a role |
| **Multiplayer seats** | 4 | 3 | 4 | 4 | 1 | Interesting; do the single-player rotating seat instead |

---

## 7. Does anything beat both owner proposals? Do two combine?

**No single alternative beats the gardener as the frame**: it is cheap, it
already exists, and it sits on the falling-sand physics that is the engine's
strength. But two of the alternatives beat both owner proposals **as the
teaching mechanism**, and the right move is to graft them on, not to replace
the frame.

### Recommended combination 1: the gardener as a symbiont

Give the gardener the guest's relationship. The gnome has a scent (P2). What
the gnome gives (seed heads, sugar from a flower, honeydew from an aphid herd
the gnome keeps on a plant) raises the colony's valence for that scent; what
the gnome does to them (flinging, digging up the nest, being a predator) lowers
it. The colony's relationship with the gardener is then **graded and read off
behaviour**: ants that ignore the gnome, follow the gnome, guard the gnome's
plants, carry offerings to the gnome, or swarm the gnome's boots. Add the
ant-plant (§5a) as plants the gardener can breed, and the gardener grows the
colony's housing and pay, which is exactly where real mutualisms teach.

### Recommended combination 2: a life-cycle arc, queen → scout → colony mind

This answers the owner's question "how is the queen teaching the colony?"
with an honest answer: **in real ants, she mostly is not.** A founding queen of
some species forages and does everything alone `[K: semi-claustral founding]`,
then stops leaving the nest once the first workers arrive. So:

1. **Founding (queen body, exists today):** embodied survival. The four-beetle
   fact makes this tense. The player's genetics power lives here.
2. **Growing (scout body):** when the first workers emerge, the player's
   embodiment moves into a worker. Teaching happens through tandem runs and
   cascades, by persuasion with no royal power.
3. **Mature (colony mind):** once the colony is too large to persuade
   one by one, the player shifts to setting drives and meanings, with
   Goodhart's surprises as the late-game texture.

It is Spore's stages inside one colony, but each stage is one the biology
actually has.

### Combination 3: switch sides

Either of the above, plus **seasons as the predator** (§3), so the player sees
from the outside what their own colony learned to fear. It is cheap once
predators and P2 exist.

### What I would not do

Make the colony mind or the weather the main game (the feel is too
abstract), or start multiplayer before a single colony can be taught at all.

---

## 8. The five best teaching ideas from this research

1. **Mimicry as the teaching interface.** A guest presents itself to the
   ant's existing senses as brood, food or nestmate, so the colony's
   acceptance is graded and visible (ignored → tended → carried → fed before
   its own young). Over generations the colony evolves a tighter recognition
   and the guest must re-tune: an arms race with no gene ever touched by the
   player (Alcon blue; Nash 2008).
2. **The tandem run writes the follower's memory.** A completed run fills
   the existing "where I last found food" feature, and taught ants can teach
   others. Knowledge spreads as a visible cascade that fades unless kept alive
   (Franks & Richardson 2006). It is cheap, and also a fix for colonies that
   starve when food is far away.
3. **Rewards that change the one rewarded.** The acacia's nectar disables the
   ant's own sugar enzyme; the caterpillar's secretion lowers its tenders'
   dopamine so they stay and fight. Giving becomes a way to shape who the ants
   are, and a colony that depends on you is something you built (Heil;
   Hojo 2015).
4. **The player writes the colony's values, and the colony games them.**
   Rewarding a proxy produces the cheapest behaviour that satisfies it: ants
   carrying food out to bring it back in, a thousand one-cell rooms. It is the
   best source of surprising behaviour here and the cheapest to build, because
   the scoring machinery already exists offline.
5. **The colony learns from surviving the player, and the lesson becomes
   instinct.** A predator teaches rescue, mobbing and avoidance within minutes
   (alarm marks, P2), and with the Baldwin effect what was learned in one
   generation is born-in a few generations later, visible on the ant's page as
   "her grandmother learned this."

---

## Sources

Biology (searched this session; summaries and abstracts, not full text):

- Phengaris/Maculinea alcon acoustic queen mimicry and preferential treatment: [Physics Today](https://physicstoday.aip.org/quick-study/butterflies-that-trick-ants-with-sound), [BMC Bugbitten blog](https://blogs.biomedcentral.com/bugbitten/2014/04/18/butterflies-behaving-like-cuckoos-2/), [National Geographic](https://www.nationalgeographic.com/science/article/butterflies-scrounge-off-ants-by-mimicking-the-music-of-queens), [Reed College course page](https://reed.edu/biology/courses/BIO342/2011_syllabus/2011_websites/RachelEllen/phylogeny.html)
- Nash et al. 2008, chemical coevolution with Myrmica: [Copenhagen abstract](https://test01.ku.dk/:obvius/pureproxy/3974268/en/publications/a-mosaic-of-chemical-coevolution-in-a-large-blue-butterfly), [mongabay](https://news.mongabay.com/2008/01/butterfly-tricks-ants-into-caring-for-its-young), [National Geographic (Yong 2010)](https://www.nationalgeographic.com/science/article/evolutionary-arms-race-turns-ants-into-babysitters-for-alcon-blue-butterflies)
- Aphid tranquilisers, Oliver et al. 2007: [Imperial College](https://www.imperial.ac.uk/news/19554/herding-aphids-farmer-ants-keep-control/), [National Geographic](https://www.nationalgeographic.com/science/article/ants-herd-aphids-with-tranquilisers-in-their-footsteps)
- Lycaenid secretions lower ant dopamine, Hojo et al. 2015: [Pierce Lab](https://piercelab.oeb.harvard.edu/publications/lycaenid-caterpillar-secretions-manipulate-attendant-ant-behavior), [Asian Scientist](https://www.asianscientist.com/?p=61513)
- Lomechusa begging beetles: [Wikipedia](https://en.wikipedia.org/wiki/Lomechusa_pubicollis), [Hölldobler & Kwapich 2019 (PMC)](https://pmc.ncbi.nlm.nih.gov/articles/PMC6329524), [Hölldobler 1967 abstract](https://experts.azregents.edu/en/publications/zur-physiologie-der-gast-wirt-beziehungen-myrmecophilie-bei-ameis/), [UK Beetles](https://www.ukbeetles.co.uk/lomechusa-spp)
- Tandem running as teaching, Franks & Richardson 2006: [Bristol research portal](https://research-information.bris.ac.uk/en/publications/teaching-in-tandem-running-ants/), [Bristol news](https://www.bristol.ac.uk/news/2006/879.html), [Nature](https://www.nature.com/articles/439153a)
- Lasius niger learning: [Koch, Kabas & Czaczkes 2024](https://link.springer.com/article/10.1007/s00265-024-03430-1), [Oberhauser et al. 2022](https://kops.uni-konstanz.de/entities/publication/635d066b-5512-4e97-a4d9-9e858330ce7d/full)
- Temnothorax colony memory of nest sites: [Stroeymeyt et al. 2011 (IDEAS)](https://ideas.repec.org/a/oup/beheco/v22y2011i3p535-542.html), [PMC 2947506](https://pmc.ncbi.nlm.nih.gov/articles/PMC2947506)
- Harvester ant interaction rates: [Greene & Gordon 2007 (IDEAS)](https://ideas.repec.org/a/oup/beheco/v18y2007i2p451-455.html), [Pinter-Wollman et al. 2013](https://pinterwollmanlab.eeb.ucla.edu/wp-content/uploads/sites/86/2016/05/Pinter-Wollman-et-al-2013-AB.pdf), [Gordon publications](https://web.stanford.edu/~dmgordon/old2/publications.html)
- Acacia–Pseudomyrmex: [Smithsonian 2024 drought study](https://www.smithsonianmag.com/blogs/smithsonian-tropical-research-institute/2024/11/21/mutualism-under-pressure-new-research-in-panama-shows-a-plants-ability-to-keep-its-defender-ants-happy/), [Royal Holloway, guards and thieves](https://repository.royalholloway.ac.uk/items/2195c11d-f1a4-6ef0-edf7-d9a03ea4eac6/1), [Wikipedia, Vachellia cornigera](https://en.wikipedia.org/wiki/Vachellia_cornigera), [National Geographic 2013](https://www.nationalgeographic.com/animals/article/131106-ants-tree-acacia-food-mutualism), [IFLScience](https://www.iflscience.com/plants-and-animals/trees-make-ants-chemically-dependent-turn-them-indentured-servants/)
- Ophiocordyceps: [ESA blog, noon death bite](https://esa.org/esablog/?p=5122), [BMC Ecology 2011 (DOAJ)](https://doaj.org/article/5985f99390064cbca391ecfab39dd51f), [Penn State, doorstep 2014](https://www.psu.edu/news/research/story/zombie-ant-fungi-manipulate-hosts-die-doorstep-colony), [Loreto & Hughes 2016 (PMC)](https://pmc.ncbi.nlm.nih.gov/articles/PMC4986943)
- Social immunity: [ISTA social immunity, PNAS 2018](https://socialimmunity.ista.ac.at/?p=2250), [ISTA record](https://research-explorer.ista.ac.at/record/1395.aref), [Cosmos](https://cosmosmagazine.com/biology/ants-kill-the-sick-to-save-the-rest)
- Anteater feeding bouts: [Zoo Leipzig](https://www.zoo-leipzig.de/en/animal/giant-anteater/), [Cleveland Metroparks](https://resourcelibrary.clevelandmetroparks.com/api/assets/documents/2b0f4dec-20bd-4fd5-b264-5a6df2788748), [New World Encyclopedia](https://www.newworldencyclopedia.org/entry/Vermilingua)
- Rescue behaviour: [Nowbahari et al. 2009 (PMC)](https://pmc.ncbi.nlm.nih.gov/articles/PMC2719796), [Live Science](https://livescience.com/7954-ants-save-mates-trapped-sand.html), [Formica preprint 2024](https://preprints.org/manuscript/202402.1552/v1)
- Megaponera medics: [Science Breaker](https://thesciencebreaker.org/en/breaks/evolution-behaviour/ant-medics-wound-treatment-in-a-predatory-species), [Smithsonian 2023](https://www.smithsonianmag.com/smart-news/these-ants-can-diagnose-and-treat-their-comrades-infected-wounds-180983526/), [Frank 2023 Nat Commun (PDF)](https://opus.bibliothek.uni-wuerzburg.de/files/35808/s41467-023-43885-w.pdf)
- Slave rebellion: [Mainz press release](https://press.uni-mainz.de/slave-rebellion-is-widespread-in-ants/), [Achenbach & Foitzik 2009 (BioOne)](https://bioone.org/journals/Evolution/volume-63/issue-4/j.1558-5646.2009.00591.x/First-Evidence-for-Slave-Rebellion--Enslaved-Ant-Workers-Systematically/10.1111/j.1558-5646.2009.00591.x.full)

Games (searched this session):

- SimAnt: [Wikipedia](https://en.wikipedia.org/wiki/SimAnt), [Giant Bomb](https://www.giantbomb.com/games/3030-15236/)
- Alien: Isolation behaviour tree: [Game Developer](https://www.gamedeveloper.com/design/revisiting-the-ai-of-alien-isolation), [Gamasutra](https://www.gamasutra.com/design/the-perfect-organism-the-ai-of-alien-isolation)
- The Last Guardian: [Wikipedia](https://en.wikipedia.org/wiki/The_Last_Guardian), [KeenGamer](https://www.keengamer.com/articles/features/opinion-pieces/making-sense-of-trico-the-last-guardians-misunderstood-hero)
- Black & White creature (low-quality summaries; better primary sources exist): [Richard Evans (Wikipedia)](https://en.wikipedia.org/wiki/Richard_Evans_(AI_researcher)), [summary article](https://www.itechguides.com/how-black-whites-creature-learning-ai-and-morality-system-work/)
- Rain World reputation: [Rain World wiki, Behavior](https://rainworld.miraheze.org/wiki/Behavior)
- Plague Inc.: [Wikipedia](https://en.wikipedia.org/wiki/Plague_Inc.), [Games for Change](https://www.gamesforchange.org/games/plague-inc/)
- Viva Piñata: [Wikipedia](https://en.wikipedia.org/wiki/Viva_Pi%C3%B1ata_(video_game))
- Niche: [GOG](https://www.gog.com/en/game/niche), [developer blog](https://www.goodreads.com/author_blog_posts/13858580-niche-a-survival-game-based-on-the-real-science-of-genetics?tab=author)

From knowledge, not checked this session `[K]`: Hinton & Nowlan 1987 (Baldwin
effect); Hölldobler & Wilson 2009 *The Superorganism*; Bonabeau, Theraulaz &
Deneubourg 1996 (response thresholds); Gordon 2013 *Nature* (restraint and
offspring colonies); Pratt et al. 2002 (quorum in house hunting); Wilson 1975
(*Pheidole* enemy specification); Provecho & Josens 2009 (odour learning
through trophallaxis); *Lasius* carrying aphids and over-wintering their eggs;
*Pseudomyrmex* clearing vegetation; semi-claustral founding; Carrion and Spore
as design references.
