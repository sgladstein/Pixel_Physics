# Visual interest: what beautiful landscape has that we don't

Lane D of the worldgen revamp, 2026-08-29. Written against the owner's
redirection of the whole programme:

> *"One complaint could be that these are too similar, but it isn't the biggest
> issue. **It is overall visual interest.** Look at some images from the
> internet for beautiful nature and be inspired. How does it differ from our
> world. Caves also need to be fully redone as they are crap."*

and, the same minute, *"identical. seems like you are doing minor tweaks and
not a revap as asked"*.

**The short answer to "worldgen, render, or content": worldgen, by a long way,
and for a reason that is not the obvious one.** It is not that worldgen owns
the most items on the list. It is that worldgen owns the *shape of the ground*,
and the shape of the ground is the thing every render-side improvement needs in
order to have somewhere to land. Lighting, texture, ambient occlusion and
palette all multiply a quantity that is currently near zero. Content is a clear
second and is the cheapest item on the list. Render is third, with one genuine
exception called out in §1.5.

**The owner has since confirmed this from the other side, and his sentence is
better than mine** (§6a, on a blind A/B of the lighting work):

> *"Still you are focusing on lighting which is not the issue. **it is the
> build**"*

and, asked directly whether what is missing is colour variety or shape:

> *"**Shape**, large rock formation (not just tall pillars), cave openings,
> mountains."*

---

## 0. The reframing that makes the rest of it make sense

**We are not drawing a landscape. We are drawing a cutaway, and half the screen
is the inside of the ground.**

Measured: on a normal above-ground screen, 50% of the pixels are ground, and
`stone_massif` writes **18.9 million of the world's 21 million cells** — 90% of
everything. The skyline, the part that has any counterpart in landscape
photography, is a band across the top fifth. Everything below it is a section
through rock that no photograph of a mountain has ever shown.

This matters because *"look at beautiful nature photography"* silently supplies
the wrong referent for most of the screen, and every improvement aimed at it
lands on the top fifth. The right referent for the other four fifths exists and
is just as beautiful: **a canyon wall, a road cut, a sea cliff, a quarry face**
— geology seen from outside. Those are stunning for reasons that can be named
and built:

| what a real rock face has | what ours has |
|---|---|
| beds of different hardness standing proud by different amounts, making a cliff/ledge/bench/slope stair | every bed flush with every other — the face is a plane |
| bedding that dips, folds and is cut by unconformities | horizontal banding |
| dark vertical staining where water runs down it | nothing |
| a talus fan at the foot of every drop | 580 cells in the whole world (§1.1) |
| plants on every ledge and in every crack | 170 cells in the whole world (§1.2) |

**So the single most useful sentence in this report is probably this one:** our
cross-section currently reads as a *textbook diagram* and should read as a
*photographed cliff*, and the difference between those two pictures is almost
entirely relief and staining, not colour and not light.

---

## 1. The ranked list

Each item: what the eye sees, the mechanism that produces it, the category,
cost, payoff.

### 1.1 The ground has no shape for anything to happen on — **WORLDGEN**

**What the eye sees.** A low, bumpy line across the upper third of the screen,
and under it a flat wall. Nothing rises, nothing overhangs, nothing stands
proud, nothing casts. There is no cliff you could fall off, no boulder you
could stand on, no ledge, no spire, no notch.

**Measured.** Across five presets at seed 1, one full player screen each, at
the same world column:

| preset | skyline rise and fall over one screen | biggest single step | mean slope (rows/column) |
|---|---|---|---|
| rolling | 26 rows of 320 | 10 | 0.47 |
| terraced | 42 | 8 | 0.67 |
| canyon | 42 | 7 | 0.47 |
| arid | **12** | **1** | 0.20 |
| wetland | 28 | 9 | 0.37 |

Over three screens of continuous strip the best case is canyon seed 7 at 107
rows, and the worst is arid seed 1 at 19. **The tallest landform the player
sees on a normal screen is about an eighth of the screen height.** A mean slope
of 0.2–0.67 rows per column is a surface that is nearly horizontal everywhere.

