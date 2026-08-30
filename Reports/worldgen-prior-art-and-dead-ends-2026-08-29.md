# Worldgen: the do-not-retry list, what six rounds delivered, and the prior art

*2026-08-29. Lane C of the worldgen revamp program — a research lane. No
changes to `src/`. Written to stop a revamp plan proposing something this
project already tried, and then to widen the option space.*

**Standing of the sources used.** Every report cited below was checked
against `Reports/README.md` first. All are indexed as merged; none is in
the in-flight section (whose three former entries were all recovered and
landed 2026-08-24). `worldgen-design.md` is indexed **"direction agreed,
implemented"** — §2 shows that label is true of most of it and *not* of its
§0, which is the finding this lane was asked to nail down.

**Confidence markers** follow the house convention of
`prior-art-worldgen-slicing.md`: **[V]** verified from the primary source or
from this tree; **[S]** secondary, reported by a source I could not open;
**[U]** unverified.

**One egress note.** `dl.acm.org`, `arxiv.org`, `hal.science`,
`onlinelibrary.wiley.com`, `cs.purdue.edu` and the LIRIS author pages are all
blocked by this container's proxy. §3's paper mechanisms are therefore [S]
from search-engine abstracts and from the authors' GitHub READMEs, which do
open. Where a mechanism claim matters to a decision I say so and mark it.

---

# §1 The do-not-retry list

