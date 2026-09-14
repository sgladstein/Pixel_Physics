# When animals fight, why they mostly don't, and what this engine can do about it

**Lane D of evolution-lab round 35.** The owner's ask, in his words: *"Do
some research on when and why different types of ants, bugs and bigger
creatures fight and how that might relate to our game. Implementation."*

So this is a research report that ends in a build, not a bibliography. §1–§6
are the literature, §7 is the mapping onto levers that exist, §8 is what I
measured in the engine while writing it, §9 is what I built, and §10 is what
I deliberately did not build — which in this repo is the half that earns the
rest.

**Scope.** Conflict *between animals*: when it happens, what decides it, and
what it costs. Out of scope and owned elsewhere: the switch that makes two
colonies strangers at all (Lane C's `why-colonies-do-not-fight-2026-09-14.md`,
on an unmerged branch as this is written, so it is named rather than linked);
the colony's food economy (`colony-food-economy-design-2026-09-14.md`, same);
predator–prey population stability, which
[`population-dynamics-research.md`](population-dynamics-research.md) already
covers and this report leans on rather than repeats.

**How to read the sourcing.** This repo's research reports are trusted
because they mark the line, so:

| mark | means |
|---|---|
| **[lit]** | found in the literature this session, with the link in §11 |
| **[reasoned]** | my own inference, sourced to nothing — treat as a hypothesis |
| **[measured]** | run in this tree this session, with the command and the numbers |

Where I could not source something I have kept it and marked it
**[reasoned]** rather than dropping it or letting it borrow the authority of
the paragraph above.

---

## 0. Summary, stated first

1. **Fighting is the rare tail of a distribution whose bulk is assessment and
   withdrawal, and that is not a softening of the mechanic — it is what makes
   the rare fight legible.** Across every assessment model in the literature
   the measured regularity is the same: **escalation falls as the asymmetry
   between contestants rises**, and most encounters end before contact
   **[lit]**. The engine has the opposite shape today: an encounter *is* a
   bite, with nothing in between. That is `CLAUDE.md`'s first law failing in
   a new costume — an outcome that is a binary rather than a distribution.

2. **In ants specifically, the graded channel is the assessment and not the
   damage, which is a stronger claim than it sounds and it is measured.**
   Czaczkes et al. 2024 on *Lasius niger*: overt **aggression does not vary**
   with genetic relatedness or spatial distance, while **antennation and
   jerking do** — and both are nearly absent between species **[lit]**. So
   the thing that carries the information about *who you are* is the
   low-intensity behaviour, not the fight. Hölldobler's *Myrmecocystus
   mimicus* tournaments are the same finding at colony scale: hundreds of
   ants in stereotyped display, almost no physical fighting, and the border
   slides toward whichever colony is outnumbered **[lit]**.

3. **The engine's swarm rule is already Lanchester's square law, built for an
   unrelated reason — and the one quantitative test on a real ant says the
   square law is too strong.** Damage banks on the *victim*
   (`OrganismState::gnawed`), so many weak mouths bring down what one cannot;
   that is all-against-all attrition, in which strength goes as the square of
   number. But fire ant (*Solenopsis invicta*) mortality across numerical
   ratios came out approximately **linear**, not square **[lit]**. The
   honest position: *numbers matter more than individual quality* is well
   supported; *numbers matter quadratically* is not. Ship the numerical term
   with a weight on it and let the owner find the number.

4. **The shipped bed is not "a world with no rivalry mechanism". It is a
   supercolony**, in the precise sense the invasive-ant literature uses the
   word: a population in which nestmate recognition has been lost and
   intraspecific aggression with it **[lit]**. `Behavior::scent_spread`
   defaults to 0, every colony sits at one scent point, and every ant is
   every other's nestmate. That reframing matters because it says the default
   is a *real ecology* with a name and a known signature, not an absence.

5. **A stranger is already food, and nobody has noticed.** `ant` material
   carries `food_class: 1.0` and the shipped ant's gut sits at a neutral
   bias, so the moment two colonies fall outside each other's tolerance,
   each is **prey to the other's ordinary mouth** — through
   `adjacent_food_counted`, with no `Attack` weight anywhere **[measured]**.
   The engine therefore already models **intraguild predation** (eat your
   competitor) and models **interference competition** (fight it without
   eating it) only through the unwired `Attack` verb. The owner asked
   whether colonies eat each other; the answer is *yes, and eating is the
   path of least resistance, not fighting.*

6. **`scent_spread = 1.0` does not reliably make two colonies strangers; the
   dial wants to be at least 2.** The founding offset is uniform in `±spread`
   per signature slot and the acceptance radius is `tolerance + 1` = **1.0**
   at the authored tolerance, so two colonies drawn a spread apart are
   frequently still inside each other's tolerance. Measured over four seeds:
   at `spread=1` only **9.3%** of ordered live-ant pairs read as non-kin and
   **one seed in four produced no encounter at all**; from `spread=2` every
   seed meets and the figure plateaus around **0.34–0.37** **[measured,
   §8.3]**. Anyone shipping the rivalry switch should set the dial from that
   column, not from the name of the field.

7. **The verb that was missing is not a better fight. It is the encounter
   that does not become one, and *still tells the colony something*.** A
   withdrawal that produces nothing is a non-event, which fails
   `CLAUDE.md`'s second law as surely as no mechanic at all. A withdrawal
   that writes a quiet mark on the alarm plane is the tournament: the colony
   learns there are strangers at a place without anyone dying, and because
   `BrainInput::Alarm` is today the **only** wired route to `Attack`, that
   mark is also what massed a border in the first place.

8. **What I built** (§9): `src/sim/contest.rs`, assessment before
   commitment, shipped **on** with four dials in the environment and in the
   harness; a ~40-line hook in `creature.rs`'s existing fight branch;
   `contests` and `displays` on `CreatureStats`; and
   `examples/conflict_arena.rs`, the first instrument in this repo that can
   ask what two colonies do to each other. The change is **invisible to
   everything that ships** — the whole path sits behind `attack_urge > 0.0`,
   which no shipped genome opens, and when it is not assessing it does not
   even consume a random draw. Measured: **escalation 1.000 with assessment
   off against 0.520 with it on**, four seeds, same binary, one argument
   apart **[measured, §8.5]**. Half of contact is now withdrawal where all of
   it used to be a bite.

