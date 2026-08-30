# Worldgen: what the architecture cannot express

2026-08-29. Lane B of the worldgen revamp program. An audit, not a fix: no
`src/` behaviour was changed. One read-only instrument was added
(`examples/wg_ceilings.rs`).

The question is not "what is broken" — `Reports/open-bugs-handoff.md` owns
that — but **what no amount of tuning inside this pipeline can produce**, and
which of the owner's standing complaints each ceiling explains. A ceiling with
no complaint attached is not in this document.

Six rounds of worldgen work have each ended with the owner saying it is still
not right. The finding of this audit is that this is not six failures of
execution. **Every one of those rounds tuned a scalar, and every one of the
owner's complaints is about a shape, a kind, or a cause.** The pipeline has
scalars.

---

## The one-paragraph answer

**The generator has no representation of a feature.** It has a heightfield —
`ColumnPlan`, four `i32` per column — and it has the cell grid. There is
nothing in between. A tor, a boulder, a cave, an overhang, a pool: none of
these is an object anywhere in the code. Each is a burst of cells painted by
one pass from one parametric primitive, invisible to every other pass from the
moment it is written.

Everything below is downstream of that. A thing with no representation cannot
be **shaped** (only stamped), cannot be **varied** (only its stamp parameters
can), cannot have a **cause** (nothing upstream can act on it), and cannot
**connect** to anything (nothing can see it). Those four are, near enough
exactly, the owner's four standing complaints.

---

## The ceilings, ranked

Ranked by how much of the owner's recorded disappointment each accounts for.
C1 is the root cause and is ranked third on that scale only because its damage
is delivered through C2, C3 and C5.

| # | ceiling | escapable by |
|---|---|---|
| C2 | The primitive is the silhouette | representation |
| C4 | A region is a gain, not a kind | tuning (partly) + representation |
| C1 | No feature is an object | **representation only** |
| C3 | Every cave is one texture at one topology | representation |
| C5 | Only one thing has causal history, and it is 1-D | representation |
| C7 | The world has one stratigraphy | new pass |
| C6 | Pass interference, live today | **pass order — cheap** |

---

## C2 — The primitive is the silhouette

### The evidence

The owner was shown a residual and wrote, confirmed as "true":

> *"they read as flat vertical slabs — straight sides, uniform width, flat
> top — rising abruptly from flat ground with no talus at the foot and no
> broken profile"*

That sentence is a **line-by-line description of the painting loop**
(`src/worldgen/residual.rs:392-416`). A residual is a stack of 3-8
axis-aligned rings:

* `let w_i = raw.min(prev).max(a * 0.12); prev = w_i;` — half-width is
  **monotonically non-increasing** going up. That is *straight sides* and it
  cannot be otherwise: the module doc states the constraint outright, because
  painting has to be "a simple bottom-up accumulation with no floating cells".
* `FlatCapped` draws `a * (0.55 + 0.45 * hard)` — the width lives in
  55%-100% of the base, and the monotone clamp removes most of even that.
  That is *uniform width*.
* The topmost ring is a horizontal cut at constant half-width. That is *flat
  top*, by construction. There is no taper, no crest, no spall.
* The profile is `|dx| <= w_i` — **mirror-symmetric about the site's own
  centre column**. No lean, no asymmetric shoulder, no notch.
* `talus` is pass 5 in `mod.rs`'s table; `residuals` is pass 7. **The pass
  that makes talus has already run.** And `talus` reads `ctx.plans[x]
  .surface_y` (`passes.rs:761`) — the plan heightfield, which a residual is
  not in. *No talus at the foot* is a pass-ordering fact.

The other primitives are equally legible in the complaints:

| complaint | primitive |
|---|---|
| *"That overall cave shape here is bad... It looks like a perfect oval"* | `grow_monumental_chamber` dilates an **ellipse** at the void's point of greatest clearance (`passes.rs:2010-2014`, `(dx/rh)² + (dy/rv)² > 1.0`); a geode vug (25% of placements) is a bare ellipse |
| *"too much voroni patterns"* / *"The honey comb... shouldn't be everywhere"* | the cave **is** a Worley `F2 - F1` field at one constant threshold (`CAVE_THRESHOLD = 0.09`) |
| *"stalagmite and stalactites still as single pixels"* | partly closed — measured today, base width median **16-23**, range 12-31 (`cave_probe`, 16 seeds) |
| *"They don't look anything like real rock formations"* | see C7 — three shape classes read off ~13 bands of a 340-band global sequence |

