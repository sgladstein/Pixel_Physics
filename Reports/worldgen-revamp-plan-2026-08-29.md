# Worldgen: the revamp

2026-08-29. Written by the coordinating session of a six-lane worldgen
program, from six audits landed the same day. **Start here; the audits are
this document's evidence.**

| lane | report | what it establishes |
|---|---|---|
| A | `worldgen-appearance-audit-2026-08-29.md` | what the player actually sees, in rendered pixels |
| B | `worldgen-architecture-ceilings-2026-08-29.md` | what no tuning inside this pipeline can produce |
| C | `worldgen-prior-art-and-dead-ends-2026-08-29.md` | what was already tried, and the outside prior art |
| D | `worldgen-visual-interest-2026-08-29.md` | how our world differs from beautiful landscape |
| E | `cave-redesign-2026-08-29.md` | what replaces the cave generator |
| F | `rock-vocabulary-design-2026-08-29.md` | the rock the world is made of — built, measured, and it overturns this plan's own ordering |

---

## 1. What the owner asked for

Six rounds of worldgen have shipped. Asked whether the problem was that every
world is the same shape, he answered:

> *"One complaint could be that these are too similar, but it isn't the
> biggest issue. **It is overall visual interest.** Look at some images from
> the internet for beautiful nature and be inspired. How does it differ from
> our world. **Caves also need to be fully redone as they are crap.**
> Waterfalls are also crap, but not work the effort right now. Should be
> postponed."*

and, the same minute, on a separate card:

> *"identical. seems like you are doing minor tweaks and not a revap as
> asked"*

Pressed further the same evening — shown a blind A/B of terrain lighting, and
asked directly whether what is missing is colour or shape — he was sharper
still:

> *"A is interesting, but not convinced yet. Still you are focusing on
> lighting which is not the issue. **it is the build**"*

> *"**Shape**, large rock formation (not just tall pillars), cave openings,
> mountains."*

**That is the brief, in his words.** Shape, not colour — he has now said so
twice. Large rock formations, and explicitly *not just tall pillars*. Cave
openings. Mountains. Caves are a full redo; water is **postponed**; and the
bar is a revamp, not a round.

---

## 2. The diagnosis, in one sentence

**The world is flat, and everything else is downstream of that.**

Measured over one full player screen, five presets at seed 1 (Lane D §1.1):

| preset | skyline rise and fall over one screen | biggest single step |
|---|---|---|
| rolling | 26 rows of 320 | 10 |
| terraced | 42 | 8 |
| canyon | 42 | 7 |
| **arid** | **12** | **1** |
| wetland | 28 | 9 |

The ground line moves between **4% and 13% of screen height across an entire
screen**. There is no cliff to fall off, no ledge, no spire, no overhang, no
notch — not because they are rare, but because the relief they would be made
of does not exist.

And the second half of the same fact, which is what makes it structural
rather than cosmetic: **only 4.1–4.9% of the ground on screen lies within six
cells of air.** The rest is interior. Our world is a solid block with a
slightly bumpy top.

---

## 3. The evidence that this is not round 7

### 3.1 The work went into 0.6% of the picture

Lane A, measured in *rendered pixels* at the player's viewport, 16 viewports
covering the world's width, daylight pinned:

| what | share of the player's view |
|---|---|
| `stone_massif` — what the rock is | **49.98%** |
| `soil_blanket` | 19.02% |
| `soil_moisture` — how dark the soil is | 18.38% |
| every landform pass — brows, talus, residuals, boulders, caves | **at most 0.59% each** |
| `life_scatter` — the entire biosphere | **0.031%** |
| `vaults` — a 43,208-cell cave system | **0.000%** |

Nine of fourteen passes change under 1% of the viewport; four change nothing.
The three that move the picture are all **colour** passes. This is
`plant-appearance-design.md` transposed exactly, and the project learned it on
the plant line without ever crossing it over.

**The strongest control in the program is a retrodiction.** Lane A's
instrument was built without reference to the review queue, then predicted
**seven of the nine "no difference" verdicts**: three of those cards asked the
owner to judge a retune of `vaults`, which moves zero pixels. He was right
every time and the pictures could not have shown him anything else.

### 3.2 The premise the program started on was false, and the correction matters

*"Every world looks the same"* is **wrong**: between-preset colour distance is
**4.76x** within-preset. Presets do differ — **by repainting, not by
reshaping.** Colour distance 0.453 against skyline-shape distance 0.078, and
**94% of the skyline-step distribution is shared between any two presets**.

