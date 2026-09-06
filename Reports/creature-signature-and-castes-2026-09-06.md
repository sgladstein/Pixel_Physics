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
| can a colony grow workers and soldiers? | no mechanism: every ant buds a copy of itself and nothing about the parent's state reaches the child's body | **Yes, if the engine lets a child's body depend on the state its parent was in when it budded** — plasticity, and nothing more. The first draft of §2 authored what a soldier *is* (which sense it answers, which slots move, which way); the owner ruled that out, and §2 now designs the general mechanism — a brain output the parent evaluates at budding, a heritable plasticity block, one line in the trait reader — under which a soldier caste is something a lineage *finds*, and castes we never thought of are equally reachable. Not built; the polymorphism route needs nothing built and is the control |

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

Every run below pins `RAYON_NUM_THREADS=4` (a counter is only
load-independent at fixed parallelism -- `CLAUDE.md`), and every paired
comparison is against `main` at `15ed3d6c`, built clean in its own worktree
rather than against a remembered number. The box was not quiet: three
harnesses and a clippy shared four cores throughout, so no timing here is
quoted, only counters.

**The shipped bed is byte-identical, not merely equivalent.**

| check | `main` | this branch | |
|---|---|---|---|
| `ascii`, *deposition follows the moisture gradient* | 237 drops, 2,962 laden ants, 2.5692 vs 1.8838, **1.36x** | 237 drops, 2,962 laden ants, 2.5692 vs 1.8838, **1.36x** | digit for digit -- the gate the fight handoff named as the one that notices a widened kin |
| `ascii`, every non-timing line of the whole run (1,121 lines) | — | **0 lines differ** | the outdoor game is untouched |
| `labstats colonies=2 founders=8 frames=24000`, seeds 1–3, every frame line and the group tally | — | **identical on all three seeds** | the lab at the shipped dials is untouched |
| `creature_arena species=ancestor arm=ablate input=KinNear frames=24000 seeds=6` | B share of animals median 49.1% (q1 42.9, q3 50.0); seeds below 50%: 4, above 1, tied 1 | **the same six rows, digit for digit** | the kin-sense null the design report's §3c asked for first, and the branch reproduces it exactly |

The `KinNear` ablation on `main` is a **null**: four seeds under 50%, one
over, one tied, inside the harness's own 2.4-3.1x seed floor. So the
kin-sense weights authored against "every ant is kin" are not carrying a
measurable advantage in this bed at this horizon to begin with, which is
the honest starting point for any re-calibration under a narrower
tolerance: there is no fitted advantage to lose.

**And past the shipped tolerance the kin sense is still a null.** The
same race with the ancestor authored at `tolerance -0.5` and `scent_drift
0.3` (a temporary species-file edit, rebuilt, reverted before anything was
committed), so children drift out of their parents' family within a few
generations and `KinNear` sees fewer kin: arm B (ablated) share of animals
median 41.7% (q1 40.7, q3 53.7), seeds below 50%: 4, above: 2. The median
moved from 49.1% to 41.7% and the seed count did not — four against two is
inside the harness's own floor. So the authored `KinNear`/`KinBearing`
weights carry no advantage in this bed that a narrower family can lose,
and the re-calibration the design report's §3c budgeted for has nothing
to re-fit yet; the number to re-take is this one, on a bed where the kin
sense is shown to pay at all.

**The narrow end reproduces the rivalry switch it replaced** — `labstats
rivalry=1`, now an alias for `tolerance=-1 spread=1`, two colonies, 24,000
frames, three seeds, against the same bed at the shipped dials:

| seed | shipped dials | narrow end: ANT 1 | ANT 2 |
|---|---|---|---|
| 1 | 0 kills either way | 1 alive, 40 starved, **12 killed by ANT 2** | 0 alive, 36 starved, 6 killed by ANT 1 |
| 2 | 0 kills either way | 14 alive, 39 starved, 8 killed by ANT 2 | 10 alive, 30 starved, 10 killed by ANT 1 |
| 3 | 0 kills either way | 7 alive, 44 starved, 7 killed by ANT 2 | 10 alive, 32 starved, 6 killed by ANT 1 |

Kills both ways on every seed, none at the defaults — the design report's
§2 finding on the new mechanism (its own numbers were 3–9 per side on the
pre-soil bed; the bed has since changed under it, and the baseline above is
the one to quote now). Still predation and not war: a fifth of the deaths,
the rest starvation.

