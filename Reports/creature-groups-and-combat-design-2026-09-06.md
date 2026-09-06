# Groups, friend and foe, and what a fight is — design, 2026-09-06

*Owner, after the first session in which ants and beetles lived, bred and
fought to a fluctuating equilibrium — "the most fun I have had in the game
yet": (1) I had to use the gut-bias overlay to see who was who; (2) I could
only graph the total creature count; (3) how are we defining groups? If I
place ants as multiple clicks, are they separate colonies, and does that mean
anything? Do ant colonies ever attack each other? How do creatures know
friend from foe? Once they evolve, is an ant always an ant? And: think
outside the box about how this could be played, decide whether the combat
mechanics and the evolutionary mechanisms under them are sufficient.*

*This report answers the questions from the code as it stood at PR #255,
records what landed today against (1)–(3), and lays out the design for
inter-colony and inter-species dynamics. Status: **design of record for the
creature line's group and combat work**; the shipped half is README's
"Creature groups status".*

---

## 0. The answers, stated once

| question | answer at PR #255 | after today |
|---|---|---|
| are two clicks two colonies? | **No.** Nothing in the engine knew a click had happened. The ants were all one population with one label: their species | **Yes, as an identity.** Every colony-tool click, single placement and jar release founds one `OrganismState::colony`; children are born into their parent's. It is graphed and coloured. Whether it *means* anything is a dial |
| do ant colonies ever attack each other? | **Never, and they could not.** Kin is `same SpeciesId`, so every ant is every other ant's nestmate | **Only with `colony rivalry` on** (ANTS page of the parameters panel, off by default). Then an ant of another colony is not kin, so it is prey to any gut that digests flesh — a hungry ant eats a stranger exactly as it eats a beetle. No aggression verb was added; §4 says why |
| how do creatures know friend from foe? | One rule, `creature::is_living_kin`: living tissue of my own species is kin and nothing else is. Foe is not a concept at all; there is only *food I can digest* (`diet_yield` over the gut gene) and *not food* | Same rule, now `same species && (rivalry off \|\| same colony)`. Still one predicate, still one definition |
| is an ant always an ant? | **Yes, by construction.** `state.species` is written once and never changes; a genome can drift as far as it likes and the animal is still kin to every ant and prey to every beetle. Speciation is impossible (`README.md` already records this for plants; it is equally true of animals) | Still yes. §3 is the design that makes it graded, and it is the next thing to build |
| could I see who was who? | Only with the overlay | Animals wear their colony's colour (or species', or their own — a three-way mode on the ANTS page), and the graph line under them is the same colour |

**The owner's "fluctuating equilibrium" was real and was predator–prey, not
war**: beetles are carnivores that see 64 cells and ants are their food; ants
are blind generalists that eat a beetle only when standing beside one. There
was never a second ant colony in the fight because there could not be one.

---

## 1. What a group was, in the code, and what each one meant

Four partitions existed at PR #255. Only one of them reached behaviour.

| partition | where | reaches behaviour? | reaches the player? |
|---|---|---|---|
| **species** (`SpeciesId`) | `state.species`, fixed at birth | **yes** — kin, prey, every `CreatureDef` number | the roster's SPECIES column; the census' `animal_species` |
| **lineage** (`state.lineage`) | claimed per founder, copied to children | no | the `FOUNDING LINES` overlay, the roster's `LINE` filter, the cell page |
| the placement gesture | nowhere | no | no |
| the gut gene | `traits[TRAIT_GUT_BIAS]` | yes — what is food | the `GUT BIAS` overlay, which is what the owner was using to tell ants from beetles |

Two facts about the first row that set the whole design:

- **Kin is one predicate, used in three places** — the mouth (`adjacent_food`
  and its counted twin), the eye (`is_visible_prey`) and the kin sense
  (`is_visible_kin`, feeding `KinNear`/`KinBearing`). That is deliberate and
  is the reason the colony rule could land in one function today. Anything
  that widens "who is kin" widens it everywhere at once, which is right.