`rolling` against `terraced` scores 0.149 — *below* `rolling`'s own
seed-to-seed distance of 0.173. **Changing that preset moves the picture less
than changing the seed does.**

**That metric is now owner-calibrated.** Two cards were posted before either
verdict existed: the matrix said `rolling`/`terraced` were one country and the
owner replied *"identical"*; it predicted 2–3 distinguishable countries among
five presets and he replied *"2, maybe 3"*. **It can be used to measure
whether the revamp worked**, which is the first objective success criterion
this line of work has had.

### 3.3 The generator has no representation of a feature

Lane B's root cause:

> It has a heightfield — `ColumnPlan`, four `i32` per column — and it has the
> cell grid. **There is nothing in between.**

A thing with no representation cannot be **shaped** (only stamped), **varied**
(only its stamp parameters can), **caused** (nothing upstream can act on it),
or **connected** (nothing can see it). Those are the owner's four standing
complaints.

Concretely: the owner's confirmed-*"true"* sentence — *"straight sides,
uniform width, flat top… no talus at the foot and no broken profile"* — is a
line-by-line description of `residual.rs:392-416`. `w_i.min(prev)` forces
monotone width; the top ring is a horizontal cut; `|dx| <= w_i` forces mirror
symmetry; and `talus` is pass 5 while `residuals` is pass 7, so **a residual
cannot have talus**.

And every cave is the same 8.2 x 3.9 Worley lattice at a different zoom —
derived aspect 0.400, **measured 0.401 in all five presets**.

### 3.4 Character is computed and then thrown away

This one is cheap to fix and is pure loss:

| | the plan phase computes | the realise phase writes |
|---|---|---|
| talus | median volume **244.5** | **3 cells** |
| boulders | **179** markers over 80 worlds | **3 seated** |
| springs | a causal source, built | **0** across 4 presets x 6 seeds |

`boulders` writes **0 cells on all six presets**. `wiki/the-world.md` gives
boulders a paragraph — *"an event, not a decoration"*. The ablation names a
cause outright: `without brows: boulders APPEARS (was zero)` in five presets.
That is `pass-interference-2026-08.md`'s R4-1, **recorded 2026-08-20 and still
firing**. Three more eaters are live: `pockets` suppresses caves, `ponds`
suppresses vegetation and springs, `talus` suppresses ponds.

### 3.5 The scale a formation lives at is empty, and erosion cannot fill it

At reaches 15 and 30 cells — exactly the scale a rock formation occupies —
**the tallest thing in the entire world is 4 to 10 cells**. Not rare: absent.
And instrumented, **no column ever peaks above its iteration-0 prominence**,
0 of 2048, both presets. Erosion only removes. The relief has to come from
somewhere else.

### 3.6 The world is bare

The entire flora of an 8192-column world is **343 one-cell seeds**. Only
`moss` establishes without a long run; **no `wood`, `leaf` or `grassblade`
cell exists in any world at settle**. One plant every 24 columns of skyline,
each of them one cell, in a world of 20,971,520 cells.

---

## 4. The revamp

**Keep the skeleton; replace what runs inside it.** Lanes B and C agree
independently, and it is worth saying plainly because "revamp" should not be
read as "rewrite". Pure plan, declared margins, collect-verify-write — that
discipline is *what made every one of these failures measurable*. Five of the
six causes were found because the architecture permitted an ablation. Keep it.
Change the thing the passes work on.

### The workstreams, in the owner's priority order

One correction is carried openly below: the rock item was demoted on first
writing and the measurement overturned it.

**W0 — Rock that differs. Demoted on first writing, and that was wrong.**

This was written up as a palette item and ranked last, because the owner has
twice said colour is not the issue. Lane F built it anyway and the demotion
does not survive its measurement — **not on colour, on strength.**

*"Large rock formation"* is differential erosion, and differential erosion
needs beds that differ in what they resist. Today `Character::resistance`
multiplies terracing while **every cell in the massif is exactly as strong as
every other**. So W1 has nothing to erode differentially, and W2's profiles
have nothing to be a profile *of*. **This is the substrate the shape work
stands on**, and it has to land with W1 rather than after it.

The measured case, six presets x 3 seeds x 16 viewports at the shipped size:
six rocks move **24.89% of the player's view** — more than `soil_blanket`
(19.02%), and roughly **40x the entire landform programme of six rounds**.
Colours covering half the ground go from 4–8 to 5–14 with speckle unchanged,
so it is palette rather than added grain. And it is **2.2x faster**:
`stone_massif` 2,109 → 979 ms, because the region tint was sampling two 2-D
fBm fields — five noise evaluations per cell over 18.7 million cells — and a
per-bed material deletes them.