9. **Two of this work's own controls found real errors, one in the shipped
   verb and one in my own change, and both were caught by machinery rather
   than by review.** The arena's **specificity** arm found the first: `nearest_foe` targets *any* living
   non-kin organism, and **a plant is an organism** — so an armed ant bites
   herbs, and a first version of my numerical term read a stand of foliage as
   an enemy army **[measured, §8.4]**. Nobody had seen it because no shipped
   species authors `Attack`. Fixed in the same change; kept here because it
   is the third time this repo has paid for a counter that was
   arithmetically correct about the wrong question. The second was mine: my
   numerical term counted **cells**, so a lone attacker facing one two-celled
   defender read as outnumbered two to one and **both sides of every duel in
   the world assessed themselves as the underdog**. It was caught by a
   shipped guard going red — the median time to breach a maximally armoured
   ant went 73 frames to 126 — and the fix is to count **animals**, with the
   animal counting itself onto its own side **[measured, §8.6]**.

---

## 1. When is fighting worth it at all

### 1a. The contest is decided by asymmetry, and the asymmetry is assessed

The modern frame is fifty years old and stable. Maynard Smith & Price's
hawk–dove game establishes that unconditional aggression is not an ESS in a
population that can be injured; Parker's assessment strategy adds that
contestants should weigh **resource holding potential** (RHP) — their
capacity to win a fight — against the value of what is contested; Enquist &
Leimar's **sequential assessment model** adds the part that matters most for
a simulation: *no useful assessment is possible before the contest begins*,
so information is gathered **as the contest proceeds**, and each round is a
sample of the RHP difference **[lit]**.

The regularity to build against: **contest escalation is negatively related
to RHP asymmetry** **[lit]**. Two closely matched animals fight long and
hard; a lopsided pair resolves almost immediately, usually with no contact at
all. Species span the range from pure display to lethal combat, and where a
species sits is set by size, weaponry and experience.

**The part that changes the implementation.** A 2019 meta-analysis of animal
contests found **stronger support for self-assessment than for mutual
assessment** **[lit]** — i.e. most animals appear to give up on the basis of
their *own* state (their fatigue, their damage) rather than on a read of
their opponent. That is a cheaper mechanism than it sounds and it is the one
a cell-grid engine can actually express, because "my own state" is local and
"their RHP" is a perception problem. **[reasoned]** It also argues against
building an elaborate opponent-perception channel as the first step: the
literature's own weight of evidence says the simpler thing is the commoner
one.

### 1b. Ownership, and why the resident usually wins

Prior residence is the most robust asymmetry in the whole literature after
size. The standard explanation is informational rather than mystical: **the
owner is better informed about the value of the resource than the intruder**
**[lit]**, having sampled it, so the two sides are not playing the same game
even when they are the same size.

### 1c. Assessment itself is costly, and that cost is the point

Displays are not free — they take time, energy, and exposure — which is
exactly why they are informative rather than cheap talk. **[reasoned, from
the standard signalling frame]** For a simulation this is the design note: a
withdrawal mechanism that costs nothing will be used by everything, and the
distribution collapses back to a binary at the other end.

### 1d. Economic defendability: fight over *what*

Brown's 1964 criterion is the one to reach for whenever the question is
territory rather than a single item: **defend when the benefit of exclusive
use exceeds the cost of exclusion** — which makes territoriality a function
of resource **density and predictability**, not of temperament **[lit]**.
Dense, predictable resources shrink the area that must be defended and so
make defence pay; sparse, unpredictable ones select for dispersal and
mobility instead **[lit]**.

**This is the single most transferable finding in the report for the lab,
and it is a statement about the *bed*, not about the ants.** A box of evenly
spread herbs is the least defendable larder that can be built. If the owner
wants to see territoriality, the bed has to be clumped before the ants have
anything to be territorial about. **[reasoned]**

---

## 2. Ants

### 2a. Nestmate recognition is a tolerance, and we already model it exactly

Colony odour in ants is a **gestalt**: cuticular hydrocarbon (CHC) profiles
are blended between nestmates so the colony carries one shared signature, and
an individual classifies others by **a recognition threshold** on the
difference between the profile it smells and the one it carries **[lit]**.
CHC profiles are heritable and vary between colonies **[lit]**.

This is not an analogy for what the engine does; it is a description of it.
`TRAIT_SCENT_A/B/C` is the profile, `TRAIT_TOLERANCE` is the acceptance
threshold, `creature::scent_accepts` is the comparison, and
`blend_with_nest` is the gestalt — an ant standing on its nest takes some of
the nest's odour and leaves some of its own, which is how a real colony
re-mixes one signature through the nest material. **The engine's kin model
is the literature's model**, and `is_living_kin`'s own doc already says the
important half out loud: *kin is a distance, not a bit.*

Two consequences worth stating because they are free:

* **A drifted lineage becoming a stranger to its own colony is speciation
  with a speed dial** (`scent_drift`), and it is the same mechanism as the
  rivalry, not a second one.
* **Recognition is asymmetric by construction** — `scent_accepts` takes the
  judge's tolerance against the other's scent — which is biologically right
  (two colonies can disagree about whether they are enemies) and is a
  property no ordinary distance metric would have given us.

### 2b. Dear enemy versus nasty neighbour: the honest answer is "no pattern"

The two named effects are real and opposite. **Dear enemy**: neighbours
become *less* aggressive to each other once borders are established, because
a known neighbour is a smaller threat than an unknown intruder. **Nasty
neighbour**: the reverse, because the neighbour is the one actually competing
for your resources **[lit]**.

**In ants, the literature does not resolve.** Several studies report dear
enemy, several report nasty neighbour, others report complex interactions or
no effect at all **[lit]**. Weaver ants are nasty neighbours **[lit]**;
*Azteca* in *Cecropia* show a dear enemy effect **[lit]**; *Formica
pratensis* nasty **[lit]**.

**So: do not model it.** §10 has this in full, but the short version is that
a mechanism whose empirical sign is genuinely unsettled is one this engine
should not be asserting — and the good news is that we get the *axis* for
free anyway, because scent distance is continuous and a neighbouring colony
that exchanges no ants drifts further from you than a distant one does.

### 2c. The Czaczkes result, which is the one that shaped the build

*Lasius niger*, 2024: pairs of workers from colonies of known genetic
relatedness and known spatial separation, scored for antennation, jerking,
and overt aggression. **Aggression did not vary with relatedness or with
distance. Antennation and jerking did** — both decreasing between less
related and more distant pairs — **and both were nearly absent in
interspecific interactions** **[lit]**. The authors' reading is that the
species has intraspecific communication strategies that avoid costly
fighting.

Read that as an instruction for a simulation and it says: **the graded,
informative, common behaviour is the sub-lethal one.** An engine that models
only the bite has modelled the rarest and least informative part of what two
ants do when they meet. **[reasoned]**

### 2d. Tournaments: assessment at colony scale, and the border that moves

