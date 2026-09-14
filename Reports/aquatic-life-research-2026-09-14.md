# Four predicates say no to water, and everything under them is already built

*Research and brainstorm, 2026-09-14. **No engine code, no build order, no
prices, no briefs** — this is a map of the design space with the forks named
and, where one exists, the instrument that settles each. Written engine-shared
with the lab as the proving ground, at the owner's scoping. Numbers marked
**(measured)** were taken on this branch on 2026-09-14 at
`RAYON_NUM_THREADS=1`, release; numbers marked **OWED** have not been taken and
name the run that would take them.*

**Status: research. Nothing built, nothing decided.** One review card carries
the decision that is not a lane's to make: `20260914T030227009Z-0c5076` (does a
deep pool go dark — §2).

---

## 0. The answer

**Water in this engine is a floor.** Not a medium, not a habitat — a surface
things stand on. And the reason is four one-line predicates, each of which
refuses `MaterialKind::Liquid` for a locally good reason:

| | where | what it refuses |
|---|---|---|
| `creature::move_cost` | `creature.rs:923-931` | an animal entering water — the comment says *"not modelled as aquatic this milestone"* |
| `plant::growable` | `plant.rs:349` | a shoot or a root extending one cell into water |
| `plant::cell_carries_nutrient` | `plant.rs:10159` | water carrying any nutrient, so a water-rooted plant earns nothing |
| `field::rebuild_blocked` | `field.rs:2999` | water attenuating light, so a pool has no depth |

**Underneath those four, the hard parts are already built and correct.**
Buoyancy, quadratic drag and terminal velocity ship and are used every frame by
the flitter (`creature.rs:8940-9024`). The water cycle is genuinely ledgered.
The scent plane is material-blind, so a trail already crosses water. Water is
thermally live. A gas cell already displaces a liquid one. The density table in
`assets/materials/` already sorts the world into things that float and things
that sink — and it already, by accident, sorts the *ecology*.

**The trap is thinking this is four pieces of work.** Open any one alone and
you get a verb with nothing behind it. That is not a guess: the hop shipped
2026-08-29, correct and guarded, with the owner's own verdict on it, and
measured **0 launches in every real scene** for six weeks because nothing
authored it and nothing rewarded it. Across 24 random genomes, survival in
this bed correlates **+0.895 with how much an animal eats and with nothing
else** (`creature-behaviour-ceiling-2026-09-05.md`). A swim verb costs energy. If
there are no calories in the water, selection deletes the swimmer, and the only
thing that puts calories in water is plants — which is why the creature half
and the plant half are one piece of work.

**And half of it is already ruled on.** The owner answered the *drown, float or
swim* card on 2026-08-29 — all three, heritable, with the float limit
mechanical rather than genetic. That is **E9**
(`creature-evolution-plan.md:56`). It was deferred pending reproduction;
reproduction has landed. §4 shows that **float is already built too**, which
nobody has written down.

**If one thing is worth taking from this document it is §2**: there is no depth
in the water, and the mechanism that would give it depth is already in the tree
implementing exactly the right equation for a different reason.

---

## 1. What the water already does, measured today

### 1.1 The density table is already an ecology, and one code comment is wrong about it

Every creature's mass comes from its material's density, summed per cell
(`BodyDrag::mass`), and `buoyant_share(body, fluid) = (fluid/body).min(1.0)`
decides what the water carries. So the assets already decide who floats:

| material | kind | density | in water |
|---|---|---|---|
| `deadleaf` | Powder | 0.25 | floats high |
| `litter`, `snow` | Powder | 0.30 | floats |
| `seed`, `pip` | Powder | 0.60 | floats |
| `deadwood` | Powder | 0.70 | floats |
| `log` | Solid | 0.80 | floats (`rigid.rs` floats it outright) |
| `oil` | Liquid | 0.80 | floats **on water** |
| `ice` | Solid | 0.92 | floats |
| **`ant`, `longant`, `worm`, `flitter`, `hopper`** | Creature | **1.00** | exactly neutral |
| `water` | Liquid | 1.00 | — |
| `windfall` | Powder | 1.05 | sinks |
| **`beetle`** | Creature | **1.20** | **sinks** |
| **`corpse`** | Powder | **1.20** | **sinks** |

Read as ecology rather than as physics, that table says: **the prey hangs at
the surface, the predator sinks, and the dead sink.** Nobody authored that as
an aquatic fact — the beetle's 1.2 arrived with PR #313, about a seed riding
home — and it is a better start than anything a design would have invented.

**One live comment is stale and it is load-bearing.** `creature.rs:8943` still
reads *"Cell count when every cell is density 1.0, which every creature
material currently is."* The beetle is 1.2. This matters for E9 because it
means **the "mechanical, not genetic" float limit already has a per-species
dial that the assets already vary** — see §4.2.

### 1.2 What else is already there