**Adoption and raiding are one asymmetry, and it is measurable** — the
design report's §5.5, run rather than argued: `spread=0.5 tolerance=0.3
tolerance2=-0.9`, so ANT 1 judges by a radius of 1.3 and ANT 2 by 0.1, the
two colonies 0.9–1.4 apart in scent (printed per run), 24,000 frames:

| seed | ANT 1 (tolerant) | ANT 2 (intolerant) |
|---|---|---|
| 1 | 8 alive, 36 starved, **14 killed, 14 by ANT 2** | 0 alive, 42 starved, **0 killed** |
| 2 | 6 alive, 36 starved, **14 killed, 13 by ANT 2** | 11 alive, 41 starved, **0 killed** |
| 3 | 7 alive, 35 starved, **15 killed, 15 by ANT 2** | 8 alive, 41 starved, **0 killed** |

The tolerant colony walks up to animals that bite it and never bites back;
the intolerant one is never touched. That is a raid from one side and,
from the other, a colony that treats its raiders as family — which is
what adoption looks like *before* anyone is relabelled, and the reason the
report says adoption is visible in the kill tally rather than the legend.
(Seed 2's fourteenth kill is booked to no group: one `Killed` death has no
attacker, which is the severed-half path that dies as `Killed` without a
bite booking it — pre-existing, and worth a line in the fight lane's
handoff rather than a fix here.)

**Speciation by drift, on the shipped bed, named on the page** — two
colonies, `tolerance=-0.5` (radius 0.5), 48,000 frames, three seeds, the
`regroup_by_scent` counter printed at the moment it fires:

| seed | drift 0.3 | end state | kills booked before and after the split |
|---|---|---|---|
| 1 | **`ANT 1b` named at frame 32,793** (3 animals), `ANT 1c` at 37,755 (4) | ANT 1 3, ANT 1b 5, ANT 1c 3 alive; ANT 2 starved out | ANT 1 lost 13 (11 to its own label before the split, 2 to 1c); 1b lost 4; 1c lost 3 |
| 2 | `ANT 1b` at 36,504, `ANT 2b` at 42,738, `ANT 1c` at 45,072 | ANT 1 1, ANT 2 3, 1b 3, 2b 2, 1c 3 alive | ANT 1 lost 16 and ANT 2 11, every one to its own label |
| 3 | **`ANT 2b` named at 23,028** (10 animals) | ANT 1 6, ANT 2b 5 alive; ANT 2 starved out | 2b lost 15 (12 to its own label, 2 to ANT 2); ANT 1 lost 5 |

| seed | drift 0.6 | end state | kills |
|---|---|---|---|
| 1 | **no split** — the colony never holds three drifted cousins at once (7 → 5 animals from frame 12k on) | ANT 1 3 alive; ANT 2 starved out | ANT 1 lost 8, all to its own label |
| 2 | `ANT 1b` at 12,355, `ANT 2b` at 13,206, `ANT 2c` at 21,948, **`ANT 1bb` at 43,376** — a split off a split | six labels, 6 animals alive across them | 1b lost 11, all to its own label; ANT 1 lost 8 (6 own, 2 to 1b) |
| 3 | `ANT 1b` at 30,288 (3 animals) | ANT 1 3, ANT 2 3, 1b 1 alive | ANT 1 lost 11 (9 own, 1 to 1b); ANT 2 lost 7 (6 own); 1b lost 3 |

Doubling the drift halves the time to the first split where the colony
can afford one (12k against 33k on seed 2) and does nothing where it
cannot (seed 1) — the bed, not the rate, is the binding constraint on the
shipped bed, and the rate is what the owner will turn to see it on a fed
one.

Every seed speciates inside the run, and the name lands on the legend the
frame the cluster parts — the owner's question *"do they ever change into
new creatures separate from the original"* answered by a counter and a
row. Three things the table says that the design report only predicted:

- **A colony eats its own before it splits.** Nearly every kill above is
  booked *within* a label (`killed by ANT 1 x11`): drifted cousins bite each
  other while the drifting cluster is still under `MIN_SPLIT_GROUP` or the
  mutual-kin graph is still connected through intermediates. Under drift,
  intolerance is a cannibalism strategy until the bitten can bite back —
  the `Attack` verb and the alarm plane (other session) are what turn it
  into war. This is the shared-budget rule landing on kin: nothing prices
  biting a cousin who cannot retaliate.
- **The bed decides whether drift can be seen at all.** At the authored
  tolerance (radius 1.0) `drift=0.15` and `0.3` are **byte-identical** to
  each other over 48,000 frames — the scent values differ and no pair ever
  reaches 1.0 apart, so no predicate ever flips and the two worlds are the
  same world. The positive control (`tolerance=-0.9 drift=1.0`) bites on
  the first birth and names nothing, because this bed starves both
  colonies to three animals by frame 24,000 and a cluster of three is the
  floor. Speciation needs a colony that stays big enough to have three
  drifted cousins alive at once, which the shipped bed barely manages.
- **The split is graded in the two ways the first law asks for.** It
  arrives at different frames on different seeds (23k–45k), in different
  sizes (3–10 animals), sometimes twice off one parent (`1b`, `1c`) and
  sometimes off both (`1b`, `2b`); and the tally beside it says what each
  new group did to whom.

**The guards go red when the fault is put back** (2026-09-06, one test
build): with `is_living_kin` made to ignore the distance (kin = same kind
again), `a_stranger_colony_is_kin_until_its_scent_leaves_my_tolerance`,
`tolerance_is_judged_from_my_side_only` and both regroup guards fail; with
`regroup_by_scent` made to mint nothing,
`a_lineage_that_drifts_past_tolerance_is_named_as_a_new_group` and
`a_lone_drifter_is_not_a_new_group` (its positive-control arm) fail.
`a_beetle_is_never_family_unless_its_kind_crosses_kinds` stays green under
the first fault, correctly: it guards the species gate, not the distance.

**Posted for the owner's judgement**: review card
`20260906T165111553Z-b2d7cb` (board `lab`), an A/B of the ANTS page at
frame 30,000 on seed 3 — the shipped dials against `drift 0.3, tolerance
-0.5` — with the group counts and kills in `meta`, carrying the two §7
decisions as its question: whether `ANT 2b` is the right kind of name for
a split-off group (against numbering, or naming from the roster), and
whether scent should be allowed to cross kinds (`kin_crosses_kinds`, off).
Both ship as dials; the card decides the defaults.

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
predator bed, plasticity has to do better than that to earn its slots.

**Route (b): one genome, two bodies — as the brief sketched it, and as the
first draft of this section designed it.** A heritable threshold on a
parental *threat* signal, a `morph` scalar set at birth, and a rule that a
high morph means more armour, a harder jaw and a smaller crop. It is cheap
and it is a caste, and **it is ruled out** (owner, 2026-09-06: *"how much
are we hard-coding this behaviour, which I don't want, versus making it
possible within the engine?"*). Three things in it are authored that the
engine has no business deciding: **which sense** a parent answers when it
makes a child (threat, and nothing else); **which slots move and which
way** (a designer's soldier, so a lineage could never discover a small fast
disperser when crowded or a big-crop forager when rich); and **that the axis
is soldier-versus-worker at all**, so the measurement counts a thing we
defined and the feature reads as ours rather than the ants'. Only two
pieces of it were honest: the price, which falls out of the armour and jaw
levies that already exist, and a brain input that lets behaviour depend on
how an animal was made.

### 2c. The design: plasticity, and nothing named

The engine lacks exactly one capability: **a child whose expressed body
depends on the state its parent was in when it budded.** Build that and
only that.

- **One brain output, `Provision`**, evaluated by the *parent* at the
  moment `try_bud` places a child. The network decides, from whatever its
  senses carry — crowding, energy, kin near, threat near, the alarm plane
  once it lands — what number to hand the child. Nothing here says which
  sense matters; a blind ant's brain has crowding, energy and being-bitten
  to work with and an eyed one has more, and which of them a lineage comes
  to answer is selection's to find. A positional append like `ThreatNear`
  was: `live_slots` grows by one output's weights, every species'
  `mutation_rate` is re-derived once, every jar pads.
- **A heritable plasticity block**, one weight per `CREATURE_TRAITS` slot,
  in the *genome* beside the brain's weights rather than in the trait
  vector — it is developmental wiring, mutated by `brain::mutate` at the
  genome's own rate, and it escapes the one-row-per-slot rule for the same
  reason the brain's 12,352 weights do. Authored zero in every species file.
- **The child's expressed traits are its inherited traits plus plasticity
  times the parent's provision**, clamped to each slot's axis, applied in
  `creature::traits_of` — the one function every `*_of` resolver already
  reads through, so armour, jaw, crop, pace, sight, gut and the scent-side
  slots all move if their weight says so and no resolver changes.
  `state.traits` stays the genotype: `try_bud` inherits genes, not bodies.
  The provision value is stored on the animal (`OrganismState::morph`, 0 for
  a founder, never inherited, not in a jar), so a founder and every shipped
  animal are byte-identical to today.
- **The provision is fed back as a brain input, `Made`**, so behaviour can
  depend on how an animal was made — a stay-at-home body and a stay-at-home
  disposition are one weight apart.
- **One dial**, `plasticity` on the GENOME page: a multiplier on the block,
  `0` the shipped bed (byte-identical, no new birth draw), `1` the authored
  reach. Everything else is the existing economy: a body with more plate
  and jaw pays `armour_fraction` and `force_fraction` every tick for what
  it *expresses*, so an over-provisioned child costs its lineage and the
  weights that make it are selected on, with no soldier-inhibition rule
  written anywhere.

**What stays hard-coded even then, stated so nobody mistakes it for a
choice made lightly:** bodies vary only along the trait axes that exist, so
a caste with a different body *shape* cannot evolve while the body plan is
per species; and development is one number at birth, never revised — a
single hormone, which is what keeps it cheap and legible. Real castes are
richer than that; this is the smallest honest version.

Under this design a colony that comes to bud armoured, hard-jawed,
stay-at-home children when its senses say danger has *evolved* a soldier
caste, and nothing in the engine knows the word. A colony could just as
well evolve a caste nobody here thought of.

### 2d. What would show plasticity being *selected for*, not merely reachable

Reachable is cheap to show and proves nothing (`CLAUDE.md`: an image says
what and where; only the number says whether it fired). With nothing named
there is no soldier to count, so the measurements are about the
*weights* and the *spread*:

1. **Connected.** With the block hand-set (one weight, one input into
   `Provision`), a bed's children express the shifted slot and the
   parent's counters show the price — the positive control that the
   plumbing reaches a body at all. Its null: `plasticity = 0` is byte-
   identical to `main`.
2. **Selected.** A `creature_arena` race, arm A with the block frozen at
   zero against arm B free to mutate it, in a bed with breeding beetles,
   past the founding grant (24,000+ frames), six or more seeds, mirrored,
   read as *how many seeds moved the same way* — beside it, the block's
   weights in B's survivors printed per lineage (do they leave zero, and
   on which slots and which inputs) and `World::group_deaths` for what the
   beetles took from each arm. The null that makes it mean something: the
   same race with `predators=0`, where the weights should stay near zero
   or drift without direction. The flight race in the design report's §4a
   is the template and its warning: that bed could not separate arms at
   24,000 frames, so the beetles must actually kill.
3. **A caste, not a polymorphism.** The tell that this is doing something
   route (a) cannot: **body spread within one lineage** (by
   `OrganismState::lineage`), conditioned on the parent's provision — a
   bimodal armour or crop distribution among siblings of one genotype.
   Under (a) a lineage is one body. The ANTS page can carry it as a
   per-group spread the way the outdoor colony panel carries trait spreads
   today.
4. **Named by the lab, not by us.** If a lineage's block puts two body
   modes far enough apart, the same clustering the scent signature uses
   can name the modes on the legend (`ANT 1 · big`, or the owner's word),
   which is the naming decision already on the owner's card.

**A caste is worthless until something can fight, and that is a statement
about order.** Today a heavier body matters against a beetle only when a
hungry ant happens to stand beside one; the `Attack` verb (other session)
is what lets a `Made`-wired body *defend*, and the alarm plane is what tells
it when. So the build follows both, and the race in (2) is only honest once
an ant can bite a beetle it is not going to eat.

### 2e. Verdict and decision

**Feasible, cheap, and the general mechanism rather than the soldier.**
One brain output, one positional genome block, one line in `traits_of`,
one input, one dial — the same size as the authored design it replaces,
with the soldier taken out of it. Route (a) is what the first measurement
runs as its control rather than a rival design. Not built on this branch:
it lands on the signature (the morph value reuses `traits_of`, the padding
and the page rows this branch adds) and its only honest test needs the
`Attack` verb and something for a provisioned body to do. Order: land §1;
the other session lands armour reach, alarm and `Attack`; then plasticity,
with the connected-control first and the race in §2d as its acceptance.

---

## 3. What this overturned, and what the next session must not re-derive

*(Filled in as the measurements land.)*
