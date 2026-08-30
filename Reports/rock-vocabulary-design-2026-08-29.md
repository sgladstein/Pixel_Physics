# A rock vocabulary: six rocks instead of one tint

Lane F of the worldgen revamp, 2026-08-29. Written against
`claude/worldgen-revamp-plan-dot67g`.

`Reports/worldgen-appearance-audit-2026-08-29.md` measured that three colour
passes are 87% of the player's view, that 94.6% or more of the solid cells in
a viewport are one of four minerals, and that **four to nine colours cover
half of everything the player sees below the skyline**. This lane owns the
first of those three passes — what the rock *is*.

**A working prototype is on this branch**, gated behind
`PIXEL_PHYSICS_ROCK_VOCAB` (and `PIXEL_PHYSICS_ROCK_WEATHER`,
`PIXEL_PHYSICS_ROCK_DAMP`), so every number below is a paired A/B in one
process, one binary, one machine. §9 says which parts are prototype-quality
and which are shippable as they stand.

---

## 1. The finding, stated first

**`assets/materials/stone.ron` is the entire geology of the world**, and its
one byte of `Cell::shade` is doing two unrelated jobs at once. The *tone*
inside a family is the sedimentary band index, which is right. The *family*
is a region tint chosen by a 2-D fBm blob — and a blob is the wrong shape for
a rock, because it cuts **across** the bedding. Rendered at play scale
(`target/rockvocab/base-*.png`) the pale cap-rock family reads as camouflage
blotches on a grey wall, not as geology.

So the proposal is two things, and only the first is large:

1. **A bed picks a material, not a tint.** Six rocks, differing in colour,
   in hardness, in how coarsely they calve, and in their joint fabric.
   Decided per `(bed, column)` and **never per cell**, so a bed is one rock
   from its floor to its roof.
2. **Two extra palette families per rock** — weathered and damp — written into
   the same shade byte at genesis, so they cost nothing per frame.

Measured over 6 presets x 3 seeds x 16 viewports at the shipped 8192x2560
world size, rendered through the shipped `Renderer` with daylight pinned at
noon:

| | pooled % of the player's viewport | range | `flat` |
|---|---|---|---|
| **the six rocks alone** | **24.89%** | 18.72 – 34.24% | **0.00%** |
| + the weathered skin | +0.65% | 0.14 – 1.48% | 0.00% |
| + damp below the water table | +19.52% | 0.00 – 34.00% | 0.00% |
| **all three** | **28.49%** | 22.07 – 40.05% | **0.00%** |

Set beside the audit's own pass table, **the six rocks alone move more of the
player's view than any generation pass except `stone_massif` itself** —
larger than `soil_blanket` (19.02%), larger than `soil_moisture` (18.38%),
and roughly forty times the entire landform programme of the last six rounds
(`brows` + `talus` + `residuals` + `boulders` + `vaults`, at most 0.6%
between them).

And it is **cheaper**. `stone_massif` is the most expensive pass in the
generator; over four alternating paired runs it goes from a median **2,109 ms
to 979 ms**, writing the identical 18,737,895 cells. §8.

### 1a. "Colour is not the issue" — read against this

While this lane was running, the owner answered Lane A's cards and said it
twice: asked whether what is missing is colour variety or shape, he wrote
**"Shape, large rock formation (not just tall pillars), cave openings,
mountains"** (`Reports/worldgen-visual-interest-2026-08-29.md` §6a, which
reorders its own recommendation to put shape first and palette fourth).

Three things about that, and none of them is a defence of a palette change.

**It was said about a different kind of change.** Every colour A/B he has
been shown — nine of them, catalogued in the audit's §2.2 — varies a *tint*
on the one rock: the palette family, the deep-rock grain, the formation
palette. He has never been shown a world made of more than one rock, and
whether that is the same question is precisely what **review card 1** asks.
It is queued and unanswered at the time of writing. If he says "no
difference" to that as well, this report's §5 is a correct measurement of
something that does not matter, and the honest thing is to say so now rather
than after.

