# Scent signatures and castes — is an ant always an ant, and can a colony grow soldiers? 2026-09-06

*Owner, after the first ant-and-beetle equilibrium: "once creatures evolve,
is an ant always an ant?" And, opening this lane: can the engine let a colony
naturally differentiate into workers and soldiers? This report is the build
record for the first question (§1, shipped on this branch) and the
feasibility verdict and design for the second (§2, designed, not built).
Status: **design of record for kin-by-scent and for castes on the creature
line**; the shipped half is README's "Creature groups status". It sits under
[`creature-groups-and-combat-design-2026-09-06.md`](creature-groups-and-combat-design-2026-09-06.md),
whose §3 it builds and whose §7 decisions it carries as dials.*

---

## 0. The answers, stated once

| question | before this branch | now |
|---|---|---|
| is an ant always an ant? | **Yes, by construction.** `state.species` was written once and `is_living_kin` was *same species* (and, behind a one-evening `colony rivalry` switch, *same colony label*). A lineage could drift as far as it liked and be nobody's stranger | **No.** Kin is *the other animal's scent within my tolerance*, both heritable. Two clicks start a little apart (a founding offset), children drift (`scent_drift`), and a lineage that drifts past every other's tolerance is, to them, a new kind — which the ANTS page names (`ANT 1b`) and draws as its own line. The rivalry switch retired into the tolerance dial's narrow end |
| is a beetle ever an ant's family? | never | never, **unless** the ant's kind has `kin_crosses_kinds` on — the owner's §7 call, shipped as a dial defaulting to no |
| what does the shipped bed do? | one family | **exactly what it did**: spread 0 and drift 0 put every ant on one point, the new slots consume no birth draw, and the two-colony census is byte-identical to `main` (§1e) |
| can a colony grow workers and soldiers? | no mechanism: every ant buds a copy of itself and nothing about the parent's state reaches the child's body | **Feasible, cheaply, and as a real caste** — one genome, a spectrum of bodies along one heritable *reaction norm* (§2). Not the way real ants do it (there is no queen and no brood, and building one is a second economy); and the polymorphism route needs nothing built and is the control |

---

## 1. The scent signature — what was built, and what it costs

### 1a. The mechanism, in one paragraph

Four `CREATURE_TRAITS` slots, mutated per birth like every slot:
`TRAIT_SCENT_A/B/C` (10–12), the signature, three numbers on `-1..=1`; and
`TRAIT_TOLERANCE` (13), read as a plain radius of `tolerance + 1` — `-1` is an
exact match only, `0` is one unit of scent, `+1` two. `is_living_kin` is now

```
same kind (unless my kind crosses kinds)  &&  |scent(other) - scent(me)|² <= radius(me)²
```

and the colony label appears in it nowhere. It is judged from the judge's
side and is **not symmetric**: a tolerant ant beside an intolerant one sees
family and will not bite, the intolerant one sees a stranger and will —
which is what adoption and raiding both look like from the inside, and
nobody writes a rule for either (guarded by
`tolerance_is_judged_from_my_side_only`). One predicate still feeds the
mouth (`adjacent_food_counted`), the eye (`is_visible_prey`, and through it
`is_visible_threat`) and the kin sense (`is_visible_kin`), so widening "who
is family" still widens it everywhere at once.

**Three slots rather than one** because two random walks on a line re-cross
constantly and strangers flicker back into kin; in three dimensions they
part and stay parted. Not more, because every slot is a jar padded and a
species-file tuple widened.

**A founder starts at its species' authored point plus its colony's
offset.** `CreatureDef::scent_spread` draws one offset per colony label,
uniform in `-spread..=spread` per slot, keyed on the world seed and the label
(`creature::colony_scent_offset`), so every station of one gesture shares
it, a jar released into a label carries that label's offset on top of the
scent it was jarred with, and a rebuilt box reproduces its colonies' scents.
`CreatureDef::scent_drift` is the per-birth width for all four slots — one
number, the speed of speciation, in place of the four `trait_variance`
entries (which are not read for these slots; `creature::trait_width` is the
one place that decides). Both default to zero.

**The colony label follows the scent.** `World::regroup_by_scent`, run at
the ANTS page's sample cadence (`Lab::advance`, and `labstats`), takes every
`(species, label)` group and finds the connected clusters of *mutual* kin.
The cluster holding the group's lowest `lineage` — its oldest surviving
founding line — keeps the label; every other cluster of at least
`MIN_SPLIT_GROUP` (3) animals is given a fresh label minted as the group's
child (`World::colony_parents`), and `World::group_label` names it `ANT 1b`,
`ANT 1c`, `ANT 1bb`. The label is *a name for a cluster that already exists
in the data*, per the design report, and it is rewritten on the animal
rather than carried as a second partition so that the colour it wears, the
line it is counted on, the group its death is booked to and the kin it bites
are one fact. **The pass splits and never merges**: two clicks that smell
alike are still two groups, because placement identity is the player's and
folding it away at the shipped dials would erase PR #255's census.