*Myrmecocystus mimicus* colonies hold **ritualised tournaments**: hundreds of
workers in stereotyped stilt-legged display, raising abdomens and heads, with
**almost no physical fighting**. The tournament is intercolony
*communication* — the colonies gauge each other's strength — and it
establishes a temporary spatial border inside which each forages. **If one
side outnumbers the other the tournament shifts toward the smaller colony's
nest**, and if the asymmetry is large enough the smaller colony retreats and
seals its entrance; if it is too slow, the larger colony raids the nest,
takes the brood and the honeypots, and kills or drives off the queens
**[lit]**. Colonies with more workers win tournaments and raids, in both
incipient and mature colonies **[lit]**.

Every element of the design is in that paragraph: a graded outcome, an
assessment that is mostly not a fight, a visible border whose *position* is
the readout, numerical asymmetry as the deciding quantity, and escalation as
the tail rather than the body of the distribution.

### 2e. Colony size, and how much numbers are worth

Lanchester's models were written for human warfare and imported to social
insects. The **linear law** applies when combat is a series of one-on-one
duels: fighting strength goes as group size. The **square law** applies when
combat is all-against-all: fighting strength goes as the *square* of group
size, so a numerically superior force of individually weaker fighters wins —
which, if true, argues for many small workers over few large ones **[lit]**.

**And the empirical tests do not support the square law.** Fire ant
mortality across numerical ratios was approximately **linear** in group size
**[lit]**; a later experimental test on the termite *Nasutitermes corniger*
also found the larger army advantaged, but **less than the square law
predicts** **[lit]**.

The engine's arrangement is worth noting against this. `st.gnawed += damage`
banks on the **victim**, so `n` attackers wear one target down `n` times
faster while the target's single mouth works on one of them — which is the
all-against-all structure, i.e. nearer the square law than the linear one.
The test's own comment records that writing it as arithmetic would have been
a tautology that *"would pass just as happily with the damage banked on the
ATTACKER, which is the one arrangement under which swarming does not work."*
**[reasoned]** So our attrition may be *stronger* in numbers than real ants
are. That is a reason to expose the numerical weight as a dial rather than
to pick a value.

### 2f. Territoriality, food monopolisation, and the trade that structures a
whole community

The organising idea in ant community ecology is the **dominance–discovery
trade-off**: species that defend food against competitors (behaviourally
dominant, good at **interference**) tend to be *worse* at finding it first,
and species that discover resources fast tend to lose them (good at
**exploitation**) **[lit]**. The mechanism offered is an investment split:
*more scouts discover more; more recruits harvest and defend better*
**[lit]**. The trade-off is credited with maintaining ant community
diversity, and high interference ability is what lets invasive species
monopolise resources **[lit]**.

**This transfers directly and it costs no new currency.** The engine's
colony already spends one budget on one population; scouts and soldiers are
the same joules. §7 has the mapping.

### 2g. Brood raiding, and why founding colonies cooperate

Incipient colonies raid each other for brood — workers take brood from other
incipient nests, often without opposition — and **brood raiding is the
proposed primary selective force behind pleometrosis**, the cooperation of
several founding queens in one nest **[lit]**. Co-founding produces a larger
first worker cohort sooner, which is protection against exactly that raiding
**[lit]**, and fecundity determines which queen survives the association
**[lit]**.

This is a *beautiful* shape for a game — the vulnerability window of a young
colony, and cooperation as the answer to it — and it is also the one part of
this report I think we should not build yet. §10.

### 2h. Polydomy and unicoloniality: why some colonies never fight

Polydomous colonies occupy several nests and their workers must recognise
nestmates from other nests; in polydomous systems workers from different
nests of the *same* colony are never aggressive, while alien conspecifics are
attacked regardless of spatial or genetic distance **[lit]**. At the extreme,
**unicolonial** invasive populations form supercolonies spanning huge
distances, in which aggression between non-nestmates is **absent because the
chemical recognition system has been lost** — which eliminates intraspecific
competition altogether **[lit]**.

That last sentence is a description of our shipped bed. See §0.4 and §8.1.

---

## 3. Other insects

### 3a. Solitary insects fight over mates and oviposition sites, not food

Territorial *Calopteryx* damselfly males defend patches of floating
vegetation that females use as **oviposition sites**, and males engage in
prolonged, highly escalated aerial contests for them; the payoff is exclusive
access to an area attractive to females plus protection of mated females
while they lay **[lit]**. Where solitary insects *do* contest a food
resource it is usually because the food is also the reproductive resource —
male roller beetles fight over food balls needed for mating and nesting, with
size and prior reproductive experience predicting the outcome **[lit]**.

**The design reading: the thing worth fighting over is the thing that cannot
be divided.** A leaf can be shared by taking turns; a nest site cannot.
**[reasoned]** Our engine's contested objects are food cells, which are
divisible and consumable, and that is part of why nothing fights.

### 3b. Scramble versus contest, which may matter more than anything else here

Nicholson's 1954 distinction. Under **scramble** competition the resource is
divided evenly among all competitors and everyone suffers equally — at high
density, *all* may get too little and die. Under **contest** competition some
individuals monopolise and do well at the expense of the rest; the costs are
asymmetric **[lit]**.

The population consequence is the part to carry away: **contest competitors
have more stable dynamics; scramble competitors show cyclic and, at high
reproductive rate, chaotic behaviour** — Nicholson's blowflies are the
canonical extreme case **[lit]**.

**Our bed is pure scramble.** Every ant races for the same cells, nothing is
held, nothing is excluded. And the lab's standing complaint —
booms and crashes, a colony that reaches the larder's edge and falls over —
is *the documented signature of scramble competition*, not obviously a
tuning failure **[reasoned]**. This connects two lines that have not been
connected: [`population-dynamics-research.md`](population-dynamics-research.md)
§4 argues that space is the stabiliser, and contest competition is the other
one. A mechanism that lets part of a colony *hold* something is a stability
mechanism as much as a combat one.

---

## 4. Bigger animals

### 4a. Interference versus exploitation

The two ways to compete: use the resource up before your rival gets there
(**exploitation**), or stop them getting there at all (**interference**)
**[lit]**. The engine does the first completely and the second not at all,
which is §7's central row.

### 4b. Intraguild predation — eating your competitor is a different strategy

Intraguild predation (IGP) is the killing and eating of a potential
competitor: predation and competition at once, since both species use the
same prey resource *and* one eats the other **[lit]**. Basic theory predicts
IGP should hamper coexistence, yet it is common in nature, and **a weaker
competitor can persist by eating its stronger competitor** **[lit]**.