**A bed is shape at the scale a cliff is read at.** The layer boundaries in
these renders are horizontal structure across the whole face — a hard band
standing proud over a soft one that has retreated. It is not the macro
silhouette he is asking for, and it is not nothing.

**And it is the prerequisite for the macro silhouette.** "Large rock
formation" at the scale of a butte or an escarpment is *differential
erosion*: hard rock outlasting soft rock. Today the world cannot do that at
all. `Character::resistance` multiplies **terracing** — a heightfield trick
applied to a surface curve — while every cell of the massif is exactly as
strong as every other, because there is one material and it has one
`max_unsupported_span`. This change is the first time the world contains
hard rock and soft rock *as materials*: 42 cells of attached reach against
280, 0.55 blast resistance against 1.6, grit against slabs. Whoever builds
the shape work asked for needs that to exist first, and it does not today.

So: **ship this for what it is measurable as** — the largest single change
available to what the ground looks like, at a large *saving* in generation
time — and treat the owner's ranking as correct about where the next lane
should go, not as a reason to leave one rock in the ground.

---

## 2. The set

Six rocks. `stone` is unchanged and is still the reference: the brush's
material, what a random shade pick lays, and the rock every one of the other
five is authored *relative to*. Every field below departs from `stone.ron`
and the departure is what is argued; the shared reasoning stays in
`stone.ron`, which is where it already is.

| | mean value | `max_unsupported_span` x `attached_span_bonus` | `blast_resistance` | fragment ladder | `joint_spacing` | weathers |
|---|---|---|---|---|---|---|
| **basalt** | 58 | 22 x 14 = **308** | 1.6 | {4,8,16,32,64} | **9** | to a brown rind |
| **mudstone** | 86 | **7 x 6 = 42** | **0.55** | **{2,4,8}** | **6** | pale, it slakes |
| **ironstone** | 104 | 20 x 13 = 260 | 1.5 | {8,16,32,64} | 17 | **brighter — rust** |
| **stone** | 124 | 16 x 12 = 192 | 1.0 | {2,4,8,16,32,64} | 13 | duller grey |
| **sandstone** | 148 | 13 x 11 = 143 | 0.8 | {4,8,16,32,64} | 15 | darker — varnish |
| **limestone** | 178 | 20 x 14 = **280** | 1.2 | **{8,…,256}** | **21** | karst grey |

**Visually**, the set is chosen to span two axes the world currently has none
of. Value: today's ground sits between 92 and 192 and the set reaches 40 at
one end and 192 at the other. Hue: **there is no red anywhere in the mineral
world**, and ironstone is the entry that is not a grey, a brown or a tan.
Both are deliberately *rare* — a marker bed you see everywhere marks nothing
(§3).

**In the hand**, each one is a different thing to mine, and the levers are
ones the engine already reads:

- **mudstone is the bed that goes first.** 42 cells of attached reach against
  stone's 192, so a drift driven into it brings its own roof down at a
  quarter of the span; `support_cost_beside: 2` and `above: 4` so a mudstone
  lip peels off a face rather than hanging. Its ladder is {2, 4, 8} and
  `rigid::MIN_FRACTURE_CELLS` is 6, so **two of its three rungs cannot become
  a body at all** and most of what comes off it is grit. That is the intended
  reading, and `filmstrip`'s `crumbled to grit` column is what to read to
  check it, not the mean region size (CLAUDE.md).
- **limestone is the cap.** 280 cells of attached reach, and a ladder from 8
  to 256 — every rung above `MIN_FRACTURE_CELLS` and the top near
  `MAX_BODY_CELLS` (400) — so a struck limestone face throws pieces that read
  as *a piece of the wall*. It is the far end of the axis mudstone holds the
  near end of, and the two together are what makes a bench a bench: hard rock
  standing over soft rock that has already retreated.