It also explains something four lanes could not. The old region tint is a
**2-D blob that cuts across the bedding**, which is why the underground reads
as camouflage blotches rather than as layers. Keying rock on `(bed, column)`
and never on `y` is the whole correction.

And it found the answer to *"why does `boulders` seat nothing"*, which Lane A
listed as undiagnosed: `pockets` and `boulders` both tested for grey stone by
identity, so **both silently did nothing** — a second cause on top of the
pass-order defect. A material *flag* replaces six such tests.

**One piece of it is not ready.** The damp-rock family is the biggest single
item by pixels and *reduces* the colour spread on three of five presets; it is
posted for review and should not ship unreviewed. The prototype also currently
**defaults on** — one environment variable restores the shipped world exactly,
and that arm is the control every number above rests on.

**W1 — Give the ground a surface: mountains, and relief with a cause. THE
revamp.** Couple bed hardness to how far a bed stands proud, so a resistant
band *outcrops* and a soft one cuts back. This is the direct answer to *"it is
the build"* and to *"shape… mountains"*.

Two measured facts make it the centrepiece. First, the relief simply is not
there — the skyline moves 12–42 rows of 320 across an entire screen. Second,
**the landform passes are starved, not broken**: Lane D found `brows` and
`talus` key on a cliff test (6 rows over 4 columns) *the terrain never
clears*, which is why `brows` writes 2,352 cells of 18.9M and `talus` 580 —
52 and 39 on `arid`. Supply the relief and, in Lane D's words, that *"switches
`brows`, `talus` and boulders back on for free"*. The passes for benches,
scree and boulders already exist and are waiting for ground worth applying
them to.

**W2 — Formations that are not all tall pillars.** The representation move:
a **profile** — width as a function of height, with a foot, a taper and a
weathered crown — and a **size distribution** with small things far commoner
than large, plus explicit arbitration so "cells already taken" stops being an
accident of pass order. The owner's *"large rock formation (not just tall
pillars)"* is precisely a request for a vocabulary of shapes where today there
is one: `residual.rs:392-416` forces monotone width, a horizontal top cut and
mirror symmetry, and a residual cannot have talus because `talus` runs two
passes earlier. Lane B measured that **every sharp vertical face in the world
is a residual** — which unifies this with the "flat vertical slabs" verdict.

**W3 — Caves rebuilt from the ground up, and given a way in.** The owner's
instruction after seeing Lane E's photographs: *"The whole shape and
generation of the cave shold be rebuilt from the ground up."*

**The finding nobody had named: there is no cave entrance in this game.** The
depth band starts 200 rows down and `cave_system` *asserts* its envelope is
sealed stone; `viewshot` has to mine a shaft to photograph one. **Every cave
verdict on record was given on a picture of a place the player cannot reach**,
which is the deepest reason six rounds of cave work produced no playtest
reaction — and it is exactly what he was asking for when he said *"cave
openings"*. On top of that, **8 or 9 of 16 worlds have no cave at all**.

The shape diagnosis: a cave is a Worley `F2−F1` field thresholded inside a
box, so its shape vocabulary *is* the field's — literally a drawn Voronoi
diagram, straight corridors meeting three at a time, with one ellipse stamped
on top because the texture has no rooms in it. Retuning can only *zoom* the
lattice, because `half_w` cancels in `CaveEnv::cell`: **every cave in every
world is the same 8.2 x 3.9 lattice**. That one mechanism produces six of the
owner's eight recorded cave verdicts.

The replacement has no field in it. **A room is not drawn — it is a roof that
falls in and stops when it reaches a bed strong enough to hold its span**, so
shape is a consequence, two rooms differ because their rock differs, and the
rubble on the floor is the volume the roof lost. Conduits are Dijkstra paths
under an anisotropic cost built from `hardness_field`/`strata_offset`, which
already exist with four consumers — so the geology input is free.

Two owner rulings that overturned Lane E's own draft: **"Remove the web"** —
the Worley pattern goes, and he withdrew his earlier liking for it by name
(*"I know I said that I liked it before but no"*) — and rooms want to be
**3–7x bigger and chained so you can walk directly from one to the other**.
Speleothems move to last: *"This problem has been solved. That said this is
not at all the main issue."*

**The largest open risk in the whole plan lives here.** At 3x a room no longer
fits on one screen; at 7x it is two screens wide and two deep, and stone's
`max_unsupported_span` is 16 — so **a room that size forces pillars**. That is
measurable today with `support_census`, and it should be measured before the
room size is chosen, not after.