The conditions under which IGP and coexistence are compatible are, every one
of them, things this engine has or can have **[lit]**:

* **Habitat complexity and spatial structure** — `compartments`, caves,
  material-gated movement. [`population-dynamics-research.md`](population-dynamics-research.md)
  already makes this argument for predator–prey and it applies unchanged.
* **Alternative food** — an omnivorous gut, which the ant has.
* **Productivity** — IGP predators persist where the limiting resource is
  abundant, i.e. the grow lights.

**The owner asked about eating, and this is the literature for it.** §8.2
says the engine already does it.

### 4c. Territory economics again, at the scale it was written for

Brown's criterion (§1d) is a large-animal model and it is where the
"defendability" vocabulary comes from. The one line worth repeating for
whoever designs the next bed: **raising the average density of a critical
resource makes a territorial system more defendable, by shrinking the area
that has to be defended** **[lit]**.

---

## 5. Scavenging and cannibalism

Ants do eat the dead, including their own. Recent work gave direct evidence
of **cannibalistic necrophagy** — fluorescently marked corpses detected in
nestmates' digestive tracts — as a route for recycling nitrogen, described as
a constantly available food source used **mainly when food is scarce**
**[lit]**. The frequency of the behaviour **increases during starvation**,
and ants moderate it by the perceived **pathogen infection level** of the
corpse: the main reason necrophagy is rare is disease risk, not squeamishness
**[lit]**.

Two clean mappings:

* **Hunger-gating is the mechanism, not a balance patch.** If corpse-eating
  is to be rarer than leaf-eating, the literature's own reason is that it is
  *conditional on scarcity*, and the engine has the condition to hand
  (`BrainInput::Energy` is already an input, and `gut_bias` already prices
  carrion against plant matter).
* **Pathogen risk is a real, sourced reason to make carrion carry a cost** —
  and `spoil` already exists as a material. I am not proposing it; I am
  recording that if anyone ever wants a downside on scavenging, this is the
  one with evidence behind it. §10.

---

## 6. What all of that says the mechanic should look like

Collapsing §1–§5 into design language, before touching a lever:

1. **An encounter is the event; a fight is one of its outcomes.** Build the
   encounter.
2. **The outcome is graded by asymmetry, and the asymmetry has (at least)
   two terms: how hard we each are to hurt, and how many of us there are.**
   Numbers probably matter more, but not quadratically.
3. **Withdrawal must produce something**, or the middle of the distribution
   is indistinguishable from nothing happening.
4. **The visible object is a border, not a kill.** The tournament's readout
   is *where the line is*, and it moves.
5. **What is worth fighting over is what cannot be shared**, and a bed of
   evenly spread food has nothing of that kind in it.

---

## 7. The mapping: lever we have, lever we would need, thing not to model

| finding | status | the lever |
|---|---|---|
| Nestmate recognition as a **tolerance** on a blended colour signature (§2a) | **have, exactly** | `TRAIT_SCENT_A/B/C`, `TRAIT_TOLERANCE`, `scent_accepts`, `blend_with_nest` |
| Speciation: a lineage drifting out of its own colony (§2a) | **have** | `scent_drift`, `World::regroup_by_scent`, `group_label`'s `ANT 3b` |
| Two colonies as strangers (§2h) | **have, mis-sized** | `scent_spread` — but see §8.3: 1.0 is not enough to clear a tolerance radius of 1.0 |
| Unicoloniality / supercolony (§2h) | **have, and it is the default** | `scent_spread: 0` |
| **Intraguild predation** — eat your competitor (§4b) | **have, unnoticed** | `food_class: 1.0` on `ant`, a neutral `gut_bias`, `eats_kin` as the gate. §8.2 |
| Swarm attrition / Lanchester (§2e) | **have** | damage banked on the victim (`OrganismState::gnawed`) |
| Armour ladder without evolution (§1a) | **have** | `chitin_pale` 0.5, `chitin_mid` 0.7, beetle 0.8 against ant 0.25 |
| A body with a middle that can be severed (§1a) | **have** | `Chain(n)`, `ant_long.ron`, `longant.ron` |
| **Assessment before commitment** (§1a, §2c, §2d) | **was missing — built, §9** | `sim::contest`, hooked into `act`'s fight branch |
| **The sub-lethal encounter that still signals** (§2c, §2d) | **was missing — built, §9** | a display writing `DISPLAY_DEPOSIT` into the alarm plane |
| **Numerical asymmetry as a read quantity** (§2d, §2e) | **was missing — built, §9** | kin/foe cells counted in the ring walk `nearest_foe` already makes |
| Prior residence / owner advantage (§1b) | **would need** | nothing reads "I have been here before". The nest gestalt is the nearest thing and it is not the same quantity |
| Interference competition proper — holding a place against a rival (§4a) | **would need** | a reason to stand somewhere that is not "there is food in my mouth-range". §10.2 |
| Economic defendability (§1d, §4c) | **would need a *bed*, not code** | clumped, predictable food. `LabBox`'s evenly spread founders are the least defendable larder available |
| Contest competition as a stability mechanism (§3b) | **would need** | the above; this is the same lever wearing a population-dynamics hat |
| Dear enemy / nasty neighbour (§2b) | **do not model** | §10.1 |
| Brood raiding and pleometrosis (§2g) | **do not model yet** | §10.3 |
| Ritualised display as a *separate* behaviour with its own animation (§2d) | **do not model** | §10.4 |
| Pathogen cost on necrophagy (§5) | **do not model** | §10.5 |

---

## 8. What I measured in the engine while writing this

All of this is **[measured]** in this tree today, at `RAYON_NUM_THREADS=4`,
with `examples/conflict_arena.rs` unless another instrument is named. Counts
downstream of the parallel sweep are only comparable at pinned parallelism
(`CLAUDE.md`), which is why the pin is stated.

### 8.1 The default bed is a supercolony, and that is a sentence with a test

`scent_spread` defaults to 0; `colony_scent_offset` is uniform in
`-spread..=spread` per signature slot; so every colony lands on the species'
authored point and `scent_accepts` is true for every pair. The arena's
specificity control runs exactly this bed and asserts the conflict counters
read **exactly zero** — which they now do.

### 8.2 A stranger is food, with no `Attack` weight anywhere

`assets/materials/ant.ron` carries `food_class: 1.0` and
`penetration_resistance: 0.25`. `ant.ron`'s gut sits at `gut_bias` 0.0, and
its own comment records that at a neutral gut *"corpse (class +1.0) and
seed/leaf/moss/litter (class -1.0) all clear `EAT_YIELD_THRESHOLD` at 30
against a bar of 12"*. `eats_kin` is false, and `is_living_kin` is the gate —
so live ant flesh is refused **only while the other ant is a nestmate**.