- **basalt is the floor and the sill**, hardest and darkest. `joint_spacing`
  9 against stone's 13, so a blast in it finds a plane every few cells and
  the halo comes apart into columns rather than blocks.
- **ironstone is the marker.** Hard enough (`blast_resistance` 1.5) to stand
  proud of the softer beds either side as a rib on the face, and blocky —
  {8, 16, 32, 64}, four large rungs, no dust rung, so it comes off in lumps
  or not at all.
- **sandstone is the ordinary bedded rock of dry country**, a little weaker
  than stone in every direction. Its four fresh tones are byte-for-byte
  `stone.ron`'s retired family 2.

**`joint_band_contrast` is left at 0.0 on all six**, including basalt, where
columnar jointing is the one place a varied pitch would genuinely earn its
keep. Its own doc measures it at **10–14 ms on the worst frame**, and frame
cost is a hard constraint here rather than a tiebreaker. It is the first
thing to turn on if a blast in basalt ever needs to read differently from a
blast in stone, and it should be measured when it is.

**`heat_conductivity: 0.1` on all six, matching stone**, deliberately. Four
of them are never a reaction product, so 0.0 would be legal and slightly
cheaper — but it would also make the A/B a thermal change as well as an
appearance one, and a basalt wall beside lava that can never warm is a
wrongness bought for nothing measurable. Dropping it for the four is a free
saving available later, on its own measurement.

---

## 3. How a bed gets its rock

**A pure function of the column plan and the band index. Nothing reads the
world.** The decide/realise split is what makes `pass_ablation` possible and
this does not spend it.

```
rock(band, x) =                      // never (band, x, y)
    marker(band)                     // ~4.5% of beds: an ironstone rib or a basalt sill
    or basement(band)                // below 0.80 of world height: basalt
    or menu(aridity)[ bucket(rank) ]
rank(unit, x)  = unit_draw(unit) + facies_dither(unit, x) + 0.22*(resistance - 1)
unit(band)     = floor( band/1.8 warped by a smooth per-unit noise )
```

Five things about it, in the order they matter.

**1. `y` is not a key, and that is the whole correction.** The existing
`palette_family` dithers per `(x, y)`, which is right for a tint and wrong for
a rock: it makes the boundary between two rocks an amorphous blob cutting
through the bedding. Dropping `y` makes a bed one rock top to bottom while
still letting it interfinger with its neighbour *along strike*, which is what
a facies change actually is.

**2. The bed's rank is global.** `unit_draw` is keyed on the unit index alone,
so a bed holds its rank across the whole world — the same property the tone
draw already has, and the reason a layer is followable from one cliff face to
the next. What varies laterally is which rock that rank *shows up as*, which
is what a facies is.

**3. Aridity picks the rock, it does not move the rank.** So a dry country
and a wet one have the same *number* of hard beds and soft beds, made of
different rock. Measured shares of the solid cells in a player's viewport,
seed 1, one arm:

| preset | the rock, as the player sees it |
|---|---|
| arid | sandstone 42.8, mudstone 24.7, limestone 17.7, ironstone 3.6, basalt 1.2 — **no stone at all** |
| canyon | sandstone 36.3, mudstone 23.4, limestone 16.5, stone 8.6, ironstone 2.2, basalt 1.7 |
| rolling | sandstone 24.1, mudstone 21.3, limestone 14.6, stone 12.2, ironstone 3.2, basalt 1.0 |
| terraced | mudstone 23.1, stone 20.7, sandstone 19.9, limestone 12.1, basalt 1.5, ironstone 1.5 |
| wetland | stone 19.3, mudstone 14.5, limestone 7.0, sandstone 1.6, ironstone 1.3, basalt 0.2 |

Against the audit's baseline — stone 60.7–100% in every preset — that is the
composition finding answered directly.