`Reports/dead-ends.md` has a `## worldgen` section of **17 entries** (lines
1185–1231), which is small enough to read whole and was. The mechanism greps
CLAUDE.md prescribes were also run across the whole file — `erosion`,
`hydraulic`, `voronoi`, `worley`, `domain warp`, `biome`, `tectonic`, `plate`,
`strata`, `karst`, `talus`, `scree`, `spire`, `tor`, `mesa`, `overhang`,
`cave`, `river`, `watershed`, `drainage`, `poisson`, `wave function`,
`template`, `prefab` — and turned up one further live entry outside the
section (`fracture_field.rs`'s Worley pitch, §1.18). `voronoi`, `tectonic`,
`karst`, `mesa`, `watershed`, `poisson` and `prefab` return **zero hits in the
entire file**: those mechanisms have never been tried here.

Six more rejections are recorded only in the round task files and the
reviews, not in `dead-ends.md`. They are §1.19–§1.24 and they are the ones a
plan is most likely to trip over, because grepping the register will not find
them.

## The scannable table

Read the last column first. **"VOID"** means a revamp of the kind being
planned changes the condition the rejection depended on, so the entry is due
for re-testing rather than obeying; **"HOLDS"** means it survives; **"HOLDS —
and it is a trap"** means the rejection also names a shape to avoid.

| # | Mechanism | Why rejected | Condition of the rejection | Under a revamp |
|---|---|---|---|---|
| 1 | Spring sourced from a pool **found** in existing terrain | 0 springs across 4 presets × 6 seeds when the lip must be the basin's lowest exit; relaxing it gives ponds that never drain | The landform does not exist *in a 1D heightfield generator*: a cliff edge is a local high point | **VOID if drainage is computed.** The rejection is of a *search over this generator's terrain*, not of the idea |
| 2 | Uniform global cut of `residual_density` (1.4 → 0.45) | Owner: *"I didn't mean a uniform decrease in spires"*; at 0.45 the census was **identical to residuals-off** | A single global scalar over a pass that runs everywhere | **HOLDS — and it is a trap.** Any global knob over an always-on pass reproduces it exactly |
| 3 | Gating `formation` on the **region's own** draw | Fires correctly (87% of blocks read zero) and is indistinguishable from #2 on screen and in the census | A `RegionMap` region is 102–256 columns — a fifth to a half of one screen | **VOID.** Any revamp that puts biome patches at multi-screen scale changes this exactly |
| 4 | `strata_thickness` 4× (12 → 48) with the cave envelope | Blind A/B: the owner picked the shipped 12 | Judged on a **surface** strip; the underground half was never tested | **PARTLY VOID** for a card showing bedding *inside* a cave |
| 5 | `partition_point` binary search over region centres | Bit-identical, measured at **zero** (5031 → 5082 ms on a 9 s build) | Region count ≤ ~80 at 8192 wide | **VOID if the revamp multiplies regions** (many small patches). Otherwise it stays worthless |
| 6 | Larger timesteps in pipe-model erosion | CFL: timestep × velocity ≤ cell size | Any **explicit** shallow-water / virtual-pipes solver | **VOID for an implicit or analytical scheme** — §3.1 names one |
| 7 | Worldgen respecting every material's repose angle; settling at generation time | Couples worldgen to material data; settling needs neighbours that may not exist (circular) | Recorded at design time, before streaming existed | **PARTLY VOID** — prior art §6.1 already answers it with a static material class, and `Cell::attached` is that bit |
| 8 | One global water table | Almost all caves below sea level flood | A design revision from literature; **no engine measurement** | Do not retry the rejected one; *implement* the replacement (per-body aquifer levels) |
| 9 | Worm-carving as the **primary** cave mechanism (subtractive) | Additive density removes the carve pass, the ordering question, and carves into ungenerated space | Applies to *primary*; worms are **explicitly not discarded** | **Deliberately open.** A bounded-radius mechanism is still wanted for surface reachability and for the `max_unsupported_span: 3` ceiling problem |
| 10 | Ridged noise as `1 − |fBm|` | The common simplification; does not give terrain-like heterogeneity. Use Musgrave's multiplicative cascade | Unconditional | **HOLDS.** Plus: ridged noise has a derivative discontinuity — a trap if ever sampled as a gradient |
| 11 | Wave Function Collapse for terrain | Order-dependent (so path-dependent), memory never released, dead ends accumulate, unthreadable; the per-chunk-constrained variant has no solution at seams | **Streamed, unbounded** content | **VOID for bounded content** — the entry itself says so: a fixed-size ruin or structure template with an edge is not covered |
| 12 | Full 3D simulation | 2048³ = 68 GB; 512³ = 819× compute; and the real objection is legibility | Unconditional for the *simulation* | **HOLDS.** The coarse (x,z) layer is explicitly kept |
| 13 | Top-down 2D | Deletes `update.rs`, sweep order, repose, all of M17, most of M16 | Unconditional | **HOLDS** |
| 14 | Inferring "underground" from world shape | Four rules, each wrong in a new way; *"there is no setting that does not exist"* | Permanent for **inference** | **HOLDS.** Consequence for a revamp: anything a later system needs to know about a generated void must be **recorded at generation**, never re-derived |
| 15 | `base_wave` as one sine at fixed phase | Guaranteed a ridge and a valley — in the same place at every seed | Holds whenever a compositional guarantee is implemented by **fixing** shape rather than **constraining** it | **HOLDS — and it is a trap.** "Every world gets a spire biome" must constrain the draw, not pin the placement |
| 16 | Deriving talus-apron shape from cliff geometry (case analysis) | Four at-rest failures of one kind; each fix produced the next | Holds while terrain can undercut an apron in unenumerated ways — i.e. always | **HOLDS.** Do not return to case analysis; clamp with a sweep-derived property |
| 17 | Per-column distance-to-water transform | Left every above-table pond's wetted perimeter unwrapped; capillarity assembled it at runtime (82 vs 31 awake-chunk frames) | Holds while ponds can sit above the smoothed table | **HOLDS** unless the water model changes so ponds cannot |
| 18 | Smoothly varying the **Worley pitch** with position (`fracture_field.rs`) | The severing rule is `domain(a) != domain(b)`, an identity test; under a smooth pitch neighbouring cells read different lattices and the web dissolves | Structural, not aesthetic | **HOLDS — and the fix is already known**: quantise the pitch on a coarse lattice of its own, so inside a band the pitch is one constant |
| 19 | Second cave sub-threshold on `F3 − F1` | Connectivity *improves* (94% → 99%); refused because contrast falls **3.2× → 2.1×** and median open column doubles — it buys size, not drama | The bar is contrast/drama | **VOID if the bar changes to size.** Recorded twice because both halves cost time |
| 20 | Discs around Worley feature **points** | Never touch the `F2 − F1` boundary web, so every chamber is a sealed satellite | True of a *disc*; does **not** carry to `F3 − F1`, whose minima lie on lattice vertices | **HOLDS for discs only** |
| 21 | Retuning `CAVE_CELL` / `CAVE_SQUASH` / `CAVE_THRESHOLD` to hit "reachable ≥ 50%" (round 6, A1) | Reached 96% by dissolving the network into one rounded bubble: span 136 → 70, contrast 5.4× → 2.1×. Reverted | The bar itself was set against a **broken ruler** (the probe counted walk-through formations as walls) | **HOLDS.** And it is the clearest case in the corpus of a bar driving a change the owner then rejected |
| 22 | Growing residuals by retuning erosion rates | **No column ever peaks above its iteration-0 value, ever** — 0 of 2048 in both presets. Max prominence at reach 15 decreases monotonically | The **raw pre-erosion heightfield's** own ceiling is 8.34 (canyon) / 5.00 (rolling) against a bar of 20 | **VOID if what builds `elev` is replaced.** This is the single most re-testable entry in the list — see §3.1 |
| 23 | "Protect what erosion finds promising" | Erosion is never *offered* a residual-scale candidate to protect | Same as #22 | **VOID with #22** |
| 24 | Sparse acceptance draw for spring placement | Cut placement 1.0 → **0.2 springs/world**; rotating the scan start costs nothing and spends no candidates | Candidates are scarce (8 in a whole `wetland` world) | **HOLDS while candidates are scarce**; a generator with real drainage has many more |
| 25 | Spring outlet at `table_y` | `ponds` runs first and fills every cell where ground drops below the table, so the table's exposure surface **is** the pond surface. 26 of 26 rejected on one seed | A **pass-ordering** fact | **VOID if the water passes are unified or reordered** |
| 26 | Spring outlet at a fixed depth under the plan's `surface_y` | The built face is rough — talus, brows, pillars — so **331 of 339** plan-clear faces are occupied | Plan-space and world-space disagree after the realise passes | **VOID if placement moves into plan space**, or if the plan is made to predict its own realise passes |
| 27 | Early gate requiring the table between rim ground and the foot of the face | Rejected 65–92% of every preset and **all** of canyon — which has the best waterfall faces in the game | Was measuring the wrong thing once the outlet became perched | **HOLDS** |
| 28 | Wholesale seal rejection ("every cell + 2-cell dilation must be stone, else reject the system") | **One grain of sand deleted a whole cave.** `pockets` removes 46–53% of cave cells elsewhere and **100% in arid** | CLAUDE.md's size-cap landmine wearing a costume: the cap gated *whether*, not *how much* | **HOLDS — and it is the trap most likely to recur.** Any new collect-verify-write pass must reject a **breach**, not a system |

## The five entries a plan is most likely to trip over

**1 — #22/#23, "erosion will grow the formations."** It reads like tuning and
it is not. The instrumented measurement is unambiguous: over 2048 columns,
both presets, *no column ever peaks above its iteration-0 prominence at any
iteration*, and the counter for the hypothesised rise-then-fall lifecycle is
**0 of 2048**. Erosion here only ever *removes* formation-scale relief — max
prominence at reach 15 is 10 at age 0 and 3–5 at every shipped age. A plan
that says "turn erosion up and the hoodoos will come" is proposing something
already measured to be impossible **at the current input**. What makes it the
most interesting entry rather than the most closed one is that its condition
is entirely about the *input*: the raw heightfield never offers a candidate.
Replace what builds `elev` with something that *creates* relief — §3.1 — and
this rejection is void.

**2 — #2/#3, "make spires rarer / gate them on the region."** Both were built,
both fired, and both measured **identical to the control**. The owner's
verdict names the reason: *"They should not exist at all in most biomes but
some biomes should have them."* A revamp must not reach for a density
multiplier or a region-scale gate for this; it needs a place large enough to
be a place (#3's condition) and a decision that is categorical rather than
scalar (§2's cause finding).

**3 — #28, wholesale rejection.** This has now been shipped twice in this
generator, in `vaults` and in `boulders`, and `pass_ablation` measured a third
(`brows` deletes 100% of boulders in four of six presets). Any new
collect-verify-write pass inherits the bug unless it is written to reject a
breach.

**4 — #18, "vary the Worley cell size across the world for cave variety."**
The exact thing a plan reaching for "there should be variability between
caves" would write, and it is structurally broken wherever the consumer is an
identity test between neighbours. The known-good form is a coarse quantising
lattice, so the pitch is constant inside a band.

**5 — #11, WFC.** The rejection is real and it is **scoped to streamed,
unbounded terrain**. The entry's own re-test clause keeps it live for
bounded, generated-once content with an edge. A plan that wants authored-but-
varied assemblies — a ruin, a chamber's furnishing, a mine — should read the
clause rather than the headline.

## What has never been tried here at all

Zero hits across all 595 entries, so nothing in the register speaks to them:
**Voronoi/Poisson-disk placement of landform objects**, **tectonic uplift**,
**karst/speleogenetic conduit generation**, **mesas as authored objects**,
**watershed extraction**, **domain warping as a heterogeneity mechanism**
(`Purpose::Warp` exists and is used to warp the height sample, but no entry
records it being tried for anything else), and **prefabs/templates**. That is
the option space §3 goes into.

---

# §2 The six rounds: what shipped, what it earned, and the cause underneath

## The ledger

| Round | Intent | What shipped | The verdict it earned |
|---|---|---|---|
| **1** | Worlds that differ *from each other*; fix legibility artifacts. Region-keyed palettes, pockets follow bedding, dune/riser fixes, brows-talus at region scale | Region-keyed palettes; the 16-seed census sweep; six findings, all load-bearing | *"I see no difference between the images"* class |
| **2** | Tame the terrace-snap keyhole risers; kill the vertical palette piers; **the sealed vault pass** (secret caves) | Slope-attenuated terracing; 2D-modulated palette transitions; vaults | *"It is a plain oval."* A bubble, not a cave |
| **3** | Cave *anatomy*: Worley `F2 − F1` chambers-linked-by-passages; speleothems | Cave systems replacing the oval; speleothems; bedding anisotropy | Round 5's census: **179 × 65–69 in every preset, every seed**; open column median 30 — one bore, no passages; formation height median **3** |
| **4** | Make the landed erosion core *visible*: talus as gravel, boulders at sockets, ages on by default | Deposits plumbed to the realise side; gravel talus; the `boulders` pass; aged presets | R4-1: boulders **mostly reject** — later quantified as `brows` deleting **100%** of them in four of six presets |
| **5** | The three *structural* causes: seal a breach not a system; a lattice with more than nine cells; one deliberate chamber. Then decoration | All bars met but three. Presence 3–10/16 → **12/16 every preset**; open column 30 → 4–5; contrast 2.0× → 5.2–5.8×; near-pairs 0–2 → 45–53 | *"The bars were met and the cave got worse."* The chamber reads as a **picket fence**; owner independently: *"large uniform gray blocks"*, *"totally full of stuff"*, *"all 1 pixel thick"* |
| **6A** (caves) | Passages the player fits through; heavy-tailed cave sizes; fewer, thicker, tapered formations | A1 **rejected and reverted**. A2: envelope 181×71 fixed → per-system draw to 401×161. A3: 45.8 → 15.4 formations/system, base width median 1 → 3 | *"That overall cave shape here is bad though. It looks like a perfect oval, not natural."* *"Both images show stalagmite and stalactites still as single pixels and look bad."* |
| **6B** (formations) | Rock at the player's scale — prove the diagnosis, then build residual landforms and believable boulders | B1's measurement (see below). B2: `residual.rs`, the first mechanism that makes rock at player scale — strip it and the world's p99 relief at reach 15 is **3 cells** against a 14-cell character | *"They don't look anything like real rock formations."* Then: *"This is fine for now. We are spending too much time on this... my overall desire is for rocks be of all different shapes and sizes"* |

## The pattern the lane was asked to test: symptom or cause?

**Answered: rounds 1–5 attacked symptoms; round 6 found the causes and could
not act on them within the round.** That is a sharper claim than "each round
attacked a symptom", and the evidence for both halves is in the tree.

The symptom rounds share a signature — **every bar was met and the screen did
not change**, which is exactly the failure CLAUDE.md's method section is
about:

- Round 5 is the purest case. Six bars met, three missed, and the composed
  result was worse. The review's own diagnosis: *"Bars on per-column
  statistics cannot see composition"* — three formation mechanisms each
  verified alone became a picket fence together — *"and cannot see the player"*
  — a contrast bar maximised itself by shrinking the world below the
  character's size.
- Round 6A's A1 is the same failure with the ruler moving too: a bar of
  "reachable ≥ 50%" was set against a probe that counted walk-through
  formations as walls, and the retune it justified reached 96% by dissolving
  the cave into one bubble. Six of that round's problems were the ruler
  rather than the cave, and the handoff tabulates all six.

Round 6 is where causes start being named, and three of them are stated
plainly in the tree. They are the material for a revamp:

**Cause A — the base heightfield has no formation-scale relief to begin
with, and erosion only removes what there is.** B1, §1.22. Measured
prominence at four reaches shows a world that has landforms (reach 60:
canyon s7's mesa at 39 cells) and texture (reach 5: 1–3 cells) and, *at
reaches 15 and 30 — exactly the scale a rock formation occupies — the
tallest thing in the entire world is 4 to 10 cells*. Not rare: absent.

**Cause B — the generator authors shape as *extents*, and an extent-shaped
object is a rectangle at every scale.** Round 6B, verbatim, and it names the
cave failure and the spire failure as one thing: *"That is the same failure
the owner named in the caves ('all 1 pixel thick ... should have a taper'), in
a different pass, which makes it worth naming as a pattern rather than a
bug."* What is missing is a **profile** — width as a function of height, with
a foot and a weathered crown. The owner's *"They don't look anything like
real rock formations"* and *"no perfect ovals"* are both this.

**Cause C — a heightfield cannot represent half the shapes asked for.**
`worldgen-erosion-design.md`, verbatim: the erosion plan is one `h[x]` per
column, so it can express a tall narrow column and **cannot express an
undercut** — the mushroom cap on a thin stem that makes a hoodoo read as a
hoodoo, or a balanced rock. *"Any spec that promises hoodoos purely from
plan-space erosion is promising something the representation forbids."*

**And a fourth cause this lane found, which is not written down anywhere and
is the one that explains the spire verdicts.** `region.rs`'s `Character` is
the type that decides what kind of place a region is, and its doc comment
states the rule: *"Every field is a multiplier or a 0..1 axis, never an
absolute size, so a region modulates whatever preset it finds itself in."*
Six fields — `elev`, `relief`, `aridity`, `resistance`, `sediment`,
`formation` — all continuous, all multiplicative, and blended smoothly across
region boundaries.

That is a **degree** architecture. The owner is asking for **kind**:

> *"Spires should not just be thinned out. They should be part of a specific
> biome. They should not exist at all in most biomes but some biomes should
> have them."*

A continuous multiplier on an always-running pass cannot express "does not
exist at all here, exists there" — it can only express *less*, which is
precisely the rejected uniform thinning (#2) and precisely what the
region-scale gate measured as (#3). `Purpose::RockCountry` at
`ROCK_COUNTRY_SCALE = 1700` columns was the partial fix and it is still a
scalar field feeding a multiplier. **The `Character` type is the reason the
spire complaint has survived three separate attacks**, and no amount of
tuning inside it reaches the request. This belongs in a revamp plan as a
first-order item.

One thing the ledger should *not* be read to say: the pipeline's shape is not
the problem. `pass-interference-2026-08.md` argues this directly and I agree
with it — pure plan, declared margins, collect-verify-write, at-rest by
construction is what made every one of these failures *measurable*. Five of
the six causes above were found because the architecture allowed an ablation
or a purity test. A revamp should keep that skeleton and change what runs
inside it.

## The unbuilt half: status of "2D play through 3D coarse worldgen"

**Status: designed in detail, deliberately deferred, and never built. Nothing
of it exists in `src/` except one reserved always-zero field.** [V — read
from the tree, not from a report]

The evidence, in the order I checked it:

1. **The design is real and thorough.** `worldgen-design.md` §0 is 90 lines,
   marked *"The single most important framing in this document"*. It settles
   the play world as 2D side view, the coarse layer as planar over `(x, z)`
   carrying elevation, drainage and climate, and the play world as a vertical
   slice through it. Slice topology (straight cut vs curved route along the
   drainage network) is marked **`[open, deliberately deferred]`**; layered
   slices are marked **"Deferred, not rejected"**.

2. **The generator has no `z` and no coarse map.** `src/worldgen/` is nine
   files, 10,228 lines. `Ctx` holds `plans: Vec<ColumnPlan>` — *"one entry per
   column of the world, indexed by x"*. `Terrain::character(x)`,
   `RegionMap` along x, `erosion.rs` over `h[x]`. Every mention of the coarse
   map in the source is a comment saying what it **will** be for. There is no
   `(x, z)` field anywhere.

3. **The only artifact is `ChunkCoord`'s reserved field.** `src/sim/chunk.rs:50`
   carries a third field documented as *"Reserved for M10's worldgen redesign
   (`Reports/worldgen-design.md` §0, issue #11): a generic slice identifier...
   Always `0` today"*. §0's advice to reserve the identifier now rather than
   migrate 42 sites later was taken, and nothing else was.

4. **The debt it was designed to pay is still outstanding, and is named as
   such.** `GLOBAL` in `mod.rs` marks a pass that reads the whole world:
   *"Every pass carrying this is a stated prerequisite for streaming, and the
   coarse map is what removes it."* Three passes carry it today — `ponds`,
   `soil_moisture`, `moisture_init` — and two more (`brows`, `talus`) declare
   margins of 40 and 380 columns that `mod.rs` says *"Shrinking them is a job
   for the coarse map, not for optimism here."*

**What this means for a revamp plan, stated precisely.** The coarse map is not
abandoned, not partly built, and not blocked on anything measured. It is the
one designed piece of worldgen architecture that has never been attempted, and
three things a revamp is likely to want depend on it or on something like it:

- **Drainage networks.** §0: *"a linear world has nowhere for a river to
  branch"*. The spring dead end (#1) is this fact wearing a different costume
   — the reason no basin spills over a cliff is that a 1D heightfield has no
  drainage structure for one to sit in.
- **Biomes at a scale larger than a screen.** #3's rejection depends entirely
  on regions being small; a planar coarse map is where a multi-screen biome
  patch would naturally live.
- **Per-chunk streaming.** The three `GLOBAL` passes are the named blocker.

**And one caution the prior-art report already filed against it, which a plan
must not skip.** `prior-art-worldgen-slicing.md` §2.1 names a fork
`worldgen-design.md` §0 does not: **3D-A** (a planar `(x,z)` coarse map, fine
detail in the slice's own `(x,y)`) is cheap and is what §0 actually describes;
**3D-B** (a genuine `(x,y,z)` density field evaluated on the cut) is what
coherent or movable slices require. *"3D-A does not give you 3D-B."* And the
slice theorem says a single slice of an isotropic fBm is *exactly* an fBm of
the lower dimension with the same Hurst exponent [V, in that report] — **so no
single slice looks better for having been cut out of 3D.** If the plan wants
the coarse map for drainage and biome scale, that is 3D-A and it is cheap. If
it wants it because slices look better, the literature says it will not.

---

# §3 Prior art: architectures that deliver heterogeneity and causality

`prior-art-worldgen-slicing.md` is the standing survey and is good. It covers
the slicing question, "terrain must be at rest" (six shipped strategies,
Noita's static material class the best of them), herringbone Wang tiles for
structural connectivity, the CA cave method's order-dependence and its halo
fix, position-indexed RNG, and hydrogeological conventions for drawing a
believable vertical section. **This section does not repeat any of that.** It
goes after the four demands the owner's verdicts actually state:
difference *in kind*, features with a *cause*, *distributions* of size and
shape, and primitives that do not read as primitives.

## §3.1 Simulation-first: uplift + stream-power erosion as the *shaper*

**The mechanism.** Cordonnier et al., *Large Scale Terrain Generation from
Tectonic Uplift and Fluvial Erosion* (CGF 2016) [S]. Terrain is not noise that
is then eroded; it is the **equilibrium of two competing processes**. A
user-painted (or generated) uplift field raises crust; the **stream power
law** removes material at a rate set by drainage area and local slope. The
solver builds a **stream graph** over the domain — every cell's steepest
descent, depressions resolved into lakes — and integrates the stream power
equation on it. The output is dendritic river networks, watersheds and ridge
lines that *are consequences*, not decorations.

Two supporting pieces, both directly usable:

- **Priority-Flood** (Barnes, Lehman & Mulla, *Computers & Geosciences* 2014)
  [S/V — pseudocode is ~20 lines, reference implementation under 100 lines,
  per the paper's own abstract]. Fills or carves depressions optimally by
  pushing edge cells into a priority queue ordered by increasing elevation.
  Variants label **watersheds** and compute **flow directions**. This is the
  routine that turns a heightfield into a drainage structure, and it is the
  one that would tell you where a basin's lowest exit is.
- **Analytical stream-power erosion** (Tzathas et al., CGF 2024) [S] — solves
  the stream power law analytically rather than by time-stepping. Relevant
  here specifically because **it is the mechanism that voids dead end #6**:
  the CFL rejection is scoped to explicit solvers.

**What it gives the owner.** Every one of the four demands, and it gives them
as *consequences* rather than as features:

- *A spring that comes from somewhere.* With a drainage network and filled
  depressions you know which basins exist, what their outlets are, and where
  the flow accumulation is high enough to be a stream. A spring at a
  permeability contact where flow accumulation crosses a threshold **is where
  a spring must be**, which is exactly the owner's *"should originate in
  depressions so they fill up and spill out into a waterfall"*.
- *Difference in kind.* Uplift rate, rock hardness and precipitation are three
  independent inputs, and their combinations give qualitatively different
  country — a high-uplift/hard-rock region is a canyon, low-uplift/soft is a
  peneplain — without anyone authoring "canyon" as a preset.
- *Formation-scale relief that erosion creates rather than removes.* This is
  the point that matters most: dead ends #22/#23 are conditioned on the input
  heightfield having a prominence ceiling of 8.34 cells. Uplift + stream power
  *generates* relief where the current pipeline only relaxes it.

**What it costs.** Cordonnier's headline is "large realistic terrains at low
computational cost" [S] and the class is O(n) to O(n log n) per iteration over
the domain with tens to hundreds of iterations. Against this project's budget:
`PASS_TIMING` puts `stone_massif` at **3946 of 5188 ms** at 8192×2560 and
201 ns per placed cell, on a ~9 s total build [V, from
`world-scale-phase-2.md` §7]. A plan-space solver over `w = 8192` floats is
cheap by comparison — `erosion.rs` already runs 600 iterations per unit of age
in *tens of milliseconds*. Cost is not the objection.

**Does it survive the hard constraints?**

- *Determinism (required, same-build):* **yes**, with care. `erosion.rs`
  already establishes the pattern — fixed iteration counts, all randomness
  through a keyed `Purpose`, ties broken by column index. Priority-Flood needs
  the same treatment: a priority queue's tie order is not determined by the
  comparator alone, which is the exact shape of CLAUDE.md's `sort_unstable`
  gotcha, so **the tie-break must be explicit** (by index) or the drainage
  network is not reproducible.
- *Frame cost:* **free.** This is build-time work, not per-frame work.
- *Streaming (per-chunk):* **this is the objection, and it is real.** Flow
  accumulation is inherently global — a cell's drainage area depends on every
  cell upstream of it, arbitrarily far away. It cannot be evaluated per chunk.
  It *can* be evaluated once at coarse resolution over the whole world and
  cached (which is what a coarse map is), then read per chunk as a boundary
  condition. That is 3D-A, and it is the argument that the coarse map is the
  prerequisite for this candidate rather than an optional extra.

## §3.2 Hydrology-first: generate the network, then the terrain around it

**The mechanism.** Génevaux et al., *Terrain Generation Using Procedural
Models Based on Hydrology* (SIGGRAPH / ACM TOG 2013) [S]. The inverse of
§3.1: instead of eroding a surface until rivers emerge, **build the river
network first as a geometric graph** by growth/expansion from the mouths
inland, analyse it for watersheds and Horton–Strahler order, and *then*
generate elevation by combining terrain and river patches with **blending and
carving operators** in a construction tree.

**What it gives the owner.** A river is an object with an identity, an order,
a source and a mouth, before any ground exists. The valley is generated *to
contain* it. That directly answers *"it looks like it comes from nowhere and
goes nowhere"* — under this architecture the question cannot arise, because
the network is the thing that was authored and the terrain is downstream of
it. It also gives the play world a legible causal chain the player can walk:
tributary → confluence → main stem, which is what `worldgen-design.md` §0's
curved route wanted a spine for.

**What it costs.** Cheaper than §3.1 at build time — a graph expansion plus a
construction-tree evaluation, not an iterated PDE. The real cost is
architectural: it inverts the pipeline. `column.rs` currently decides
elevation and everything else reads it; here the network decides and elevation
is derived.

**Does it survive the constraints?**

- *Determinism:* yes — graph expansion from a seeded set of candidate nodes.
- *Frame cost:* free, build-time.
- *Streaming:* **better than §3.1, and this is its main advantage.** The
  network is a **tree built by recursive expansion**, so the elevation near a
  point depends on that point's ancestors in the tree — a path of length
  O(log n) — and on nearby patches, not on the whole domain. The construction
  tree is explicitly designed for level-of-detail evaluation [S]. Marked [S]
  because I could not open the paper to confirm the locality bound; **a plan
  should verify this before relying on it**, since it is the property that
  makes this candidate streamable and §3.1 not.
- *The catch for a side view:* a drainage network is a top-down object. A
  straight slice through it crosses rivers perpendicular and sees puddles,
  which is §0's own "slice topology" problem. This candidate is therefore
  coupled to the curved-route decision in a way §3.1 is not.

## §3.3 Object-level generation: features as parameterised objects with a shape grammar

**The mechanism.** Peytavie, Paris, Galin, Guérin & Gain, *Terrain
Amplification with Implicit 3D Features* (ACM TOG 2019 / SIGGRAPH Asia) [S,
with the GitHub README as a second source [V]]. Terrain is a **construction
tree of implicit skeletal primitives**. Landform primitives are **positioned
by Poisson sampling** and **built using open shape grammars guided by
stratified erosion and invasion percolation**. The paper's stated outputs are
*slot canyons, sea arches, stratified cliffs, fields of hoodoos, and complex
karst cave networks* — which is close to a literal transcript of this owner's
open requests.

Its sibling, *Modeling Rocky Scenery using Implicit Blocks* (Paris et al.
2020) [S], is the one that answers *"rocks be of all different shapes and
sizes"* mechanically: it **generates a distribution of fractures in the
bedrock**, and the blocks those fractures bound become the rocks. Size and
shape are *consequences of the fracture distribution*, not parameters of an
ellipse.

**Why this is the strongest match to the verdicts.** It is the direct answer
to §2's cause B and cause C:

- *Cause B (extents give rectangles).* A shape grammar produces a **profile**:
  a hoodoo is a growth process under a resistant cap, so it gets a foot, a
  waist and a crown because the grammar's rules put them there. Width as a
  function of height comes free.
- *Cause C (a heightfield cannot express an undercut).* An implicit primitive
  is volumetric. A mushroom cap on a thin stem is representable. So is a
  balanced rock, and so is an arch. This project already has the pattern —
  `brows` hangs an overhang the plan cannot hold — and this generalises it.
- *No perfect ovals, no flat-sided slabs.* Both of the owner's shape verdicts
  are complaints about primitives that read as primitives. A block bounded by
  a fracture set has no axis of symmetry to notice; a grammar-grown column has
  no analytic silhouette.

**And this project is closer to it than it looks.** `residual.rs` already
places parameterised objects. `fracture_field.rs` already has a Worley joint
lattice that partitions rock into domains — which is, structurally, the
fracture-set half of the implicit-blocks method already built and used for
destruction rather than for generation.

**What it costs.** Per-object work at generation time, proportional to the
number of objects and their size, not to world area. Poisson-disk sampling is
cheap. The grammar is the expensive part to *write*, not to run.

**Does it survive the constraints?**

- *Determinism:* yes, and this project's `Purpose`-keyed position-indexed
  noise is the right substrate. A grammar seeded from `hash(seed, purpose,
  site_x)` is reproducible.
- *Frame cost:* free, build-time.
- *Streaming:* **the best of the three candidates.** A Poisson-sampled,
  position-keyed object is generated from its own site coordinate and nothing
  else, so a chunk can generate every object whose footprint overlaps it by
  enumerating candidate sites within the margin. This is exactly the
  `margin`-declaring contract `mod.rs` already has, and `residual.rs` already
  declares `RESIDUALS_MARGIN` on that basis. **No coarse map required.**
- *The at-rest constraint:* an object-level pass writes solid, attached
  material and can be verified before writing — the collect-verify-write
  skeleton the generator already uses. But dead end #28 applies: it must
  reject a **breach**, not the object.
- *The honest limit:* it does nothing for *causality*. A Poisson-placed hoodoo
  is a beautiful object with no reason to be where it is. This candidate
  answers "what kind of shape", and §3.1/§3.2 answer "why here". They compose;
  neither substitutes for the other.

## §3.4 Caves with a cause: speleogenetic conduit networks

**The mechanism.** Paris, Peytavie, Guérin, Collon & Galin, *Synthesizing
Geologically Coherent Cave Networks* (CGF / Pacific Graphics 2021) [S, plus
the authors' GitHub [V]]. A cave is not a thresholded noise field. It is
**the path water took**: given **inlets** (sinkholes, where water enters) and
**outlets** (springs, where it leaves), the conduit skeleton is computed by an
**anisotropic shortest-path** search on a nearest-neighbour graph, where the
cost function encodes the geology — **faults, inception horizons, fractures,
permeability contrasts**. Conduit geometry is then a signed-distance
construction tree over that skeleton, with blending and warping operators. The
hydrology literature has the same algorithm independently (KarstNSim, pyKasso,
anisotropic fast marching for karst networks) [S].

**What it gives the owner.** This is the cave answer to *"it looks like it
comes from nowhere and goes nowhere"* and to *"there should be variability
between caves"* at once:

- A cave has **endpoints that mean something**. Its mouth is where the spring
  is; its far end is where the water went in. A player who follows a passage
  arrives somewhere.
- **Variability is structural.** Two caves differ because their inlet/outlet
  pairs and the rock between them differ, not because a size parameter was
  drawn from a wider distribution — which is what round 6's A2 did (envelope
  181×71 → up to 401×161) and which still earned *"looks like a perfect oval"*.
- **Bedding-parallel passages come free.** An anisotropic cost that is cheap
  along the strata and dear across them produces exactly the bedding-following
  anatomy round 3 tried to get by shearing the Worley frame — and it produces
  it as a consequence of the cost field rather than as a warp of a noise
  lattice.
- It **composes with springs**: the same outlet is the cave mouth and the
  spring, so two systems the owner has complained about separately get one
  shared cause.

**What it costs.** A shortest-path search over a graph the size of the cave's
neighbourhood, not the world — so it is a bounded, local computation per cave.
Cheap.

**Does it survive the constraints?**

- *Determinism:* yes, with the same caveat as Priority-Flood — Dijkstra/A\*
  tie order must be broken explicitly by index, or two equal-cost paths are a
  build-to-build coin flip.
- *Frame cost:* free, build-time.
- *Streaming:* **good, with one condition.** A cave is generated from its
  endpoint pair, and its footprint is bounded, so it declares a margin exactly
  as `vaults` does today (`VAULTS_MARGIN`, derived from `MAX_CAVE_HALF_W`).
  The condition is that the **inlets and outlets must themselves be
  determinable locally** — which they are if they come from a coarse map, and
  are not if they come from a global flow-accumulation pass. This is the third
  place in this report where the coarse map turns out to be the enabling
  piece.
- *This engine's own constraint:* `max_unsupported_span: 3` for stone means a
  noise-defined ceiling has no bounded thickness — which is dead end #9's
  surviving argument for *"a controllable-radius mechanism somewhere in the
  pipeline"*. A skeleton-plus-radius conduit **is** that mechanism. The dead
  end asked for this and nobody has built it.

## §3.5 Separating "what kind of place" from "what is the ground height"

**The mechanism, from a shipped game rather than a paper.** Dwarf Fortress
generates **elevation, rainfall, temperature, drainage, volcanism and
savagery** as independent seeded fields, then **classifies biome from a
discrete lookup over rainfall × drainage**, with weighted frequency bands
[S — DF wiki]. Elevation is adjusted first; then rainfall is adjusted for
**rain shadows and orographic precipitation**; then temperature is reset from
elevation and rainfall. Minecraft's density-function graph does the analogous
thing with `slice`, `flat_cache` and `cache_2d` as first-class primitives
[V, recorded in `prior-art-worldgen-slicing.md` §2.3].

**Why this matters more than it looks.** It is the fix for §2's fourth cause.
The classification step is what turns continuous fields into **kinds**. A
biome is not "0.7 of the spire axis"; it is a *label*, and a label can own a
categorical fact — *this kind of place has spires and that kind has none* —
which a multiplier structurally cannot express. That is the owner's sentence,
in an architecture.

Three consequences worth stating for a plan:

1. **The classification must be coarser than a screen.** Dead end #3 is the
   measured proof: a place smaller than the view reads as a cluster, and a
   statistic aggregated over that scale reports success either way. `Purpose::
   RockCountry` at 1700 columns (~3.3 screens) is the right order; the
   `Character` blend is not.
2. **Passes should be gated by kind, not scaled by degree.** A `spires` pass
   that does not run at all outside spire country is a different object from a
   `residuals` pass whose density is multiplied down — and dead end #2 is the
   measurement that they are not interchangeable.
3. **Rain shadow is the cheapest causality win available.** One directional
   pass over the coarse map makes aridity a *consequence of* relief rather
   than an independent axis that happens to coincide — and `Character`'s own
   doc comment already argues for exactly this logic ("one axis moving four
   things, because that is what makes a place read as *dry* rather than as
   four unrelated settings that happen to coincide"). Extending it from
   within-region to between-region is a small change with a legible payoff.

## §3.6 Constraint/collapse for bounded assemblies — and why the WFC rejection does not cover it

Dead end #11 rules WFC out **for streamed terrain**, on four grounds that are
all about unboundedness. Its own re-test clause keeps it live for *"bounded,
non-streamed content generated once — a fixed-size ruin or structure template
where the constraint region has an edge"*.

That is not a loophole to exploit for terrain; it is a genuinely different
application. If a revamp wants authored-but-varied interiors — a ruin, a mine
with timbering, a chamber's furnishing — a bounded solve inside a declared
envelope has an edge by construction, is generated once, and can be made
deterministic because the visit order is fixed by the envelope rather than by
the player's path. `vaults`' collect-verify-write skeleton is already the
right container for it.

**What it does not solve:** the herringbone Wang tile finding in
`prior-art-worldgen-slicing.md` §6.4 is a better answer than WFC wherever the
question is *structural connectivity from position alone* — it gets full
connectivity with no neighbour consultation, which is exactly what a streamed
generator needs and what WFC cannot give deterministically. Do not reach for
WFC where a tiling with a connectivity guarantee will do.

## §3.7 Making primitives stop reading as primitives

Three cheap mechanisms, listed because the owner's verdicts name this symptom
four separate times (*"perfect oval"*, *"flat-sided slabs"*, *"1-pixel
columns"*, *"less visible Voronoi"*) and none of them requires an
architectural change:

- **Domain warping.** Displace the input coordinate by the output of another
  noise before sampling — `f(p + a·f(p + b·f(p)))` — which turns a regular
  cellular structure into an organic one [S, Quílez; and the technique is
  standard]. Applied to the *Voronoi/Worley input coordinate* it is the
  documented fix for cells reading as cells. `Purpose::Warp` already exists in
  this generator and is applied only to the height sample position.
  **Caution:** dead end #18 is the warning. Warping the *coordinate* is safe;
  varying the *pitch* smoothly is not, wherever the consumer is an identity
  test between adjacent cells.
- **A profile function instead of an extent.** Cause B's direct remedy and the
  cheapest thing on this list: any pass that currently draws from `(width,
  height)` should draw from `width(h)`. A single monotone-plus-noise profile
  with a foot and a crown converts a fence post into a tor without touching
  placement, and round 6B named this as the candidate for round 7's spine.
- **Fracture-bounded shapes.** For rock specifically: a block bounded by a
  fracture set has no analytic silhouette to recognise. `fracture_field.rs`'s
  Worley joint lattice is this mechanism, already in the tree, already used at
  destruction time — and using it at *generation* time would make broken rock
  and generated rock share one shape language, which is a coherence win beyond
  the immediate complaint.

## Ranking, for the plan

**Strongest two, and they are complementary rather than competing:**

**§3.3, object-level generation with a shape grammar** — the strongest on
*fit to the verdicts* and on *this project's constraints*. It is the only
candidate that addresses causes B and C directly, it is the only one that is
fully streamable with no coarse map, it composes with the existing
`margin`/collect-verify-write contract without changing the pipeline's shape,
and two of its building blocks (`residual.rs`'s parameterised placement,
`fracture_field.rs`'s joint lattice) are already in the tree. Its honest
limit is that it gives shape without cause.

**§3.1, simulation-first uplift + stream power** — the strongest on *causality*
and the only candidate that voids the most consequential dead end in §1
(#22/#23: erosion cannot grow what the heightfield never offered). It is what
makes a spring originate somewhere, a valley have a reason, and a place differ
in kind from its neighbour without anyone authoring the difference. Its cost is
that flow accumulation is irreducibly global, which makes the coarse map a
prerequisite rather than a nicety — and that is the same coarse map §2 found
has been designed and never built. That is not a coincidence and a plan should
treat it as the load-bearing decision.

**§3.5 is not a third candidate so much as the connective tissue**: whichever
of the two above is chosen, the *kind*-versus-*degree* fix has to happen or
the spire verdict recurs a fourth time. It is also the cheapest item in this
report.

---

## Sources

Papers, all [S] — the primary PDFs are blocked by this container's egress
proxy; mechanism descriptions come from search-engine abstracts and the
authors' GitHub READMEs.

- [Cordonnier et al., *Large Scale Terrain Generation from Tectonic Uplift and Fluvial Erosion*, CGF 2016](https://onlinelibrary.wiley.com/doi/10.1111/cgf.12820)
- [Tzathas et al., *Physically-based analytical erosion for fast terrain generation*, CGF 2024](https://onlinelibrary.wiley.com/doi/abs/10.1111/cgf.15033)
- [Barnes, Lehman & Mulla, *Priority-Flood: An Optimal Depression-Filling and Watershed-Labeling Algorithm for Digital Elevation Models*, Computers & Geosciences 2014](https://www.sciencedirect.com/science/article/abs/pii/S0098300413001337)
- [Génevaux, Galin, Guérin, Peytavie & Benes, *Terrain Generation Using Procedural Models Based on Hydrology*, SIGGRAPH 2013](https://history.siggraph.org/learning/terrain-generation-using-procedural-models-based-on-hydrology-by-genevaux-galin-guerin-peytavie-and-benes/)
- [Peytavie, Paris, Galin, Guérin & Gain, *Terrain Amplification with Implicit 3D Features*, ACM TOG 2019](https://dl.acm.org/doi/10.1145/3342765) — [source code](https://github.com/aparis69/Implicit-Volumetric-Terrains) (the README notes the hoodoo shape-grammar growth process is **not** in the released code)
- [Paris et al., *Modeling Rocky Scenery using Implicit Blocks*, 2020](https://www.researchgate.net/publication/343227434_Modeling_Rocky_Scenery_using_Implicit_Blocks)
- [Paris, Peytavie, Guérin, Collon & Galin, *Synthesizing Geologically Coherent Cave Networks*, CGF / Pacific Graphics 2021](https://onlinelibrary.wiley.com/doi/10.1111/cgf.14420) — [source code](https://github.com/aparis69/Karst-Synthesis)
- [*A karst networks generation model based on the anisotropic Fast Marching algorithm*, Journal of Hydrology 2021](https://www.sciencedirect.com/science/article/abs/pii/S0022169421005540); [pyKasso](https://www.sciencedirect.com/science/article/pii/S1364815225000465); [KarstNSim](https://github.com/ring-team/KarstNSim_Public)
- [Dwarf Fortress wiki — World generation](https://dwarffortresswiki.org/index.php/DF2014:World_generation) and [Advanced world generation](https://dwarffortresswiki.org/index.php/Advanced_world_generation)
- [Domain warping](https://www.mysimulator.uk/domain-warping/); [Organic Voronoi via domain warping](http://thingonitsown.blogspot.com/2018/11/organic-voronoi.html)

In-tree sources, all [V]: `Reports/dead-ends.md` §worldgen and §rendering;
`Reports/worldgen-design.md` §0, §5–§7; `Reports/prior-art-worldgen-slicing.md`
§0, §2.1–§2.3, §6.1, §6.4, §7–§9; `Reports/worldgen-erosion-design.md`
(Status 2026-08, "The scale band between texture and landform is empty",
owner directive 2026-08-20); `Reports/worldgen-implementation-tasks-*.md`
(rounds 1–6, findings sections); `Reports/cave-beauty-review-2026-08.md`;
`Reports/pass-interference-2026-08.md`; `Reports/springs-in-generated-worlds.md`;
`Reports/world-scale-phase-2.md` §5, §7; `Reports/worldgen-round6-handoff.md`;
`src/worldgen/mod.rs`, `region.rs`, `erosion.rs`; `src/sim/chunk.rs:47-55`.