Measured, two colonies in contact, 4,000 frames, seed 0: **`x-kills 9`** —
nine kills whose attacker and victim carried different colony labels —
against **`fights 18`** and **`attack_kills 4`**. The kill log is verb-blind,
so more than half the killing was not the fight. And the whole-run `eats`
column rises **54 → 750** (four-seed median) when the only thing that changes
is whether the colonies recognise each other **[measured, §8.3]**: at the
shipped `spread` there is one family and 48 ants eat plants; as strangers,
they are each other's largest standing food source.

**This is the answer to the owner's question and it is not the answer
anybody expected.** Turning rivalry on does not produce a war. It produces
predation.

### 8.3 `spread=1.0` is not a stranger; `spread=2` is enough

`tolerance_radius` is `TRAIT_TOLERANCE.clamp(-1,1) + 1.0`, and the ant
authors tolerance 0.0, so the acceptance radius is **1.0**. Two colonies
whose offsets are each drawn uniformly from `±spread` per slot are frequently
closer than that.

Four seeds, 4,000 frames, two colonies of 24 on a 384x224 bed, `attack=4.0`,
`RAYON_NUM_THREADS=4`, medians. `strangers` is the fraction of **ordered**
live-ant pairs reading non-kin (`scent_accepts`, both directions, because the
predicate is deliberately asymmetric); `met` is how many seeds produced any
encounter at all.

| `spread` | `strangers` | `met` | contests | fights | displays | escalation | x-kills | `eats` |
|---|---|---|---|---|---|---|---|---|
| 0 | **0.000** | **0/4** | 0 | 0 | 0 | — | 0 | **54** |
| 1 | 0.093 | 3/4 | 36 | 20 | 16 | 0.545 | 10.5 | 583 |
| 2 | 0.342 | 4/4 | 42 | 22.5 | 20 | 0.521 | 12.5 | 768 |
| 3 | 0.334 | 4/4 | 41.5 | 23 | 20 | 0.520 | 12.5 | 762 |
| 4 | 0.354 | 4/4 | 43 | 23 | 20.5 | 0.520 | 13 | 750 |
| 6 | 0.374 | 4/4 | 43 | 23 | 20.5 | 0.520 | 13 | 750 |

Three readings:

* **The dial is a threshold, not a slope.** Everything saturates by
  `spread=2` and rows 3, 4 and 6 are within noise of each other. Below it the
  switch is unreliable rather than weak: at `spread=1` **one seed in four
  produced no encounter at all**, which in a single-run report would read as
  "the mechanism does not work".
* **`contests` is a bad readout of whether the switch took and `strangers`
  is the good one.** Contact saturates while recognition is still only a
  third resolved, because a border needs only a few stranger pairs to start.
  That is why the arena prints both.
* **`eats` goes from 54 to over 700 — a fourteenfold rise in total eating
  caused by nothing but kin recognition.** That is §8.2 in one column: the
  strangers are not mainly fighting, they are mainly *food*.

The rows are monotone-then-flat, which `CLAUDE.md` warns is more often an
artifact than an effect. What protects this one is that the mechanism is
arithmetic rather than emergent — a uniform offset against a spherical
acceptance region — so a saturating curve is what the geometry predicts, and
that the two ends are controls (`spread=0` reads exactly zero in every
column; `spread=6` is byte-identical to `spread=4`). It is still four seeds.
Treat the shape as settled and any single number as ±one seed.

### 8.4 The specificity control found a defect in the shipped verb

`nearest_foe`'s own doc is explicit that it asks *"is it alive, is it
somebody else, and is it not mine"* and deliberately does not filter by diet,
so that an animal can defend itself against something it cannot digest.
**A plant is alive, is somebody else, and is not mine.**

So an armed ant attacks herbs. Nobody had seen it because no shipped species
authors a weight on `Attack`. It surfaced here because my first numerical
term counted foes the way the target rule finds them, which made a bed with
**no strangers in it at all** report **710 contests** in 4,000 frames — the
ants were assessing the foliage. Worse than the wrong count: an ant standing
in a herb would have read itself as hopelessly outnumbered and gone timid in
exactly the places a colony forages.

Fixed by splitting the two questions: the **target** rule is unchanged (a
plant is still attackable, exactly as before), and the **count** is over
`MaterialKind::Creature` only. The assessment itself now runs only when the
target is an animal, so biting a plant takes the original code path and does
not even consume a random draw.

Recorded at length because of what caught it. Not a test, not review — the
**specificity arm of a positive control**, written because `CLAUDE.md`
insists on one, run once, red immediately.

### 8.5 The mechanic has a middle, and here is the A/B

Same bed, same four seeds, **same binary**, one argument apart:

| arm | contests | fights | displays | escalation |
|---|---|---|---|---|
| `assess=off` | 31.5 | 31.5 | 0 | **1.000** |
| shipped | 43 | 23 | 20.5 | **0.520** |

**All of contact used to be a bite. Half of it is now a withdrawal.** That is
the middle the first law asks for, and it is stable across the whole `spread`
sweep (0.520–0.545) while the number of encounters triples, so it is a
property of the assessment rather than of how busy the bed happened to be.

**Why 0.52 and not the 0.05-ish the literature describes, which is the honest
part of this section.** Two colonies of one species are *identical*:
`bite_progress` returns the same number in both directions, so `mine -
theirs` is exactly zero and the only live term is the local numerical one,
which in a border skirmish is usually one against one — parity. A coin at
parity is precisely what the war of attrition says an evenly matched contest
is. The withdrawal-dominant regime the ant literature reports is the regime
where the sides *differ*, and a symmetric bed cannot produce it.

Measured, and it is a good consistency check on the arithmetic: in this bed
`numbers=0` and `boldness=0` return **byte-identical summaries** (escalation
0.533), as do `numbers=4` and `boldness=12` (0.497). **In a symmetric bed the
two dials are the same knob**, because one of the two terms they weigh is
identically zero. They separate only when the colonies differ.

`armour=<allele>` is the knob that makes them differ — `TRAIT_ARMOUR` on the
second colony's founders. Four seeds at `spread=4`, survivors as
colony 1 / colony 2:

| `armour` on colony 2 | survivors | x-kills | escalation |
|---|---|---|---|
| 0 (symmetric) | 15.5 / 15.5 | 13 | 0.520 |
| 4 | **11.5 / 18.5** | 13 | 0.454 |

**The plated colony wins, which is the whole claim.** And note what the
escalation column does *not* show: **a rate pooled over both colonies cannot
see an asymmetry, because the two sides move in opposite directions** — the
outmatched side withdraws more and the plated side commits more, and the
pooled median is their average. Reading the mechanism off that column would
have been the "different question, same number" failure again. The readout
that works is the outcome.