**4. Marker beds are drawn per *bed*, on their own stream.** Drawn on the unit
stream alongside the ordinary rocks a marker inherits the unit's thickness,
and the first rendering of this had a **60-cell rust formation** rather than
a rib. 3.0% of beds are ironstone and 1.5% are a basalt sill; at
`strata_thickness` 8–12 that is a rib roughly every 30 beds.

**5. A rock *unit* spans several beds, and giving the rock real contrast is
what forced that.** Every bedding plane in this world is exactly
`strata_thickness` apart — dead parallel, no exceptions — and the old
low-contrast palette was hiding it. Six distinct rocks at one rock per bed
came out as a **layer cake**: the wallpaper failure `strata_shade`'s own note
warns about, arriving through thickness instead of through period. A unit
averages 1.8 beds and the count is warped by a smooth noise, so unit
thickness varies while the bedding inside stays regular and still draws its
own tone. **This is the one thing I could not settle by measurement and it is
review card 2.**

### What it costs

Nothing per frame, and **it is a large saving at genesis**: §8. The two
mechanical costs are

- **`ColumnShade` re-decides the rock once per band crossing**, exactly like
  the tone draw it sits beside, so the per-cell work is one comparison.
- **`stone_massif` must cut its `fill_run` at material changes.** `fill_run`
  asserts one material per run and already re-enters the chunk map every 64
  rows; this raises the entries per column from `height/64` to
  `height/64 + material changes`. It is inside the 2.2x saving below, so it
  is not separately visible.

**Measured legibility.** Along a horizontal line of the rendered picture, the
longest run of one rock is a **median of 42–85 columns and a maximum of 296**,
out of a 512-column viewport (four worlds, ~24 rows each, classified from the
PNGs by nearest palette entry). Read that as *how far the picture stays in one
rock*, **not** as "how far a bed holds its rock": the strata are displaced by
`strata_offset(x)`, so a horizontal scan line leaves a bed as well as
crossing facies boundaries, and this number cannot separate the two. The
mechanism's own scale is regional — the dominant lateral term is
`0.22*(resistance - 1)`, and a region is 102–256 columns at the shipped size.

---

## 4. Weathering and staining

`Cell::shade` is one byte, written once at genesis and read by `render.rs` at
no frame cost. **Every proposal in this section keeps that property**: they
are palette families, `family * 4 + tone`, chosen at genesis from data the
column plan already holds. None of them adds a per-frame read, a field
sample, or a redraw.

### Cheap, built, and measured

**The weathered skin — 0.65% of the view.** A cell within `WEATHER_DEPTH`
(5) of open air takes family 1. **Distance to air, not depth below the
surface**, and the difference is the point: on a cliff the air is *sideways*,
so measuring depth alone weathers the lip of a face and leaves the face
itself fresh, which is backwards. Implemented per column as a small
lower-envelope over the plan surfaces of the 11 columns within reach, with a
single-comparison early out below the local deepest surface.

Each rock weathers in **its own direction**, which is what makes a mixed face
read as two rocks rather than one rock under a gradient: sandstone darkens
(varnish), limestone dulls (karst), basalt goes brown, **mudstone goes
paler** because it slakes rather than case-hardening, and **ironstone goes
brighter** because it is the one rock that gains saturation with exposure.

Its real value is not the static share. It is that **a fresh cut reads as a
fresh cut** — mine into a hillside and the colour steps once, at the depth
the weather stops. That is a verb delivering something, and it is not what a
genesis-time census measures. At 0.65% it is nonetheless the *smallest* item
here, and honestly it sits alongside `brows` (0.59%) on the audit's own
scale.

**Damp rock below the water table — 19.52% of the view, and the largest
single item in the proposal.** Rock at or below the column's `table_y` takes a
third family: family 0 multiplied by (0.78, 0.79, 0.83). Derived rather than
eyeballed, so it can be re-derived — a water film kills the diffuse scatter
off the grain and loses least at the blue end, so one multiplier per channel
says the whole thing and no rock needs its own artistic judgement about being
wet.