**W4 — Plants: no change. Settled, and out of scope.** Owner ruling,
2026-08-30, after this plan proposed changing it twice — first "germinate at
genesis", then "tune the sowing density, growth rate and spread":

> *"Don't change anything. Keep it how it is today. Your job is not to manage
> plant growth rates. Right now the world starts with no plants, just seeds and
> they grow as I play. Don't change that."*

**The current behaviour is correct and deliberate: the world is sown with
seeds, and they grow while the player plays.** Do not change sowing density,
germination, growth rate, or spread. Do not pre-grow a world at genesis.

The barrenness measurements elsewhere in this document stand as *facts about
what a freshly generated world contains* — they are why a review card rendered
at settle shows no vegetation, which matters when reading any card. **They are
not a brief.** Nothing downstream depends on them: the one workstream that
cited vegetation as a prerequisite was terrain lighting, and that is killed
too.

**A future session reading the 343-seed figure should not re-propose this.**
It has now been proposed twice and declined once in plain terms.

**W5 — Stop throwing away what is already computed.** The §3.4 losses, the
`brows`→`boulders` pass-order defect, and the three other live eaters. Cheap,
and it makes an entire wiki-documented feature exist for the first time.

**W7 — Provinces of a kind, not a gain.** `region.rs`'s `Character` is six
continuous multipliers; five are amplitude knobs and none touches a wavelength
or which passes run. Every wavelength varies at most **1.70x** across all five
presets while amplitudes vary 3.3–3.9x — and regions are 96–241 columns
against a hill wavelength of 150–200, so **the amplitude knob modulates at the
carrier frequency**, which is why *"the patterns don't flow"*. A multiplier can
only say *less*; the owner asked for *"should not exist at all in most biomes
but some biomes should have them"*. This is why the spire complaint has
survived three attacks.

**Postponed: water and waterfalls**, on the owner's explicit instruction.

### The reframing worth carrying: we are drawing a cutaway

Lane D's most useful observation, and it changes what reference to reach for.
`stone_massif` is ~90% of the world's cells, so most of the screen is not
landscape at all — it is a **cross-section through rock**. Landscape
photography is the wrong referent for four fifths of the picture. The right
one is a **quarry face, a road cut, a canyon wall**, and the gap to that is
relief and *staining* — seeps, oxidation, wet rock, dust on ledges — rather
than a wider palette. That is the second case for W0, and it is a better case than
"more colours".

### The loop that let six rounds fail

1. **Judge at the shipped world size.** `examples/filmstrip.rs` builds
   **512x320** (lines 73–74) and its own source calls `scene=worldgen` *"the
   thing worldgen is judged on"*. The app ships **8192x2560** — 256x the area.
   Features that fire at world scale read as dead there, and the reverse.
2. **Measure a pass in pixels, not cells.** The gap is up to **100x**.
3. **Gate the interference matrix.** `pass_ablation` already finds the eaters;
   nothing runs it, and R4-1 sat unfixed for nine days while deleting a feature.
4. **Re-check every ruler against the grown world.** This program found three
   instruments measuring the wrong object. `viewshot vault=1` searched below
   `world_h/2` — correct at 2048x640, *beneath every cave* at the shipped
   8192x2560 — and printed `NO VAULT in this world` on a seed whose own pass
   counter said `systems 1` in the same run; **every "photograph a cave"
   instruction in the repo has been running against that since the world
   grew**. `vaults detail`'s `base-width` reported the width the draw
   *intended*, not what rasterised. And the "sharp vertical faces" complaint
   was dismissed on a number measuring the plan heightfield rather than the
   finished world.
4. **Use Lane A's calibrated distance matrix as the success measure**, and
   re-run the `rolling`/`terraced` cell at 12+ seeds first, as Lane A asks.

### What the revamp does *not* need

**Not the 3D coarse map.** Lane C found it designed in detail and never built,
and identified it as the load-bearing decision because the simulation-first
prior art needs global flow accumulation. But W3's fix for flatness — couple
bed hardness to relief — is a *per-column* change. **The revamp does not hinge
on the largest piece of infrastructure in the backlog**, and that is worth
knowing before anyone starts building it. Revisit it only if W3 lands and the
world still reads as one country.

---

## 5. Sequencing, and a stop gate

Split on the contested-file table so lanes do not collide. `passes.rs` is
4,874 lines and contested — whoever holds it lands quickly rather than holding
a large diff across a session.

**Phase 0 — clear the drains (days, one lane).** Not the revamp; cheap, and a
*prerequisite* for the revamp being visible. If relief lands while `brows` is still deleting every boulder, W1 will pay
into passes that are being thrown away.