### The "sharp vertical faces" half of the #1 complaint was dismissed on a number that measured the wrong object

The owner's 2026-08-22 verdict was two things: *"the repeating hard boundary
at 1/3 and 2/3"* **and** *"there are sharp vertical faces (could sometimes be
ok if done naturally. look horrible here)"*.

The reply card (`20260822T084126139Z-201147`) established that the first half
was the picture — three viewport tiles butted together with 2,219 unshown
columns between them — and I confirm that (see C4). But it dismissed the
second half with this:

> *"the largest step worldgen produces anywhere, in any preset, is **5 rows**.
> The seam was twelve times bigger than anything the generator can make."*

**That is measuring the plan heightfield, not the world.** Measured today on
finished worlds, 8192x2560, 3 seeds, largest single-column skyline step, with
the p99.9 step in brackets — both arms through the same `generate_ablated`
code path so they differ only by the pass under test:

| preset | full build | no `residuals` | no `brows` | no `boulders` |
|---|---|---|---|---|
| canyon | **62** [39] | 49 [15] | 62 [39] | 62 [39] |
| rolling | **59** [24] | 35 [10] | 59 [24] | 59 [24] |
| terraced | **40** [29] | 22 [11] | 40 [29] | 40 [29] |
| arid | **38** [27] | 20 [5] | 38 [27] | 38 [27] |
| wetland | **36** [18] | 12 [**2**] | 36 [18] | 36 [18] |

The generator makes 36-62 cell vertical faces in every preset — **about the
same size as the 61-row stitching seam**, not a twelfth of it. Switch
`residuals` off and the routine extreme step falls by 2.4x-9x; on `wetland`
the terrain's own p99.9 step is **2 cells**, which is where the "5 rows"
figure came from. So the eroded heightfield really is smooth, and **every
sharp vertical face in the world is a residual** — the same feature the owner
independently called a flat vertical slab.

`brows` and `boulders` move the number by exactly zero in all five presets.
That is the reason to trust the row: it is not a uniform effect, it is
specific to one pass, and it has a known explanation on the other two
(`boulders` writes 0-3 cells per world — see C6).

### Escapable?

Not by tuning. `MIN_ASPECT`/`MAX_ASPECT`/`SIZE_SKEW`/`CAP_CONTRAST`/
`LOW_VARIANCE` all move *proportion*; none can produce a non-monotone,
asymmetric or non-flat-topped profile, because the monotone clamp is what
makes the paint legal. Escaping needs a different way to describe a
formation's shape — a 2-D mask, or carving rather than stacking.

Note *why* the constraint exists: the generator must hand the simulation a
world at structural rest on frame one, and a floating cell is not at rest. The
at-rest guarantee is what forces the monotone profile. Any replacement has to
carry its own at-rest argument, which is what `brows` does (attached solid,
`MAX_BROW_REACH = 20`) and is the existence proof that it can be done.

---

## C4 — A region is a gain, not a kind

### The evidence

Six `Character` axes, and every consumer of every one of them:

| axis | what it reaches |
|---|---|
| `elev` | multiplies `relief_amplitude` (`column.rs:166`) |
| `relief` | multiplies `hill_amplitude` (`column.rs:284`) |
| `resistance` | multiplies `terrace_strength` (`column.rs:411`) |
| `sediment` | multiplies cover supply (`column.rs:576-578`) |
| `formation` | multiplies `residual_density` and the size ceiling (`residual.rs:209,231`) |
| `aridity` | **the only axis that switches anything discrete**: sand vs soil at `SAND_ARIDITY`, plus dunes; and rain supply in erosion |

Five of six are gains. **Not one axis changes a wavelength, a mechanism, or
which passes run.** Confirmed by grep: every wavelength in `column.rs` and
`passes.rs` is a bare `p.<name>_wavelength` with no `Character` term on it.

And the presets — the coarsest expressive unit in the generator — barely
change the wavelengths either. Across the five playable presets:

| quantity | spread (max/min) |
|---|---|
| `life_cluster_wavelength` | **1.00** |
| `warp_wavelength` | 1.15 |
| `dune_wavelength` | 1.26 |
| `hill_wavelength` | 1.33 |
| `detail_wavelength` | 1.42 |
| `mask_wavelength` | 1.64 |
| `terrace_step` | 1.70 |
| — | — |
| `hill_amplitude` | 3.33 |
| `relief_amplitude` | 3.89 |
| `aridity` | 11.5 |

**Every wavelength in the generator varies by at most 1.70x across every
preset.** So the spatial rhythm of the terrain — hills every 150-200 columns,
detail every 24-34, benches every 20-34 rows — is a constant of the engine.
`region.rs`'s own module doc says the old presets failed because "`canyon` and
`wetland` differed only in amplitude, which is why they read as the same world
taller and flatter rather than as different country". **That is still true**;
the region layer added more amplitudes, not a second kind of country.

Three fields are literally identical in all five playable presets, and two of
them are the ones the owner keeps complaining about:

* `residual_density: 1.4` — identical everywhere
* `vault_density: 1.6`, `vault_min_depth: 200`, `vault_bedrock_margin: 16` —
  identical everywhere. **There is no such thing as a canyon cave or a wetland
  cave.**

### The regions are smaller than the view

`MIN_REGIONS`/`MAX_REGIONS` are 2-5 *per 512-column window*, scaled by world
width. Measured at the shipped 8192 (`wg_ceilings mode=region`, 8 seeds,
`rolling`): **34 to 85 regions per world**, i.e. each region is **96 to 241
columns**. `hill_wavelength` is 150-200.

**The parameter that sets hill amplitude changes on the same length scale as
the hills it modulates.** That is amplitude modulation at the carrier
frequency; it does not read as "a different place", it reads as noise. With
`TRANSITION = 0.42`, 42% of the world is permanently mid-blend, and a player
crosses 2-5 boundaries per screen.