**Its specificity control is clean and worth stating**: it moves **exactly
0.00% on `arid`**, which ships `table_offset: 4000.0` and therefore has no
water table in the world at all, and 34.00% on `wetland`. A number that fires
only where there is water is a number keyed on water.

**But it is the one item I would not ship unreviewed.** It moves an enormous
number of pixels by pulling a very large area toward one value: mean ground
luma on `rolling` goes 112.5 → 97.5, and the count of colours covering half
the ground goes **down** on three of five presets (rolling 15 → 13, terraced
12 → 11, wetland 7 → 6). That is a large pixel share bought partly at the
expense of the variety this lane exists to add. **Review card 3.**

### Cheap, designed, not built

Each of these is the same shape — one more genesis-time family, or a
different rule choosing between the families that exist — and each needs a
pixel share measured before it is worth building. The instrument is
`world_look mode=vocab`, which now takes a stage name and prices one item at
a time.

- **Seep staining below a spring.** `passes::springs` already knows where the
  aquifer daylights (109 cells written, 0.008% of the view). A vertical wash
  of the damp family below each seep is a handful of columns from a plan
  number. Expected share: small, but every pixel of it is on a face the
  player is looking at.
- **Lichen and moss on damp faces.** Not a family — a *material*. `moss` is a
  shipped organism and `life_scatter` already sows it; the audit measured
  moss at 0.4% of the skin. This is a `life_scatter` density question rather
  than a rock question, and it belongs to whoever owns that pass.
- **Dust on ledges.** A horizontal surface with air above it takes the pale
  end of its own palette. Free — `Exposure` already knows which cells are
  within reach of air and which direction it is in.
- **Bleached crowns.** A cell above a per-region elevation takes the palest
  tone. One comparison, and it puts a value gradient on the macro relief that
  nothing currently does.

### Expensive, and named so it is not tried by accident

- **Anything keyed on a live field read.** Wet rock that tracks a *moving*
  waterline, staining that follows the moisture field, temperature-tinted
  rock. `Reports/world-review-2026-08.md` §7.21 records the precedent: a tint
  keyed to a live field forces full redraws for ever and defeats the
  dirty-rect skip, which is exactly where the render budget lives on a
  settled world.
- **Rock decay / weathering as a `decay.rs` process.** `decay_chance_damp` and
  `decays_into` exist and would express crumbling mudstone beautifully. It
  would also put ~19 M stone cells on a per-frame decay path. Do not.
- **Per-rock rubble.** `stone.ron`'s `breaks_into` is `rubble`, whose colour is
  deliberately stone's greys so a collapsed span lands in the colour it fell
  in. Under a vocabulary, a limestone cliff collapses into grey rubble. The
  fix is a `breaks_into` per rock and five more debris materials, and it is
  not free — see §7 for a live bug on that path.

---

## 5. Every item, with its pixel share

The unit is **a rendered pixel in a player's viewport**, never a cell, for
the reason the audit gives: the two differ by two orders of magnitude and
only one of them reaches the owner. Both arms are built in one process from
one binary; cameras are computed from the OFF arm and reused, so a bed that
changes colour cannot be scored for moving the camera.

| item | pooled | arid | canyon | flat | rolling | terraced | wetland |
|---|---|---|---|---|---|---|---|
| six rocks | **24.89%** | 26.8–30.3 | 31.5–34.2 | **0.00** | 30.5–32.4 | 31.7–33.4 | 18.7–29.2 |
| weathered skin | 0.65% | 0.51–0.54 | 0.93–1.48 | – | 0.55–0.91 | 0.48–0.66 | 0.14–0.27 |
| damp below table | 19.52% | **0.00** | 10.3–11.5 | – | 29.3–29.5 | 29.2–29.8 | 21.6–34.0 |
| **all three** | **28.49%** | 27.0–30.5 | 34.0–40.1 | **0.00** | 36.9–38.4 | 37.4–39.7 | 22.1–35.2 |