### 8.6 What my own change got wrong, and what caught it

The numerical term counted **cells**, not animals. A lone attacker facing one
two-celled defender therefore read as outnumbered two to one — and so did the
defender, so **both sides of every duel in the world assessed themselves as
the underdog** and neither committed.

It was caught by `a_maximally_armoured_ant_is_graded_only_when_the_reach_
allows_it` going red: the median time to breach a maximally armoured ant went
**73 frames to 126** against a bar of 100. Fixed by counting distinct
animals, with the animal counting **itself** onto its own side, so a fair duel
reads as parity rather than as 0 against 1.

Two things worth carrying out of it:

* **`BrainInput::Crowding` counts cells and is right to**; it is asking how
  full the neighbourhood is. This is asking how many opponents there are.
  Same walk, different question, and reusing the first answer for the second
  was the error.
* **The bar itself then had to be re-derived, and that is part of the fix
  rather than scope creep** (`CLAUDE.md`). Measured one env switch apart,
  same binary: **73 with assessment off, 126 with it on**, because at a reach
  of 1 both sides saturate the damage curve, the assessment reads an exact
  parity, and half the closures take roughly twice the frames. The claim the
  bar is named for is *better* satisfied than before — `narrow_alive` fell
  from **2 of 6 to 1 of 6**, so the useless plate saves fewer defenders, not
  more — and the contrast the test exists for is unharmed at **15.9x**
  against a bar of 5. The bar moved to 300: 2.4x the measured value, and
  still 6.7x under what a scene whose ants never meet would report, which is
  the fault it is there to catch.

---

## 9. What I built

### 9.1 `src/sim/contest.rs` — assessment before commitment

Pure arithmetic, no state, no world access, tested without a bed. Given what
one bite of mine does to them, what one bite of theirs does to me, and the
local numerical odds, it returns **how likely this animal is to commit**.

```text
asymmetry = (my_progress - their_progress) + numbers_weight * numbers
commit    = COMMIT_FLOOR + (1 - COMMIT_FLOOR) * logistic(boldness * asymmetry)
```

* `my_progress` / `their_progress` are the engine's **own** damage curve,
  `(bite/armour)²` clamped — moved into this module so the assessment and
  the bite that follows it cannot hold different opinions about how hard a
  cell is. `creature.rs`'s own comment already warned about that second copy.
* `numbers` is `(kin - foes) / (kin + foes)` over the **creature** cells
  touching this body, from the ring walk `nearest_foe` already makes.
* Four properties are asserted by tests in the module: the floor holds at
  every asymmetry and every dial setting; an even match reads ~0.5; the
  function is monotone in the asymmetry; and **the numerical term moves the
  commitment on its own with the strength terms held fixed**, by more than
  half the range — the positive control for the term, because a term that
  moves the third decimal is a term nobody will ever see fire.

**It is a capacity, never an exemption.** `COMMIT_FLOOR = 0.05` is the whole
of this module's compliance with `dead-ends.md` :272/:276/:403, the rejection
that killed four successive support models and that
[`held-world-game-concept-2026-09-13.md`](held-world-game-concept-2026-09-13.md)
§10a names as binding here. There is no configuration under which an animal
cannot be attacked — a hopelessly outmatched attacker still commits on one
encounter in twenty, which is exactly the trickle the swarm rule needs in
order to bring down what one mouth cannot.

### 9.2 The hook, in `creature.rs`'s existing fight branch

About forty lines inside the `if attack_urge > 0.0` block that already
existed. Nothing outside that block changed. Three properties:

* **Invisible to everything that ships.** No shipped genome carries a weight
  on `Attack`, so the gate is one float comparison and nothing below it runs.
* **Byte-identical when not assessing.** The commitment roll is inside the
  branch rather than taken unconditionally against 1.0, so
  `PIXEL_PHYSICS_CONTEST=off` — and every bite at a plant — consumes no
  random draw and runs the original path exactly. A draw spent on a tick that
  used to spend none re-phases every later decision in the world, and an
  "off" arm that diverged within a few hundred frames would be measuring the
  shuffle.
* **The withdrawal delivers.** A declined encounter writes
  `DISPLAY_DEPOSIT` (40, against the 240 a wound writes) into the alarm plane
  **at the displaying animal's own cell** — the opposite of `cry_alarm`'s
  choice, for the matching reason: a bite is a fact about the victim, a
  display is a fact about the displayer.

### 9.3 Two counters on `CreatureStats`

`contests` (encounters assessed — the near side) and `displays` (encounters
that ended without a bite). `attacks` is the far side and was already there.
**`displays / contests` is where `CLAUDE.md`'s first law becomes a number**:
a fight mechanic whose every encounter escalates has no middle however busy
it looks.

### 9.4 `examples/conflict_arena.rs`

The first instrument in this repo that can ask what two colonies do to each
other. `instruments.md` was checked first: `creature_arena` races two
*genomes* in one bed for the same resources — exploitation competition by
construction, with one colony and attribution by lineage; `predation_probe`
is cross-species predation; `labforage` censuses one colony's larder.
Interference had no harness.

It founds two colonies by hand (because `scent_spread` is read at
**placement**, so a spread written after `LabBox::build()` reaches nobody and
the bed silently stays one family), edits **both sides identically**, and
reports per seed:

`contests` · `fights` · `displays` · `escalation` · `plantbites` ·
`attack_cells` · `attack_kills` · `eats` · `gnaws` · `x-kills` · `own-kills` ·
`deaths` · `casts`/`prey`/`threat` · `groups` · `strangers`

with a median summary over seeds. Three columns earn their place hardest:

* **`x-kills`**, read off `World::kills_log` and therefore **verb-blind** —
  it counts the same death whether the mouth or the fist did it, which is
  what makes the two verb counters readable. `own-kills` beside it is its
  control: a bed where both are high is a famine, not a war.
* **`strangers`**, the premise check of §8.3. A run reporting no conflict and
  a run whose colonies were never strangers look identical in every other
  column.
* **`fights` rather than `attacks`, and `plantbites` beside it.** `fights` is
  `contests - displays`, i.e. encounters *with an animal* that escalated.
  `attacks` is the raw verb counter and includes every closure on
  vegetation — reading escalation off it gave 0.860 on a bed whose real rate
  was 0.093, and on a leafier bed it would have exceeded 1.0. §8.4.