**Note for the drains lane:** `ponds` also suppresses `life_scatter`. Under
W4's ruling, do **not** change how many seeds a world ends up with — report
that conflict rather than landing a fix that moves the seed count.

| lane | work | files |
|---|---|---|
| 0a | W5 the discarded character, the four live eaters, judge at shipped size | `passes.rs`, `examples/*` |

**Phase 1 — the build. This is the revamp.**

| lane | work | files |
|---|---|---|
| 1 | **W0 rock that differs + W1 relief with a cause — mountains** | `column.rs`, `erosion.rs`, `assets/materials/*` |
| 2 | **W2 formations with a profile — not just tall pillars** | `residual.rs`, new feature module |
| 3 | **W3 caves as rooms and passages, with openings** | `vaults`, new cave module |

W0 rides with W1 because relief with a cause needs rock that differs in
strength; the rest are, one for one, the owner's own list: *"Shape, large rock
formation (not just tall pillars), cave openings, mountains."*

**GATE — show him. If this is not visibly a different world, stop and
re-diagnose rather than starting phase 2.** He has said we are spending too
much time on this, and six rounds have each ended by asking for one more. The
gate is what keeps that honest — and Lane A's distance matrix now gives it a
number as well as a picture.

**Phase 2 —** W7 provinces of a kind; the staining half of W0 (seeps,
oxidation, wet rock) once the damp family has been judged. The 3D coarse map only if W1 proved insufficient.

Every phase ships with a **seed sweep read at an order statistic**, not a
single seed, and an owner card **before** it is called done.

## 6. What this plan refuses to do

From Lane C's do-not-retry list — each was tried, fired, and measured
identical to its control:

- **No global density knob for spires, and no region-scale gate.** Both built,
  both null. The rejection depends on a region being 102–256 columns, so a
  **multi-screen** province voids it — which is why W6 specifies kind at
  country scale rather than another multiplier.
- **No "turn erosion up and the formations will come".** Measured impossible.
  The rejection is entirely about the *input*, so W3 supplying relief voids
  it — Lane C calls it the most consequential re-testable entry in the corpus.
- **No new collect-verify-write pass that can be sealed off wholesale.** One
  grain of sand has deleted an entire cave system.
- **No smoothly-varying Worley pitch for cave variety.** Structurally broken
  where the consumer is an identity test between neighbours.
- **Do not start with lighting.** Lane D was commissioned to test the
  hypothesis that shading terrain is the cheapest large win and **killed it,
  with a positive control** — and the owner then killed it in his own words,
  unprompted, on a blind A/B: *"Still you are focusing on lighting which is
  not the issue. it is the build"*. On the blind surface card he picked the
  **shipped, unlit** pane. The measurement says why: a shading term needs a surface, and 95% of the
  ground on screen is interior. `ao` moves *exactly* the ground fraction —
  a uniform dim, not local contrast, functionally the depth grade the owner
  already rejected. What survives is `ink`, a drawn edge at the rock/air
  boundary, and it should follow W3 and W5 rather than lead them, because both
  make it worth more.

Never tried here at all, per a grep of all 595 dead-end entries: `voronoi`,
`tectonic`, `karst`, `mesa`, `watershed`, `poisson`, `prefab`.

---

## 7. Risks, and what would falsify this

- **No photograph was ever seen.** Lane D could not fetch a single image —
  every host returned `403 CONNECT tunnel failed` from the container's egress
  proxy. The reference half of the owner's instruction is therefore weaker
  than intended and is written from the lane's own knowledge, flagged as such.
  **If the owner can drop reference images into the repo, that gap closes.**
- **`rolling` vs `terraced` at 0.149 against a 0.173 diagonal is close.**
  Re-run at 12+ seeds before building on it.
- **Lane A's shape channel is the skyline only.** *"Presets barely differ in
  form"* is a claim about the ground line and must not be read wider.
- **`vaults` at 0.000% is a surface-viewport measurement.** It cannot separate
  "invisible because underground" from "eaten by `pockets`". Caves are seen
  from inside; crossing Lane A's pixel measure with the interference matrix is
  the next measurement.
- **No claim here is a performance claim.** Nothing in five audits measured a
  frame. W3 and W5 both add generation-time work; measure before believing.
- **The whole plan rests on W3 being able to produce relief.** If coupling bed
  hardness to prominence does not lift the 15–30 reach band off 4–10 cells,
  the diagnosis in §2 is right and the remedy is wrong, and the coarse map
  comes back onto the table as the way to get relief with a cause.
