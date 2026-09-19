# The biology of ant nests — architecture, stores, digging, microclimate

*2026-09-19. Research lane of the nest program, on a docs-only brief.
**Nothing is built here**: no `src/`, `assets/`, `examples/` or test change.
The sibling document is
[`nest-design-2026-09-14.md`](nest-design-2026-09-14.md) §2, which owns
homing and navigation; that ground is not repeated. Excavation *shaping* —
how a round cavity becomes a branched network — is owned by
[`stigmergy-research.md`](stigmergy-research.md) §5 and is likewise not
repeated. What is here is the part neither covers: the **vertical profile**
of a nest, **what is kept in it and why**, **what starts and stops a dig**,
and **what chambers are for**.*

## How to read the evidence in this report

Every claim carries one of three marks, and the marks are the point: a
wrong citation here becomes the premise of a build and nothing catches it.

| mark | means |
|---|---|
| **[measured]** | a specific published study whose existence and headline finding I am confident of, with the finding stated as that study's, on its species |
| **[repeated]** | widely stated in the literature and in reviews; I have not held the primary source and the number may have drifted in retelling |
| **[general]** | my own synthesis from general knowledge of the field, with **no specific source**. Treat as a hypothesis, not a fact |

**No URL is given for any paper.** The precedent report links its sources;
I can state author, year and journal with confidence for the papers below
and cannot state DOIs or URLs with the same confidence, and a plausible-
looking dead link is worse than a citation a reader has to find. §8 lists
them in a form that is searchable.

**Where a number is one species in one study, it says so.** Ant nests vary
more between genera than almost anything else in this report, and the
single most common error in games that reach for this material is treating
*Atta* or *Pogonomyrmex* architecture — the spectacular, heavily cast,
heavily published end — as what an ant nest is.

---

## 0. The seven findings that contradict what this engine assumes