The codebase already found this — for exactly one axis. `FORMATION_BARREN`'s
comment records that gating rock country at region scale "makes a rock country
smaller than the view, so it reads as a cluster rather than as a place", and
moves that one gate to a far coarser field (`ROCK_COUNTRY_SCALE = 1700`, "a
little over three screens per feature"). **The other five axes were left at
region scale.**

Boundary placement, since it was asked: centres are at `(i + 0.5) / count`
jittered by `±0.33 / count`, so boundaries sit at `i / count` plus a bounded
offset — measured, they track `i/n` to within a couple of percent of world
width. At `n = 3` that is exactly 1/3 and 2/3, which is why the region model
was such a plausible suspect. **At the shipped width `n` is 34-85, so
boundaries fall every 96-241 columns and nothing lands at 1/3 or 2/3.** The
complaint was the stitched picture; this is an independent confirmation of the
earlier finding.

### Complaints explained

* *"Spires should not just be thinned out. They should be part of a specific
  biome. They should not exist at all in most biomes but some biomes should
  have them **and they can be more regular**."* — The first half **was** built
  (the `FORMATION_BARREN` gate; `formation` draws exactly 0.0 outside rock
  country). The second half is unreachable: sites are drawn one per
  `REGION = 256` columns with a count, and there is no representation for an
  *arrangement*. "Scattered field" versus "regular colonnade" is not a value
  this architecture has anywhere to put.
* *"My overall desire is for rocks be of all different shapes and sizes"* /
  *"Again heterogenity is best"* / *"there should be variability between
  caves"* — CLAUDE.md's law 1 applied to worldgen. The engine offers a
  distribution over **size** (`SIZE_SKEW`, `CaveEnv::draw`) and a single value
  for **kind**.
* *"I don't really see much difference between the images"* (cave formations
  A/B), *"I see no difference between A and B except a few rock formations"*
  (rock country sizing), *"Both images show stalagmite and stalactites still
  as single pixels"* — **three separate A/B cards where the owner could not
  see the lever.** This is the most diagnostic data in the review corpus and
  it is CLAUDE.md's *"ask which pixels a lever moves"* failure, in worldgen:
  the levers on offer were all gains, and a gain on an invisible quantity
  moves no pixels.

### Escapable?

Half by tuning: coarsening the region scale (or moving the other five axes
onto country-scale fields the way `formation` already was) is a change of
constants and would make regions read as places. That is worth doing and it is
cheap. But it buys *bigger* patches of the same country. "A region that is a
different kind of place" needs mechanisms that a region can switch on and off,
and there are none — every pass runs everywhere, gated only by a density.

---

## C1 — No feature is an object

### The evidence

The entire decide phase per column is:

```rust
pub struct ColumnPlan {
    pub surface_y: i32,     // topmost solid
    pub soil_depth: i32,
    pub table_y: i32,
    pub bedrock_top_y: i32,
}
```

Four scalars. A monotone stack of at most four layers, plus a water level.
`brows`' own doc states the consequence:

> *"A heightfield alone can only produce a function of x — no overhang, no
> undercut, nothing to stand under."*

**All fourteen named passes read `ctx.plans`** (thirteen in `passes.rs`,
plus `residuals` at `residual.rs:254,294`). Every pass that reasons about
terrain shape reasons about the heightfield. So a feature
written by a realise pass is invisible to every other pass:

* `talus` finds cliffs in `ctx.plans` → a residual cannot shed talus.
* `ponds` is a **1-D trapped-water scan** over `plans[x].surface_y`
  (`passes.rs:3422-3461`, running left/right rim minima) → water can only
  stand in a depression of the 1-D profile. Never behind a residual, never in
  a cave, never under a brow.
* `soil_blanket` drapes the plan → nothing drapes a residual or a boulder.
* `residuals` and `boulders` cannot see each other except through
  collect-verify-write on raw cells.

The only shared representation between passes is **the cell grid**. That is
why every interaction between features in this generator is a *collision*
rather than a *relationship*, and it is the direct cause of C6.

Two features do read the finished world — `springs` (via `world_top`) and the
collect-verify-write seals. Both read it to check for *obstruction*. Neither
can ask what is there.

### Complaints explained

*"no talus at the foot"*, *"rising abruptly from flat ground"*, *"it looks
like it comes from nowhere and goes nowhere"*, *"It is also looks like a
single room instead of a cave system"* — and it is what makes C2's monotone
constraint and C6's interference class unfixable in place.

### Escapable?

**Representation only.** This is the root ceiling. Everything else on this
list is either a symptom of it or is bounded by it.

---

## C3 — Every cave is one texture at one topology

### The evidence

A cave system is a Worley `F2 - F1` field thresholded inside a rectangular
envelope, faded at the edges (`carve_cave_void`, `passes.rs:1775`). Two facts
make it the same cave every time:

**1. The lattice count is a constant, independent of cave size.**

```
CaveEnv::cell()  =  CAVE_CELL * half_w / ROUND_3_HALF_W
lattice cells across =  2*half_w / cell  =  2*ROUND_3_HALF_W / CAVE_CELL
                     =  180 / 22  =  8.18       -- half_w cancels
```

and the envelope aspect is fixed too — `half_h/half_w` is `88/220 = 320/800 =
0.400` at both ends of the draw, because both are drawn from the same unit
sample `u`. So the vertical count is `2 * 0.4 * CAVE_SQUASH * 90 / 22 = 3.93`.

**Every cave system in every world is an 8.2 x 3.9 lattice — the same
topology, the same chamber-to-passage contrast, at a different zoom.** This is
deliberate and documented (`CaveEnv::cell`'s comment: "a large system is a
*scaled-up* version of a small one: same topology, same chamber-to-passage
contrast"). It is exactly what the owner is objecting to.

The measured aspect confirms the identity: over 16 seeds x 5 presets, max span
across / max span down is 1543 / 619 = **0.401**, matching the derived 0.400
in every preset.

**2. Caves are statistically identical in every preset.** `cave_probe`, 16
seeds, 8192x2560:

| preset | systems/world | med void cells | med span across | **max span across** | **max span down** |
|---|---|---|---|---|---|
| arid | 1.5 | 3007 | 137 | **1542** | 620 |
| canyon | 1.4 | 3648 | 223 | **1543** | 619 |
| rolling | 1.6 | 2998 | 127 | **1528** | 619 |
| terraced | 1.3 | 5446 | 286 | **1544** | 619 |
| wetland | 1.4 | 2351 | 198 | **1544** | 619 |

Five presets, five maxima agreeing to within 16 cells in 1,544 — because they
are all pinned to the same `MAX_CAVE_HALF_W`. Nothing in a preset touches
caves, and nothing in a region does either.

**3. The texture is stationary.** One constant threshold over a uniform
lattice is homogeneous by construction: every part of every cave has the same
passage-width distribution and the same chamber frequency. The only
non-stationary term in the whole carve is the edge fade, which is what draws
the envelope's silhouette. *"The honey comb is interesting background
sometimes, but shouldn't be everywhere. Again heterogenity is best"* is a
request for a non-stationary field, and there is no place to put one.

**4. A system cannot leave its envelope.** *"you could go bigger or more even
better longer or have chains of caves for the bigger"* — a chain is two
systems that connect. Systems are placed from independent draws, each carves
inside its own box, and `keep_seed_component` deletes everything not connected
to the single seed component. Two systems cannot join, by construction.

### Escapable?

Not by tuning: the *only* free parameter is `u`, and it sets scale. Making
caves differ in kind requires the field, the threshold or the envelope to vary
in space — a change of representation.

---

## C5 — Only one thing has causal history, and it is one-dimensional

### The evidence

`erosion.rs` is a real landform-evolution model, and the wiki's claim that
"every world has been through a stretch of simulated history" is **not false**.
It runs `world_age * 600` iterations of: thermal shed against a
hardness-dependent stable angle, hillslope creep (a volume-conserving
Laplacian), and — every 8th iteration — stream-power incision with flow
accumulation and capacity-limited deposition. Those are the textbook terms.

But it operates on `h: &mut [f32]` — the 1-D surface profile — and it runs in
the **plan phase, before all fourteen realise passes**. Two consequences:

* **In 1-D there is no drainage network.** Flow is a chain along x; there are
  no basins, no confluences, no divides. A "valley" is a dip in a profile.
  There is no undercut, no overhang, no cave, and no closed depression that
  spills.
* **Nothing it produces can act on a feature, and no feature is ever eroded.**
  Every tor, boulder, brow, cave and pond is painted after erosion has
  finished and is never touched by it.

The sharpest evidence is that **the causal version was built and measured
empty.** The owner asked for the waterfall to have a source: *"spring should
originate in depressions so they fill up and spill out into a waterfall."* The
source comment records what happened (`passes.rs:3170-3177`):

> *"Looking for one is a recorded dead end: for a basin to spill over **this**
> cliff, this cliff's lip has to be the basin's lowest exit, and requiring
> that **placed zero springs across four presets and six seeds**. It is not a
> tuning failure — a cliff edge is a local high point, so the ground behind it
> rises."*

and, for the back wall, *"requiring one placed **nothing** on any preset"*. So
the source pool is **cut, not found**: the pass excavates a basin into the
shelf behind the lip. That is provenance replaced by authoring, on
measurement, because the eroded heightfield contains no closed basins that
spill over cliffs.

The same shape shows in boulders, which are the one feature that *is* meant to
be causal — `erosion::Deposits::boulder` marks a column where cumulative
thermal shed from hard rock passes a threshold, and `boulders` seats a cluster
there. Measured over **80 generated 8192x2560 worlds** (`cave_probe`, 16 seeds
x 5 presets):

* boulder markers proposed: 179 total; **48 of 80 worlds propose none at all**
* boulders actually seated: **3, in 80 worlds**

And talus, the other causal output: erosion computes a median talus volume of
**244.5** per world, and the realise side turns a median of **3 cells** of it
into visible talus (max 89 over 80 worlds). The rest folds into `soil_depth`
and comes out as ordinary blanket — which is `pass-interference-2026-08.md`'s
R4-2 finding, still live.

### Complaints explained

*"it looks like it comes from nowhere and goes nowhere"*, *"no talus at the
foot"*, and the causal half of *"rocks be of all different shapes and
sizes"* — the mechanism that would produce small scattered rock exists, fires
179 times over 80 worlds, and delivers 3.

### Escapable?

Not by tuning — the rates were swept and the finding was that the profile
never develops the features to begin with (`residual.rs`'s B1: "Max prominence
at reach 15 is monotonically decreasing from iteration 0 in both presets, and
its own pre-erosion ceiling (8.34 canyon, 5.00 rolling) never once reaches the
12-120 cell band a residual occupies"). A 1-D profile cannot grow a 2-D
landform. Representation.

---

## C7 — The world has one stratigraphy

### The evidence

```rust
pub(crate) fn at(&self, x: i32, e: f32) -> f32 {
    let band = (((self.datum - e) + self.offset[i]) / self.band_thickness).floor() as i32;
    let raw = noise::unit(self.seed, Purpose::Hardness, band, 0);   // <- band only
    ((FLOOR + (1.0 - FLOOR) * raw) * self.regional[i]).clamp(0.0, 1.0)
}
```

The hardness draw is keyed on **the band index alone** — no `x`. A stratum has
exactly one hardness along its entire outcrop across the whole world,
modulated only by `regional[x]`, which is the region's `resistance` gain. The
band coordinate is `strata_tilt * x + strata_fold * fbm(x/130)`: a **single
constant dip direction** for the whole world plus one 1-D fold.

Counted (`wg_ceilings mode=strata`), for an 8192x2560 world:

| preset | thickness | tilt over the world | bands in the entire world |
|---|---|---|---|
| arid | 9.0 | 573 cells | **349** |
| canyon | 12.0 | 737 cells | **276** |
| rolling | 9.0 | 492 cells | **340** |
| terraced | 10.0 | 410 cells | **298** |
| wetland | 8.0 | 246 cells | **352** |

**The entire geology of a world is 276-352 numbers.** There are no faults, no
intrusions, no unconformities, no lateral facies change, no second fold axis.

This is why the residual shape work produced so little: `Shape::classify` reads
~13 consecutive bands out of ~340, and `strata_tilt = 0.06` shifts the sequence
by only ~1.7 bands per 256-column `REGION`. Two neighbouring residual sites see
a nearly identical hardness sequence and classify the same way. The variety
mechanism is real, fires, and has almost nothing to draw from.

### Complaints explained

*"They don't look anything like real rock formations"*, and the A/B cards
where the owner saw no difference.

### Escapable?

By a new pass: adding `x` to the hardness draw, or a second field for faults
and intrusions, is contained work. It does not need the representation change
C1 does — but note the calibration warning: `residual.rs`'s `CAP_CONTRAST` and
`LOW_VARIANCE` are set against the current variance, so widening the field
re-derives them (CLAUDE.md's "fixing a bug exposes a constant that was
compensating for it").

---

## C6 — Pass interference, live today

Not a ceiling — a consequence of C1 that is individually fixable, listed
because it is cheap and because it is **still firing nine days after it was
written up**. `pass_ablation`, 6 seeds, all presets, 8192x2560, run today:

```
without brows        : boulders APPEARS (was zero)   -- arid, canyon, rolling, terraced, wetland
without soil_blanket : residuals -97% .. -100%       -- all five presets
without ponds        : life_scatter +23% .. +80%,  springs +44% (rolling)
without talus        : ponds +61% (canyon)
without pockets      : vaults +8% .. +14%
```

`brows` deleting 100% of boulders is `pass-interference-2026-08.md`'s R4-1,
recorded 2026-08-20 and unfixed. `soil_blanket` feeding residuals at 97-100%
and `talus` suppressing ponds by 61% are **new**, not in that report.

Baseline cells written per 8192x2560 world, against `stone_massif` at
~18.4-19.3 **million**:

| preset | brows | talus | residuals | **boulders** | vaults | ponds | life_scatter |
|---|---|---|---|---|---|---|---|
| arid | 251 | 229 | 4354 | **0** | 106633 | 0 | 0 |
| canyon | 10078 | 4187 | 9411 | **3** | 106570 | 487 | 162 |
| rolling | 3488 | 1452 | 9362 | **0** | 119655 | 75880 | 342 |
| terraced | 2513 | 867 | 8088 | **0** | 118777 | 23163 | 567 |
| wetland | 128 | 80 | 8145 | **0** | 116137 | 45187 | 766 |

Talus is 0.0004%-0.02% of the world. Boulders are 0-3 cells. This is the
measured form of `region.rs`'s own admission that *"barren country reads as
empty, because the only other standing-rock pass is `boulders`"* — and the
owner's *"rocks of all different shapes and sizes"* is asking for precisely
the pass that writes nothing.

---

## What the streaming future (M10) forbids — and what it does not

The declared margins, printed from `pass_summary()` against `CHUNK_SIZE = 64`:

| pass | margin | to generate one 64-column chunk |
|---|---|---|
| soil_blanket | 2 | 1x |
| brows | 40 | 2x |
| residuals | 55 | 3x |
| talus | 200 | 7x |
| springs | 296 | 10x |
| **vaults** | **802** | **26x** |
| ponds, soil_moisture, moisture_init | GLOBAL | — |

`vaults` costs 802 columns of context either side because `MAX_CAVE_HALF_W` is
800; a 64-column chunk needs 1,668 planned columns.

**And there is a fourth global stage that carries no label.**
`erosion::erode` sorts all `w` columns by height (`erosion.rs:248`) and
accumulates flow downhill across the whole array (`:284`). A column's eroded
height depends on flow from every upslope column in the world. It runs in the
plan phase, has no row in `PASSES`, and therefore declares neither a margin nor
`GLOBAL`. `mod.rs` says the decide phase is "pure functions of
`(seed, params, x)` with no world access and no dependence on traversal order";
that is true of `Terrain::plan(x)` and **false of
`plan_all_with_deposits()`, which is what generation actually calls**. Every
playable preset has `world_age` 0.7-1.0; only `flat` has 0.0. Corroborated
empirically: `Deposits::exported` — volume leaving at the world edges — is
nonzero in nearly every one of the 80 worlds measured, so the flow chain
genuinely spans the world.

**So: is a more global architecture off the table? No.** The pipeline is
already global in four places, three labelled and one not. A revamp that adds
global structure does not cross a line this generator has not already crossed;
it changes the *size* of the debt the planned coarse `(x, z)` map has to pay
off, and that map is already the designated instrument for exactly this. What
a revamp must not do is add unbounded reach to a *local* pass without saying
so — the failure this table has already had three times (`talus` declared 3
while walking 120; `vaults` declared 96 while reaching 202; erosion declares
nothing at all).

The decide/realise split is worth keeping for a different reason than
streaming: it is what makes `pass_ablation` possible, and the ablation is the
only instrument that sees C6.

---

## What I could not establish

* **Which primitive the "perfect oval" cave was.** Three candidates produce an
  oval — the geode vug (25% of placements), the monumental chamber's ellipse,
  and a small system whose void is mostly one chamber. The card
  (`20260823T103359957Z-2eaf50`) was shot from inside a cave, which favours
  the monumental chamber, but I did not reproduce that exact frame.
* **The lattice-count invariance is an arithmetic identity over stated
  constants, not a direct measurement.** `CaveEnv` is `pub(crate)`, so an
  example cannot instantiate it. I cross-checked the half of it that is
  observable — the envelope aspect ratio, derived 0.400 and measured 0.401 in
  every preset — but the "8.18 lattice cells across at every size" figure is
  read off the source, not off a run.
* **Whether coarsening the five region axes onto country-scale fields would
  actually read as "different places".** It is the cheap experiment C4 points
  at and I did not run it; the `FORMATION_BARREN` precedent says it helped for
  one axis, which is suggestive and not evidence for the other five.
* **Why `soil_blanket` feeds `residuals` by 97-100%.** The ablation says it
  loudly and consistently across all five presets; I did not trace the
  mechanism. The `MAX_SOCKET_DEPTH` walk through cover is the obvious suspect
  and I have not confirmed it.
* **How much of the "no difference between A and B" verdicts is the levers
  and how much is the rendering/zoom of the cards.** C4 explains it
  architecturally; a rendering explanation is not excluded.

## The instrument

`examples/wg_ceilings.rs`, read-only, three modes:

```
cargo run --release --example wg_ceilings -- mode=step   seeds=3 preset=canyon
cargo run --release --example wg_ceilings -- mode=region seeds=8 preset=rolling
cargo run --release --example wg_ceilings -- mode=strata
```

`mode=step` is the one worth keeping: it measures the finished world's skyline
and ablates a pass to attribute it, which is the check that would have caught
the "5 rows" error. `mode=step` prints one tagged line per measurement rather
than a table, because the passes print during generation and a `print!`-built
row interleaves with them.

This report is not yet listed in `Reports/README.md` — the coordinating lane
is indexing all three lane reports together, so `scripts/docscheck.sh` will
flag it as unindexed until that lands.