**It renders, too.** `gif=<path>` captures the bed through `render::Renderer`
— the renderer the game draws with, not a path of this file's own — with
`CreatureColour::Colony`, so the two colonies wear different colours and
*where the line is* can be seen rather than inferred. `seq=<dir>` writes the
same frames as individual PNGs for a scrubbable review card (the review skill
records a sequence playing for the owner where a GIF did not), `shot=WxH`
sets the capture viewport independently of the world so a card can reach the
700–950 px the owner can actually judge, and `zoom=`/`look=` frame it.

`control=selftest` runs both halves of the control in about a minute:
sensitivity (strangers in contact move every counter) and specificity (a
one-family bed reads exactly zero), plus the arithmetic check that the
assessment can say *no* and can never say *never*.

### 9.5 The dials, and the standing rule they ship under

*"Give me the tools, data, access to the parameters that need to be tweaked
and I do that testing myself in the game. That is the game."* So it ships
**on**, with everything exposed and nothing balanced:

| dial | default | what it asks |
|---|---|---|
| `PIXEL_PHYSICS_CONTEST` | `on` | the A/B arm, out of one binary |
| `PIXEL_PHYSICS_CONTEST_BOLDNESS` | 4.0 | how sharply an asymmetry changes a mind. 0 reads every encounter as parity — assessment that gathers no information, the null for *does reading the opponent matter as against merely hesitating* |
| `PIXEL_PHYSICS_CONTEST_NUMBERS` | 1.0 | numbers against strength. **The most interesting dial here**, because §2e's literature disagrees with itself about exactly this |
| `PIXEL_PHYSICS_CONTEST_DISPLAY` | 40 | how loud a display is, against 240 for a wound |

**In a symmetric bed `boldness` and `numbers` are the same knob** (§8.5), so
the arena also takes `armour=<allele>`, which plates the second colony's
founders and is the only way to give the strength term something to read.

The arena takes all four as arguments and sets them before any world exists,
so both arms run from one build.

**The in-game knob is not mine to add.** `src/lab/params.rs` is Lane B's
this round; the four values are plain floats on a `OnceLock` and a params row
for each is a one-line addition whenever that lane wants it.

---

## 10. What I deliberately did not build

### 10.1 Dear enemy / nasty neighbour

The empirical sign is unsettled **in ants specifically** (§2b): studies
report both effects and no effect. Asserting either would be the engine
taking a position the literature has not taken. And we would be paying for
it: it needs per-neighbour memory, which is state per colony pair, in a
system whose whole kin model is deliberately one distance computed on the
spot.

**What we get for free instead** is the axis without the claim. Scent
distance is continuous, and a colony that exchanges no ants with you drifts
further from you over time — so "how much of a stranger is this" is already
graded and already feeds every consumer. If the owner ever wants a dear-enemy
effect he can have it by making tolerance heritable in the right direction,
which it already is.

### 10.2 Interference competition proper — holding a place

This is the biggest real gap (§7) and I am not closing it tonight, because
it is not a combat mechanic. Holding a place requires a reason to *stand*
somewhere with no food in reach, which means a spatial goal the brain can
express, and the brain's outputs are currently all verbs about the cell you
are in. That is a design piece, not a patch, and it collides directly with
the movement work.

**And the cheaper half of it is not code at all.** §1d and §4c say
territoriality is a function of the *bed*: a clumped, predictable larder is
what makes defence pay. Before anyone builds a hold-position behaviour,
someone should run the existing colony on a bed with two rich patches instead
of eight even ones and see what happens. That is a `LabBox` argument away.

### 10.3 Brood raiding and pleometrosis

The most game-like material in the whole report (§2g) — a vulnerability
window for young colonies, and cooperation as the counter to it — and it
depends on two things the lab does not have settled: brood as an object
distinct from an adult, and colony founding as an event with a timeline. The
coordinator note is explicit that a queen is *"three authored values over
existing mechanisms, never a type the engine knows"*, and brood raiding needs
the queen line to land first. Filed, not built.

### 10.4 Display as its own animation or behaviour

Tempting, because §2d's tournaments are visually spectacular, and wrong at
this stage. It would be a second mechanism for the thing the assessment
already decides, it would need art, and `CLAUDE.md` is clear that a *look*
ships default-off pending the owner's eye while *behaviours* ship on. The
display is currently a mark on a field plane and a counter; if the owner
looks at the arena and wants to *see* it, that is the next card, not the next
commit.

### 10.5 A pathogen cost on scavenging

Sourced (§5) and real, and the material to carry it (`spoil`) exists. Not
built because it is a change to the food economy, which is another lane's
this round, and because adding a cost to a behaviour nobody has yet measured
the *rate* of is the unpriced-lever trap `why-changes-cost-so-much-2026-08-27.md`
is about. The place it belongs is a hunger-gating question, and that is §10.6.

### 10.6 Hunger-gating carrion

§5 says necrophagy is conditional on scarcity, and the engine has both halves
(`BrainInput::Energy`, `gut_bias`). I did not wire it because it belongs to
the gut and the gut belongs to the food-economy lane this round. Recorded
here so that whoever does it has the citation rather than a hunch.

### 10.7 A new brain input for "a rival is over there"

The obvious next step, and I want to argue against doing it *first*.
`Sightings` has four slots — prey, kin, threat, bloom — and a rival
conspecific is reported through **prey** and **threat**, because
`is_visible_prey` and `is_visible_threat` are diet questions and a
non-nestmate ant is food (§8.2). So the eye is not blind to rivals; it is
**blind to the category "rival"**, and reports them as dinner.

Whether that is a bug depends on a question the literature has an answer to
and we do not have a measurement for. §1a's meta-analysis finds **more
support for self-assessment than mutual assessment** — most contests are
decided by what an animal knows about itself, not about its opponent. So a
distal rival-detection channel may be a large piece of work buying a small
behavioural difference. Adding a brain input also widens the genome, which
re-derives every species' `mutation_rate` and invalidates `main` as a control
arm (round 33 paid exactly that). **Measure the contact-range mechanism
first; the arena is how.**

---

## 11. Sources, and the commands

### The literature