**Mechanism.** `src/worldgen/passes.rs` builds the surface from a column plan,
and the two passes that would put relief on it are gated behind a cliff test
that the terrain almost never passes. `cliff_edges` (`passes.rs:954`) requires
a **6-row drop over 4 columns** (`CLIFF_DROP`/`RUN_NEAR`, a slope of 1.5) or a
**20-row drop over 20 columns** (`CLIFF_DROP_FAR`/`RUN_FAR`). Measured mean
slope is 0.2–0.67 and the biggest step on a screen is 1–10 rows. So:

- `brows` (`passes.rs:1006`) — the overhang pass — fires almost never;
- `talus` (`passes.rs:1094`) — the scree apron — has nothing to fall from;
- `erosion` reports `boulder-markers 3 boulders-seated **0**` on canyon seed 1,
  so the one pass that would put a standing rock in the view seats none.

**Measured rather than inferred** (`PASS_TIMING=1`, clean tree at HEAD, seed 1,
cells written per pass over the whole 8192x2560 world):

| preset | `stone_massif` | `brows` | `talus` | `life_scatter` |
|---|---|---|---|---|
| canyon | 18,894,054 | **2,352** | **580** | 170 |
| terraced | 18,772,577 | **708** | **191** | 636 |
| arid | 19,088,122 | **52** | **39** | **0** |

**The overhang pass writes 0.012% of the world and the scree pass 0.003%.**
Spread over 8192 columns that is 0.29 and 0.07 cells per column — about 147 and
36 cells on a 512-column screen, against ~83,000 ground cells in view. They are
not a small effect; they are not present. On `arid` they are 52 and 39 cells in
the entire world, and `life_scatter` writes **zero** — arid worlds contain no
plants at all.

The machinery for relief exists and is starved of the input it keys on. Note
which way round that is: this is not a case for writing new passes, it is a
case for making the terrain rough enough that the passes we already have fire.

**Cost.** Large. This is the revamp. **Payoff.** Largest available — and it is
the only item that *unlocks* others: §1.5 and the whole of §2 are worth little
until the ground has shape, and `brows`, `talus` and the boulder pass switch
themselves back on for free the moment the terrain is rough enough to clear
their thresholds.

**The concrete target, from the reference (§3):** a canyon wall is not a plane,
it is a *repeating cliff / ledge / bench / slope sequence* produced by
differential weathering — hard beds stand out as near-vertical steps, soft beds
recess into ramps. We already generate rock units with different hardness
(`Purpose::RockType`, `Purpose::RockFacies`); what we do not do is let hardness
set how far a bed stands proud of the face. That single coupling is the
highest-value worldgen change on this list, because it produces relief
*everywhere on the wall*, not just at the skyline — and the wall is half the
screen.

### 1.2 The world is bare — **CONTENT**

**What the eye sees.** No trees. No grass. A few specks of moss on a ridge.
Rock, soil, sand, sky.

**Measured** (Lane A/B, `Reports/worldgen-appearance-audit-2026-08-29.md`, and
confirmed by eye in every render taken for this lane): the entire flora of an
8192-column world is **343 one-cell seeds**. `life_scatter` moves **0.031%** of
the player's view. Only `moss` (66 of 66) establishes without a long run; the
other five species are sown and ungerminated at any timescale a review card is
rendered at. **No `wood`, `leaf` or `grassblade` cell exists in any world at
settle.**

In player terms: 343 living cells in 20,971,520. One plant every 24 columns of
skyline, each of them one cell.

**And it varies by preset in a way the 343 figure hides.** Lane A's number is
`rolling`; measured here on the clean tree with `PASS_TIMING=1`, `life_scatter`
writes **170** cells on canyon, **636** on terraced, and **0** on arid. An arid
world contains no plant of any kind, sown or grown.

**Mechanism.** `life_scatter` sows seeds; five of six species need thousands of
frames to germinate; nothing in worldgen produces a *grown* plant.

**What a grown world looks like, since nobody had rendered one.** Canyon seed 1
run forward to frame **29,400** — which is 600 + 8 x `DAY_NIGHT_PERIOD_FRAMES`,
so the clock phase is identical to the bare render and the pair differs by the
world's age and by nothing else — puts **two large trees** in the same viewport
that was bare at frame 600. (The first attempt at this was rendered at frame
30,000 and came out at night, which would have confounded the comparison with
the time of day; the figure below is the daylight-pinned pair.) They are the tallest thing in the picture, they fill the
sky half of the frame, and they are the only green on screen. The picture is
transformed, and *nothing about the terrain changed* — this is the world we
already ship, seen after it has had time to grow. It is the strongest single
argument on this list that the barrenness is worth fixing at genesis: the
content is already implemented and simply is not present when the player
arrives.