### 1b. Where the levers are

| lever | where | default | what it is |
|---|---|---|---|
| ancestral scent (3 rows) and tolerance | GENOME page, `TRAIT_ROWS` | ant `(0,0,0)`, tol `0`; beetle `(0.8,0.8,0.8)`, tol `0` | the point a click starts from and the radius it judges by |
| `scent_drift` | GENOME page, species field | `0` | per-birth width of the four slots — the speed of speciation |
| `scent_spread` | ANTS page under COLONIES, species field | `0` | how far apart two clicks start |
| `kin_crosses_kinds` | ANTS page under COLONIES, species toggle | off | whether species is consulted at all |

Every one is saved to the species file by the page's own span edit, felt on
the next tick (tolerance, crossing) or the next founding/birth (spread,
drift). `labstats` exposes the same four as `tolerance= spread= drift=
crosskin=`, and keeps `rivalry=1` as an alias for `tolerance=-1 spread=1`
so the design report's §2 table can be re-run on the new mechanism as its
positive control.

### 1c. What retired, and why it could

`World::colony_rivalry` and its `Dials` entry are gone. Its whole meaning
— *an ant of another click is a stranger* — is tolerance `-1` (radius 0)
with any non-zero spread: within a colony every scent is the founding
offset exactly, so distance 0 is inside radius 0; across colonies the
offsets differ, so nothing is. A saved `lab_dials.ron` that still carries
the key loads (serde ignores it; guarded).

### 1d. What it costs, honestly

- **Four slots.** Every species `.ron` tuple widened 10 → 14; every jar on
  the shelf pads, and it pads *from the species' ancestral vector* now
  rather than from zero (`specimen::padded_from`), so a pre-signature beetle
  jar lands on the beetle's point and not on the ant's. Four `TRAIT_ROWS`
  rows; `every_trait_slot_has_a_row` refused the build until they existed.
- **The predicate.** Three subtractions, three multiplies and a compare per
  neighbour per tick where there was one integer compare; the
  `world.organism` lookup it hangs off was already paid. Not measurable
  against the sweep.
- **The regroup pass.** A group whose scents are one point — every group at
  the shipped dials — is one pass over its members and no pairwise work. A
  drifted group of `k` pays `k(k-1)/2` distances at the sample cadence,
  never per tick; measured (§1e).
- **Births.** At `scent_drift = 0` the four slots take the `width > 0`
  branch that draws nothing, so the birth stream is untouched and every body
  slot's mutation is bit-identical to before. Guarded by
  `the_scent_slots_drift_only_at_scent_drift`, both arms.
- **The kin sense.** `KinNear`/`KinBearing`'s authored weights were fitted
  against a kin that was every ant; under a narrow tolerance a young colony
  sees fewer kin and the aggregation term weakens. The null at the
  shipped-equivalent tolerance and the sweep past it are in §1e.

### 1e. Measured

*(Filled in below as the runs complete; each table names its binary,
seeds, frames and `RAYON_NUM_THREADS`.)*

### 1f. What it does not do, stated so nobody measures its absence as a bug

- **Merge.** A tolerant lineage that drifts *into* another group's family is
  adopted in behaviour (it aggregates on them, they do not bite it) and the
  kill tally shows the raid when they do; it is not relabelled. That is the
  next cut of `regroup_by_scent` if the picture wants it.
- **Give a colony a private trail.** Both colonies still write the same two
  pheromone planes — the design report's §4d, the other session's file.
- **Add a verb.** A stranger is bitten when a hungry animal stands beside it,
  by the `Feed` path, and for no other reason. `Attack` is the other
  session's.
- **Cohere a colony.** A signature that is only inherited and mutated has no
  homogenising force: under drift a colony's own cloud spreads as fast as
  two colonies separate, and what holds a colony together is that its
  tolerance exceeds its cloud. Real ants share a *gestalt* odour mixed by
  contact; if the owner finds colonies dissolving into strangers rather than
  splitting cleanly, a contact-blended worn scent beside the heritable one
  is the mechanism, and it is deliberately not in this cut.

---

## 2. Castes — can a colony grow workers and soldiers?

### 2a. What is true today, from the code

- **There is no queen and no brood.** Every ant that can afford it buds
  (`creature::try_bud`, called from `creature_tick` on every survived tick);
  the child is placed beside the parent's head (`place_creature`,
  `Origin::Bud`), copies the parent's genome and trait vector, and mutates
  both on its own handle after placement. Reproduction is individual, not
  colonial.
