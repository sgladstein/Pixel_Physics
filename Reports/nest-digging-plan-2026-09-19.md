# Building a nest that is not a lens — the plan, and what a night of measurement pays for

*2026-09-19. Written by the session that measured `examples/digbox` into
existence. **Nothing here is built.** The switches it names all exist,
default-off and bit-exact; the stages are not started.*

Companion to `nest-biology-2026-09-19.md` (PR #468, merged),
`nest-biology-digging-signals-2026-09-19.md` and `nest-build-plan-2026-09-19.md`
(PR #472, open), and `digbox-eight-days-ago-2026-09-19.md`.

---

## Status, 2026-09-19 (later the same day): §2's Stage 1 is refuted, and Stage 4 should be first

**Read [`nest-shape-three-negatives-2026-09-19.md`](nest-shape-three-negatives-2026-09-19.md)
before acting on the stages below.** Three of this plan's levers were built
or refuted that night and the ordering did not survive:

- **Stage 1 as written — "a bias on the dig target at the call site" — is
  refuted, from the code rather than by a sweep.** The dig target and the
  step target are the same cell: the dig reads `DIRS[heading]`
  (`creature.rs:8716`) and `step_chain` chooses among
  `[heading+AHEAD_LEFT, heading, heading+AHEAD_RIGHT]` (`creature.rs:9606`),
  with the middle candidate being the cell just dug and `passable` requiring
  it to be empty. **The dig is what licenses the next step**, so an override
  severs the coupling that makes a tunnel at all.
- **Its replacement — steering the heading by `MoistureLateral` /
  `MoistureFront` — is also negative**, measured over five arms on a bed
  where the channel has its full range. The control was the best of the
  five.
- **The door's reach is not the nest's shape either.** A new
  `PIXEL_PHYSICS_NEST_SITE_COLS` narrows where ants *stand* and not where
  they dig; the narrowest door gives the *widest* room.

**What stands, and is stronger than when this was written:** §1's structural
fact (*"interventions on whether cannot produce a shape"*) predicted the
dig-target failure before the record was re-read, and **Stage 4 — fresh
spoil attracts digging — is the only candidate left.** It is Toffin's
self-amplification in physical form, it needs no pheromone (which is as
well: a dig-face pheromone is measured and negative), and the measurement
that points at it is that with `(Bias, Dig)` swept 0.15 → −1.0 everything
scales together and nothing concentrates. **It should be first, not fourth.**

**And Toffin is now in the repo with his constants, 2026-09-20.**
[`nest-excavation-mechanics-2026-09-19.md`](nest-excavation-mechanics-2026-09-19.md)
§2 gives the self-amplification rule this stage names in the form the paper
published it — `x²/(x²+K²) + ξ` over the marker in a cell's eight neighbours,
K = 50, ξ = 0.001, 100 units a dig, ν = 0.05 per step — plus §4's rate law and
the §13 tables to score an arm against. Two cautions carry over from that
file's header. Its marker is a **decaying scalar on dug cells, not a
pheromone**, which is the distinction this paragraph already relies on; and
its §3/§12 rules for *where a pellet lands* are not adoptable here, because
both hand-written placement rules were built, measured and withdrawn and the
owner ruled the drop is the ant's, on `drop_urge`. Take §2's shape and the
constants as a starting point to sweep from, not as values to install: they
are a 0.07 mm² cell and a four-cell ant, and this engine's cell is 2–5 mm.

**And Stage 0 is done**: the ant-blind census is fixed in `lab::census` and
`examples/burrow_probe`, with a guard proven red both ways.

**One warning about scoring anything here.** `digbox`'s `trace` line
*"spread over N columns × M rows"* is the spread of **at-nest ants** and
follows the reach dial by construction — it read "6 columns × 21 rows" and
was nearly reported as a shaft. Score shape on `SUMMARY`'s `room WxH vert`.

---

## 0. The finding the plan turns on

**Chambers in the biology are density-dependent digging *around a thing*. This
engine has density-dependent digging and no thing.**

That sentence is the whole plan. It was reached from the wrong end: three
separate interventions on the `Crowding` → `Dig` gate — making the reading
local, re-centring the gain, sweeping `ROOM_TARGET` — each achieved its own
stated precondition and moved the nest **not at all**. The natural reading was
that the mechanism is dead. The paper says otherwise.

According to PubMed, Römer & Roces 2014
([DOI](https://doi.org/10.1371/journal.pone.0097872), *PLOS ONE*), read in
full rather than as a search summary:

> we propose a **density-dependent mechanism** for the emergence of nest
> chambers through a self-organized process, with relocated brood and fungus
> acting as **cues that elicit worker aggregation at their deposition sites,
> indirectly influencing the intensity of digging activity**

and

> workers of [*Acromyrmex lundi*] with neither brood nor fungus excavated
> **only tunnels, but not chambers**. Chambers were excavated as soon as the
> ants were allowed to relocate symbiotic fungus inside a digging arena, and
> **digging activity concentrated around the deposited fungus**.

So contents do **not** trigger widening directly. Contents are an *aggregation
cue*; the aggregation raises local density; density does the widening. The
chain has three links and this engine is missing the first.

**Two consequences, and they point opposite ways to the obvious reading.**

1. **The `Crowding` work is not a dead end.** It was a correct mechanism with
   nowhere to act. Do not file it in `dead-ends.md` as a failed lever; file
   the *condition* — that it was tested without any aggregation point.
2. **Our lens is failing in the direction opposite to the biology.** A
   contents-free colony should produce **tunnels and no chambers**. Ours
   produces all chamber and no tunnel, because nothing distinguishes advancing
   from widening. There is no tunnel mode to end.

**Scope limit, stated because we will otherwise build it as a general rule:**
one species (*Acromyrmex lundi*), laboratory, binary-choice clay arenas, 20
pupae and 0.5 g of fungus. It is a strong result about that setup.

---

## 1. What the night established, as premises the stages rest on

Every row measured in `examples/digbox` — a bare box of stone, soil and air
with no food, no plants, no weather. Deterministic: two shipped runs are
byte-identical, so an arm's difference is its switch and nothing else.

| | |
|---|---|
| **The nest is 3x bigger than any census in this repo says** | `roofed`/`open` count materially EMPTY cells, and a gallery with an ant in it is not empty. Measured: roofed 157 + open 165 + **bodies 692** = **1,014**, against 941 cells hauled above the original surface. Conservation closes to 8%; on `roofed` alone it fails by 619 cells |
| **Material is moved, never destroyed** | `spoil_dumped/digs` = 0.986. Net void can only come from material carried clear of the ground |
| **Four of five dig senses are colony-constant** | At one tick, 51 ants at the nest: food 0, at-nest 1, crowding one scalar to four decimals, moisture 0.000 — **one distinct value each**. Only `SurfaceCurvature` varies (16 distinct) |
| **`MoistureGrad` is inert, not inverted** | 0.000 at wilting point, 400, field capacity and saturation. It is an unsigned magnitude and a uniform fill has no gradient |
| **The gate cannot modulate** | Across the band `Crowding` actually occupies, the decision moves 0.013 — a **26x compression** |
| **`AtNest` is 2 rows deep** | 8-adjacency to a one-cell skin. At-nest ants span 46 columns x **2 rows** |
| **Curvature → `Dig` works** | −0.6 gives **2.3x** roofed chamber; **+0.6 collapses it to 36**. An effect that reverses with the sign |
| **Digging got faster, not better** | Against 2026-09-11 at matched colony size: **2.11x the digs, identical standing void** |

**And the structural fact under all of it:** the dig target is `DIRS[heading]`
— one cell, no choice. **The engine has no notion of *where* to dig**, only
whether. That is why both dig-target dead ends failed across their ranges,
and why interventions on *whether* cannot produce a shape.

---

## 2. The stages

Ordered by cheapness x independence x how much it moves the picture. Each has
a check that can fail and a number it is scored on. **Score everything on
`room total`, never on `roofed` alone** — §1 row 1.

### Stage 0 — make the instruments honest (blocking)

The ant-blindness is not local to `digbox`. `lab::census`, `burrow_probe` and
`latecensus` all census raw `material == EMPTY` for their roofed columns and
undercount any dense colony the same way. Fix it there, then **re-score the
three `Crowding` interventions**, which were judged on the wrong number and
may not be the nulls they appear to be.

*Check that can fail:* the conservation identity — room total against material
hauled above the original surface — must close on a colony that has dug.

### Stage 1 — gravity and repose give the dig a direction

The cheapest large win, and the one thing with no analogue in the engine at
all. Tunnels follow gravity downward from the surface and *upward* when begun
mid-medium, sitting near the angle of repose (~40°) — Buarque de Macedo et al.,
*PNAS* 118 (2021), real-time X-ray CT. **Unvalidated: this is `[search]`-grade,
and it is on the handoff's list.**

**Validated 2026-09-19, and it came back partial. According to PubMed**, the
paper ([DOI](https://doi.org/10.1073/pnas.2102267118)) confirms
*"ants tend to dig piecewise linearly downward"* — the direction claim holds.
**The ~40° angle of repose and the "upward when begun mid-medium" claim are
not in the abstract and remain unvalidated**; full text was not available.
Build the downward bias; do not build the repose angle on this citation.

**And the abstract carries a mechanism neither report had, which may be the
better rule:** intergranular forces fall around ant tunnels because *arches*
form in the soil, so grains on a tunnel surface are already under low stress —
and **ants avoid removing grains under high force "without needing to be aware
of the force network"**. That is a purely mechanical selection rule: dig the
grain that is carrying least.

**This engine already has that quantity.** `src/sim/structural.rs` and
`src/sim/load.rs` maintain a per-cell support/load model, and `M17` is the
collapse build. *Dig where the load is low* is expressible here with no new
field, no new sense, and no genome slot — and it would produce arching and
tunnel stability as a consequence rather than as a target. **Measure it
against the plain downward bias before choosing.**

Build the direction as a **bias on the dig target at the call site**, not a
brain input: no new slot, no genome widening, no baselines voided.

*Scored on:* aspect ratio of the workings against today's **46 x 2**, plus
room total. *Check that can fail:* the ablation must reproduce today's lens.

### Stage 2 — an aggregation point, so density has somewhere to act

§0's missing first link. Before eggs exist, the cheapest honest proxy is a
**place ants gather below ground** — the nest site given real depth, so
`AtNest` is a region rather than a 2-row skin, combined with the **local**
`Crowding` reading already built behind `PIXEL_PHYSICS_CROWDING_LOCAL`.

This is the stage that decides whether §0's reading is right. If the crowding
mechanism was only ever missing an aggregation point, giving it one should
produce widening **where the ants gather** and not elsewhere.

*Check that can fail:* widening must be **local to the aggregation**, not a
uniform increase everywhere. A global rise means the aggregation point is not
what is doing it.

### Stage 3 — curvature on the dig

Already measured at 2.3x. Lands **after** Stage 1 because the two interact —
curvature deepens hollows, gravity picks which hollow — and measuring them
together is the only way to know whether the 2.3x survives a directed dig.

A genome weight in `ant.ron`, so it wants a seed sweep and re-derivation of
anything calibrated against today's behaviour.

### Stage 4 — fresh spoil attracts digging

The one stigmergic cue in ant excavation with a positive result, and `spoil`
already exists as a distinct material. A material adjacency test. Gives
excavation the positive feedback it has never had.

### Stage 5 — contents, properly

Eggs or a granary, then widening keyed to them. The dearest item, and the
real version of Stage 2. Blocked on nothing except Stage 2 telling us whether
the aggregation chain works at all.

---

## 3. What not to build, and why

- **A digging pheromone at the face.** **Validated 2026-09-19. According to
  PubMed**, Bruce, A. I. (2015), *Behavioural Processes* 122:12-15,
  ([DOI](https://doi.org/10.1016/j.beproc.2015.10.021)): groups of **5**
  *Acromyrmex lundi* choosing between a freshly exposed face and one where
  digging ceased an hour earlier — *"No significant difference in digging
  activity between 'fresh' and 'aged' sites was detected."*
  **Two corrections to the companion report.** The author is **Bruce**, not
  Pielström & Roces. And the authors scope their own null narrowly:
  *"while digging pheromones may play other roles in other parts of the
  digging system, they do not play an important role in regulation of soil
  excavation at the digging face."* So this rules out a **dig-face** pheromone
  at n=5 groups; it does not rule out pheromones elsewhere in construction.
  Enough to cancel the feature that was nearly built — which came from a
  content-farm page asserting seasonal "pheromone blends", a fabricated
  mechanism — and not enough to close the question generally.
- **A CO₂ field.** Tested from atmospheric to 10% for dumpsite choice and not
  used, while humidity and temperature were.
- **`Persist` as the home of tunnel advance.** Real biology, but documented as
  what operates in the *absence* of external cues. With gravity available it is
  the straightness half at best, behind Stages 1 and 3. This corrects an
  earlier recommendation of mine that ranked it first.
- **An entrance-convergence rule.** The literature has the function and not the
  mechanism. Anything built here would be invention.

---

## 4. How this plan could be wrong

- **Stage 1's direction claim is validated and its repose angle is not.** If
  the ~40° limit turns out weaker or absent, the downward bias still stands;
  it is the slope that would go. The load-based rule is the hedge, and it may
  simply be better.
- **Stage 2 assumes aggregation is sufficient.** The paper's ants aggregate
  around a *deposited object*; a region with no object may not concentrate
  them. If Stage 2's check fails — widening rises everywhere rather than at
  the gathering — then contents are load-bearing and Stage 5 moves ahead of it.
- **The `Crowding` nulls may be real.** Stage 0's re-scoring settles that
  before Stage 2 spends anything on it.