**Cost.** Small — this is a sowing-density and a germination-at-genesis
question, not new simulation. **Payoff.** Very large, and it is the cheapest
large item on this list. Every real landscape photograph that reads as
beautiful is substantially covered in living things, and ours has effectively
none. It is also the only item here that adds a *colour* the ground palette
does not otherwise carry: no green appears among the twelve most common ground
colours on a canyon screen (§1.4), and the only green on screen anywhere is the
66 cells of moss.

**Caveat worth stating.** Plants are drawn one cell wide and
`Reports/subpixel-rendering-2026-08-29.md` §1 is the standing account of why
they look bad at that width. Sowing more of them multiplies whatever a plant
currently looks like. The two want doing together.

### 1.3 The caves are a Voronoi diagram — **WORLDGEN**

**What the eye sees.** A network of straight black tubes of constant width
meeting at junctions, with a smooth black ellipse in the middle. It reads as
cracked dried mud, or as the lead came in stained glass. It does not read as a
cave.

**Mechanism, exactly.** `passes.rs:2142`:

```rust
let (f1, f2) = noise::worley_f2_f1(sys, Purpose::Cave, dx as f32 / cell, v / cell);
...
if f2 - f1 < CAVE_THRESHOLD * fade { void[env.idx(dx, dy)] = true; }
```

`f2 - f1` below a threshold **is the definition of a Voronoi cell boundary**.
The passage network is not cave-shaped noise that happens to look angular; it
is literally the edge set of a Voronoi diagram, so straight segments between
junction points is what it is guaranteed to produce. The chamber is a separate
ellipse drawn with half-extents from `Purpose::CaveChamber` (12–24 cells).

**What real caves look like** (§3): passages are *sinuous* where fracture
control is weak, and angular only where joints control them — so straight
everywhere is one of the two real cases used for all of them. **Flowstone is
the single most common formation of all**, and it is a sheet on the *wall*, not
a spike from the ceiling. Rimstone dams form stair-stepped pools on the floor.
Colour ranges white through yellow, red and brown with iron.

Ours: 2,528 speleothems generated on canyon seed 1, drawn as a handful of
narrow cones; a `flowstone` material exists in `assets/materials/` and does not
appear on any wall in any render I took.

**Cost.** Medium — the passage generator is one function. **Payoff.** Large,
and the owner has already asked for it by name.

**This item is owned by Lane E, which reached the same diagnosis
independently.** `Reports/cave-redesign-2026-08-29.md` landed on this branch
while this lane was measuring, and its one-paragraph answer is the same
sentence: *"every corridor is a straight Voronoi boundary segment, every
junction is a three-way Voronoi vertex"*. It carries the replacement design
(rooms grown by roof collapse, passages by shortest path through the strata
cost field) and should be read instead of this section for anything beyond the
diagnosis — what is here is corroboration from a different direction, arrived
at by looking at the picture rather than at the generator.

### 1.4 The palette is three greys and a tan — **RENDER (asset data)**

**What the eye sees.** Everything is the same colour. Even the presets that are
supposed to differ (arid orange, wetland dark) are the same picture in a
different wash.

**Measured**, on one canyon screen, colour binned at the width of the
renderer's own grain jitter (an unbinned count reads every jittered cell as its
own colour and would say the palette is rich — `world_look`'s note records that
trap). The twelve most common ground colours reduce to **three hue families**:

| family | hue | saturation | what it is |
|---|---|---|---|
| neutral | 0° | **0.00** | stone |
| tan | 40° | 0.19–0.22 | sand, sandstone |
| slate | 240° | 0.06–0.07 | the cool stone family |

Ground saturation median is **0.074** on canyon. **No green appears among the
twelve most common ground colours at all** — the only green on screen is the
moss on the ridge, which is 66 cells in the whole world. (One further bin, hue
210 at saturation 0.45, is the water line at the world edge, not rock.)

**Mechanism.** `assets/materials/*.ron` give each material four tones, and the
four tones are a **value ramp with no hue rotation**: `stone.ron` family 0 is
(128,128,132), (118,118,124), (138,136,140), (110,112,118) — a brightness
spread of 28 out of 255 and a hue shift of essentially nothing.