- **Nothing about the parent's state reaches the child's body.** The
  `Origin::Bud` arm carries `genome`, `traits`, `generation`, `lineage`,
  `colony`; the parent's energy, crop, injuries (`gnawed`), what it has seen
  (`threat_sightings`) and what stands around it are read for the *decision*
  to bud (`bank + reachable >= bar`) and for nothing else.
- **The body plan is per species, not per individual.** `CreatureDef::body`
  is `Chain(2)` for the ant; `place_creature` lays `def.body.offsets(..)`
  for every birth. Body size does not evolve at all
  (`creature-genome-flexibility-2026-09-02.md` §2d; `creature-direction.md`
  D2 kept the colony creature to one plan).
- **Every body number that a caste would differ on is already heritable and
  priced per individual**: armour (`TRAIT_ARMOUR`, `armour_fraction` per
  tick), jaw and bite (`TRAIT_DIG_FORCE`, `force_fraction`), pace, sight,
  crop capacity, gut. A soldier is a point in a space the engine already
  has; what it lacks is a way for *one genome* to put two of its children at
  two different points.
- **Generations are slow and geometry decides who breeds**: ~8,600 frames per
  generation on the shipped bed, one generation in 27,000 on the played one,
  and 88% of affordable births refused for want of a free cell
  (`creature-behaviour-ceiling-2026-09-05.md` §3). Any caste that has to be
  *selected* into existence is working against that clock.

So real castes — a queen's brood fed into two developmental paths — cannot
arrive here: the machinery they run on does not exist, and building a
queen, brood and feeding is a second economy the design guide deliberately
did not take on.

### 2b. Two routes, and which one is a caste

**Route (a): genetic polymorphism.** Lineages within one colony diverge
under disruptive selection into biters (armour, bite, meat gut) and
foragers (crop, plant gut). **It needs nothing built** — every trait it
turns on is heritable today — and the signature does not separate the two
morphs into groups unless their *scents* also drift apart, which is
independent of their bodies. What it is not is a caste: a soldier under
(a) must breed soldiers to persist, so it is selected *against* in peace
(it pays armour and jaw every tick and forages no better) and *for* in war,
and the colony oscillates between strategies rather than holding a division
of labour. That is a real and interesting dynamic (the design report's §5
*seasons of war*) and it is the **control** against which any caste
mechanism must be read: if (a) alone produces biters and foragers in a
predator bed, a reaction norm has to do better than that to earn its slots.

**Route (b), as the brief sketched it: one genome, two bodies.** A heritable
second trait vector plus a heritable threshold on a parental signal. That
is how real castes work, and it doubles the trait vector (14 → 28 slots,
every jar and every species file) for a switch between two authored points.

**Route (c), which is what I would build: one genome, a *spectrum* of
bodies along one axis.** A caste is a *reaction norm* — the map from the
environment a parent is in to the body its child gets — and the cheapest
honest reaction norm here is one per-individual scalar and two heritable
slots:

- **`morph`** on `OrganismState`, `-1` (worker) to `+1` (soldier), a
  phenotype set at birth and never inherited (jars do not store it; a
  founder is `0`, so every shipped animal is exactly what it was).
- **`TRAIT_MORPH_BIAS`** and **`TRAIT_MORPH_GAIN`**, heritable, mutated per
  birth: `child.morph = clamp(bias + gain × signal(parent) + jitter)`. A
  lineage with gain `0` has no castes and buds copies of its bias; a lineage
  with a steep gain and a bias near zero buds soldiers when the signal is
  high and workers when it is low, which is a bimodal body distribution —
  Wilson's allometry — from a continuous rule. The jitter is the last draw
  of the birth stream so it shifts nothing else, and it is what makes caste
  determination near the threshold a distribution rather than a step.