And what it does to the palette, which is the audit's own headline statistic:

| | colours covering half the ground (`b50`) | colour bins occupied |
|---|---|---|
| baseline | **4 – 8** | 104 – 192 |
| six rocks | **5 – 14** | 102 – 191 |
| all three | **5 – 14** | 112 – 224 |

**The rocks carry all of the variety and the damp family carries none of
it.** `b50` rises on every real preset with the rocks in (terraced 5 → 12,
rolling 7 → 14) and then *falls back* when damp is added on three of five.
That is the trade stated as a number.

**Speckle is essentially unchanged** — 8.82 → 9.32 on rolling, 9.62 → 9.69 on
canyon — which is the check that matters most for reading this honestly. The
audit found that 73–92% of the world's apparent surface texture is
`render.rs`'s per-cell grain rather than geology, so a change that improved
the picture by adding *noise* would show up here. It does not. This is
palette, not grain.

### Controls

| control | asks | result |
|---|---|---|
| `flat`, 3 seeds | can this report **zero**? `region_variation = 0` is exempted by the same rule `palette_family` uses | **0 pixels moved, all 3 seeds, all stages** |
| `arid`, damp stage | does the damp family fire only where there is water? | **0.00%** — no water table in the world |
| the whole integration suite, arm OFF | is the control arm really the shipped behaviour? | **44 of 44 pass** (7 fail with the arm ON — §7) |
| cell counts, both arms | did the faster pass skip work? | **18,737,895 cells written in both arms** |

---

## 6. What a player sees change

Stated in the vocabulary of the world, because that is the language the wiki
uses and the language the owner judges in.

- The ground stops being grey rock with brown patches on it and becomes
  **layers**: a pale hard band you can follow along a cliff, a dark soft one
  under it, a rust rib you can find again on the next face.
- Dry country and wet country are made of **different rock**, not the same
  rock in a different tint. There is no grey stone at all in the desert.
- Digging into a hillside **changes the colour**, because the outside of the
  rock is weathered and the inside is not.
- Ground below the water line is **wet**, and there is a line where it stops.
- Cutting into a soft bed brings the roof down sooner than cutting into a
  hard one, and the pieces that come off are a different size.
- Deep mining eventually arrives somewhere: below four fifths of the world's
  height every bed is basalt.

---

## 7. Interactions, and what broke

**The brush and every random shade pick are unaffected, and that needed no
new machinery.** `base_colors: 4` on all six rocks caps a random pick to
family 0 — fresh rock, one flat family, never confetti of fresh and weathered
and damp. `stone`'s `base_colors` is unchanged at 4 and its family 0 is
byte-for-byte what it always was, so **a wall a player paints or builds is
bit-identical to what it has always been**. That is the guarantee the brief
asked for and it falls straight out of the existing field.

**Five entries appear in the material picker.** `app.rs`'s `paintable` is
built from the registry, so mudstone, sandstone, limestone, ironstone and
basalt are now paintable. That is a feature; it also means a player can build
in six rocks that genuinely differ structurally, which is the first time the
brush has had a hardness choice in it.