**Read [§10](#10-owner-ruling-2026-09-19--the-nest-has-no-purpose-and-that-is-prior-to-everything-above) first.**
The owner ruled on this report's *premise* the day it was written: **the
nest has no purpose in this game** — no granary, no eggs, nothing
environmental that can kill an ant, and a predator measured as a null. All
four check out in the tree. The findings below stand as statements about the
code, and §§2-5's biology stands; what §10 changes is the **order**, because
every decision in §9 is a mechanism in service of a function the box does
not yet have. §10.3 is one number off an existing harness that decides
whether any of it is worth starting.

Stated first and bluntly, per the brief. Each is expanded in its section.

**1. A colony-wide scalar cannot produce nest architecture, and this is
why `(Crowding, Dig, 0.6)` came out a coin flip.** The regulating quantity
in the literature is **worker density along the excavation perimeter**
(Toffin et al., *PNAS* 2009) **[measured]** — a *local* reading at the
face. `NestRoom::occupancy` is roofed void over ants for the whole nest,
so **every ant at the door reads the same number**. A rule that reads one
global scalar can modulate how *much* is dug and can never decide *where*,
which is precisely what `dead-ends.md`'s own null found: the wired arm cut
a *smaller* cavity for the same digging, and buds never appeared in either
arm. That entry is correctly scoped as "this weight, this value, this bed";
the biology says the missing thing is not a tuning but a **second, local
reading**. §4.2. This is `CLAUDE.md`'s *which object does this rule
evaluate* — the answer today is "the colony", and the biology's answer is
"the ant's own square metre of wall".

**2. `ROOM_TARGET_DEFAULT = 2.0` has a real analogue, and it is a
*template* rather than a volume.** The best-measured per-capita set point
in the literature is *Temnothorax albipennis* building its wall at a radius
proportional to the brood cluster it encloses (Franks & Deneubourg 1997)
**[measured]** — the colony regulates **area against a local reference
object**, not volume against a headcount. §4.2. Two consequences: the
engine's choice of a hyperbola that never reaches a stop is **right** and
matches the literature (real colonies slow, they do not stop); and the
*quantity* is arguably wrong — an ant that can see how much room is round
*it* is doing what the biology does, and an ant reading a colony total is
not.

**3. The nest's vertical profile is the distribution the ethos asks for,
and the engine has no depth term anywhere in the dig.** Real nests are
steeply top-heavy: chambers are large and closely stacked near the surface
and become smaller and further apart with depth (Tschinkel on
*Pogonomyrmex badius*) **[measured]**. Nothing in `creature.rs`'s dig verb,
`ant.ron`'s dig wiring, or `NestRoom` reads depth at all. A nest whose
chambers are all the same size at all depths is the *uniform powder*
failure of `CLAUDE.md`'s Law 1, in a subsystem nobody has yet judged by
eye. §2.1, §2.6.

**4. The world is one to two orders of magnitude too shallow for the nests
this material describes.** At `ant.ron`'s `body: Chain(2)` an ant is two
cells, so a cell is roughly 2–5 mm. The lab bed's 80 rows of soil is then
**16–40 cm**; the outdoor blanket is bound by `column::taper_cover` to
about **22 cells — 5–10 cm** (`dead-ends.md`, the `soil_depth` entry). A
*Lasius niger* nest is 30–50 cm **[repeated]**; a mature *P. badius* nest
runs about 2 m **[measured]**; *Atta* reaches 6–8 m **[repeated]**. **The
lab bed holds a founding chamber, not a nest, and the outdoor ground holds
nothing.** §2.5. This is the most decision-relevant finding in the report
and it is a *scale* decision, not a mechanism one.

**5. The engine's ant digs *away* from moisture; the excavation
literature has ants digging *toward* a preferred soil moisture.**
`ant.ron` authors `(MoistureGrad, Dig, -0.55)`. Leaf-cutter workers select
digging sites by soil water content and build to control nest water loss
(Bollazzi & Roces) **[measured]**, and the deep nest is the humid refuge
brood is carried into. **But the engine's channel does not measure what
its name says**: `moisture_gradient`'s own doc records it reading the
vertical air/soil step — *"it is a depth signal"*, 1.91x for twenty rows
lower, curvature flat at 1.01x across every span. So the shipped weight is
a **depth** weight wearing a moisture name, and its sign says *dig less
when deeper*. That is backwards for a nest and was never chosen as a nest
rule. §5.1.

**6. There is no CO₂ in this engine, and CO₂ is one of the two
best-supported excavation and orientation cues.** `grep -i co2 src/sim`
returns one comment in `material.rs` about photosynthesis. Ants carry
antennal CO₂ receptors, nest air runs at percent-level CO₂ against an
0.04% atmosphere, and *Atta* nests are ventilated by wind over the mound
(Kleineidam, Ernst & Roces 2001) **[measured]**. §5.3. The decision is
**not to build a CO₂ field** — §5.3 says what to reuse instead.

**7. "There is no granary in this box" is a defensible design position,
not a hole — but it costs the player the only thing in nest biology that
is an *object*.** Storing food in living workers (repletes) is real ant
biology and is what this engine already ships as the crop. Storing it in a
chamber is also real, and it is the version a player can *see*, *rob*, and
*lose*. §3.5. Law 2 — there must be a verb and it must deliver something —
lands here harder than anywhere else in this report.

---

## 1. What was checked in the tree, and how

The brief supplied context and asked for it to be verified rather than
trusted. Where it was wrong the correction is here and the wrong form is
not repeated below.

| claim as briefed | check | verdict |
|---|---|---|
| Chamber digging is gated on `AtNest` through hidden units 5/6 | `grep -n "AtNest\|Crowding" assets/species/ant.ron` | **true**: `(AtNest,5,30.0) (Crowding,5,6.0)`, `(AtNest,6,30.0) (Crowding,6,-6.0)`, `(5,Dig,2.5) (6,Dig,-2.5)`. `Bias` on those units is −30, so `Bias+AtNest` nets to 0 — the pair sits in the linear part of `squash` at the nest and cancels away from it |
| `BrainInput::Crowding` reads room-per-ant only where `AtNest` is true, else a saturated local density | `sed -n 5028,5037p src/sim/creature.rs` | **true**: `if world.room_gate && inputs[AtNest] > 0.0 { nearest_nest_site → nest_room → occupancy(room_target) } else { density }`, with `None` falling back to `density` rather than to a number |
| `Crowding` is `NestRoom::occupancy` — roofed void per ant, `target/(target+room_per_ant)`, `ROOM_TARGET_DEFAULT = 2.0` | `sed -n 1211,1234p src/sim/world.rs; grep -n "pub const ROOM_TARGET_DEFAULT" src/sim/creature.rs` | **true**, and the constant is **derived rather than chosen**: `latecensus` on the played bed reads ~220 net dug cells at 500,000 frames against colony peaks of 116 and 495 ants, i.e. 1.90 and 0.44 cells/ant, and *neither colony ever stopped digging*. 2.0 is the top of the observed range, deliberately placed so behaviour sits on the steep part of the curve rather than against a stop |
| **"Today that reach is ±2 rows, chosen as a placeholder"** | `sed -n 7242,7247p src/sim/creature.rs` | **false as stated, and the correction matters.** `adjacent_nest` on `main` is still the 8-neighbour *material* test: `NEIGHBOURS_8.iter().any(\|&(dx,dy)\| world.get(x+dx,y+dy).material == nest)`. The `\|hy − site.surface\| ≤ 2` form is the **proposal** in `nest-design` §13, not the tree. So "the geometry of am-I-home is the geometry of should-I-dig" is true today with the geometry being *one cell of contact*, not a strip |
| A nest is about to become a site; no material painted at founding | `sed -n 608,648p Reports/nest-design-2026-09-14.md` | **true as a ruling, unbuilt as code.** §13 is the design of record; `paint_nest_patch` and `nest.ron` are still in the tree |
| There is no granary | `grep -rn "GRANARY" src/lab/ui.rs` | **true, and the UI says it in words**: *"…food in transit and not food put by: THERE IS NO GRANARY IN THIS BOX."* `larder_probe mode=turnover` is the instrument that established it — `resident` 0 from frame 200, which turned "a granary of ten cells" into "ten cells in transit" |
| Soil is a `Powder` so galleries collapse; spoil needs footing | `cat assets/materials/spoil.ron; examples/burrow_probe` row in `instruments.md` | **true, with numbers**: a hand-carved gallery is gone in **5 frames** and a chamber in **30** in `soil`; `spoil` is `needs_footing` precisely so a dumped pellet is a wall only while something is under it |
| Roofed void is attributed by column at `ROOF_REACH` | `grep -n "pub const ROOF_REACH" src/sim/world.rs` | **true**: `ROOF_REACH = 16`, void below the original ground datum with ground within 16 rows directly above, attributed to the nearest site **by column in x only** |
| The engine has no CO₂ | `grep -rn -i "co2\|carbon dioxide" src/sim/*.rs` | **true**: one hit, a comment in `material.rs` about photosynthetic mass loss |
| The engine has temperature and humidity fields | `sed -n 230,280p src/sim/field.rs` | **true, and better than expected**: `FieldCell` carries `temperature`, `sky_temperature` (the day/night forcing, **attenuated with depth by the same Beer-Lambert column transmission as light**) and `moisture`. `temperature − sky_temperature` is the thermal temperature with the oscillator exactly removed — §5.2 leans on this |
| `TempAboveAmb` is a live brain input | `grep -rn "TempAboveAmb" assets/species/*.ron` | **true and wired to one output in every species**: `(TempAboveAmb, Turn, -0.8)`. Nothing in the tree wires temperature to `Dig`, `Move` or `Drop` |
| `SurfaceCurvature` is a live sense | `grep -n "SurfaceCurvature" assets/species/ant.ron; sed -n 7318,7350p src/sim/creature.rs` | **true**, geometric (a Chebyshev-disc lattice count, flat-referenced, flesh excluded), and wired only to **deposition**: `(SurfaceCurvature, Drop, 0.169)`, `(SurfaceCurvature, DropSpoil, 0.169)`. **Not wired to `Dig`** |
| Free genome capacity for a new sense | `sed -n 245,272p src/sim/brain.rs` | **30 named inputs against `INPUT_SLOTS = 64`.** An appended sense moves no existing weight; the manifest is a name-prefix check, so a lawful append does not invalidate stored specimens |

Two of these are worth carrying out of the table. **The engine already has
a depth-graded temperature field with the diurnal oscillator separable
exactly** — that is the substrate §5 needs and I had assumed would have to
be built. And **`MoistureGrad` is a depth signal**, by its own doc's
measurement, which reframes finding 5 above from "wrong sign on moisture"
to "there is already an unlabelled depth term in the dig, and its sign is
backwards for nest-building".

---

## 2. Chamber architecture and depth

### 2.1 The vertical profile, which is the whole shape

The single most reproducible fact about subterranean ant nests is that
**they are top-heavy and graded**, not a uniform warren.

Walter Tschinkel's casting programme — aluminium, plaster and dental
plaster poured into live nests and excavated whole — is the source of
record here, and *Pogonomyrmex badius* (the Florida harvester ant) is its
best-characterised subject (Tschinkel 2004, *Journal of Insect Science*)
**[measured]**. The architecture that comes out:

- A **vertical or helical central shaft** runs the full depth.
- **Chambers hang off the shaft, roughly horizontal**, like shelves.
- **Chamber size decreases with depth** and **the spacing between
  chambers increases with depth**. Tschinkel describes the decline in
  chamber area as approximately **exponential**; the top of the nest holds
  most of the total floor area, and the bottom holds a few small chambers a
  long way apart. **[measured for the shape; the exact fraction in the top
  fifth I will not quote — I do not hold the figure]**
- **Total depth** for a mature colony is about **2 m**, with the deepest
  casts past 3 m. **[measured]**
- Chamber count runs into the **tens to low hundreds** for a mature nest.
  **[repeated]**

The important part for this engine is not the metres. It is that **one
species, digging one nest, produces a graded size distribution along a
single axis** — and that grading is *species-typical*, reproduced when the
colony relocates and rebuilds from scratch. *P. badius* relocates roughly
annually and rebuilds the same architecture in new ground **[measured,
Tschinkel]**, which is the strongest available evidence that the profile is
a **construction rule** and not an accumulation of history.

This matters because an emergent-digging engine will naturally produce the
second and not the first. A colony that digs where it happens to be, with
no depth term, accumulates a warren whose shape is a record of where ants
walked. That is *not* what a cast nest looks like.

### 2.2 What sets the depth

Two candidate answers, and the literature favours the second.

**Not "until there is enough room".** Depth and volume are separable, and
the same volume can be a wide shallow nest or a narrow deep one. Nothing in
the architecture literature treats depth as a volume by-product.

**Soil temperature, and by extension the seasonal thermal profile.**
Bollazzi, Kronenbitter & Roces (2008, *Oecologia*) is the direct study:
across South American *Acromyrmex* species and populations, **nest depth
tracks soil temperature** — colder climates dig deeper, and workers'
digging responds to the thermal profile they encounter **[measured]**.
This is the cleanest "what sets depth" result I know of, and it says the
answer is a **field the ant reads**, not a target it holds.

Secondary and consistent: depth buffers humidity and temperature swing
(§5), depth escapes surface predators and fire, and depth reaches the water
table or the frost line depending on climate. **[general]**

There is a real cost side too — excavation is expensive, and Mikheyev &
Tschinkel (2004, *Insectes Sociaux*) on *Formica pallidefulva* explicitly
priced nest construction against colony resources **[measured]**. A nest is
not dug as deep as possible; it is dug to where the profile stops
improving.

### 2.3 How the architecture grows with colony size

**Nest volume scales with colony size, roughly proportionally, across many
species.** **[repeated as a generalisation; measured within individual
studies]** Tschinkel measured this for *Solenopsis invicta* and for
*P. badius*; Mikheyev & Tschinkel did it for *F. pallidefulva*.

What grows is mostly **the top**: a growing colony adds and enlarges
shallow chambers far more than it deepens the nest. **[general, inferred
from the top-heavy profile being stable across colony sizes]** This is
worth flagging as an inference rather than a citation — it is the
prediction the profile makes, and I have not held a study that measures
the profile at several colony sizes in one species.

The laboratory version of the scaling is well controlled. Buhl, Gautrais,
Deneubourg & Theraulaz (2004, *Naturwissenschaften*) measured tunnelling
network size and structure against group size in a 2D setup **[measured]**;
Rasse & Deneubourg (2001, *Journal of Insect Behavior*) measured *Lasius
niger* excavation dynamics and nest-size regulation against group size
**[measured]**. The shape of both results, which is the transferable part:
**excavated volume rises with group size; digging rate is high initially
and decays; the colony slows rather than stops.**

### 2.4 Species differences worth knowing

The genus determines almost everything, and picking the wrong exemplar
sends a build after architecture that most ants do not have.

| group | nest | why it matters here |
|---|---|---|
| *Atta* / *Acromyrmex* (leaf-cutters) | 6–8 m deep, thousands of chambers, a metres-wide mound, engineered ventilation **[repeated]** | The spectacular case. Everything about it — the fungus garden, the ventilation, the refuse chambers — is *downstream of farming a crop*. Do not import it into a forager |
| *Pogonomyrmex* (harvesters) | ~2 m, shafts with stacked graded chambers, **seed chambers in the upper nest** **[measured]** | The right exemplar for this engine's ant, which carries seeds |
| *Formica* (mound / wood ants) | a thatch mound above ground, shallow soil chambers below; the mound is the thermal organ **[repeated]** | The architecture is *above* ground, which is a different build entirely and interacts with the engine's spoil heap |
| *Lasius niger* | 30–50 cm, irregular chambers, a shallow soil nest often under a stone **[repeated]** | The closest match to this engine's achievable depth. Most of the nest-size-regulation laboratory work uses it |
| *Temnothorax* / *Leptothorax* | **not dug at all** — a colony of tens occupies a flat crevice (an acorn, a rock cleft) and builds a *wall* to enclose it **[measured]** | The best-measured regulation of nest size in the literature, precisely because the geometry is one dimension (§4.2) |

**The engine's ant is closest to *Lasius* in scale and *Pogonomyrmex* in
diet.** That combination is not a species, and it does not need to be —
but it means the harvester granary (§3) and the *Lasius* depth are the two
exemplars to aim at, and the *Atta* material is background.

### 2.5 Scale — the finding that constrains everything else

This is the section that should change the build, and it is arithmetic
rather than biology.

`ant.ron` authors `body: Chain(2)`, described in its own comment as *"the
cheapest thing that is unambiguously a body with a front"*. Real ant
workers run about 3 mm (*Lasius niger*) to 10 mm (*Pogonomyrmex badius*
majors). **So one cell is somewhere between 1.5 mm and 5 mm**, and the
honest span to carry is **2–5 mm per cell**.

Against that:

| | rows | at 3 mm/cell |
|---|---|---|
| lab bed soil (`params::soil_depth`, shipped) | 80 | **24 cm** |
| outdoor soil blanket, *effective* (`taper_cover`-bound, `dead-ends.md`) | ~22 | **7 cm** |
| `ROOF_REACH` — the overburden a chamber may have and still count | 16 | **5 cm** |
| *Lasius niger* nest **[repeated]** | 100–170 | 30–50 cm |
| *Pogonomyrmex badius* nest **[measured]** | ~670 | 2 m |
| *Atta* nest **[repeated]** | ~2,000 | 6 m |

**Three things follow, and none of them is a mechanism.**

- **The lab bed can hold the top shelf of a *Lasius* nest and nothing
  more.** At 80 rows there is no room for a graded profile: two or three
  chamber tiers is the whole budget. A depth rule with an exponential decay
  constant has nothing to decay over.
- **The outdoor world, today, cannot hold an ant nest at all.** The
  `soil_depth` dead end is explicit that the blanket is bound by the
  distance to the nearest bare column and reads ~22 cells however deep the
  parameter says the soil is. That is the finding to bring to any outdoor
  nest proposal, and it has a named remedy in that entry (remove the bare
  columns first; `soil_depth` only binds afterwards).
- **`ROOF_REACH = 16` is a plausible *roof thickness* and an implausible
  *nest depth*.** The two are different quantities and the census only
  needs the first, so this is not a defect — but a reader who takes
  `ROOF_REACH` as "how deep the nest is" will be wrong by a factor of
  forty.

**The honest hedge**: if the intended reading is that an ant is *one* cell
and `Chain(2)` is head-plus-body abstraction, a cell is 3–10 mm and every
figure above doubles. The lab bed becomes 48–80 cm — a whole *Lasius* nest
— and the conclusion softens from "impossible" to "tight". **Nothing in
the tree states a metres-per-cell convention**, and this is worth settling
once in `README.md` rather than re-derived per report, because three
different subsystems (plants, the gnome, nests) each carry an implicit one.

### 2.6 → Decisions for this engine

**D2.1 — Give the dig a depth term, and make it graded.** The literature's
single most robust architectural fact is a size-and-spacing gradient with
depth, and the engine has no depth term in the dig at all. This is Law 1
stated in soil: a nest whose chambers are all one size is the uniform
powder. **The cheapest form**: depth below the founding surface is already
computable at the dig site (`room_surface` computes the datum per column),
so this is a value at the call site, not a new field.

**D2.2 — Do not implement it as a dig-*target* preference.** Two target
rules are already dead and the entries are specific: weighting the roll by
how **buried** a cell is inverted (roofed void 172→43 across the sweep,
monotone down) and weighting by **cover overhead** did nothing (every arm
at or below the ablated one, seed spread 100–186 swamping it). Both
entries' shared re-test note is that *the colony already tunnels* —
91–96% of roofed void is one connected run. So the depth term belongs on
**how much is dug and how large a chamber is allowed to grow**, not on
which cell is chosen. Proposing a third target rule would be
`CLAUDE.md`'s *two fixes failing the same way means the approach is
wrong*.

**D2.3 — Settle the metres-per-cell convention before sizing any nest
work, and write it down once.** Everything in D2.1 is bounded by §2.5, and
§2.5 is bounded by a convention nobody has stated. This is a one-paragraph
decision and it gates the rest.

**D2.4 — Treat the outdoor nest as blocked on soil depth, not on nest
code.** A nest build that lands in the lab and is then pointed at the
outdoor world will find 22 cells of soil. The `soil_depth` dead end names
the remedy and it is a worldgen change, in another lane's territory.

**D2.5 — Do not chase *Atta*.** The architecture that is famous is the
architecture of a farming colony with a crop that has to be ventilated. The
engine's ant is a forager at *Lasius* scale. Aim at three or four graded
tiers with a shaft, not at a cathedral.

---

## 3. Granaries and food storage

### 3.1 The harvester granary

*Pogonomyrmex* and *Messor* are the seed-storing genera, and the granary is
a real, locatable feature of the nest.

- **Seeds are concentrated in chambers in the upper nest**, not scattered
  through it, and not in the deepest chambers. In *P. badius* casts
  Tschinkel recorded chamber contents and found seeds held in identifiable
  upper chambers **[measured for the concentration; the specific depth band
  I will not quote as a number]**.
- **A colony's store is large enough to matter**: harvester colonies hold
  seed stores that carry them through seasons with no foraging.
  **[repeated]**
- **The store is worked, not just piled.** Workers **husk** seeds, separate
  chaff, and *Messor* species mill seeds into a chewed mass ("ant bread").
  **[repeated]**
- **Germinated seeds are culled.** A sprouting seed in a granary is removed
  — carried out, or killed. **[repeated]** This is the behaviour that makes
  a granary a *tended* store rather than a heap, and it is the one most
  worth having in a game, because it is a visible verb performed on a
  visible object.
- **Seeds are carried out to dry after rain.** Reported for harvester ants
  as a response to nest wetting. **[repeated — I am confident this is
  reported and I do not hold the primary source]**

### 3.2 Why food is kept where it is kept — the conflict that shapes the nest

This is the answer the brief asks for, and it is not "because that is
where there is room".

**Seeds must stay dry. Brood must stay wet.** Those are opposite
requirements in the same nest, and the nest resolves them **along the
vertical axis**:

- A seed in a humid chamber **germinates or moulds**. Dry storage is not a
  preference, it is the condition under which a store exists at all.
- Ant **eggs and larvae desiccate** and are kept at high humidity, which
  underground means **deeper**. **[repeated; the requirement is standard,
  the numbers vary by species]**
- Therefore the granary sits **shallow and dry**, the brood sits **deep and
  humid**, and the colony **moves things between them** as conditions
  change.

**That vertical sorting is the reason chambers are differentiated at
all**, and it is the honest answer to "what is a chamber for": a chamber is
a place with a *microclimate*, and different contents want different ones.
Everything in §5 is the machinery that makes the different chambers
different.

A second, weaker organising principle: **proximity to the door costs and
pays.** Food coming in stops near the entrance; refuse going out leaves by
it; brood that must never leave sits furthest from it. **[general]**

### 3.3 Fungus gardens

Attine agriculture is a different economy and is included for contrast
rather than as a model.

- The **fungus garden is the food**, living in chambers on a substrate of
  chewed leaf. It is not a store; it is a crop with a metabolism.
  **[measured/standard]**
- It is **narrowly sensitive to humidity and to CO₂**, which is what drives
  the ventilation architecture in §5.3. **[measured]**
- **Refuse is lethal to it.** In *Atta colombica* waste goes to an external
  dump; in *Atta cephalotes* to dedicated underground refuse chambers. Hart
  & Ratnieks (early 2000s, *Behavioral Ecology* / *Animal Behaviour*)
  measured waste management as a specialised, age-based task with strong
  segregation between refuse workers and garden workers **[measured]**.

**The transferable finding is the segregation, not the fungus**: refuse is
handled by a *different set of individuals* in a *different place*, and the
separation is enforced.

### 3.4 Middens and refuse organisation

Beyond leaf-cutters this generalises well. Ants of many genera maintain a
**discrete refuse pile** — a "kitchen midden" — rather than scattering
waste: spent food, corpses, husks and chaff go to one place, inside a
dedicated chamber or outside the entrance. **[repeated; the behaviour is
standard across the literature]**

Corpse handling specifically (necrophoresis) is one of the most studied
single behaviours in the field: a dead nestmate is carried out and
deposited, triggered by the chemistry of decomposition. **[measured —
Wilson's classic oleic-acid work]**

**The game-relevant fact is that refuse is *localised and ringed*.** A
visible midden outside the door, growing, is a piece of nest architecture
the player can read from the surface without seeing underground at all —
which matters enormously here, because §13's ruling is that at rest the
player sees nothing.

### 3.5 The replete fork — which this engine already shipped

Storage in **living workers** is the other real answer. Honeypot ants
(*Myrmecocystus*, *Camponotus inflatus*) keep repletes — workers distended
with liquid food, hanging in deep chambers, functioning as a larder that
walks in **[measured/standard]**. Beyond the specialists, **trophallaxis
makes every colony's crops a distributed store**: food in the social
stomach is colony food, not the individual's.

**That is exactly what this engine ships**, and `dead-ends.md` records the
design argument already being had: the `TRAIT_STORE_IN_BODY` slot was cut
because the two forks — surplus in the body, surplus put down — are already
corners of one space expressed by brain-output weights conditioned on crop
fill, food adjacency and both pheromone planes. The biology **endorses** the
shipped position: food in crops is a real larder.

**What it does not endorse is the absence of the other fork.** Both exist
in nature, and only one exists here.

### 3.6 → Decisions for this engine

**D3.1 — Build the granary, and build it as a *place with a rule*, not as
a stockpile number.** The biological granary is a chamber whose contents
are sorted, dried and culled. The minimum version that is recognisably
that: **a `Drop` that prefers a roofed, dry cell near the site** — both
readings already exist (`roofed_in_column`'s test, `FieldCell::moisture`)
— plus **germinated-seed culling**, which the engine can already express,
because a set-down seed becoming a plant is a thing that happens today and
a digging ant already clears one as spoil (`wiki/ants.md`). Turning that
accident into a *rule* is close to free and is a genuine ant behaviour.

**D3.2 — The granary's payoff is that it can be lost.** A store in a
chamber can flood, be dug out from under, be robbed, or sprout. A store in
crops cannot. Law 2 asks for a verb that delivers something visible; the
store is the *noun* that makes the existing verbs legible. Frame the
feature that way rather than as a colony-economy buffer — the economy
argument is weak (crops already buffer) and the legibility argument is
strong.

**D3.3 — Give refuse its own destination before giving it its own
chamber.** The cheap, high-return half of §3.3–3.4 is *localisation*:
corpses and spent matter accumulating at **one** place near the door,
visible from the surface. That is a midden, it is standard ant behaviour
across genera, and it is player-readable while the nest itself is invisible.
A dedicated refuse *chamber* underground is the expensive half and buys
less, because nobody sees it.

**D3.4 — Do not model the fungus garden.** It is a farming economy with a
crop that needs ventilation, and every part of it is downstream of a
mechanism this engine has no reason to build. Keep it as the explanation
for why the *Atta* ventilation literature exists.

**D3.5 — When the granary lands, price it against `larder_probe`, not a
new harness.** `instruments.md` is explicit that `larder_probe` is not
larder-specific: it is a **proximity census with both controls in the
binary**, and its `mode=turnover` is what distinguishes a store from a flow
(`resident` 0 from frame 200 is the reading that established there is no
granary). The success criterion for D3.1 is that same column becoming
non-zero. **That instrument already exists and already has the number that
must move.**

---

## 4. Digging regulation and division of labour

### 4.1 What makes an individual ant start digging

The proximate triggers reported in the literature, with what each costs an
animal to sense:

1. **Local worker density at the face.** The best-controlled result.
   Toffin, Di Paolo, Campo, Detrain & Deneubourg (*PNAS* 2009) ran
   excavation in deliberately homogeneous 2D conditions and found a
   morphological transition from a circular cavity to a branched structure,
   with **density along the perimeter** driving it: high density →
   uniform digging → round; falling density → localised buds → branching
   **[measured]**. Owned by `stigmergy-research.md` §5; not re-derived here.
2. **CO₂.** Ants carry antennal CO₂ receptors, and nest CO₂ rises with
   colony metabolism in a poorly ventilated nest. Excavation relieving CO₂
   is the standard interpretation, and the *Atta* ventilation work rests on
   it. **[measured for the receptors and the nest concentrations;
   [general] for CO₂ as a direct digging trigger, which I would not state
   as established]**
3. **Soil moisture.** Leaf-cutter workers select digging sites by soil
   water content, and build to control nest water loss (Bollazzi & Roces)
   **[measured]**. Soil that is too dry does not hold a tunnel; soil that
   is waterlogged is not dug.
4. **Temperature.** Bollazzi, Kronenbitter & Roces (2008) — nest depth
   tracks soil temperature (§2.2) **[measured]**. The ant digs *toward* a
   thermal preference.
5. **Stigmergy — the pellet.** A digging ant deposits its pellet where
   others have deposited, and the resulting accumulation attracts further
   deposition. This is the positive-feedback half; it is what turns a
   scatter of digging into a structure. **[measured — the general
   mechanism; Deneubourg's amplification framework]**

### 4.2 What stops one — is there a per-capita set point?

**This is the brief's headline question and it deserves a careful
answer, because the easy one is wrong in a way that would be expensive.**

**The short answer: yes, nest size is regulated against colony size; no,
the regulated quantity is not a colony-wide volume-per-ant.**

What is actually measured:

- **Excavated volume rises with group size and digging rate decays.**
  Rasse & Deneubourg (2001) on *Lasius niger* **[measured]**; Buhl et al.
  (2004) on group size and tunnelling networks **[measured]**. The shape in
  both: a fast initial phase, then slowing, then a long tail. **The colony
  never stops.** This is a direct endorsement of `NestRoom::occupancy`'s
  hyperbola over a clamped ramp — the engine's stated reason (nothing
  saturates at either end) happens to match the biology's, which is worth
  recording as a point in the design's favour rather than a coincidence.
- **The one crisp set point in the literature is a *template*.** Franks &
  Deneubourg (1997) on *Temnothorax* (then *Leptothorax*) wall-building:
  workers build the nest wall at a **radius proportional to the size of
  the brood cluster it encloses**, so nest area scales with colony size
  **[measured]**. The earlier Franks, Wilby, Silverman & Tofts (1992)
  "blind bulldozing" paper gives the individual-level rule **[measured]**.
  **The ant does not know the colony's volume. It knows how far it is from
  a reference object it can see, and the reference object's size is the
  colony's size.**
- **Nest volume per worker as a *reported statistic* exists** — it falls
  out of any study that measures both — **but I do not hold a defensible
  per-capita figure for any species**, and I will not supply one. §7 says
  what would settle it.

**The three consequences for `ROOM_TARGET_DEFAULT`:**

1. **The dial is legitimate.** A per-capita room target is a reasonable
   *abstraction* of a regulation that really happens. It is not fabricated.
2. **Its value being derived from this engine's own census is the right
   method and should not be replaced by a number from the literature.** The
   constant's doc derives 2.0 from `latecensus`'s 220 net dug cells against
   116 and 495 ants. A biological figure would be a different world's
   answer to a different question, and `CLAUDE.md` is explicit that bars
   come from measurement with headroom.
3. **The quantity is global where the biology's is local, and that
   predicts the observed null.** `(Crowding, Dig, 0.6)` came out 16 of 33
   seed pairs with the sign reversing between colony sizes, and **buds never
   moved in either arm (0 vs 0, 0 vs 5, 6 vs 5)**. Buds are exactly what
   Toffin's density mechanism produces, and they are exactly what a global
   scalar cannot produce: every ant reads the same value, so there is no
   spatial variation for a bud to form at. **The null is evidence about the
   reading, not about the mechanism**, and that entry's own caution
   ("narrower than density-dependent digging does not work here") is
   correct and now has a named reason.

### 4.3 Stigmergy and the pellet — and what this engine already has

The deposition half is where this engine is closest to the biology and
does not know it. `ant.ron` authors `(SurfaceCurvature, Drop, 0.169)` and
`(SurfaceCurvature, DropSpoil, 0.169)` — **a positive weight on convexity
for deposition**, which is the termite/ant construction rule exactly:
material accumulates on bumps, bumps become pillars, pillars become walls.

Two things temper it:

- `surface_curvature` is a **real geometric sense** (a Chebyshev-disc
  lattice count with the flat reference subtracted, flesh and nestmates
  excluded — and its doc records that including flesh read exactly −0.083
  at all 18,720 samples, the tidiness tell). So the channel is sound.
- **It cannot see a feature wider than its own disc.** At radius 2 a
  five-wide slot sampled at its centre reads *convex*. A chamber is wider
  than five cells, so this sense is blind to chamber-scale geometry by
  construction. That is a stated limit in the code, not a bug, and it
  bounds what the deposition rule can build.

**The excavation half has no equivalent.** `SurfaceCurvature` is not wired
to `Dig`. The channel that *was* meant to carry excavation shaping is
`MoistureGrad`, and its own doc says flatly: *"if you are here to make
deposition follow curvature, it currently does not, and no re-derivation of
these offsets will make it"* — curvature moves it 1.006–1.012x at every
span while twenty rows of depth move it 1.91x.

### 4.4 Who digs

- **Digging sits in the temporal polyethism sequence**, generally done by
  workers younger than foragers and older than nurses. **[repeated —
  standard, and the sequence varies by species]**
- **In polymorphic species it is size-linked**: *Atta* has physical castes
  and excavation is done by particular size classes. **[measured/standard]**
- **Effort is strongly skewed.** In most task studies a minority of
  individuals perform the majority of the work, with a substantial fraction
  of workers doing very little. **[repeated — this is one of the most
  robust generalisations in social-insect behavioural ecology]**
- **Tschinkel's spatial finding is the interesting one**: in *P. badius*
  the colony is **vertically sorted by worker age**, with younger workers
  deeper in the nest and older workers (foragers) near the surface
  **[measured]**. Age polyethism is *spatial*: an ant's job follows from
  where it is, and where it is follows from how old it is.

**The engine already reproduces the skew by accident and knows it**:
`lab/ui.rs` says *"IT IS USUALLY A FIFTH OF THEM DOING ALL OF IT"* about
deliveries. That is the biology's own number-shape arriving without being
authored, and it is worth not breaking.

### 4.5 → Decisions for this engine

**D4.1 — Add a *local* room reading beside the global one; do not replace
it.** The biology's regulating quantity is read at the face. The minimum
faithful form is **roofed void within a small radius of this ant's head**
— not a new field, a local lattice count in the same family as
`surface_curvature`, which is already the engine's proven answer to "a
coarse field cannot support a per-cell decision". Keep `NestRoom::occupancy`
as the colony-scale term; the two answer different questions and the
literature has both.

**D4.2 — The success criterion for D4.1 is buds, and it is already
instrumented.** The `(Crowding, Dig, 0.6)` entry kept its harness
deliberately (`circ`/`inradius`/`buds`, `arms=selftest`, `crowddig=`/
`digbias=`) *"because the null is only readable through it"*. A local
reading that produces buds where the global one produced 0 vs 0 is the
result; anything less and this is the same mechanism again. **Twelve seeds
minimum** — that entry is the repo's own worked case of six seeds being
tidier than the truth, and its power analysis says 33 seed pairs only
detects a true rate around 0.74.

**D4.3 — Do not add a CO₂ field.** It is the one major biological cue with
no substrate here, building it means a new diffusing channel in the sweep,
and its *function* — "the nest is crowded and under-ventilated, dig" — is
what D4.1's local density reading already expresses. §5.3 says the same
thing from the microclimate side. This is the clearest "say no" in the
report.

**D4.4 — Leave who-digs alone.** The engine has no worker age, no castes on
the ant, and a skew that already matches the literature. Adding age-based
task allocation is a large mechanism whose visible output is a statistic.
**If the vertical sorting in §4.4 is ever wanted, it comes free with D2.1**:
if young ants stay near the brood and the brood is deep, the sorting is a
consequence of the depth gradient rather than a rule.

**D4.5 — Fix the `MoistureGrad` sign question before adding any depth
term, because there is already an unlabelled one.** `(MoistureGrad, Dig,
-0.55)` is, by that function's own measurement, **a depth weight**, and its
sign says *dig less the deeper you are*. Whatever D2.1 does about depth
will be fighting it. This is `CLAUDE.md`'s *a term in a weighted sum is not
an independent knob* arriving before the change rather than after: **name
this constant in the budget for any depth work**. Note also that the weight
is heritable — a sign and a magnitude both — so the population may already
have moved off the authored value.

---

## 5. Nest microclimate — what chambers are actually for

### 5.1 Humidity

**The requirement is brood.** Ant eggs and larvae have little defence
against desiccation and are kept at high relative humidity; workers move
brood to whichever chamber is closest to the preference as conditions
change through the day and the season. **[repeated; the requirement is
standard and the preferred values differ by species]**

**Depth is how humidity is achieved.** Soil water content and RH are more
stable and higher with depth; the surface dries and rewets and the metre
below does not. So "the brood is deep" and "the brood is humid" are the
same statement.

**Ants also build for it.** Bollazzi & Roces measured leaf-cutter workers
controlling nest water loss through building behaviour — plugging and
opening — which is a *construction* response to a microclimate reading
**[measured]**.

**What the engine has.** `FieldCell::moisture` is a real diffusing
ambient-humidity channel sourced from liquid cells, and the lab bed
develops a vertical drying profile on its own. Three live brain inputs read
it: `MoistureFront`, `MoistureLateral`, `MoistureGrad`.

**And the one that is wired to digging is measuring depth, not moisture**
(§1, §4.5). This is the contradiction from §0 finding 5, and having read
the function's doc it is sharper than the brief's framing: the sign is
backwards *for a nest*, but the weight was authored for **surface
deposition**, where it does the right thing (`ascii`'s deposition scene
reads 2.94x before the coefficient moved into the genome and 2.97x after).
**It is one weight serving two purposes and it was only ever derived for
one of them.**

### 5.2 Temperature

- **Brood is kept warm**, and the warmest usable chamber is chosen —
  shallow by day in cool weather, deeper when the surface is too hot.
  **[repeated]**
- **Depth is set by the thermal profile** (Bollazzi et al. 2008)
  **[measured]** — §2.2.
- **Mound nests are thermal organs.** *Formica rufa*-group thatch mounds
  are warmed by insolation and by microbial fermentation in the thatch, and
  workers shuttle heat inward by basking at the surface and returning.
  **[repeated; the heat-shuttling is measured in the *Formica* literature
  and I do not hold the paper]**

**What the engine has, and it is more than expected.** `FieldCell` carries
`temperature` **and** `sky_temperature`, where the second is the day/night
forcing at that cell, **attenuated with depth by the same Beer–Lambert
column transmission that attenuates light**. So:

- **The engine already has a depth-graded diurnal thermal profile.** That
  is the physical substrate the whole of §5.2 needs, and it exists.
- **The oscillator can be divided out exactly.** `temperature −
  sky_temperature` is the thermal temperature with the diurnal cycle
  removed, and the doc is explicit that the subtraction is exact rather
  than approximate *because* the forcing is depth-graded. `CLAUDE.md`'s
  *a designed oscillator must be divided out of every number it reaches*
  is already satisfied here, by construction, and
  `field::noon_equivalent_temperature` is the named accessor.
- **Nothing reads it for a decision.** Every species wires `(TempAboveAmb,
  Turn, -0.8)` and nothing else: the ant turns away from heat and never
  digs, moves or drops because of it.

**The per-cell caveat, which is already a filed bug.** `Cell::temperature`
is `i16` in whole degrees and `CLAUDE.md` names it (§Z27) as an instance of
*a channel that decays and is read as a gradient needs range for both*.
Any nest rule reading a *thermal gradient* should read the field, not the
cell, and should check the discriminator that rule names: **the consumer's
decision threshold against the storage quantum.**

### 5.3 Gas exchange

- **Nest air is not atmosphere.** Colony metabolism (and, in attines, the
  fungus) raises CO₂ well above the 0.04% outside — percent-level in large
  *Atta* nests. **[measured that it is greatly elevated; [repeated] for the
  specific percentages, which I would not build against]**
- **Ants sense it.** Antennal CO₂ receptors are characterised, and
  Kleineidam and colleagues did much of that work in leaf-cutters.
  **[measured]**
- **Big nests are ventilated passively.** Kleineidam, Ernst & Roces (2001,
  *Naturwissenschaften*) measured **wind-induced ventilation of the giant
  nest of *Atta vollenweideri***: wind over the mound drives outflow
  through central openings and inflow through peripheral ones **[measured]**.
  This is the clearest case of nest *shape* serving gas exchange.
- **For a small nest none of this is needed.** Diffusion through a short
  shaft is sufficient for a colony of hundreds. Ventilation architecture is
  a *scaling* response, not a general ant feature. **[general, but it
  follows directly from the size of the nests that have it]**

### 5.4 What chambers are for — the synthesis

Pulling §3.2 and §5 together, because this is the sentence the report
exists to deliver:

> **A chamber is not storage. It is a *microclimate*, and the nest is a
> stack of them.** The vertical axis carries a temperature gradient, a
> humidity gradient and a gas gradient, all three monotone with depth, and
> the colony's whole architecture is a way of having several points on
> those gradients available at once so it can put each thing where that
> thing belongs — seeds dry and shallow, brood humid and deep, the queen
> deepest, refuse out.

That is a *distribution along an axis*, which is Law 1 in its natural
habitat. And it says what the nest build should optimise for: **not volume,
but the number of distinguishable places.**

### 5.5 → Decisions for this engine

**D5.1 — The microclimate substrate exists; build a *consumer*, not a
field.** `FieldCell` already has depth-graded temperature with an exact
oscillator separation, and moisture. The gap is that no weight reads either
for a nest decision. That is much cheaper than it looked, and it is the
single highest-return item in this report per unit of work.

**D5.2 — The minimum honest version of the whole of §5 is one rule: *put
this thing where it belongs on the vertical gradient*.** Seeds shallow,
brood deep. It needs no new field, no new sense (`MoistureFront` and
`TempAboveAmb` are live and free of dig wiring), and it produces the
graded, vertically-differentiated nest that §2 says is the recognisable
shape. It also gives §3's granary a *reason* to be where it is, which is
the difference between a feature and a stockpile.

**D5.3 — No CO₂, and no ventilation.** Restating D4.3 from this side: the
function CO₂ serves is "this place is crowded and stale", and the
under-ventilated case only arises at *Atta* scale, which §2.5 says this
world cannot hold. If a staleness cue is ever wanted, **the existing
`FieldCell` is where it goes** — a depth-attenuated air-exchange term on a
channel that already diffuses — not a new plane in the sweep.

**D5.4 — When a thermal gradient is read, read the field and run the
quantisation check first.** `Cell::temperature`'s whole-degree `i16` is a
filed instance of the decaying-gradient defect, and the discriminator is
one ratio: the consumer's decision threshold against the storage quantum.
That check killed two of four candidates in the 2026-09-15 survey without
building a world, and it is cheaper than discovering the channel reads
0.000 after the rule is built.

---

## 6. Where the literature disagrees, and what I could not settle

**Stated plainly, because a report that sounds uniformly confident is
lying somewhere.**

1. **Whether nest architecture is a "template" or purely emergent is a
   live argument.** Tschinkel's casting work emphasises species-typical,
   reproducible architecture — which implies a rule the animal carries.
   The Deneubourg/Theraulaz school emphasises self-organisation from local
   rules with no plan anywhere. Both are partly right and the split is not
   settled. **For this engine it matters**: the first says build a depth
   rule, the second says build local feedback and let the shape fall out.
   **My reading is that it is not either/or** — the local rules are
   parameterised by things (density, moisture, temperature) that vary with
   depth, so a species-typical profile can emerge from local rules in a
   graded environment. That is the version this engine can build, and it is
   what D2.1 + D5.2 together amount to.
2. **Per-capita nest volume: I could not find a figure I would stake a
   build on.** The scaling relationship is well supported; a canonical
   cells-per-ant number is not something I hold. §7 says what to do
   instead, and the answer is *do not import one*.
3. **CO₂ as a direct digging trigger** is plausible, widely assumed, and I
   would not state it as established. The receptors are measured; the
   concentrations are measured; the causal link to excavation rate I have
   marked **[general]** deliberately.
4. **The "top fifth holds most of the chamber area" shape is solid; the
   fraction is not.** I state the exponential decline as Tschinkel's
   description and decline to give a percentage.
5. **Depth figures are strongly species- and soil-dependent.** Every depth
   in §2 is one genus, usually one study system, often one soil type.
   Treating 2 m as "how deep an ant nest is" would be exactly the error
   §2.4 warns about.
6. **The engine's own metres-per-cell is unstated** (§2.5), so every
   comparison in §2.5 carries a factor-of-two uncertainty that is *this
   repository's*, not the literature's.

---

## 7. What would settle the open questions here — measurement, not reading

Per `CLAUDE.md`: check `instruments.md` before proposing a harness. All
four below are existing instruments.

**Q1. Does the colony's nest have a vertical profile at all today, and is
it graded?** — **`latecensus` already reports the footprint** (roofed void,
open pit, packed above and below the original surface) but as **totals**,
not banded by depth. The question is one column banded by depth rather than
a new harness. Its `control=selftest` carves a known chamber, so the
positive control exists. **This is the measurement that would say whether
finding 3 is a real absence or an unmeasured one, and it should be taken
before any depth rule is built** — §0 finding 3 is currently an argument
from the code, not from a census.

**Q2. What is room-per-ant's *distribution* across a nest, not its mean?**
— `NestRoom` computes one number per site. Finding 1 says the biology's
quantity is local. **`larder_probe`'s banding design is the template**: it
is described in `instruments.md` as a proximity census with both controls
in the binary, for any "is this concentrated at X, or merely present"
question. Room-per-ant banded by distance from the ant, with a hand-carved
control, answers whether a local reading would even vary — and
`CLAUDE.md`'s *a change that moves nothing* says to check that **before**
wiring it, not after.

**Q3. Does anything in the world hold still long enough for a chamber to
be a place?** — **`burrow_probe` already answered the pessimistic half**: a
hand-carved gallery is gone in 5 frames and a chamber in 30, in `soil`,
with the `stone` arm as the positive control at 100%. **Anything in §3 or
§5 that treats a chamber as a persistent location is downstream of that
number**, and `labnest`'s non-reproduction (roofed 230–260 recorded,
45–58 on re-run) says the standing figure is not currently trustworthy
either. **Re-establish the chamber-lifetime number before designing
anything that stores something in one.**

**Q4. What is a cell worth in metres?** — **Not a measurement. A decision**
(D2.3), and it should be written into `README.md` once.

**One thing this report deliberately does not propose**: a harness for any
of it. This is a research lane with a bounded deliverable, and three of the
four questions above are a column added to an existing instrument.

---

## 8. Sources

Author, year and journal, as confidently as I can state them. **No URLs —
see the header for why.** Marked **[measured]** where I am confident of the
study and its headline finding, **[repeated]** where the finding is
standard and I have not held the source.

**Architecture and casting**
- Tschinkel, W.R. (2004) *The nest architecture of the Florida harvester
  ant,* Pogonomyrmex badius. *Journal of Insect Science* 4. **[measured]**
- Tschinkel, W.R. (2003) *Subterranean ant nests: trace fossils past and
  future?* *Palaeogeography, Palaeoclimatology, Palaeoecology* 192.
  **[measured]**
- Tschinkel, W.R. (2005) *The nest architecture of the ant,* Camponotus
  socius. *Journal of Insect Science* 5. **[measured]**
- Tschinkel, W.R. (2021) *Ant Architecture: The Wonder, Beauty, and Science
  of Underground Nests.* Princeton University Press. **[measured]** — the
  single best entry point, and the source of record for the vertical
  profile, the relocation finding and the age stratification.
- Mikheyev, A.S. & Tschinkel, W.R. (2004) *Nest architecture of the ant*
  Formica pallidefulva*: structure, costs and rules of excavation.*
  *Insectes Sociaux* 51. **[measured]**

**Excavation and regulation**
- Toffin, E., Di Paolo, D., Campo, A., Detrain, C. & Deneubourg, J.-L.
  (2009) *Shape transition during nest digging in ants.* *PNAS* 106.
  **[measured]** — already cited by `stigmergy-research.md` §5 and by
  `dead-ends.md`.
- Buhl, J., Gautrais, J., Deneubourg, J.-L. & Theraulaz, G. (2004) *Nest
  excavation in ants: group size effects on the size and structure of
  tunneling networks.* *Naturwissenschaften* 91. **[measured]**
- Rasse, P. & Deneubourg, J.-L. (2001) *Dynamics of nest excavation and
  nest size regulation of* Lasius niger. *Journal of Insect Behavior* 14.
  **[measured]**
- Franks, N.R. & Deneubourg, J.-L. (1997) *Self-organizing nest
  construction in ants: individual worker behaviour and the nest's
  dynamics.* *Animal Behaviour* 54. **[measured]** — the template result
  §4.2 rests on.
- Franks, N.R., Wilby, A., Silverman, B.W. & Tofts, C. (1992)
  *Self-organizing nest construction in ants: sophisticated building by
  blind bulldozing.* *Animal Behaviour* 44. **[measured]**

**Microclimate**
- Bollazzi, M., Kronenbitter, J. & Roces, F. (2008) *Soil temperature,
  digging behaviour, and the adaptive value of nest depth in South American
  species of* Acromyrmex *leaf-cutting ants.* *Oecologia* 158.
  **[measured]** — the "what sets depth" paper.
- Bollazzi, M. & Roces, F. — a series on humidity, building behaviour and
  nest water loss in *Acromyrmex* (*Insectes Sociaux* and related, late
  2000s–2010s). **[measured that the series exists and its findings;
  [repeated] for any individual year/volume]**
- Kleineidam, C., Ernst, R. & Roces, F. (2001) *Wind-induced ventilation of
  the giant nest of the leaf-cutting ant* Atta vollenweideri.
  *Naturwissenschaften* 88. **[measured]**

**Waste and storage**
- Hart, A.G. & Ratnieks, F.L.W. (early 2000s) — waste management and
  refuse-worker segregation in *Atta colombica*, *Behavioral Ecology* /
  *Animal Behaviour*. **[measured for the findings; [repeated] for the
  exact year and venue]**
- Hölldobler, B. & Wilson, E.O. (1990) *The Ants.* Harvard University
  Press; and (2009) *The Superorganism.* W.W. Norton. **[measured]** — the
  general source behind every **[repeated]** claim about middens,
  necrophoresis, repletes, seed handling and temporal polyethism.

**In this repository**
- [`nest-design-2026-09-14.md`](nest-design-2026-09-14.md) — homing (§2),
  what the engine does today (§3), the owner's no-paint ruling (§13).
- [`stigmergy-research.md`](stigmergy-research.md) §5 — excavation shaping
  and the Toffin mechanism.
- [`dead-ends.md`](dead-ends.md) — the `(Crowding, Dig, 0.6)` null, the two
  dig-target rules, the `soil_depth` entry, `TRAIT_STORE_IN_BODY`.
- [`instruments.md`](instruments.md) — `latecensus`, `larder_probe`,
  `burrow_probe`, `labnest`, `spoil_destination`.

---

## 9. The decisions in one list

For a session picking up the nest build. Each is stated in §2.6, §3.6,
§4.5 or §5.5 with its evidence.

| | decision | cost |
|---|---|---|
| **D2.3** | Settle and write down metres-per-cell. **Gates everything else in §2.** | one paragraph |
| **D5.1/D5.2** | Build a *consumer* of the existing depth-graded temperature and moisture fields: put things where they belong on the vertical gradient. **Highest return per unit of work in this report.** | genome weights; no new field |
| **D2.1** | Give the dig a depth term — on *how much* and *how large*, never on which cell (**D2.2**) | a value at the call site |
| **D4.5** | First name `(MoistureGrad, Dig, -0.55)` as the unlabelled depth term it already is, and budget re-deriving it | part of D2.1, not extra |
| **D4.1/D4.2** | Add a *local* room reading beside the global one; success is **buds**, on twelve seeds, through the harness `dead-ends.md` kept | a lattice count |
| **D3.1/D3.2** | A granary as a place with a rule (roofed, dry, cull the sprouted), framed as the thing that can be *lost* | a `Drop` preference plus a cull |
| **D3.3** | Refuse *localised* near the door before refuse *chambered* underground | a destination |
| **D3.5** | Price the granary with `larder_probe mode=turnover`; `resident` is the column that must move | one command |
| **D2.4** | Outdoor nests are blocked on soil depth (~22 cells), which is worldgen's | not this lane's |
| **D4.3/D5.3** | **No CO₂ field, no ventilation.** The function is covered by D4.1 | says no |
| **D3.4** | **No fungus garden.** | says no |
| **D4.4** | **Leave who-digs alone**; vertical sorting comes free with D2.1 | says no |

---

## 10. Owner ruling, 2026-09-19 — the nest has no purpose, and that is prior to everything above

> *"Nests have no real purpose. As of now, we don't have a granary, ants
> don't lay eggs, we have no temperature or weather or fire in the evolution
> lab that can kill ants, and our predators are easily killed off by the ants
> without a nest."*

**He is right on all four, and the tree states each of them more strongly
than he did.** Checked rather than taken:

| the claim | what the tree says |
|---|---|
| no granary | **confirmed.** `lab/ui.rs`: *"food in transit and not food put by: THERE IS NO GRANARY IN THIS BOX."* `larder_probe mode=turnover` measured `resident` **0 from frame 200** |
| ants don't lay eggs | **confirmed, and stronger.** Reproduction is **budding**: `Origin::Bud` places a fully-formed adult in a clear adjacent cell funded by `TRAIT_BIRTH_GRANT` (`creature.rs:2995`). No egg, no larva, no queen, **no brood object of any kind** |
| nothing environmental kills | **confirmed, and this is the decisive one.** `DEATH_CAUSE_LIST` (`organism.rs:6545`) is `Unknown`, `Starved`, `StarvedInFlight`, `Killed`, `Culled`, `LostVitalTissue`, `FelledOrLost`, `OldAge` — **not one is environmental**, in *either* game rather than only the lab. No burned, frozen, drowned or desiccated cause exists. The lab additionally pins `Pin::Clear` (`lab/scene.rs:925`), so there is no weather even in principle |
| predators are no threat | **worse than "easily killed".** `beetles=0` against `beetles=9` measured **bit-identical over 6,000 frames** (`creature-evolution-plan.md` E7, §13o) — the predator never caught anything. E13 and `predation_probe` have worked the line since, so re-read that before quoting it as current, but the recorded null is a null |

### 10.1 What this does to §§2–5

**It removes their premise.** §5.4's synthesis — *a chamber is a
microclimate, and the nest is a stack of them* — is the correct answer to
*what is a nest for* in biology, and it names four purposes: **store food,
raise brood, buffer climate, hide from enemies.** The box has **none of the
four.** So D2.1 (a depth term) and D5.2 (put each thing where it belongs on
the vertical gradient) build the *machinery* of a nest for a function that
does not exist, and a graded profile in a world where nothing wants a
gradient is decoration.

This is `CLAUDE.md`'s *check that a planned step can demonstrate itself,
before promising it will*, arriving one level up from where that rule
usually fires: not *which cell does this rule evaluate*, but **what in this
world would be worse off if the nest were not there.** Today the answer is
nothing, and the question was not asked before the research was
commissioned — including by this report, which asked what a nest is *for*
in ants and never asked what it is for *here*.

**§§2–5 are not withdrawn.** They are the right material and the findings
in them stand as biology; §0's seven contradictions are all still true of
the code. What changes is the **order**: none of the decisions in §9 should
start until this section's question is answered, because every one of them
is a mechanism in service of a purpose.

### 10.2 The purpose that is already half-built

**Budding needs a clear adjacent cell, and the engine already counts the
failures.** Three facts that are in the tree today and, as far as this lane
can find, have never been read together:

1. `try_bud` places a child in one of the eight neighbours and **fails when
   none is clear**. Measured when that path was built: *"a colony of 12
   richly-funded ants, 60 frames, `births_denied_no_space` **104** and zero
   actual births"* — the parent's own body stood in the one direction every
   candidate was willing to grow toward.
2. `CreatureStats::births_denied_no_space` and `births_denied_animals` are
   live counters with a doc that says exactly how to read them: attempts
   alone cannot separate *"one ant walled in for a thousand ticks"* from
   *"a thousand ants each waiting a tick"*, and **those are opposite
   findings — the first is a bed that forecloses reproduction, the second
   is a queue.** Both are surfaced: `app.rs:2977` and `lab/stats.rs:906`
   print `BIRTHS REFUSED NO ROOM`.
3. **Soil is a `Powder`, so open ground fills in.** The only reliably clear
   space underground is *roofed* space — which is precisely what a chamber
   is, and precisely what `NestRoom::roofed` already censuses.

Put together: **a chamber is the space a colony needs in order to grow.**
That is a purpose which requires almost nothing built, and it is the most
legible one available — *dig, and more ants fit* is a verb with a graded,
visible consequence, which is both of the owner's laws in one mechanism. It
is also the only one of the four biological purposes that has a substrate
here already, and it arrived from the tree rather than from the literature.

**It is not free of doubt.** Real ants do not dig for floor space to stand
a worker on; they dig for brood, stores and climate. So this is an
*engine-native* purpose wearing a nest's shape rather than the biological
one, and it should be argued on whether it plays well, not on whether it is
faithful.

### 10.3 The measurement that decides it, which is one number

**Read `births_denied_no_space` and `births_denied_animals` on the played
bed, over a session.** Both already print; nothing needs building. Their
ratio is the mean wait in ticks, which read against a generation says
whether room is binding at all.

- **If room is binding** — births foreclosed rather than merely queued —
  then the nest already has a purpose nobody noticed, §9's decisions become
  worth taking in service of it, and D4.1's *local* room reading is the one
  that matters (an ant should dig where *it* cannot bud, which is a local
  question, exactly as §4.2 argued on different evidence).
- **If room is not binding** — the counter is small, or it is a queue that
  clears — then the nest has no purpose in this box and **the honest
  recommendation is to build a purpose before building a nest.** §10.4
  prices those.

`latecensus` is the harness that already runs the played bed to 500,000
frames and reports the colony and the footprint per 20,000; this is a column
on it, not a new instrument. **This measurement now sits ahead of Q1–Q4 in
§7**, and it is cheaper than all of them.

### 10.4 If a purpose has to be built, what each costs

Priced against what exists, worst first.

- **Environmental death — expensive, and it has a named trap.** Adding a
  cause means adding a row to `DEATH_CAUSE_LIST`, and `CLAUDE.md`'s own
  gotcha is that *adding a member to a set something sweeps enrols it in
  every rule over that set, silently* — the `spoil` material broke five
  censuses and registering one worldgen preset turned `main` red for two
  hours. It also needs a field consumer that does not exist (§5.5 D5.1) and,
  in the lab, something to unpin `Pin::Clear`. **This is the biologically
  right answer and the most expensive one.**
- **Brood — expensive, and it changes reproduction.** A brood object means
  budding stops being instantaneous: an egg, a place to put it, a clock, and
  a carrier. That is a rework of the one mechanism every evolution result in
  this repo is measured through, so it would void baselines across the whole
  creature line.
- **A granary — moderate, and it needs a lean season to matter.** §3.6 D3.1
  is cheap to build (a `Drop` preference plus a cull), but a store is only
  worth having if there is a time when foraging does not pay. The lab has no
  seasons, and the bed runs at ~1.03x subsistence continuously
  (`dead-ends.md`, the unit-7 odometer entry). **Build the store and the
  lean time together or neither.**
- **A real predator — moderate, and it is another lane's.** Refuge is worth
  nothing while the predator is a measured null. `predation_probe
  mode=range` was built for exactly the question that decides this — *does
  predation kill non-randomly with respect to how far an ant ranges from
  home*, i.e. is a predator a selective force or a flat tax. **If that
  reads as a force, refuge becomes the nest's purpose and it needs no new
  mechanism at all**, because ants already go home.
- **Room to grow — nearly free, and it may already be true.** §10.2. The
  measurement in §10.3 is the whole of the work required to find out.

### 10.5 → The decision this section makes

**D10.1 — Nothing in §9 starts until §10.3 is read.** One number, off an
existing harness, decides whether the nest work has a purpose to serve.

**D10.2 — If room is not binding, do not build the nest; build a purpose.**
Of the five in §10.4, the two worth putting to the owner are **a predator
that is actually a force** (`predation_probe mode=range` already asks it,
and refuge needs no new mechanism) and **a granary with a lean season**
(the store is cheap, the season is the real work). Environmental death is
the faithful answer and the dearest; brood would void the creature line's
baselines.

**D10.3 — Record that this question was not asked.** The brief commissioned
four areas of nest biology and this report answered them; neither asked what
a nest is for *in this game* until the owner did, after the work. The
transferable form, which is why it is written here rather than left in a
chat log: **before researching how to build a thing, ask what in this world
would be worse off without it.** A mechanism with no consumer is the
`phototropism_dir` failure with the weights removed — everything correct,
nothing downstream.