- **`morph` moves the expressed traits in one place.** Every `*_of`
  resolver already reads through `creature::traits_of`; it returns the
  *expressed* vector, which is the genotype with `TRAIT_ARMOUR` and
  `TRAIT_DIG_FORCE` shifted up by `morph × MORPH_SPAN` and
  `TRAIT_CROP_CAPACITY` shifted down by it. `state.traits` stays the
  genotype, so `try_bud` inherits the genes and not the body. (One call
  outside `traits_of` reads `st.traits` directly today — `armour_at`, the
  other session's — and would read the expressed vector.)
- **The price is already authored.** `armour_fraction` and `force_fraction`
  charge per tick for the plate and the jaw an animal *expresses*, so a
  soldier costs more to keep alive than a worker with the same genes, an
  all-soldier colony burns faster and starves sooner, and the norm that
  buds soldiers in peace is selected against by the existing ledger. No new
  price, and no authored soldier-inhibition rule: the negative feedback that
  sets a soldier *ratio* in real ants (Pheidole) is emergent here from cost.
- **`BrainInput::Caste`**, one appended slot carrying `morph`, so behaviour
  can differ — a soldier that stays where kin are dense (`Caste × KinNear →
  Move`) and a worker that forages are one and two weights away. Positional
  append, `live_slots` and every species' `mutation_rate` re-derived as
  `ThreatNear` did.

**The signal the parent reads is the weak point, and it should be stated
rather than designed around.** A blind ant today can know two things about
threat: that it has been bitten and survived (`gnawed > 0` is banked on the
victim and never heals), and how crowded it is (`Crowding`, already in the
input vector). An eyed animal knows `ThreatNear`. When the alarm plane
lands (other session, design §4d.3) a blind ant can smell that the colony
is under attack, which is the signal real soldier determination actually
uses. So `signal(parent)` in the first cut is `max(ThreatNear, gnawed > 0,
alarm at head)` with the alarm term reading 0 until the plane exists — a
composite, in `0..1`, documented as such, and a bed of blind ants will only
bud soldiers off bites received, which is a lagging signal and will be
visible as one.

**Visibility.** A soldier must be seen or it is a number. Two options, both
judge-by-eye and one of them cheap: a `CASTE` overlay on the ramp every
scalar channel uses (build it *before* the mechanism, per `CLAUDE.md`), and
a fourth `CreatureColour` mode. The option that would read at play zoom
with no colour trick is a **longer body** for a high morph — `Chain(3)`
against the worker's `Chain(2)`, laid at birth from a per-birth plan — which
also prices itself (idle cost is per body cell) and makes a soldier sturdier
(one more cell to chew through). It reintroduces a discrete step over a
continuous morph, which is the first law's objection; the honest answer is
to build the overlay first, look, and decide whether the eye needs the body.

### 2c. What would show castes being *selected for*, not merely reachable

Reachable is cheap to show and proves nothing (`CLAUDE.md`: an image says
what and where; only the number says whether it fired). The measurements,
in order, each with its counter:

1. **Reachable.** `labstats predators=8` with the ant's `morph_gain` set
   high: births by morph (a histogram printed beside `births`), soldier
   fraction per group on the legend. Control: the same bed at `gain 0`
   reads every birth at the bias.
2. **Priced.** Paired bed, no predators, `bias +1` (all soldiers) against
   `bias -1` (all workers), same seed: the soldier arm's per-tick cost and
   its time to first starvation, both higher. If they are not, the price is
   disconnected and nothing downstream means anything.
3. **Selected for.** A `creature_arena` race, arm A `gain 0` against arm B
   `gain > 0`, in a bed with breeding beetles, **past the founding grant
   (24,000+ frames)**, six or more seeds, mirrored, read as *how many seeds
   moved the same way*; beside it `World::group_deaths` (kills by the beetles,
   per arm) and the beetle's own count. The null that makes it mean
   something: the same race with `predators=0`, where B must *not* win. The
   flight race in the design report's §4a is the template and its null is
   the warning — that bed could not separate a fleeing arm from a blind one
   at 24,000 frames, so the soldier race needs beetles that actually kill
   (eight took few enough ants that the arms could not separate).
4. **A caste, not a polymorphism.** The tell that (c) is doing something (a)
   cannot: the *same lineage* (by `OrganismState::lineage`) contributing
   both high- and low-morph children within one run, counted per lineage.
   Under (a) a lineage is one morph.

**Soldiers are worthless until something can fight, and that is a
statement about order.** Today a soldier's armour and bite matter against a
beetle only when a hungry ant happens to stand beside one; the `Attack` verb
(other session) is what lets a `Caste`-wired soldier *defend*, and the alarm
plane is what tells it when. So the caste build follows both, and the
measurement in (3) is only honest once an ant can bite a beetle it is not
going to eat.

### 2d. Verdict and decision

**Feasible, cheap, and route (c).** Two heritable slots, one per-individual
scalar, one line in `traits_of`, one brain input, and an overlay — no new
price, no new body plan in the first cut, no queen. The owner has said the
route is mine to choose; this is the choice, with the reasons above, and
route (a) is what the first measurement runs as its control rather than a
rival design. It is not built on this branch because it lands on the
signature (the morph axis reuses `traits_of`, the slots, the padding and the
page rows this branch adds) and because its only honest test needs the
`Attack` verb and something for a soldier to do. Order: land §1; the other
session lands armour reach, alarm and `Attack`; then the morph axis, with
the overlay first and the race in §2c as its acceptance.

---

## 3. What this overturned, and what the next session must not re-derive

*(Filled in as the measurements land.)*