**This is already written up and never actioned.** `PLAN.md`'s M19 research
section says exactly this: organise colours as **HSL ramps, shifting hue while
shifting value** — darks rotate toward blue/purple and desaturate, lights
rotate toward yellow — and distinguish adjacent materials by **hue, not value**,
because value-only differences vanish at small pixel sizes. It names Resurrect
64 and Endesga 32 as directly adoptable.

**Cost.** Very small — it is editing `.ron` files. (Gotcha: `include_str!`, so
a headless harness needs a rebuild between edits; only the app's F5 reads the
directory.) **Payoff.** Medium to large, and it is the best cost/payoff ratio
on the whole list. This is the one item I would do first regardless of
everything else.

### 1.5 The cave void is one flat colour, and the sky is the only thing lit — **RENDER**

**What the eye sees.** Cave air is a hole of dead flat black. The sky is a
smooth gradient and is the best-looking thing on the screen.

**Measured.** On an underground screen, **11.0% of the frame is below
luminance 40, and every one of those pixels is byte-identical: RGB (31, 29,
33), luminance standard deviation 0.00.** Not a gradient, not a grain, not a
falloff — one fill. The rock beside it has a luminance std of 18.1.

And the split that says where the light is:

| | p05–p95 luminance range | how much of that range is a smooth vertical ramp |
|---|---|---|
| sky | 73 | **97%** |
| ground | 134 | 25% |

The sky's whole range is a gradient. The ground's range is *material patches* —
grey next to tan — not light. That is why the sky reads as beautiful and the
ground reads as wallpaper, and it is measurable rather than a matter of taste.

**Mechanism.** `render.rs`'s `cell_colour` computes a cell's colour from
material, per-cell shade index, sky light, depth grade and glow, plus a
position-keyed grain. **Nothing in it reads a neighbouring cell.** The cave
fade floors deep air at a constant. The reference note from the Starbound art
guide is apposite: *"true black often looks flat or harsh"*.

**Cost.** Small for the void specifically — a falloff and a slight colour on
cave air, and a rim on the wall facing it. **Payoff.** Medium, and it is
concentrated exactly where the owner has already complained.

---

## 2. The hypothesis I was asked to kill: "shading terrain is the cheapest
large gain, and it is not a worldgen change"

**Verdict: half right, and the wrong half is the half that would have made it
cheap.** Shading is not a substitute for the missing geometry; it is a
*multiplier* on it, and our multiplier is currently applied to nearly zero.

### 2.1 How it was tested

`examples/terrain_shade.rs` (new, in this branch). It builds the shipped
8192x2560 world, renders a viewport through the app's own `Renderer`, and then
multiplies a per-pixel scalar over the result. Colours come from the shipped
renderer, so **no arm can show a colour the engine would not have drawn** —
`subpixel.rs`'s rule, and what makes an A/B off it admissible. The occupancy
the shading reads comes from `World` directly, not from the image: deep air and
night rock are both near-black, so an image-derived mask would have holes
exactly where the caves are.

Three terms, separable: **ao** (crevice darkening from a coverage kernel),
**sun** (Lambert against a normal from the coverage gradient), **ink** (a dark
shell just inside the rock/air silhouette — the *flat, cartoony* direction, see
§2.4).

### 2.2 The measurement that kills it

**A shading term needs a surface. Below the skyline our ground has none.**

An occlusion reading is constant wherever the whole neighbourhood is ground,
and a normal derived from a coverage field is identically zero there. So the
quantity that bounds everything is: *how much of the ground on screen lies
within one kernel of air?*

| view | ground on screen | ground within 6 cells of air |
|---|---|---|
| surface, canyon | 50.9% | **4.7%** |
| surface, rolling | 50.4% | 4.1% |
| surface, terraced | 50.4% | 4.9% |
| surface, arid | 50.8% | 4.4% |
| surface, wetland | 50.3% | 4.1% |
| **cave, canyon** | 89.0% | **21.8%** |

Measured two independent ways — off true occupancy in the harness, and off the
rendered image in a separate Python pass — which agree to within a percentage
point.