**"Is this the massif?" had to become data.** Six passes asked
`material == ctx.stone` to mean *intact country rock*, a question with
exactly one right answer while there was exactly one rock. With six it
silently means "is this specifically the grey one". A `rock: bool` on
`Material`, read at the call site that already holds the `Cell` (a `Vec`
index, per CLAUDE.md's hot-path rule), replaces all of them. `bedrock` is
deliberately **not** rock.

**Two of those were real bugs, not test assumptions**, and both were invisible
until a test caught them:

- **`pockets` placed no gravel lens at all.** Its seal requires every
  neighbour of every lens cell to be stone; in a world of six rocks that
  essentially never holds.
- **`boulders` seated nothing.** Its socket walk digs down until the column
  "threads real rock" and only recognised `stone`, so it fell through
  `MAX_SOCKET_DEPTH` every time. (The audit separately reports
  `boulders-seated 0` in the *shipped* build for a different reason, still not
  diagnosed.)

**Seven integration tests encoded "stone is the only rock".** All seven pass
with the arm off and failed with it on; all seven are now updated and green.
They are worth listing because they are the shape of what a landing has to
touch:

| test | what it assumed |
|---|---|
| `a_forced_vault_world_is_sealed_and_arrives_at_rest` (x2 assertions) | the vault envelope was `MaterialId(2)` |
| `every_solid_is_anchored_and_no_liquid_carries_a_stale_fill` | counted `stone` cells for its vacuity guard; went vacuous on `arid`, which has none |
| `a_varied_world_uses_more_than_one_rock_family` | variety can only be a palette *family*; reported "every rock cell is in family {}" over an empty set |
| `a_seated_boulder_stands_at_a_believable_height` | identified a boulder by `stone` + cap-rock family |
| `a_forced_boulder_world_seats_stone_and_arrives_at_rest` | (the real `boulders` bug above) |
| `buried_gravel_is_not_the_same_colour_as_scree` | (the real `pockets` bug above) |
| `a_generated_world_grows_a_spring_that_actually_runs` | downstream of the two above |

**A boulder keeps a fixed rock, and it is the one exception in the set.**
Painting a boulder with `strata_rock_at` — the bed it is standing in — was
tried first and is wrong: a mudstone boulder in a mudstone bed is invisible,
and the height guard's measured p50 fell from **11 to 2** because it could no
longer find one. A boulder is a hard-band survivor by construction
(`erosion.rs`'s `BOULDER_HARDNESS` gate), so it is limestone: the honest
answer and the legible one.

**A live bug found on the way, unrelated to this lane and left alone.**
`rigid::convert_to_debris` draws a debris shade with
`rng.below(palette.len())` where the brush uses `base_colors`. It is harmless
today because every `breaks_into` target has one family — but it means that
the moment any material breaks into a multi-family one, **a collapse comes
out as confetti**, which is precisely the failure `base_colors` exists to
prevent, on a code path that never consults it. Anyone adding per-rock rubble
(§4) hits this first.

---

## 8. Cost

**Genesis gets substantially cheaper.** Four alternating paired runs, one
binary, `rolling` at 8192x2560:

| | arm OFF | arm ON |
|---|---|---|
| `stone_massif` | 2,133 / 2,103 / 2,115 / 2,086 ms | **968 / 985 / 986 / 977 ms** |
| whole pass table | 3,016 / 3,034 ms | **1,778 / 1,820 ms** |
| cells written | 18,737,895 | **18,737,895** |

**A cost that vanishes may be work that vanished** (CLAUDE.md), so: the cell
count is identical in both arms, and the mechanism is exactly the work this
change set out to remove. `ColumnShade`'s own doc names it — of the four
things `strata_shade` evaluates per cell, "only the last of those genuinely
varies per cell", and that last one is `palette_family`'s **two 2-D fBm
samples, five noise evaluations per cell over 18.7 M cells**. Replacing a
per-cell tint with a per-bed material deletes them. The extra `fill_run`
segments and the weathering test are inside what is left.

**Frame cost is unchanged, by construction and by check.** `render.rs` is
untouched; the shade byte is written once at genesis and read exactly as
before. `heat_conductivity` matches stone, so no cell moves on or off
`fire::update`'s thermally-inert fast path.

- `cargo test --lib` — **1040 passed, 0 failed, 54 ignored**
- `cargo test --test worldgen --test determinism` — **44 + 2 passed, 0 failed**
- `cargo +1.98.0 clippy --all-targets -- -D warnings` — clean (CI pins 1.98;
  the container ships 1.94)
- `bash scripts/acceptance.sh` — all cases met their expectations
- `cargo run --release --example ascii` — 31 scenes, 0 skipped

---

## 9. What to ship first, and what is prototype-quality

**Ship the six rocks.** 24.89% of the player's view, `flat` byte-identical,
the most expensive pass in the generator 2.2x faster, every gate green. It is
the single largest visible change available in the worldgen programme and it
is the cheapest.

**Ship the weathered skin with it.** 0.65% is small, but it is what makes a
cut read as a cut, and it is one comparison per cell with an early-out.

**Hold the damp family for the owner's verdict** (review card 3). It is the
largest item by pixels and the only one that reduces palette variety.

Three things are prototype-quality and would change before landing:

1. **`stone.ron` keeps its three retired region-tint families** (1 cool damp,
   2 warm sandstone, 3 pale cap-rock) purely so
   `PIXEL_PHYSICS_ROCK_VOCAB=0` restores the shipped world byte for byte for
   the A/B. Landing deletes them — 2 and 3 have become `sandstone.ron` and
   `limestone.ron` — and the weathered and damp families move from indices 16
   and 20 down to 4 and 8, which is where they are on the other five rocks
   already. `weathered_base` and `damp_base` exist only to carry that
   asymmetry and both disappear with it.
2. **The three env switches** (`PIXEL_PHYSICS_ROCK_VOCAB`,
   `_WEATHER`, `_DAMP`) and `passes::set_rock_vocab` /
   `set_rock_weather` / `set_rock_damp` are measurement scaffolding. Keeping
   them is defensible — they are what `world_look mode=vocab` uses and what
   any re-measurement would need — but they are three atomics read per bed
   and they should be one.
3. **`residual.rs` and `brows` follow the bed** via `strata_rock_at`, which
   recomputes the band, the character and the rock per cell rather than using
   `ColumnShade`. Those passes together are under 1% of the view so it does
   not show, but it is the per-cell shape this change removed from
   `stone_massif` and it should not be reintroduced quietly.

---

## 10. What I could not establish

- **Whether the bedding is too regular.** Review card 2. The unit warp is a
  judgement made by eye off four renders; the layer-cake failure it fixes was
  real and obvious, but where the setting *should* sit is not something a
  histogram answers.
- **How far a bed actually holds one rock.** §3 measures the picture, not the
  bed, and says why the two differ. The clean measurement needs the band
  offset from inside the generator, which is a print away and was not worth a
  build cycle here.
- **What any of this does to how destruction *feels*.** The hardness numbers
  are argued from the levers' own docs and from a value ladder; not one of
  them has been swung at. `scripts/acceptance.sh` is green, but acceptance
  builds hand-placed `material::STONE` geometry, so **it cannot see any of
  these rocks** — it is a check that nothing broke, not evidence that mudstone
  crumbles and limestone calves. The instrument for that is
  `filmstrip scene=worldcrack` with a seed sweep, read at the order statistic
  and run to rest (CLAUDE.md's cascade rule), and it is the obvious next
  measurement.
- **Seed count.** 3 seeds for the headline stage, 2 for the three
  decomposed ones, 16 viewports each. CLAUDE.md's bar is that six is not a
  sweep. The effect sizes here are 25–28% against a `flat` control of exactly
  0 and are not at risk from that; the *per-preset* spread (arid 26.8–30.3
  against wetland 18.7–29.2) is the number that would move.
- **Vegetation.** Every figure here is a statement about the mineral world,
  for the audit's own reason: `life_scatter` sows 343 one-cell seeds in an
  8192-column world and five of six species have not germinated at the
  timescale any of these renders were taken at.

---

*Freshness: written 2026-08-29. Every figure is reproducible from
`examples/world_look.rs mode=vocab` (stages `rocks`, `weather`, `damp`, and
the default), `PASS_TIMING=1` for §8, and the material `.ron` files, at the
commit that carries this file.*