Contests and assessment
— [Pinto et al. 2019, *Biological Reviews*: meta-analysis, self vs mutual assessment](https://onlinelibrary.wiley.com/doi/10.1111/brv.12509)
([PubMed](https://pubmed.ncbi.nlm.nih.gov/30916473/))
· [Bradbury & Vehrencamp, *Principles of Animal Communication* ch. 11 web topics — sequential assessment, RHP](https://learninglink.oup.com/access/content/bradbury-animalcomm-2e-student-resources/bradbury-animalcomm-2e-chapter-11-web-topics)
· [The influence of experience on contest assessment strategies, *Scientific Reports*](https://www.nature.com/articles/s41598-017-15144-8)
· [Spatiotemporal dynamics of animal contests, *PNAS*](https://www.pnas.org/doi/full/10.1073/pnas.2106269118)

Lanchester and colony size
— [Do Lanchester's laws of combat describe competition in ants?, *Behavioral Ecology*](https://academic.oup.com/beheco/article/11/6/686/221706)
· [An empirical test of Lanchester's square law: fire ant *Solenopsis invicta*](https://pmc.ncbi.nlm.nih.gov/articles/PMC1559866/)
· [Experimental test in the termite *Nasutitermes corniger*, *Proc. B*](https://royalsocietypublishing.org/rspb/article/289/1975/20220343/86432/An-experimental-test-of-Lanchester-s-models-of)

Nestmate recognition
— [Cuticular Hydrocarbons, AntWiki](https://www.antwiki.org/wiki/Cuticular_Hydrocarbons)
· [Ant cuticular hydrocarbons are heritable and associated with colony productivity, *Proc. B*](https://royalsocietypublishing.org/rspb/article/287/1928/20201029/85700/Ant-cuticular-hydrocarbons-are-heritable-and)
· [Harvester ants use CHCs in nestmate recognition, *J. Chem. Ecol.*](https://link.springer.com/article/10.1023/A:1005529224856)

Dear enemy / nasty neighbour
— [Czaczkes et al. 2024, *Ecological Entomology*: "Not dear neighbours" — antennation and jerking, but not aggression, correlate with relatedness](https://resjournals.onlinelibrary.wiley.com/doi/10.1111/een.13291)
([open copy](https://epub.uni-regensburg.de/54939/))
· [Dear enemy effect in an *Azteca*–*Cecropia* system, *Scientific Reports*](https://www.nature.com/articles/s41598-021-85070-3)
· [Weaver ants encounter nasty neighbors rather than dear enemies, *Ecology*](https://esajournals.onlinelibrary.wiley.com/doi/10.1890/09-0561.1)

Tournaments, raiding, founding
— [Ritualized combat and intercolony communication in ants, *J. Theor. Biol.*](https://www.sciencedirect.com/science/article/abs/pii/0022519383900930)
· ["Fighting Rituals", *Science*](https://www.science.org/doi/full/10.1126/science.336.6083.838)
· [Intraspecific brood raiding, territoriality, and slavery in ants, *Am. Nat.*](https://www.journals.uchicago.edu/doi/abs/10.1086/284901)
· [Fecundity determines the outcome of founding queen associations, *Scientific Reports*](https://www.nature.com/articles/s41598-021-82559-9)
· [Colony co-founding in ants is an active process by queens, *Scientific Reports*](https://www.nature.com/articles/s41598-020-70497-x)

Polydomy and unicoloniality
— [Polydomy: the organisation and adaptive function of complex nest systems in ants](https://www.sciencedirect.com/science/article/pii/S221457451400073X)
· [Chemical discrimination and aggressiveness in a supercolony-forming ant, *Formica yessensis*](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3480379/)
· [Behavioral assays reveal mechanisms of supercolony formation in odorous house ants, *Scientific Reports*](https://www.nature.com/articles/s41598-023-35654-y)

Community competition
— [Dominance-discovery and discovery-exploitation trade-offs promote diversity in ant communities, *PLOS One*](https://journals.plos.org/plosone/article?id=10.1371%2Fjournal.pone.0209596)
· [Discovery–dominance trade-off among widespread invasive ant species, *Ecology and Evolution*](https://onlinelibrary.wiley.com/doi/10.1002/ece3.1542)
· [Maintaining diversity in an ant community: modeling, extending and testing the dominance-discovery trade-off, *Am. Nat.*](https://www.journals.uchicago.edu/doi/full/10.1086/510759)

Scramble and contest
— [Contest competition, ScienceDirect topic overview](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/contest-competition)
· [Scramble competition, ScienceDirect topic overview](https://www.sciencedirect.com/topics/earth-and-planetary-sciences/scramble-competition)

Intraguild predation
— [Intraguild predation, overview](https://en.wikipedia.org/wiki/Intraguild_predation)
· [Reciprocal intraguild predation and predator coexistence](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC6065335/)
· [Coexistence of intraguild predators and prey in resource-rich environments](https://pubmed.ncbi.nlm.nih.gov/18959316/)

Territory economics
— [The economics of territory selection (Brown 1964 criterion)](https://www.sciencedirect.com/science/article/abs/pii/S0304380020303975)
· [A test of the economic defendability model in cichlids](https://link.springer.com/article/10.1007/BF02984444)

Necrophagy
— [Direct evidence for cannibalistic necrophagy as nitrogen recycling in ants, *Ecology and Evolution* 2025](https://onlinelibrary.wiley.com/doi/abs/10.1002/ece3.72253)
([PMC](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12522066/))
· [Cannibalistic necrophagy is modulated by perceived pathogen infection level, *Scientific Reports*](https://www.nature.com/articles/s41598-020-74870-8)

Solitary insects
— [Wing shape and territorial contests in *Calopteryx virgo*, *J. Insect Science*](https://academic.oup.com/jinsectscience/article/12/1/96/887859)
· [Contests over reproductive resources in female roller beetles](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC5552088/)

### The commands

```text
# the control, both halves, about a minute
cargo run --release --example conflict_arena -- control=selftest

# the A/B, one binary, both arms
cargo run --release --example conflict_arena -- seeds=6 frames=24000
cargo run --release --example conflict_arena -- seeds=6 frames=24000 assess=off

# is it strength or is it numbers -- identical in a symmetric bed, see 8.5
cargo run --release --example conflict_arena -- seeds=6 numbers=0
cargo run --release --example conflict_arena -- seeds=6 boldness=0

# ...so make the colonies differ, which is what the strength term is for
cargo run --release --example conflict_arena -- seeds=4 spread=4 armour=4

# a card: the border, through the real renderer, animals coloured by colony
cargo run --release --example conflict_arena -- gif=/tmp/border.gif \
    seq=/tmp/seq shot=768x448 zoom=4 look=144,132 \
    spread=4 gap=0.15 ants=24 plants=6 frames=4000 every=125

# §8.3, the dial that actually makes strangers -- read the `strangers` column
for s in 0 1 2 3 4; do cargo run --release --example conflict_arena -- seeds=2 frames=4000 spread=$s; done

# the supercolony control
cargo run --release --example conflict_arena -- spread=0
```

Pin `RAYON_NUM_THREADS` before comparing two runs: every column here is a
counter, and `CLAUDE.md` records a pure count swinging 2.2x with machine
load because rayon's thread count moved under it.