- **Buoyancy, drag, terminal velocity** — `body_drag`, `surrounding_density`,
  `buoyant_share`, `terminal_speed` (`creature.rs:8940-9024`), shared in spirit
  with `rigid::drag_through_liquid`. `surrounding_density` already reads the
  8-neighbourhood for a `Liquid` and returns its density.
- **A ledgered water cycle.** `water_equivalents(w) + w.atmospheric_bank` is
  the invariant (`weather.rs:1924`); evaporation credits, rain debits.
- **`FLAG_FLOWING`** — a per-cell "this water is moving" bit, set generically
  by `CellSurface::move_cell` (`surface.rs:493`) and already rendered as foam.
  A current sense, for free. (`cell.rs:303`'s note that it is "meaningful only
  for `Powder`" is stale.)
- **The scent plane is material-blind.** `src/sim/pheromone.rs` contains
  **zero** `MaterialKind` references **(measured**, `grep -c`**)**. An aquatic
  animal's chemical sense needs no porting at all.
- **Water is thermally live** — `heat_conductivity: 0.08`, boils at 100°C,
  freezes at 0°C, and `water.ron`'s own comment calls that *"the single most
  expensive line in the file"*.
- **A gas cell displaces a liquid one.** `MaterialKind::is_displaceable`
  returns true for `Liquid | Gas` (`material.rs:219`), so a bubble should rise
  through a water column with no new mechanism. See §9.1.
- **Ponds already generate, including sunless ones.** `worldgen::passes::ponds`
  runs after `vaults` and deliberately fills cave mouths, so flooded caves are
  in the outdoor game today.
- **The gnome already swims, and the owner has already judged it.**
  `player.rs` carries a whole `WaterFeel` — buoyancy, `swim_damp`,
  `stroke_impulse`, `surface_hop`, `wade_slowdown` — four presets, splash and
  wake, and on record at `player.rs:903`: *"Water is off, with swimming, I
  didn't like the buoyancy."* This is the only aquatic thing in the repo that
  has actually been played, and every creature-swim proposal should be read
  against it.

---

## 2. Depth is the missing gradient — and the mechanism for it is already right

**Measured, 2026-09-14.** `filmstrip scene=waterbed channel=light`: a body of
water roughly 40 rows deep reads **uniform from surface to floor**. There is no
attenuation of any kind.

**The positive control passes**, which is what makes that a finding rather than
a dead overlay: `filmstrip scene=cavern channel=light`, same binary and
settings, renders the void inside rock **pitch black**. The instrument is
sensitive; the water genuinely passes light like air.

Both images, plus the plain render of the same pool (identical blue at the
floor and at the surface), are on review card
`20260914T030227009Z-0c5076`.

This is the defining gradient of aquatic life. It is why a lake has a
compensation depth, why anything below one lives differently, and why a plant
down there would have a reason to reach upward. Without it a pond is a flat
blue sheet with a uniform interior, and every aquatic design downstream is
building on a medium with no structure.

### 2.1 The engine already implements Beer–Lambert, for foliage

`FieldTile::transmission` (`field.rs:308-347`) counts **opaque-cell depth per
CA column** and reads transmission off `COLUMN_TRANSMISSION` (`field.rs:2315`),
a Beer–Lambert table normalised so a full `FIELD_SCALE`-deep column lands
exactly on `SKY_TRANSMISSION`. It exists because `blocked` was too coarse for a
canopy: one twig marked a whole 8×8 block opaque, **88% of leaf cells sat below
0.05 light**, and both shade-driven abscission and phototropism were stuck
behind it.

Beer–Lambert is literally the law that governs light in water. So the proposal
here is not *build depth attenuation*. It is: **the right equation is already
in the tree, for the right reason, and water is not in the predicate.**

### 2.2 Two things that travel with it

- **It needs per-material extinction to be honest.** A cell of water must not
  attenuate like a cell of wood, and `column_depth` is a plain `u8` count today.
  `dead-ends.md:224` already names the fix site for the parallel case (*"buried
  plants die of darkness" is impossible under the current light model*, because
  a `Powder` passes the lamp whole): **opacity as a material property**.
- **It must be `transmission`, never `blocked` — and this is the trap.** A
  blocked block is skipped entirely by `step_diffusion`. That is the measured
  cause of the bug recorded at `field.rs:815`: **96.8% of grass cells read
  field moisture exactly 0.000 at every wetness level from wilting point to
  saturated**, because the presence of fuel in a block is what makes the block
  read bone dry. **Water is the moisture source.** Marking a pond `blocked`
  would make the pond read as having no water in it.

### 2.3 What it costs, honestly

The field is `FIELD_SCALE = 16` CA cells per field cell, so depth would be
coarse — a 16-row band, not a smooth ramp. Whether that is enough resolution
for a pond to read as layered is a judge-by-eye question and belongs on a card,
not in this paragraph. **OWED**: `skylight_cost` and `field_cost` for the bill;
`sky_light_probe` already sweeps the five geometries that decide sky visibility
and a deep pool would be a sixth.

---

## 3. Still water and moving water are two habitats, and one bit already tells them apart

Lentic versus lotic — standing versus flowing — is freshwater ecology's master
axis, and `FLAG_FLOWING` is one already-set bit away from it. It buys three
things at once:

1. **A direction that is not gravity.** This is exactly what the rejected
   rotation problem lacked (§4.4): a swimmer in a current has something to
   orient against that is not "up".
2. **Passive transport.** A thing that floats currently floats *in place*. Seed
   density 0.6 means water dispersal is already half-expressed and goes
   nowhere.
3. **Scent downstream.** Advecting the pheromone plane by `FLAG_FLOWING` is the
   single most characteristic aquatic sense, and it is a change to a plane that
   already exists rather than a new field. It is also the honest re-test case
   for `PheroALateral`/`PheroBLateral` — see §4.5.

**The standing objection, which belongs in the same paragraph as the
proposal:** `CLAUDE.md` records *stopping work early is a legitimate
optimisation*, and *"a pool that is visually flat but still shuffling fill for
another quarter of an hour is a real cost buying nothing."* Any mechanism that
**increases** liquid activity to model a current is arguing against a stated
owner value and has to say so out loud.

---

## 4. A body in water: E9 is a third built, and the two halves want different homes

### 4.1 Float already exists, with a counter and an A/B switch

`creature.rs:9971`, closing §Z9:

> **Weightless and not flying is standing on water, not flying over it.**

It sets `landed_afloat`, has a counter in `creature_stats`, and ships with an
env switch — `LAND_AFLOAT=0` puts the defect back. It was built because a
flitter that came down on a pond hung there being charged the airborne rate
until it died: **57–87% of every flitter and hopper death on the played bed.**

So of E9's three outcomes, **float is done**. Drown and swim are what is
missing, and they arrive with an existing control arm — which is exactly what
this repo wants before touching a model.

### 4.2 The mechanical limit and the heritable trait want different homes

E9 says the float limit is **physical, not genetic**. `creature.rs:9017` already
claims the shipped buoyancy satisfies it: a body no denser than what it is in
has `g_eff == 0` and hangs, *"and it cannot be evolved around, because the only
way to get it is to be made of something light."*

That is true, and §1.1 shows the dial is **material density**, which the assets
already vary (ant 1.0, beetle 1.2). So there is a real fork, and this document
names it rather than resolving it:

- **density-as-material** — authored, one number per species, not evolvable.
  Satisfies "mechanical not genetic" exactly as written, and already works.
- **density-as-trait** — slot 14 of `CREATURE_TRAITS` (14 used of a 64-slot
  reserve; appending is lawful and guarded). Evolvable, which is the other half
  of what E9 asked for — but then "mechanical, not genetic" has to mean
  something else, most likely *"body size sets an upper bound the gene cannot
  cross"*.

A third reading is that these are **two traits**, not one: drown *tolerance*
(how long submerged before the clock runs out) and swim *drive* (whether the
animal tries). Each costs a slot, and the positional genome law means neither
can ever be removed. Worth deciding before spending one.

### 4.3 Drowning must be a clock, not a predicate

`CLAUDE.md` law 1: an outcome is a distribution, not a binary. Drowning that is
either instant or never has the rubble defect exactly. A consecutive-frames
counter is the shape, and `flight.stall` is a working pattern for one already
in this file.

**And drowning is worth more than it looks.** `creature-evolution-plan.md` §2.5
measures it as the **only mechanism yet proposed that puts carrion in the
world**, which is why the survival-versus-`gut_bias` curve has one hump instead
of two. The aquatic layer is the carnivore enabler.

### 4.4 What a swimming body actually *does* is genuinely open

Three motion models exist and a swimmer is none of them: the walk is discrete
cell-stepping with no velocity (`step_chain`); flight is a velocity integrator
with substeps (`step_flight`); the worm has its own burrow economics. A swim
mode has to pick one or be a fourth.

**The orientation trap.** `dead-ends.md:1106` records decision D1: rotating a
rigid body through the grid was rejected for aliasing, self-overlap and cells
appearing from nowhere, and the entry's own re-test clause says
**arbitrary-orientation creatures (swimming, flying) would reopen the hard
problem**. The cheap answer — and the right one for a side-view pixel game — is
that a swimmer keeps the canonical up and mirrors like everything else. A fish
that stays upright is not a compromise here; it is what the view wants.

**The predicate trap**, from the flitter, 2026-09-12: *a buoyancy test written
against zero can never fire, because air has a density too.* Any swim predicate
of the form "is this body being held up" inherits that bug directly. Its
sibling lesson is sharper — *a rule that needs a quantity it cannot see is in
the wrong function* — so the swim decision has to live where `carried` and
`surrounding_density` are in scope, beside `step_flight`, not inside a brain
tick.

**The guard trap.** `a_weightless_body_is_put_down_on_water_unless_it_is_flying`
is a guard whose *name is the design decision*. A swim verb adds a third state
and must make that guard **fail for the replacement**; `CLAUDE.md` warns
specifically that a superseded mechanism's tests keep passing while testing
nothing.

### 4.5 Two dead brain inputs whose named re-test case is a swimmer

`PheroALateral` and `PheroBLateral` are wired to nothing. `dead-ends.md:1068`
records why: the Jones/Physarum lateral sensor pair does not work for a
surface-walking creature in a side-view world, because both sensors land in
open air — measured at exactly 0.000 over a cell holding A=27. The entry's
re-test clause reads:

> *Correct for anything moving in open space (a flier, a swimmer) — rewire and
> re-test the laterals then.*

A swimmer in a medium has a genuine lateral gradient. The slots exist, cannot
be removed, and are currently dead weight.

---

## 5. Plants: the waterline turns out to set the height, and the cheapest habit needs no new growth at all

### 5.1 What breaks, in order

1. **`growable` (`plant.rs:349`)** — a shoot requires `material == EMPTY`; a
   root requires `EMPTY`-touching-substrate or a penetrable `Powder`. `Liquid`
   is refused on both paths. **A submerged plant cannot extend one cell.** One
   function, wide blast radius: every growth site funnels through it, and "can
   I advance" and "is this air" are currently the same question.
2. **`cell_carries_nutrient` (`plant.rs:10159`)** = `Powder && water_capacity > 0`,
   feeding a Michaelis–Menten `nutrient_availability` that is **structurally
   zero at zero** — deliberately, so that *"no soil at all is fatal"* is
   structural rather than a tuning outcome. `wiki/plants.md:224` states it as a
   headline rule: *"standing water is a drink and nothing more."*
3. **`is_structural_anchor` (`plant.rs:7723`)** — `Liquid` is never ground, and
   non-root tissue anchors only on ground *directly beneath it* (`dy == 1`).
   Plant cells never move under the CA, so a submerged plant is a rigid
   terrestrial cantilever with water as decoration: a frond would have to hold
   itself up.
4. **Seeds float and are deliberately stopped from germinating on water.**
   `plant.rs:6198`'s guard exists because seed density is 0.6 and `resting`
   accepts any non-empty cell — *"without this a seed bobbing on a full pond
   would read bone dry"*.

### 5.2 The height ceiling is already a hydrostatic model, and that is the finding

`plant.rs:4905` — turgor at the apex is the collar's turgor **less the
gravitational cost of lifting water to this row**, and a cell wall extends only
while that exceeds the yield threshold. It is Lockhart's equation, it bounds
hydraulic *path length* rather than height (so it bounds width too), and it is
the only gate in the plant system built from geometry rather than resource
state — deliberately, because every resource signal equalises when growth
stops.

**A submerged plant does not pay that cost. That is exactly why kelp reaches
lengths no stem could hold up in air.** The physical story the engine already
tells is the aquatic story, unmodified.

The obvious move is `turgor_per_cell: 0.0`, which the code explicitly supports
(*"a real species value meaning this plant has no height limit — a moss mat, a
vine"*). **It does not survive the guard.**
`every_growing_species_has_a_height_ceiling_to_be_charged_against`
(`plant.rs:15075`) exempts moss only because moss has no `Grow` at all; any
species that grows a shoot must have a ceiling that is positive **and** finite,
so that the price actually binds.

**Resolving that tension is this section's proposal: charge lift only for the
part of the column above the waterline.** An emergent reed pays for the stem in
air and nothing for the stem in water. A submerged frond pays nothing until it
reaches the surface, and starts paying above it. The ceiling stays finite and
bindable — water depth plus whatever the plant can lift in air — so the guard
is satisfied rather than weakened.

What that buys: **how deep the water is decides how tall the plant can be.**
One lever, physically correct, expressed in a mechanism that already exists and
is already priced — and it makes §2's depth and plant height the *same*
gradient rather than two features.

**The caveat that must travel with it**, because it is the failure mode: the
ceiling is what bounds size, and relaxing it removes a bound without supplying
a replacement. A fully submerged plant under a naive version grows without
limit; the first kelp is 300 cells tall. The waterline rule above is the
replacement bound, and whether it actually binds is **OWED**, not assumed.

### 5.3 Five habits, ordered by what they cost — and the cheapest is not the obvious one

| habit | needs | note |
|---|---|---|
| **floating mat** (duckweed, pleuston) | **none of the four predicates** | `deadleaf` 0.25 and `litter` 0.3 already float; `rigid.rs` already floats a log outright. A mat grows *on* a raft, in air, on top of the water. The cheapest aquatic anything in the whole space, and it doubles as a walkable bridge (§9.4) |
| **emergent at the waterline** (reed, lily) | roots in sediment; shoot tolerates a wet column | The case the §5.2 rule is *for*. Reuses the most, breaks the least |
| **rooted submerged** (pondweed) | `growable` **and** nutrient **and** the height rule | The expensive one — all three plant predicates |
| **anchored holdfast, buoyant frond** (kelp) | the anchor rule to stop demanding `dy == 1` | Wants water to push back; §9.6 |
| **free-floating algal** | a water-column nutrient concept | The one genuinely new scalar in the area, and §11.1 says where that lands |

**The ordering is the point.** A floating mat is reachable with zero changes to
the growth model, and the design instinct — "make a plant that lives
underwater" — goes straight to the most expensive row.

### 5.4 The appearance lesson, applied before rather than after

`plant-appearance-design.md` §5: *a lever that changes which cell gets a label
cannot change a silhouette that is set by texture and colour.* Three
architectural levers were built, all three demonstrably fired with counters
beside the sheets, and the owner's reading was that nothing had changed —
because every plant was ~90% wood, ~5% leaf, and drew from one four-brown
palette and one four-green one.

**So a submerged plant drawn from the existing `leaf` and `wood` materials will
read as a tree standing in water, and no growth-rule work will fix that.** The
levers that move a silhouette here are material and palette band first, foliage
share second, branch angle third. The plumbing exists — a species already
declares its own `shoot_material` / `leaf_material` — and `plant-appearance-
design.md` §3's rule is satisfied honestly, because a frond's *physics* genuinely
differ: it is buoyant and does not hold itself up.

---

## 6. Where the calories are, and the first income that is not a function of travel

The selection objection, stated properly.

`food_height` was built for exactly this question about flight and answers it:
**95.8% of all food worth sits 5+ cells above ground on `wetland`, 87.6% on
`rolling`** — the calories are in the canopy, 1–2% at ground level. That is the
niche the flyer got.

**Water has no equivalent, because nothing grows in it.** Every one of the 20
shipped species is terrestrial; `growable` refuses `Liquid`; so the calories in
a pond today are exactly whatever falls in and drowns. A swimmer authored now
would be the hop again: a correct mechanism with 0 uses.

**OWED, and it is the measurement that decides whether any of this is worth
building**: `food_height` with a below-the-waterline bin (its `ground_row`
skips `Liquid` and would silently score food in a water column as food in air —
an addition to an existing harness, not a new one), plus `labforage`, whose
stated question is already *"gone, or never got to it"*, on a bed with and
without a pond. **If the delta is ≈0, the aquatic niche is decoration and this
document's recommendation changes.**

> **The `labforage` half was taken 2026-09-14 — see the implementation plan's
> §1.9 — and it splits the question in two.** Three arms, six seeds, each the
> one above it with one thing deleted. Columns the colony ever reached:
> **flat 279.5, dry pit 196.5, water pit 199.5.** So the delta between pond and
> no-pond is emphatically *not* ≈0 — the pit costs 29% of the colony's range on
> 6 of 6 seeds with the distributions completely separated — while the delta
> attributable to **the water** is: three seeds up, three down, with
> `unvisited` and `eats` equally flat. **The hole is the spatial structure; the
> water is scenery — in a basin nothing can enter.** *(Corrected the same day:
> all three arms were vertical-walled, which stops an ant at the rim, so the
> water was never reachable. On a sloped bank the identical water costs 49.5
> columns of range, 6 of 6 seeds separated — the implementation plan's §1.11.
> Water is a barrier; four instruments had been measuring a wall.)*
>
> **So this paragraph's stop condition does not fire, for a reason worth
> stating**: the recommendation never rested on water-as-barrier, it rests on
> there being calories in the water, and the `food_height` half above is
> answerable today without running it — `growable` refuses `Liquid`, so
> submerged food is **zero by construction**. A pond in this engine right now
> neither blocks a forager nor holds anything to eat. That is the hop's exact
> situation, and it is the whole argument for plants before the swimmer.

**Detritus is the interesting answer**, and it is a different trophic mode
rather than a relocated one. A pond floor accumulates what falls into it —
`windfall` 1.05 sinks, `corpse` 1.2 sinks, litter rafts and eventually
waterlogs. Feeding on drift scores **position held** rather than distance
covered, which is the first income channel in this engine that is not a
function of locomotion. That matters because `colony-economy-design-2026-09-09.md`
measures locomotion at **5,599 J of the colony's 10,796 J burn -- 52%**: an
animal that eats by staying put is a genuinely different economy, not a
re-skinned forager.

---

## 7. The pond as an object: what it costs, and what it breaks

### 7.1 The lab cannot make a pond by raining into it — measured

`waterstand scenario=played_bed frames=40000`, `RAYON_NUM_THREADS=1`, one seed,
paired arms from one binary **(measured)**:

| at frame 40,000 | rain OFF | rain STEADY |
|---|---|---|
| standing water above the soil line | **103 cells** | **1,444 cells** |
| of which resting on plant tissue | 70 | 436 |
| of which resting on more water | 7 | 829 |
| saturated soil cells | 6,822 | 18,295 |
| median tick | **1.505 ms** | **2.309 ms** |

**1,444 cells across 512 columns is not a pond, it is a scatter of films.** The
per-row census reads `4:6 20:1 21:1 22:1 36:1 43:7 …` — one to seven cells in a
row. Most of it is held in canopies and on top of other water rather than
pooled on the floor. STEADY is already double the shipped default (`Light`).

**So: if the lab is to be the proving ground, a basin has to be placed rather
than rained for.** Rain will not do it, and this is the single most actionable
thing in the document.

> **OVERTURNED THE SAME DAY, and the correction is cheaper than the claim.**
> This paragraph said a basin "is a `LabBox` field beside `soil_depth`". **It
> is not — it is a scenario file.** `Placement::Fill` already accepts
> `"water"` and scenarios load from disk at runtime, so
> `assets/lab_scenarios/the_pond.ron` holds 4,256 water cells flat from frame
> 0 to 40,000 with **no engine change at all**. The one real constraint is one
> this section did not see: the basin needs an **impermeable floor**, because
> the bed's own soil column has almost exactly enough unsaturated room to
> swallow the pond. See
> [`aquatic-implementation-plan-2026-09-14.md`](aquatic-implementation-plan-2026-09-14.md)
> §1.1 and §1.6.

**Two honesty notes on those numbers.** The animal count moved 45 → 8 and
plants 799 → 619 between the arms, which looks like "rain costs the colony" and
is **one seed, one run per arm** — outcomes here have enormous spread, so that
is a signal wanting `labbatch`, not a finding. And the timing arms were
sequential rather than alternating: the medians order cleanly but **p90 inverts**
(4.570 dry against 3.498 wet), which is the machine-noise tell `CLAUDE.md`
names. Quote the median, and re-measure paired before gating anything on it.

### 7.2 What a pond breaks, already on the books

- **§1f — a pond with rock in it never stops shuffling fill.** 4–5 of 40 chunks
  awake at frame 12,000 with identical material counts and water totals over
  five frames. **Any aquatic creature is an object in a pond**, so this is hit
  on frame one, not eventually.
- **§4 — levelling is O(width²).** A 1024-wide pool takes ~70,000 frames to
  reach 1 cell of tilt and sleep.
- **§2 — sand-into-water striping**, unresolved, and visible around any dug or
  disturbed aquatic bed.
- **§5/§6 — there are two liquid models and only one runs.** `liquid.rs`'s
  heightfield bodies are called only from tests, so ~1000 lines never execute in
  play and every bug in them is latent; and it measured *slower* than the CA at
  every width up to 400 columns. A large standing pond is exactly what they were
  built for. Aquatic work forces that fork into the open; this document names it
  and does not choose.
- **§T2's root cause was standing water, and the shipped fix is untested
  against a real body of it.** The colony foraged and brought nothing home
  because `paint_nest_patch` converts ~53 surface columns to impermeable
  `nest`, so the doorstep was the one impermeable strip on a misted bed: seed
  1, **47-49 of 53 nest cells under water**, free liquid over the patch
  **89-91 against 17-19 over the same width of ground beside it**. What shipped
  is drains -- every third column left as ordinary ground. A basin in the bed
  is the first thing that would test whether drains are enough.

### 7.3 The appearance floor comes before the ecology

`pond_min_depth`'s own doc: ponds shallower than 2 cells are **not generated**,
because *"a one-cell film of water renders as a black line rather than as water
(`render.rs` dims liquid toward black by fill) and reads as an artifact."*

Every design in this document that produces thin sheets — a flooded bed, a
marsh, a drawdown zone, a tide — **will look broken before it is wrong**. That
is a rendering question (`fill_dimming` is 0.65) and it precedes the ecology.

---

## 8. The waterline is a band, not a line — and it moves

In a side-view world the water surface is the one place two media meet and a
body can be in both at once. That, rather than "underwater", is the aquatic
content specific to *this* game:

- **Emergent plants span it** — roots in sediment, stem through water, leaves
  in air. §5.2's rule makes the waterline the thing that sets their height.
- **Amphibious animals cross it**, in both directions, and the crossing is a
  state change rather than a place.
- **It migrates.** Evaporation shrinks a body, rain refills it, `table_offset`
  sets the table, and — in the sealed box — `weather::condense_under_a_lid`
  makes water without any sky. A plant zoned to the waterline lives in a
  drawdown zone, which is the aquatic form of *an outcome is a distribution*.
- **Ice closes it for a season.** Ice is the only floating solid, it bears
  weight, snow lands on it, and a cold night already freezes standing water into
  a crust that thickens downward while the water underneath stays water. A pond
  with a lid is a habitat cut off from the air — a whole seasonal dynamic out of
  three mechanisms that all already ship.
- **A wet canopy does not shelter the water under it.** `water.ron` has
  `falls_through_organisms: true`, so rain drips through a crown at a rate. An
  emergent marsh canopy therefore interacts with evaporation's `shelter` term in
  a way nobody has looked at.

---

## 9. Six areas none of the four scope headings names

### 9.1 Bubbles are the oxygen model that `dead-ends.md` declined

`MaterialKind::is_displaceable` is `Liquid | Gas` **(measured**, `material.rs:219`**)**,
and `update_gas` rises by `try_move` into the cell above. So a gas cell inside a
water column should rise through it with no new mechanism.

That buys respiration, anoxia and photosynthesis-as-visible-output **without a
per-cell dissolved scalar** — which dodges §11.1's trap entirely. It is also the
most legible possible "the thing under the water is alive" signal, which is what
law 2 asks for. **OWED**: one `filmstrip` scene settles whether a gas cell
actually rises through a full liquid column.

`dead-ends.md:737` already reasoned about oxygen and declined to model it, in
soil: *"oxygen is the scarce resource in waterlogged soil, not water — a root
drinking less has no physical story."* A bubble has a physical story.

### 9.2 A stratified liquid column already exists, and nobody has used it

`oil` is a `Liquid` at density 0.8 that does not evaporate. **Water with a
slick on it is a two-layer column today** — a completely different route to a
depth gradient than a per-cell scalar. A slick is a light lid, a suffocation
layer, a fire-on-water event and a player verb at once, and it establishes the
precedent for any second liquid a design might want.

### 9.3 The sunless pond already ships, and needs no light model

`worldgen::passes::ponds` runs after `vaults` and deliberately fills cave mouths
and hollows. Flooded caves are in the outdoor game right now. So the most
distinctive aquatic trophic mode — **no primary production at all, everything
imported as detritus** — is reachable without touching `field.rs`, because the
habitat is already dark.

### 9.4 Floating things are topology, not decoration

An ice sheet, a litter mat, a log raft and a bank of emergent stems are all
**bridges**. Adding water to a bed does not only subtract walkable area; it adds
a *time-varying reachability graph*. `labforage` measures exactly this
("gone, or never got to it"), and §5.3's floating mat is the cheapest way to
produce one.

### 9.5 Scent downstream

Covered in §3, repeated here because it is the axis most likely to be missed: the
pheromone plane is material-blind and already crosses water, and advecting it by
`FLAG_FLOWING` is the characteristic aquatic sense. It is also what would make
the dead lateral inputs (§4.5) worth rewiring.

### 9.6 Scale: the same pond is three habitats

A 40-deep pond is twenty body-lengths to a two-cell ant, a swimming pool to the
gnome, and a puddle on the druid's table. `creature_scale` and `lab_resolution`
both ask this question in other contexts; nobody has asked it about water, and
it decides whether "depth gradient" means anything at the scale the animals
actually live at. **This may be the first thing to settle**, because it can make
§2 either essential or irrelevant.

---

## 10. Where the three games diverge

**Outdoor (`outdoor`) — water is generated habitat.** Ponds, the water table,
aridity and flooded caves all already exist and are parameterised. The open
questions are worldgen questions: what a generated pond *contains* at generation
time, whether a pond belongs to a biome, and whether an off-camera pond lives —
which collides directly with `ecological-lod-design.md` and
`population-dynamics-research.md` §7a, *chunk sleeping is an accidental
ecological mechanism*.

**Lab (`lab`) — water is a line in a bed spec, and the only place selection is
measurable.** §7.1 is the finding: there is no basin, and rain will not make
one. **A basin is a scenario file, not a bed field** — see §7.1's own
correction; four of them ship with the implementation plan. The lab is the proving ground because it
is the only game with `labstats`, `chronicle`, `census.rs` and `creature_arena`
with a control arm. It is also where a pond costs the most proportionally: the
bed *is* the world at 512×320.

**Druid (`held`) — water is a verb the owner has already ruled in.**
`held-world-game-concept-2026-09-13.md` lists the druid's hands as *"alarm,
fling, lamp, cull, feed, **water**, place-a-plant, found-a-colony"*. The player
half is scheduled, not speculative. It is also the only game whose protagonist
already swims, with four tuned feels and a recorded complaint about the
buoyancy — so the druid is where law 2 (*the verb must deliver something*) bites
hardest and where the most prior art sits.

**Genuinely shared engine**: the four refusal predicates; the single buoyancy
model; the material density table; `FLAG_FLOWING`; the pheromone plane; the
thermal path; the water ledger; `land_afloat`. **Not shared**: the stocking
decision, the player verb, the scale — and the constants, because
`dead-ends.md:1249` already measured that **no single water rate satisfies both
games** (`VAPOUR_PER_CELL_EQUIVALENT`, 2026-08-31). Any pond constant should be
per-game from the first line rather than unified and later split.

---

## 11. The traps this design would walk into

### 11.1 `Cell::aux` is a tagged union whose tag is material data

The `nest.ron` entry, 2026-09-12: giving a material a data field while it is a
different `kind` put two systems on one field, and **every counter read as a
clean win**. Its general form — *before giving a material a data field, check
which cell-kind readings of `aux` that material already has, because the
compiler cannot see a tagged union whose tag is material data* — **kills any
per-cell dissolved oxygen, salinity or water-column nutrient.** Note that
`aux == 0` already means *full* on a `Liquid` and *dry* on a `Powder`.

The escape routes, in order of cost: a bubble (§9.1, no scalar at all); a
`Chunk`-side lazily-allocated plane, the shape `Chunk::nutrient_deficit`
already uses; a new field channel, which is 16×16 and coarse.

### 11.2 Volume, never cell count

`evaporation_tests.rs`, via `dead-ends.md:140`: six full cells thinning into a
film reads as *more* water by cell count. **Half the natural aquatic metrics —
"how much water is in this pond" — walk straight into this.** The topmost-cell
trap is the same shape: it said chunk seams were 1.7× the interior roughness
where volume said 9×.

### 11.3 One body per world

`dead-ends.md:1275`: the first paired puddle-versus-lake scene put both in one
world, and the lake humidified the puddle. **Any A/B of two ponds needs two
worlds.**

### 11.4 A mechanism with no counter is not finished

`creature_stats.landed_afloat` is the existing model. Anything here that fires —
a drowning, a swim stroke, a bubble, a germination on a raft — needs a counter
printed beside the picture, because a collapse once read as "chunks are working"
from a sheet whose body count was zero for the whole run.

### 11.5 The stale-binary family

`cargo build --release` does not rebuild examples; an unknown argument is
silently ignored (though `filmstrip` and `waterstand` both panic on one, and
both echo their parameters — good citizens). A water sweep is exactly the
long-run shape that has produced whole invalid studies here.

### 11.6 And two more that bind

- **A material's `kind` is an affordance table for every system at once.** Grep
  `MaterialKind::` for the kind you are leaving before leaving it. This fires on
  any proposal to change water's kind, or to make a plant cell that is `Liquid`.
- **`seedsweep.sh`'s default frame budget misses the water-cycle control**, and
  the `cells lost` column rides the water cycle at about ±1,700 cells — larger
  than most damage figures in the sweep. Any aquatic census taken at a single
  frame is that frame's phase plus the signal, and the two are not separable.

---

## 12. The forks, and the cheapest thing that settles each

This table is deliberately where a build order would go.

| fork | what settles it | what each answer implies |
|---|---|---|
| **Is there anything in water a walker cannot reach?** | `food_height` + a below-waterline bin; `labforage` with and without a pond | ≈0 ⇒ the aquatic niche is decoration, and §0 changes |
| **Does depth matter at animal scale?** | `creature_scale` / `lab_resolution`, pointed at a water column | If a pond is two body-lengths deep, §2 is moot |
| **How deep is the deepest pond that ships?** | `world_look` over a seed sweep; `cave_probe` for the flooded ones | `pond_min_depth` is a *floor* of 2.0, not a cap — the max is unmeasured |
| **What does a pond cost per frame?** | `labbox_cost` (says which system pays) + `frame_profile`; arms empty / pond / pond-with-a-rock (§1f) / **frozen** | The freeze arm is free and tests whether a lid makes the cost vanish |
| **Is the thermal opt-in the pond's real bill?** | `frame_profile` by phase, against `water.ron`'s own claim | Decides whether a thermocline is free or is the whole cost |
| **Does a gas cell rise through water?** | one `filmstrip` scene | Settles bubbles-as-oxygen before any scalar design is drawn |
| **Does litter float and stay?** | `litter_probe` — read its `SUSPENDED (air underneath)` line | Settles the floating mat (§5.3) at zero cost |
| **Does a burrow under a pond flood?** | `burrow_probe` with the table raised | Settles whether a benthic burrow is a niche or a bug |
| **Does the bed still punish a worse animal with water in it?** | `creature_arena`, incl. `arm=lethal` — it built its `LabBox` from flags and had no `scenario=`, so it could not be pointed at a pond at all (checked 2026-09-14). **Built the same day**, along with the late-founding arm assignment a timeline bed needs; see the implementation plan's §6 | The direct form of the selection objection |
| **Would slot 14 ever move?** | `genome_drift` | E9 claims the trait is evolvable; this is what would say it evolved |
| **Does a submerged plant read as a plant?** | `creature_look`'s `ink`, plus a review card | The appearance trap's own instrument |
| **Is any of it seed-stable?** | `labbatch` | §7.1's arms are one seed each and say so |

Two would be **new** harnesses and are flagged as such rather than smuggled in:
a *drift census* (does a floating powder advect on flowing water or sit still),
and a *basin depth sweep*, which is `labsoil`'s exact shape pointed at water
instead of soil.

---

## 13. What would overturn this

- **If `food_height`'s submerged bin comes back near zero and stays near zero
  even with aquatic plants in the bed**, the fish niche is decoration and §0's
  framing is wrong.
- **If the animal-scale census says a pond is two or three body-lengths deep**,
  §2 is a beautiful finding about a gradient nothing can perceive, and the
  cheap habits in §5.3 are the whole of the design space.
- **If the pond cost arms show §1f dominating**, none of this is affordable
  until that bug is closed, and the document becomes an argument for closing it.
- **If the owner's answer on card `20260914T030227009Z-0c5076` is that a dark
  pool is not wanted**, §2 goes, and with it §5.2's coupling of depth to plant
  height — leaving temperature (§1.2), stratification (§9.2) and current (§3) as
  the remaining gradients, all of which are cheaper.