**And re-measured on a clean tree, which matters here.** This checkout is
worked in by several lanes at once, and by the time these numbers were taken
another lane's rock-vocabulary prototype had modified `assets/materials/
stone.ron`, `src/sim/material.rs` and `src/worldgen/passes.rs` underneath them
— with the prototype defaulting **on** (`passes.rs:269`,
`unwrap_or(true)`). So every figure in this section was re-run in a separate
worktree at a clean `HEAD` with only this lane's harness added. **The results
are identical to the digit** — 4.1 / 4.9 / 4.7 / 4.4 / 4.1 and 21.8% — so the
conclusion is about the shipped world and not about a neighbour's work in
flight.

And the shading terms behave exactly as that number predicts:

| | subpixels the term moved | mean change |
|---|---|---|
| `sun` at the surface | 6.1% | 7.9 / 255 |
| `sun` in the cave | **19.1%** | **16.4 / 255** |
| `ao` at the surface | 50.9% — *exactly the ground fraction* | 81.6 / 255 at ceiling |

That last row is the finding. **`ao` moving exactly the ground fraction means
it is not adding local contrast at all — it is a uniform dim.** At ceiling
settings it turns the whole ground black. It is functionally the depth grade
the owner already played and rejected (*"no question grade off is better"*,
`render.rs`'s `TerrainLight::Off`), arrived at from the other direction.

### 2.3 The positive control, because a null is where an instrument hides

Per `CLAUDE.md`: a number that could not have moved looks exactly like a null
result. So before believing "shading does nothing", I ran the case whose answer
I knew must be non-zero — a **circular void under a directional light must show
a lit side and a shaded side**.

It does. Aimed at the cave chamber with `sun=1.0 sun_r=5`, the rock blocks
acquire lit upper-left faces and shaded lower-right faces, the chamber gets a
bright rim, and the stalactites acquire volume: **19.1% of subpixels moved, mean
16.4/255**. The instrument works. `base` moves 0 subpixels in every run, which
is the null arm's own control.

So the surface result is a real null about the *world*, not a broken harness.

### 2.4 What survives

- **`ink` is cheap, visible, and is the direction the owner's own last verdict
  pointed toward.** `Reports/subpixel-rendering-2026-08-29.md` §9 records the
  plant A/B coming back *"the edges between color or material look weird, kinda
  3d-ish. Could it be more flat or cartoony"* — a verdict on smooth rounded
  volume, which is precisely what `ao` and `sun` produce. A drawn edge at the
  rock/air boundary is the flat reading of the same field, needs no new state,
  and moves 0.9–7.3% of subpixels at 13–28/255. It is the shading term I would
  actually ship.
- **Underground, shading earns its keep** — 22% boundary density against 5%.
  If the caves are rebuilt (§1.3) they get denser, and shading gets better still.
- **Content raises the ceiling on render, and this is the one number that
  surprised me.** Running canyon seed 1 forward to frame 29,400 — far enough
  for the sown seeds to actually germinate — grows **two large trees** on the
  ridge in the same viewport, and the boundary census moves from **4.7% to
  15.6%**: a light can suddenly reach three times as much of the screen. That
  figure is computed from occupancy rather than from the image, so it is not a
  statement about the hour it was sampled at. A canopy is exactly the kind of
  boundary-dense geometry §2.2 says our world lacks, and **plants supply it for
  free**. So the ordering in §5 is not just "content is cheap" — content is
  also a *prerequisite* for lighting being worth anything above ground, on the
  same footing as relief.

- **The cost is already measured and the fix is already named.** `render.rs`'s
  `cell_colour` carries a comment recording that fake AO was built, measured at
  **~10 ms on the 512x320 stress scene** from four `World::get` HashMap lookups
  per pixel, and cut rather than shipped over budget — with the real fix named:
  chunk-direct array access instead of a hash lookup per neighbour, the same
  lesson `ChunkView` already applied to the sweep. It is `dead-ends.md:1152`.
  So the technique is not blocked on research; it is blocked on one known
  optimisation, and on there being something worth lighting.

### 2.5 The honest statement of the result

> Lighting the terrain cannot be the cheapest large gain, because 95% of the
> ground the player sees is interior with no surface, and a light has nothing
> to say about interior. The 5% it *can* reach is the silhouette, which carries
> disproportionate visual weight — so it is worth doing, cheaply, as `ink`. But
> the reason our world looks flat is that **it is flat**, and that is worldgen.

---

## 3. Sourcing — read this before quoting anything in §1 or §4

**I could not fetch a single photograph.** Every image host I tried returned
`403 CONNECT tunnel failed` from this container's egress proxy:
`upload.wikimedia.org`, `images.unsplash.com`, `live.staticflickr.com`,
`www.nps.gov`, `commons.wikimedia.org`, `picsum.photos`. `WebFetch` is
similarly blocked on `en.wikipedia.org`, `www.nps.gov`,
`www2.paradisevalley.edu` and `www.grandcanyon.net`. **So no visual comparison
against a real photograph exists in this report, and none was posted to the
owner.** That is a real gap in what was asked for and it is not recoverable
from inside this container.

What *did* work is `WebSearch`, which returns synthesised text summaries with
source links. Everything below is from those summaries and is attributed:

- **Differential weathering / canyon profile** — canyon walls form a repeating
  sequence of cliffs, ledges, benches and slopes; resistant formations form
  cliffs, soft ones (mudstone, shale) weather into gentle ramps; undercutting a
  soft layer drops the cliff above and retreats it, producing the stair-step
  profile. [Paradise Valley CC, differential weathering](https://www2.paradisevalley.edu/~douglass/v_trips/wxing/introduction_files/differentialwx.html)
- **Desert varnish** — vertical stripes alternating black, red and tan, formed
  where rare water trickles down bare resistant rock and evaporates, leaving
  iron/manganese oxides. [NPS, Desert Varnish](https://www.nps.gov/articles/desertvarnish.htm)
- **Talus and scree** — talus is boulders to house-sized, scree is much smaller
  fragments; both accumulate at the base of a cliff by rockfall driven by
  freeze-thaw; large scree slopes accumulate soil between the rocks and get
  colonised by vegetation. [Wikipedia, Scree](https://en.wikipedia.org/wiki/Scree) · [NPS, Talus Slope](https://www.nps.gov/places/talus-slope.htm)
- **Cave morphology** — passages are linear along bedding strike, angular under
  joint control, and **sinuous** in flat or gently dipping beds with weak
  fracture control; flowstone (water running down a *wall*) is the most common
  of all cave formations; rimstone dams form stair-stepped pools; colour runs
  white to black, with iron colouring formations yellow, red or brown.
  [Geosciences LibreTexts, Karst Cave Features](https://geo.libretexts.org/Bookshelves/Geology/Environmental_Geology_(Earle)/12:_Karst_and_Caves/12.04:_Karst_Cave_Features_Cave_Contents_and_Subterranean_Life) · [Wikipedia, Flowstone](https://en.wikipedia.org/wiki/Flowstone) · [NSS, Rimstone](https://caves.org/virtualcave/rimstone/)
- **Atmospheric perspective** — distant ground is paler, cooler, lower
  contrast, shifting toward blue; foreground is warmer, richer, more saturated.
  [Britannica, Aerial perspective](https://www.britannica.com/art/aerial-perspective) · [Draw Paint Academy](https://drawpaintacademy.com/atmospheric-perspective/)
- **2D shading practice** — ambient occlusion darkens creases and contact
  points and is what gives flat illumination perceived depth; in Starbound's
  own art guide, shading makes objects read as rounder, rim highlights imply an
  edge facing the sun, and **"true black often looks flat or harsh"**.
  [Starbounder, Guide:Art](https://starbounder.org/Guide:Art) · [Wikipedia, Ambient occlusion](https://en.wikipedia.org/wiki/Ambient_occlusion)

**Marked as my own knowledge, not fetched:** the characterisation of what a
landscape photograph looks like as an image (that beauty in landscape
photography is dominated by raking light, that foregrounds are vegetated, that
real rock faces carry stain and lichen). Treat these as a prior to be checked
against a real photograph when someone can fetch one, not as evidence.

---

## 4. The difference list in full

Items already ranked in §1 are marked. The rest are real and smaller.

| # | What the eye sees | Our mechanism | Category | Cost | Payoff |
|---|---|---|---|---|---|
| §1.1 | Nothing rises, overhangs or stands proud | `cliff_edges` gate starves `brows`/`talus`/boulders | **worldgen** | large | largest |
| §1.2 | No living thing on screen | `life_scatter`: 343 seeds, 5 of 6 species ungerminated | **content** | small | very large |
| §1.3 | Caves are cracked glass | `worley_f2_f1` = Voronoi edge set (`passes.rs:2142`) | **worldgen** | medium | large |
| §1.4 | Everything is one colour | `.ron` tones are value ramps, no hue rotation | **render** | very small | large |
| §1.5 | Cave air is dead flat black; only the sky is lit | `cell_colour` reads no neighbour; cave fade floors to a constant | **render** | small | medium |
| 6 | Strata are perfectly horizontal bands | `strata_offset` gives dip and fold, but the *visible* result on a screen is level banding | worldgen | medium | medium |
| 7 | No vertical staining anywhere | nothing writes a downslope stain channel; desert varnish and seep streaks are the real referent | render or content | small | medium |
| 8 | Pale blobs read as camouflage, not as rock | `Purpose::RockFacies` patches have soft blobby boundaries and no bedding-parallel elongation | worldgen | small | medium |
| 9 | No depth: everything is in one plane | `TreeDepth::Weave` already assigns a per-tree depth bit that nothing reads; no atmospheric perspective anywhere | render | small | medium |
| 10 | The grain is uniform salt-and-pepper at every distance and on every material | one `JITTER_STRENGTH` for all non-liquids | render | very small | small |
| 11 | Water is a flat blue line | (not investigated here; waterfalls postponed by the owner) | — | — | — |

**#6, #7 and #8 are the cross-section items**, and between them they are what
would turn our wall from a diagram into a photographed cliff — see §0, which is
the same argument stated once properly.

**Two of them are already being built by another lane, which is worth knowing
before anyone picks them up.** While this lane was measuring, the
rock-vocabulary prototype appeared in this checkout: new `sandstone.ron`,
`limestone.ron`, `mudstone.ron`, `basalt.ron` and `ironstone.ron`, plus a
*weathered* tone family in `stone.ron` applied to cells within a few cells of a
free face, behind `PIXEL_PHYSICS_ROCK_VOCAB` (default on). That is #8 and part
of #1.4 landing from a different direction, and it is the right shape — real
rock types with real hardness differences are exactly the input §1.1 needs in
order to make beds stand proud by different amounts. **The coupling from
hardness to relief is the piece nobody is building yet**, and it is the one
that turns a palette change into a geometry change.

---

## 5. What I would do, in order

**Revised after the owner answered — see §6a.** The first version of this list
led with the palette on a cost/payoff argument; he answered *"shape"* when
asked directly whether the problem was colour or shape, and *"you are focusing
on lighting which is not the issue, it is the build"*. Shape leads.

1. **Relief, on the wall and at the skyline (§1.1).** The revamp, and the only
   item that unlocks others. Couple bed hardness to how far a bed stands proud
   of the face, so the cross-section becomes a cliff/ledge/bench stair rather
   than a plane; that alone lifts terrain past `cliff_edges`' threshold and
   switches `brows`, `talus` and the boulder pass back on for free. **Massive
   forms, not spindles** — the owner named *"large rock formation (not just
   tall pillars)"*, so the thin vertical columns already in every strip are the
   failure mode to avoid, not the goal.
2. **Rebuild the caves (§1.3), Lane E's design.** *"Remove the web"*, and
   bigger: he asked for rooms 3–7x the current chamber, chained so you can walk
   from one to the next. **And open some of them to the sky** — *"cave
   openings"* was in his answer and nothing in worldgen produces one today.
3. **Sow a grown world (§1.2).** Cheap, and the only item that adds a colour
   the ground does not otherwise carry. It also triples what any later lighting
   work can reach (§2.4).
4. **The palette (§1.4).** Still the best cost/payoff ratio in isolation — half
   a day of `.ron` editing, no code — but demoted, because the owner has twice
   said colour is not the issue. Do it, do not lead with it.
5. **`ink` plus a cave-void falloff (§1.5, §2.4).** Cheap render work. It
   should follow 1 and 2, because both make it worth several times more.

**Do not start with lighting.** That is the finding this lane was asked to
test; it is now confirmed both by measurement (§2.2) and by the owner directly
(§6a).

---

## 6. What this lane did not establish

- **No photograph was seen.** §3. The whole reference half of the brief is
  weaker than intended and cannot be fixed from this container.
- **The shading parameters were set by eye on one world**, not swept. They are
  a demonstration, not a calibration — the same caveat
  `subpixel-rendering-2026-08-29.md` §7 puts on its own.
- **`terrain_shade` says nothing about shipped cost.** It runs `f32` per pixel
  over a whole frame with no dirty-rect skip and no chunk-local access. The real
  number for the AO family is the one in `cell_colour`'s comment (~10 ms, 512x320
  stress scene) and the fix is named there.
- **Nothing in `src/` was changed.** This is an instrument and a measurement.
- **Waterfalls were not looked at**, per the owner's instruction to postpone
  them.
- **`ao` was not tested with a surface-relative normalisation.** As written it
  reads absolute enclosure, which in a solid massif is constant. A version that
  measured occlusion *relative to the local mean* would behave differently and
  was not tried; I do not expect it to change the conclusion, because the
  quantity it would still be reading is boundary density, but it is an untested
  variant rather than a rejected one.

## 6a. The owner answered, and he settled it

All three cards came back within four minutes of posting, and between them they
confirm §2 and **overturn my §5 ordering**. Recording that here rather than
quietly editing the recommendation, because the correction is the useful part.

**On the underground lighting A/B** (blind; he picked the **lit** pane, so he
could tell them apart):

> *"A is interesting, but but not convinced yet. Still you are focusing on
> lighting which is not the issue. **it is the build**"*

That is §2.5 in the owner's own words, and it arrived independently of the
measurement. The hypothesis this lane was asked to try to kill is dead, and it
is dead for the reason the boundary census gave: *the build* — the shape of the
thing being lit — is the binding constraint.

**On the surface lighting A/B** (blind, asked *"can you tell which pane has it
on at all?"*): he chose **`shipped (flat)`**. He did not pick out the lit pane.
The surface null is confirmed by eye as well as at 4.7%.

**On the five presets**, asked *"is what's missing colour variety, or is it
that the ground has no shape?"*:

> *"**Shape**, large rock formation (not just tall pillars), cave openings,
> mountains."*

Four concrete nouns, and none of them is a colour. Two of them are things this
report did not have on its list at all:

- **"cave openings"** — our vaults sit 200+ rows down and are sealed; nothing
  in worldgen opens a cave to the sky. A cave you can *see* from the surface is
  both a landform and an invitation, and it is currently impossible.
- **"not just tall pillars"** — the thin vertical stone columns visible in
  every strip in §1.1 are being read as the *failure mode* of adding relief.
  Whatever produces relief must produce **massive** forms, not spindles.

**What this changes.** §5 led with the palette on a cost/payoff argument. The
owner has now twice said colour is not the issue — here, and on a separate
destruction card (*"color is not the issue"*). The palette work is still cheap
and still worth doing, but it is **not the thing to lead with**, and a session
that reads only §5 would have started in the wrong place. The revised order is
below.

## 7. Review cards posted

Board `worldgen`, all fire-and-forget:

| card | question | answer |
|---|---|---|
| `20260829T171510934Z-d57fca` | All five presets — colour, or shape? | **"Shape, large rock formation (not just tall pillars), cave openings, mountains."** |
| `20260829T171528452Z-31aa76` | Lighting the rock underground (blind A/B) | picked **lit**; *"not convinced yet. Still you are focusing on lighting which is not the issue. it is the build"* |
| `20260829T171542736Z-ce8024` | Lighting at the surface (blind A/B) — can you tell it is on? | picked **`shipped (flat)`** — did not pick out the lit pane |
| `20260829T173854008Z-e538d5` | The same world grown (before/after, daylight pinned) | not yet answered |

The pair of shading cards was the experiment: the same knob at 22% boundary
density and at 5%. The prediction was that he would pick the lit pane
underground and fail to pick it at the surface, and that is what happened — so
§2 is confirmed by eye as well as by measurement. He also volunteered the
reason unprompted (*"it is the build"*), which is the finding rather than the
verdict.

## 8. Instruments

- **`examples/terrain_shade.rs`** (new) — A/B testbed for shading the ground
  over the shipped world at play scale. Arms `base|ao|sun|ink|aosun|flat|full`,
  colours from the shipped `Renderer`, occupancy from `World`. Prints the
  boundary-density census (*"the ceiling on what any shading term can reach"*)
  and per-arm "how many subpixels did this actually move", so a null is
  distinguishable from a disconnected knob. Echoes its own parameters.
- `examples/viewshot.rs` — used unchanged for every strip and every preset
  render here.