- **There is no "foe".** An animal never decides to *attack*; it decides to
  *eat*, and eating a living animal is what a fight is. The prey has no sense
  for the predator (`PreyNear` is what the *eater* sees; the ant has no eyes
  and no `ThreatNear` even if it had), no flight verb and no defence. The
  beetle's armour is a per-cell material number that the graded bite wears
  down (`fe1e225a`). So "combat" today is: a hungry carnivore walks up to
  food that happens to be alive, and bites until the cell comes off.

And one about the scent world, which no colony rule can fix on its own:
`pheromone.rs` holds **two world-sized planes with no owner**. Every animal
that emits on A writes the same A; a second colony founded across the box
reads the first colony's home scent as its own. Rivalry can make them
strangers at the mouth and still leave them sharing one map.

---

## 2. What landed today, and exactly what each piece does not do

**Colony identity.** `OrganismState::colony`, `World::claim_colony`. One
label per *gesture*: the first animal that fits founds it and every later
station joins (`found_colony_of`, `lab::release_at`, the scene's beetles), so
a founding that tries fifty sites claims one number, and `ANT 3` is the third
thing the player put down. A bud copies its parent's beside `lineage`. A
plant has colony 0. It does not change a tick unless the dial below is on:
measured by the three guards in `creature.rs` (`a_stranger_colony_is_kin_
until_rivalry_is_on` has both arms).

**Colour.** `render::CreatureColour { Off, Species, Colony }` on the
renderer, `Colony` in the lab and `Off` in the outdoor game, with
`render::group_colour` the *one* definition read by the animal on screen and
by the graph line under it. It is a **replace, not a tint** — `GUT_TINT_*`
records that a 45% pull toward green lost a blind A/B and every subtle
recolour here has read as blank — with the body's countershading kept as a
luminance ratio. Posted as a before/after card rather than argued; see the
lane note for the verdict.

**The graph.** `World::live_creature_groups` is the census; the ANTS page
draws every group on one shared axis with a legend row per group in its
colour, grouped by colony or by species as the colour mode says, and a
wiped-out group stays listed at 0 while the sample ring remembers it. Kills
are on the row (`K<n>`); births per group are not yet drawn.

**The rivalry dial.** `World::colony_rivalry`, off. On, `is_living_kin`
requires the colony. That is the whole change, and its consequences are
exactly those of the existing predation path: a generalist ant's gut yields
`480 × 0.25 = 120 J` from a stranger's cell against a `12 J` threshold, so a
hungry ant beside a rival *will* bite; the kin sense stops pulling the two
colonies together; the beetle is unchanged because its kin was never an ant.
What it does not do: give either colony a reason to seek the other out, a way
to tell the other's trail from its own, or any response to being bitten.

**Measured, the same evening** — `labstats colonies=2 founders=8
frames=24000 rivalry=0|1`, three seeds, the per-group tally
`World::group_deaths` prints (alive / starved / killed, and by whom):

| seed | rivalry | ANT 1 | ANT 2 |
|---|---|---|---|
| 1 | off | 15 alive, 49 starved, **0 killed** | 7 alive, 39 starved, 0 killed |
| 2 | off | 14 alive, 42 starved, 0 killed | 7 alive, 40 starved, 0 killed |
| 3 | off | 17 alive, 39 starved, 0 killed | 14 alive, 41 starved, 0 killed |
| 1 | on | 13 alive, 35 starved, **9 killed by ANT 2** | 10 alive, 31 starved, 6 killed by ANT 1 |
| 2 | on | 44 alive, 36 starved, 5 killed by ANT 2 | 10 alive, 36 starved, 5 killed by ANT 1 |
| 3 | on | 15 alive, 42 starved, 3 killed by ANT 2 | 9 alive, 35 starved, 6 killed by ANT 1 |

So the dial does exactly and only what §1 said it would: with it off no ant
has ever killed an ant, and with it on both colonies kill each other **in
small numbers, both ways, on every seed** — a tenth to a fifth of the
deaths, the rest still starvation. That is a graded outcome and not a
war, and it is the honest size of "rivalry as predation": a hungry ant
beside a stranger bites, and nothing sends it looking for one. Seed 2's
44-ant colony is the world diverging, not the rule — its food came 93% from
plants against 73–81% elsewhere. Nothing here is tuned; the numbers are
what the readout in §4e was built to show.

---

## 3. Friend and foe: from a bit to a distance

### 3a. The defect in the current rule

`is_living_kin` is a *bit* — same species or not. That is `CLAUDE.md`'s first
law broken at the social layer: an outcome that is binary where the real
thing is a distribution. Real ants recognise nestmates by a cuticular
hydrocarbon profile: a *chemical signature*, compared against a *template*,
with a *tolerance*. Colonies with similar profiles fight less; drifted or
mixed colonies tolerate strangers; some species' profiles converge on their
hosts' and they are adopted (slave-making, inquilines). Every one of those is
a graded version of one comparison.

### 3b. The design: a heritable signature and a heritable tolerance

Add two things to the creature genome, and change one predicate.

- **`TRAIT_SCENT` — a short signature, two or three trait slots**, mutated
  per birth like every other slot. A founder is authored at its species'
  value (ant at one point, beetle at another, far apart); a colony is its
  founders' shared point plus a per-colony offset drawn at founding, so two
  clicks start a little apart and their children drift from there.
- **`TRAIT_TOLERANCE` — one slot**: how far a signature may be from mine and
  still read as kin.
- **`is_living_kin` becomes `|scent(other) − scent(me)| ≤ tolerance(me)`**,
  and species stops appearing in it at all. Species stays what it is —
  the body plan and the `CreatureDef` prices — but *who is family* becomes
  something the animal carries and its descendants inherit.

What falls out, with no further rule:

| today's question | the answer under signatures |
|---|---|
| are two clicks two colonies? | yes, softly: they start close and diverge, or a tolerant lineage keeps treating the other as kin |
| do colonies fight? | when they have drifted past each other's tolerance — which is a thing you can *watch happen* on the graph |
| is an ant always an ant? | **no.** A lineage whose signature has drifted past every other ant's tolerance is, to every other ant, a stranger species. That is speciation, and it is graded: first one colony stops tolerating it, then two |
| the colony label | becomes what it should be — a *name* for a cluster that already exists in the data, not the thing that makes the cluster |
| adoption, parasitism | a lineage that drifts *toward* another colony's signature and inside its tolerance is adopted. Nobody has to build it |

**Rivalry becomes a dial on tolerance rather than a bit**: the parameters
page exposes the ancestral `tolerance` (wide = one big happy species, the
shipped behaviour; narrow = every click a stranger) and the per-colony
founding offset. The `colony_rivalry` switch that landed today is the
zero-cost preview of that dial's narrow end and should be retired into it.

**The lab names what emerges.** When `live_creature_groups` is re-derived by
clustering signatures instead of reading a label, a cluster that has split
gets a new name — `ANT 3b`, or the lab's own coinage — and its own line on
the graph. That is the moment the owner asked about ("do they ever change
into new creatures separate from the original") made visible, and it is the
same page that already exists.

### 3c. What it costs, honestly

- **Three or four `CREATURE_TRAITS` slots**, so every jar on the specimen
  shelf is padded (`specimen::padded` already handles a short vector) and
  `params::TRAIT_ROWS` gets rows — `every_trait_slot_has_a_row` will refuse
  the build until it does.
- **A calibration.** `CLAUDE.md`'s shared-budget rule: the kin sense's
  authored weights (`KinNear`, `KinBearing`) were fitted against a kin that
  was every ant. Under a narrow tolerance a young colony sees fewer kin and
  the aggregation term weakens. Run `creature_arena --arm=ablate` with the
  tolerance at the shipped-equivalent width first and confirm the null, then
  sweep it.
- **The mutation rate on the signature is the speed of speciation**, and it
  is the one number nobody can set from theory. Ship it on the parameters
  page as `scent_drift`, default slow, and let the owner find the rate at
  which colonies split within a session — that is the game, per the standing
  direction.
- **Per-colony scent is a separate problem** (§4d), and signatures do not
  solve it.

---

## 4. Combat: is what we have sufficient?

**For predator–prey, yes, and it is better than it looks.** The graded bite
(`(bite/armour)²` per go, remainder banked on the *victim*) already gives
the two things a fight most needs: a distribution of outcomes, and numbers
that matter — ten ants gnawing one beetle are one hole filled ten times as
fast, so a swarm overwhelms armour with no rule about swarming. Armour and
bite are both heritable and both priced. The owner's equilibrium came out of
exactly this.

**For anything the owner would call "war", no**, and the gaps are specific.
In the order they block things:

### 4a. The prey cannot see the predator — the missing sense

`PreyNear`/`PreyBearing` are what the *eater* sees. Nothing tells an animal
that something which can eat *it* is near. So flight, hiding, retreat into a
gallery, warning a nestmate — none of them is selectable, because the
triggering quantity is not in the input vector. This is the scenarios
report's own audit rule (*a bed cannot select for what the animal cannot
perceive*) landing on the fight. **Add `ThreatNear`/`ThreatBearing`**: the
nearest animal on the same sight cast whose gut could digest *me* and whose
bite could open *my* armour. Same rays as prey and kin, so the price is the
sight tax already paid. Two slots on the input side; the genome grows by
two columns and every jar pads.

### 4a′. Ant against ant is binary, and no lineage can evolve out of it — the fight session's arithmetic

From the handoff on PR #263 (`Reports/lanes/creature-fight-handoff-2026-09-06.md`),
and it is the finding this report most needed:

```
  ant flesh  penetration_resistance  0.25     (assets/materials/ant.ron)
  armour trait multiplier            [0.5, 2.0]
  best armour an ant lineage reaches 0.25 x 2.0 = 0.50
  ant bite_force                     1.00
  damage = clamp(bite/armour, 0, 1)^2 = 1.0   -- one bite, at every point on the axis
```

**A maximally armoured ant is one-shot by any other ant.** So the moment two
colonies fight, they fight the way the owner ruled against: no grading, no
being overwhelmed, no being unlucky, whoever bites first. It is the beetle's
defect (*an edible predator is not a predator*) in different clothes — the
resistance sits four times under the bite and a 2x trait range cannot close
a 4x gap. Two ways out, the same two the beetle needs, so settle them once:
raise `ant`'s `penetration_resistance` so the trait range straddles the bite
(a two-species number, wants a sweep), or widen the armour slot's clamp past
`[-1, 1]` (cheaper and more general). **Until one lands, the rivalry dial's
fights are binary and no dial over them means anything** — which is also
why §2's numbers are small: an ant beside a stranger takes one bite and the
stranger is meat.

And a second thing that arrives for free: #263's whole-body scan gates the
extra reach on *living non-self organisms*, and `is_living_kin` sits right
under that gate — so with rivalry on, a rival on an ant's flank is attached
and non-kin at once, and **body-fighting between colonies switches itself
on with no wiring.** The trap that rides with it: widening what the body may
consider food once cost `ascii`'s deposition-follows-moisture gate 1.03x →
0.82x against a 0.9 bar; every `ascii` scene holds one colony so the dial
cannot move it today, but a scene that ever holds two must re-run that gate
across the dial.

### 4b. Fighting is only eating — the missing verb

An ant bites a rival only when hungry and adjacent, because the bite is the
`Feed` path. Territorial defence — biting a stranger you are *not* going to
eat — needs a verb that costs jaw work and yields no food. **`Attack`**: the
same graded bite, charged the same `dig_cost_in_moves` per progress, target
chosen as *nearest non-kin animal in reach* rather than *best mouthful*, and
it sates nothing. Then a colony can evolve to fight for ground and the price
of doing so is visible in the ledger. Without 4a it will never be selected
for, because the animal cannot tell a threat from furniture; with it, the
hunting-ground scenario (PR #253's S6) becomes runnable.

### 4c. Injury is permanent — a missing middle

`gnawed` never heals, by design (a healing victim puts a threshold back). But
a *cell* lost is lost, and a two-cell ant that loses a segment is an injured
animal with no way back: there is no growth for creatures. That is a binary
in the shape of a distribution. A `Regrow` cost — replace a lost body cell
from banked energy, at the body's own `body_energy` stamp, slow — is a real
middle: a fight leaves survivors who limp, spend, and recover. It also makes
the beetle's 2×2 body meaningfully sturdier than an ant's chain without
another armour number.

### 4d. Scent has no owner — the missing map

Two colonies with rivalry on still share the two planes. Three options,
cheapest first:

1. **Do nothing and say so** — today. A rival's trail is a trail; ants
   follow it into the rival's larder and fight there. That is not wrong,
   only shared.
2. **Salt the plane by colony**: keep two planes, but deposit and sample
   through a per-colony *key* — each colony writes to `(channel, colony %
   K)` for a small `K`. Memory is `K` times two planes (a 512×320 plane is
   160 KB; `K = 4` is 1.3 MB) and it makes every colony's trail invisible to
   every other, which is more than real ants have.
3. **A third plane, alarm**, owned by nobody: emitted on being bitten,
   decaying fast, read as `AlarmFront`. `pheromone.rs` says *resist a third
   until a concrete consumer exists*; being bitten is that consumer, and it
   is the one signal that makes a colony act *as* a colony in a fight —
   recruit, swarm, flee — rather than as fifty animals each deciding alone.

Order: 3 before 2. Alarm changes what a fight looks like on screen; private
trails change a number.

### 4e. Nothing scored a fight — half closed the same evening

`creature_stats` counted births and deaths; nothing counted *kills* per
group, and `DeathCause::Killed` cannot say who bit. `World::group_deaths`
now tallies each animal group's deaths by cause **and kills by attacking
group**, booked at the bite where both parties are in scope
(`World::tally_kill`; guarded by
`a_kill_is_booked_on_the_victims_group_against_the_killers`). `labstats`
prints it as the `--- groups ---` block the table in §2 came from, and the
ANTS legend carries `K<n>` on each group's row with the killers named in
its note. What is still missing is the run log naming the killer, and
births per group.

### 4f. What is sufficient in the evolutionary machinery, and what is not

Sufficient: armour, bite, pace, sight, gut and crop are heritable and priced,
so an arms race is expressible on both sides. `98d6886b` measured it shifting
rather than flipping.

Not sufficient, and both are already on record: **generations are slow**
(~8,600 frames each on the shipped bed, one generation in 27,000 on the
played one — `creature-behaviour-ceiling` §3), and **88% of affordable births
fail on geometry**, so who reproduces is decided by standing room. A fight
that kills 40% of a colony is a bigger selective event than a hundred
generations of drift, which is good; a colony that cannot refill the gap
because nobody has a free cell beside them is not.

---

## 5. Outside the box: how this could be played

Ranked by how much of it exists already. Each is a *scenario* in the sense
of PR #253 — a saved box with a question on it — not a new mode.

1. **Two clicks, one bed, rivalry on.** Available today. Two ant colonies at
   opposite ends, one larder in the middle. The graph shows who is winning;
   the colour shows where. Cheap first playtest of everything in §2.
2. **Breed a champion.** Jar the ant that survived the fight (the roster's
   `IN TROUBLE` filter inverted — the one with the most `LADEN` samples and
   the most bites survived), release a colony of it at 1 brood against the
   old stock. Route B from the programme plan, and it needs nothing new
   except the per-group readouts to find the champion.
3. **The arms race, as a slider.** `beetlearmour=` exists in `labstats`;
   put the ancestral armour and bite on the parameters page for *both*
   species (the ant's is there, the beetle's is not) and watch the ratio the
   owner sets play out. Zero engine work.
4. **A wall between them, then lift it.** Partitions already exist. Two
   colonies drift apart for 20,000 frames in separate rooms, then the wall
   comes out. With signatures (§3) the question *are they still one
   species?* has an answer on screen.
5. **Adoption and theft.** With signatures, a tolerant lineage near an
   intolerant one; does the tolerant one get absorbed, or does it raid? Real
   ant biology, and nobody writes a rule for it.
6. **Alarm and swarm.** With 4d(3), a beetle walking into a colony triggers
   a recruit-or-scatter that the owner can breed toward either end.
7. **Named lineages on a scoreboard.** Per-group births, deaths by cause,
   kills, cells held; the lab coins a name when a cluster splits. This is
   Gate 5's score without a score function — it is the census.
8. **Seasons of war.** Feast-and-famine (PR #253) with two colonies:
   rivalry is selected for in famine and against in plenty. That is a
   *distribution* over strategies, which is the first law.

What is deliberately not on this list: castes, queens, brood. They are how
real ants do it and they are a second economy; nothing above needs them, and
`creature-direction.md` D2 kept the colony creature to one body plan on
purpose.

---

## 6. Recommended order, and why

1. **Playtest §2 as it stands** — two clicks, rivalry on. The owner's verdict
   on whether identity + colour + graph is enough to *see* a war decides how
   much of §3 is urgent.
2. **Per-group deaths by cause on the legend** (§4e). Half a day; the
   `Grave` field is in.
2′. **Settle the armour reach for ant and beetle together** (§4a′) — the
   fight session's own next step, and the prerequisite for any fight the
   rivalry dial can start being graded.
3. **`ThreatNear`/`ThreatBearing`** (§4a). Without it nothing on the prey
   side can evolve, and every scenario in PR #253's S6 tier is blocked on
   it. The fight session's encounter findings (`5613e534`: a beetle needing
   two bites and not landing them in 150 decisions) are the calibration
   target — until the predator's own encounter is understood, adding the
   prey's response would be fitting a second unknown against the first.
4. **Signatures and tolerance** (§3). The largest change and the one that
   answers the owner's deepest question. Budget the kin-sense re-calibration
   as part of it, not after.
5. **Alarm plane** (§4d.3), then **`Attack`** (§4b), then **`Regrow`**
   (§4c). Each is one verb or one sense; each makes the previous one worth
   having.

Every step ships with its parameters on the page and its default at the
shipped behaviour. None tunes anything.

---

## 7. Decisions that are the owner's

- **Does a lone placed animal found its own colony?** Today yes (one click,
  one colony, however small). The alternative — join the nearest colony of
  its species — is friendlier for stocking beetles by single clicks and
  worse for experiments. Cheap to change either way.
- **Should rivalry default on in the lab?** It is the more interesting box
  and the standing direction is to expose, not to balance; but on by default
  changes what a second click *does* for anyone who has not read this.
- **Signatures: how many slots, and does species still bound kin?** The
  pure design (§3b) lets an ant and a beetle become kin if their signatures
  meet. Real biology allows it (social parasites); a player may find it
  absurd. A species term in the distance keeps them apart and costs one line.
- **Where does the name of a new group come from?** Numbering (`ANT 3b`),
  a coinage, or the owner naming it from the roster. The last is the most
  fun and the most work.
