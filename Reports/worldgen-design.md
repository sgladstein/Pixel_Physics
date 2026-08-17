# World generation design

**Audience:** the coding agent working on Pixel Physics.
**Reviewed at:** `b2ebea8`, branch `master`.
**Status:** direction agreed with the repo owner. Companion to `emergent-world-architecture.md` (which this depends on heavily) and `pixel-physics-issues.md`.

**See also `Reports/prior-art-worldgen-slicing.md`** — a later prior-art survey asking what other games and disciplines have actually done with the §0 slice framing, and with §6a's "generated terrain must be at rest." It does not overturn anything settled here, but it sharpens five things (its §8 lists them) and answers three of §13's open questions from outside games: off-plane flux, the curvature bound on a curved route, and cave surface connectivity.

**Current state of worldgen:** §11 steps 1–2 are **built** (`src/worldgen/`, driven by `assets/worldgen.ron`): parameters as data, and a seeded heightfield with the six-zone vertical structure replacing the old hand-authored terrain, which survives verbatim as the `legacy` preset. The decide/realise split is in place and no pass is world-global yet, so per-chunk generation stays a change of caller. **Step 3 (water table + moisture) is deliberately not built** — it is gated on a playtest of the terrain alone, because the water table is the part of this design most at risk of being right and not fun; the `arid` preset exists as the standing proof that removing water entirely is a data change. Steps 4–7 (coarse map, water cycle, caves, world age) and streaming are untouched.

`PLAN.md`'s M10 is one line — seeded noise, LRU unload, RLE compression. That line hides essentially every problem in this document.

**Revision note (important).** The first draft of this document was written from training knowledge plus a read of the codebase — *not* from the literature. It has since been revised against primary sources, and **three of its recommendations were wrong**: the cave-carving approach (§7), the definition of ridged noise (§7), and the terrain-then-rivers ordering (§5). A further pass added §0 after noticing a larger problem — **nearly all terrain literature means "2D heightmap seen from above," while this engine means "2D vertical cross-section,"** and the first draft applied planar techniques without flagging which layer they belong to. Sections carrying a **[revised]** tag changed materially. Sources are listed in §12.

**Settled with the owner — do not re-litigate:**

- **The world is 2D side view; the worldgen is 3D.** The coarse layer is planar over (x, z); the play world is a *vertical* slice through it. Nothing in `sim/` ever sees z. See §0.
- **Slice topology (straight cut vs curved route) is open and free to defer** — but **reserve a slice-identifier field on `ChunkCoord` now**; adding one later is a 42-site migration plus a save-format break. See §0.
- Vertical structure is **defined**, not homogeneous-with-caves. Six zones (§2).
- Deep structure is **field-defined** (scheme C); shallow structure is **accumulated history** (scheme D). See §3.
- Depth is **bounded** — bedrock floor. Revisit only if something concrete needs otherwise.
- **Caves: yes.** Density-function based (caves are where stone is *not placed*), not carved. See §7 — this reverses the first draft.
- Worldgen takes **world age** as a parameter, not just seed and coord. See §4.
- Worldgen is reproducible **within one world's lifetime**; no cross-version seed promise (`emergent-world-architecture.md` §8h).

---

## 0. Dimensionality: 2D side-view play, 3D worldgen **[new]**

**The single most important framing in this document, and the one most easily got wrong when reading terrain literature.**

### The decision

- **Play world:** 2D, **side view**. `x` horizontal, `y` vertical, gravity along +y. Unchanged from today.
- **Coarse layer:** planar over `(x, z)` — a top-down map carrying elevation, drainage, climate.
- **The play world is a *vertical* slice** through that coarse map. A cross-section. Whether that slice is a straight cut at fixed `z` or a **curved route following the drainage network** is deliberately open — see "Slice topology" below. It is free to defer, because the curve lives entirely behind worldgen's interface.
- **Nothing in `sim/` ever sees `z`.** It is a worldgen coordinate only. No invariant in `world.rs`, `chunk.rs`, or `parallel.rs` changes.

The slogan: **the world is 2D; the worldgen is 3D.**

### Why not full 3D

Arithmetic settles it. The README's own figure is 2048² × 8 bytes = 32 MB. Cubed, **2048³ = 68 GB**. Even 512³ is 1.07 GB of cells before the field grid. Worse on compute: the sandbox is 512×320 = 164k cells at ~11.5 ms worst frame; 512³ is 134M cells, **819×**, i.e. seconds per frame under perfect scaling.

Secondary damage: `rebuild_blocked` already does ~164k hashed lookups per frame, and a 3D field tile is 8³ = 512 cells rather than 64, taking it to ~10M. The checkerboard proof in `parallel.rs` goes from 4 passes to 8 and needs re-deriving. `MAX_REACH == CHUNK_SIZE/2` is load-bearing in two places and needs re-proving.

**But the real objection is legibility.** The appeal of falling sand is *watching material interact* — layering, flow, fire climbing, a slope slumping. In 3D you see a surface and the interesting part is hidden inside it. Noita is 2D by choice. Minecraft is 3D precisely because it does *not* simulate every block every frame. Full 3D costs three orders of magnitude to make the world less readable.

### Why not top-down 2D

Top-down puts gravity perpendicular to the simulated plane, which removes it from the simulation entirely. What breaks:

| System | Why |
|---|---|
| `update.rs` entirely | `update_powder` falls to `y+1`; `update_gas` rises to `y−1`; `try_move`'s density displacement flips on `(ty − y).signum()`. All meaningless without in-plane gravity. |
| Bottom-to-top sweep order | Exists solely so a falling cell isn't re-examined at its new position |
| `MAX_REACH` asymmetry (±32 horizontal, ±1 vertical) | Encodes "material moves far sideways, one cell vertically" |
| Angle of repose | `roll_reach_at`, `friction_angle`, the `1/tan(angle)` derivation — piles are a gravity phenomenon |
| **All of M17** | Anchor distance, unsupported span, collapse. Nothing falls. |
| **Most of M16** | Gravitropism has no direction; phototropism has no "above" |
| The §2 vertical zones | Every one is a function of depth, and depth is not in the plane |

Surviving: the field grid, fire, diffusion, moss-style spread, creature movement — the emergent-systems half. But that discards the falling-sand half, which is M1–M15 and the engine's identity.

**One honest cost of choosing side view.** Ant foraging is natively top-down — every double-bridge diagram in `stigmergy-research.md` is drawn from above. Ant *nests* are natively side view; every real nest cross-section and every one of Toffin's digging experiments is a vertical section. The ant vision is split across orientations and **side view gets the better half**: chambers, tunnels, real depth, the density-driven branching transition, roots and water intersecting the nest. Foraging becomes trails along a line rather than across a plane — fewer competing paths, so the double-bridge scenario is harder to stage. Not fatal (a trail up a cliff, into a cave, and back is still path selection, and verticality adds route choices a flat plane lacks) but a real simplification, and it should not come as a surprise later.

### Why the 3D coarse layer is nearly free

The coarse map is needed regardless (§5). Making it planar rather than linear costs more storage for a structure that is already one value per chunk-column, and the simulation never touches the extra axis. What it buys:

- **Drainage networks become possible at all** — a linear world has nowhere for a river to branch (§5a)
- **Off-plane flux becomes principled rather than a cheat.** Water entering from a tributary behind the plane, or leaving toward one, is a legitimate boundary condition *because the coarse map actually computed where it goes*
- **Biomes and rain shadows** get a second axis to vary along
- **A top-down map view becomes possible** — see below
- **Regeneration stays cheap** — the coarse map is still a pure function of seed

### A top-down map view is a UI feature, not a physics one

Zooming out to a top-down map rendered *from the coarse layer* — drainage, biomes, colonies, where the fire is — is nearly free, since the coarse map exists anyway. It is **rendered, never simulated**, so none of the top-down objections above apply. It solves a genuine problem: in a side-view world there is no way to perceive anything larger than a screen.

### Slice topology: straight cut, or curved route? **[open, deliberately deferred]**

**The problem a straight slice has.** A drainage network meanders. A straight line through it crosses the river at several points at arbitrary angles, and **at every crossing the downstream direction is perpendicular to the slice.** The play world sees a puddle in a valley with nowhere to flow. You never see a river; you see river cross-sections. This makes §5a's "plausible vs real off-plane flux" question the *normal* case rather than an edge case.

**The curved-route alternative.** The play world's `x` is arc length along a *curve* through the coarse `(x, z)` plane rather than a straight line — follow the valley instead of cutting across it. Upstream and downstream both exist in the play world, and water has somewhere to go.

**Why routes need not conflict, which is the objection that would otherwise sink this.** If routes could cross, the same physical `(x, z)` point would exist at two different play-`x` positions — dig a hole on one route and it isn't there on the other. A genuine contradiction.

**But a drainage network is a tree: rivers don't cross, they merge.** Routes following the drainage network therefore never overlap *by construction*, and confluences become junctions. That yields:

- **Travel between slices is walking to a confluence and taking the other branch** — diegetic and visible, not a teleport or a plane-scrub
- **The top-down map becomes a traversable route network** rather than a window onto terrain that can never be reached. This matters: a map showing a river system you cannot visit is strange, so **map value and slice travel are coupled decisions**
- **Persistence keys cleanly on `(route, x)`** with no overlap conflict
- **Coarse-level water routing has somewhere to route to** — §5a's scale problem

**The real unsolved cost is junction representation.** Walking along a valley and reaching a fork has no natural side-view depiction. Camera swing, far-branch parallax, or an explicit transition moment are all plausible and none is worked out. Related: a colony left on the branch not taken is handled mechanically by catch-up, but "my ants are down the other fork" feels different from "they're behind me."

Self-intersection remains possible if a single meander doubles back tightly enough — a curve-smoothing constraint, not a structural problem.

**Why this is free to defer.** The curve lives **entirely behind worldgen's interface**. `terrain_at(play_x)` does not care whether `play_x` indexes a straight line or an arc-length parameter along a curve. **Nothing in `sim/` ever learns the difference.** Build straight now; add curvature later with zero change below worldgen and only internal change within it.

**But reserve the identifier now.** `ChunkCoord` is `(x: i32, y: i32)`, constructed in 42 places, used as the `HashMap` key throughout `world.rs`, and destined for the save format. Adding a third field later is a 42-site migration plus a save-format break; adding one now, always zero, is mechanical. Make it a **generic slice identifier**, not specifically `z` — straight slices want a `z`, routes want a route id, and one `u32` covers either without committing to which.

**Two questions to settle eventually, neither urgent:**

- **Is the route the river, or something else?** Ridgelines, coastlines, and valley floors are all plausible spines. Rivers are the best default — a natural tree, and they give water somewhere to go — but a world following a ridge would look completely different.
- **Does the player choose the route, or does worldgen?** Derived deterministically from the drainage network, worldgen picks and the player traverses. If the player can cut new routes, the overlap problem returns.

### Deferred, not rejected: layered slices

N parallel slices at adjacent z, weakly coupled so water and creatures can cross between neighbours. Cost is N×, not N³, so 3–5 slices is affordable. Would give ant nests real branching depth, visible tributaries joining from behind, caves with thickness.

Blocked on **rendering** — how to show five overlapping slices without producing soup. Parallax and transparency help and do not solve it. It also multiplies the field grid, which is already the budget constraint (`emergent-world-architecture.md` §9). **Worth prototyping later; wrong to commit to now.**

---

## 1. The principle: worldgen is initial conditions for channels

Worldgen is not a separate feature that produces terrain. **It is the initial state of every channel**, and it has an emergent version and a non-emergent one:

- **Non-emergent:** place stone here, sand there, water in the hollow. Static terrain that channels then react to.
- **Emergent:** generate the *fields* — moisture, temperature, light exposure, compaction — and let materials and vegetation follow from them.

Take the second. A valley is wet *because it is a valley*. Vegetation grows where moisture allows. **Biomes are consequences, not labels** — a desert is where moisture is low, not a region tagged `desert`. That gets you biome variation from one or two noise octaves instead of a biome table, and it is the same philosophy as everything else in the architecture.

---

## 2. Vertical structure

Six zones, top to bottom:

| Zone | Serves |
|---|---|
| **Sky** | Light source, weather, wind. Where the day/night oscillator writes. |
| **Surface / canopy** | Plants, moss, light competition |
| **Soil / regolith** | Roots, nests, decay accumulation, moisture retention. **Deepens with world age.** |
| **Saturated zone** | Below the water table. Springs where it meets the surface. |
| **Rock** | Caves, mining, structural mass. Geothermal gradient begins. |
| **Deep / bedrock** | Heat. Anchor for M17 structural checks. World floor. |

### The water table is the highest-value single structure

Moisture increases with depth to the table, then saturates. Free consequences:

- Roots get something real to grow toward (they currently scan for `Liquid` cells — `plant.rs:561`)
- Ants get a reason to dig down *and a depth they stop at*
- Springs where the table intersects the surface
- Valleys wet because they are low
- Flooded caves below the table, dry caves above (§7)

It is a function of elevation, so it comes from the coarse map (§5) for free.

**In side view the water table is a visible line**, not an abstract field — it is the horizon where saturated meets unsaturated, and the player can see it in any cut face, cave wall, or well. That legibility is a real argument for making it a first-class structure rather than an implicit consequence of moisture diffusion.

**Revised in §7:** a *single global* table is the weaker version. Local aquifers — water bodies with their own independent level — give springs, perched water, and dry-cave-next-to-wet-cave gradients for little extra cost. Read §7's aquifer subsection before implementing this.

### The geothermal gradient gives temperature a reason to exist

The temperature channel currently only means anything near fire. A depth gradient makes it a permanent, navigable field — and gives creatures a vertical axis to have preferences about. Real gradient is ~25 °C/km, which is far too slow for a game world; it needs compression, and that compression factor is a world parameter (§6).

---

## 3. How zone boundaries are defined

Four schemes were considered:

- **A — authored bands.** Fixed depths, noise-perturbed cuts. Simple, chunk-local, predictable, boring.
- **B — two anchors, interpolate.** Only the surface heightmap and bedrock depth are authored; everything between is interpolation plus noise. Very cheap, follows terrain automatically, produces nothing surprising.
- **C — field-defined.** No explicit layers. Generate moisture(depth), temperature(depth), and a compaction gradient; material is a function of the three. Maximally consistent with the architecture — layers become consequences, same as biomes. Risk: hard to control, can produce mush.
- **D — depth as accumulated time.** Layers are products of processes. Soil is where decay has accumulated; sediment is where water deposited. Digging down is digging backward through the world's history.

**Decision: C for the deep structure, D for the shallow.**

Below the water table, everything is a function of depth and compaction — cheap, static, regenerable, **needs no persistence**. Above it, soil is accumulated history — **persisted**, age-dependent, and the thing that makes an old world feel old.

That split lands exactly on the derived-vs-accumulated taxonomy in §8, which means **the biologically active zone is the only thing ever saved.**

**D cannot be the only mechanism for the shallow zone.** A brand-new world would have zero soil and nothing could grow. Starting soil depth comes from C; D accumulates on top of it.

---

## 4. `worldgen(seed, coord, world_age)`

**This is the piece most likely to be missed, and it is load-bearing for the catch-up model.**

Catch-up needs elapsed time. A chunk that has **never been visited** has no unload timestamp. If worldgen always produces the world as it was at t=0, then walking into fresh territory on day 400 means either simulating 400 days on the spot (unaffordable) or finding a pristine seedling world adjoining a mature one — **visible as a hard line at the frontier.**

So worldgen takes world age. A chunk generated on day 400 generates a 400-day-old ecology directly: mature trees, accumulated soil, established moss.

Two consequences worth stating plainly:

1. **Worldgen and succession become the same function evaluated at different times.** Worldgen is that evaluation at `world_age`; catch-up is the same evaluation at `age_now` given state at `age_then`. One model, three uses.
2. **The frontier stops being visible.** No line between "generated recently" and "lived in a long time."

This means worldgen output is not terrain alone but *terrain plus an ecological state consistent with its age* — more work, but it is the same work catch-up needs regardless.

---

## 5. The layered architecture is forced

Streaming plus determinism means chunk generation must be `f(seed, coord, age)` — computable without consulting neighbours. But everything interesting is global: rivers that reach the sea, mountain ranges, rain shadows, **anchor distance to bedrock**.

Resolution: **two layers with different lifetimes.**

- **Coarse global map** — one value per chunk, or per 8×8 chunks. Cheap enough to generate for a large region at once, and *because* it is coarse it can use **iterative, global** algorithms: hydraulic erosion, river routing, uplift. Generated once, cached, persisted.
- **Fine local detail** — pure function of the coarse map plus seed plus coord. Chunk-local, order-independent, regenerable.

Anchor distance sits at the coarse layer. Rivers sit at the coarse layer. Individual rocks sit at the fine layer.

### 5a. Rivers first, terrain second **[revised]**

The original draft said: generate a heightmap, then erode it. **Génevaux et al. (SIGGRAPH 2013) inverts this**, and the inversion matters here.

<cite index="67-1">Their framework uses rivers as the modeling element: it first creates a hierarchical drainage network represented as a geometric graph over the domain, then analyses that network to construct watersheds and characterise river types and trajectories, and only then generates terrain by combining procedural terrain and river patches with blending and carving operators.</cite>

**Read this as applying to the coarse layer only (§0).** Génevaux's drainage graph is planar — it needs an `(x, z)` plane to branch across. It has no meaning in the play world's `(x, y)` cross-section, where there is only one horizontal axis. This is exactly the kind of literature-to-engine mismatch §0 exists to prevent: nearly all terrain literature means "2D heightmap viewed from above," and this engine means "2D vertical cross-section."

Why the ordering is right *for this engine specifically*: the water table (§2) is the load-bearing structure — roots, ants, springs, flooded caves all key off it. Deriving water from terrain makes the most important structure a second-order consequence of noise. Deriving terrain from a drainage network makes it primary, and it fixes the failure mode noise-based terrain is notorious for: rivers that don't reach the sea.

### What a river *is* in a side-view slice **[new]**

The surface in the play world is a 1D function `h(x)`. So:

- A **watershed divide** is a local maximum of `h(x)` — water goes left or right from a crest
- A **river** is a channel following a local minimum
- **Tributaries can only join from above** — a stream coming down a hillside into the main channel. That is the one confluence type a cross-section can show, and it happens to be the picturesque one
- **Plan-view confluences and deltas do not exist.** You are looking at one cross-section of a network that exists on the coarse layer

### And the crucial difference from every 3D terrain generator

**They fake rivers because they have no water simulation. This engine has one.** Water is a `Liquid` with `dispersion: 5`; it flows downhill and levels on its own.

So the job is not "generate a river." It is **"generate terrain with drainage structure — local minima, valleys, an outlet — and let real water find it."** The river is emergent, which is the architecture doc's whole thesis applied to terrain.

That reframes what hydrology-first buys: not a drainage graph to rasterise, but **drainage structure as a property `h(x)` is designed to have** rather than something noise is hoped to produce.

### Which forces the water cycle

Water is conserved in this engine. A river needs a continuous **source** and a continuous **sink**, or it is not a river — it is a puddle that formed once and stopped.

- **Source:** rain, or a spring where an aquifer meets the surface (§7)
- **Sink:** the ocean, the world edge, evaporation, or genuine off-plane outflow computed by the coarse map (§0)

Without evaporation the world floods. Without rain the river dries and never returns. **A persistent river requires the closed water cycle** — the second example in `emergent-world-architecture.md` §0's list of matter cycling rather than running out.

**So rivers are not a worldgen feature at all.** They are a consequence of the water cycle plus terrain with drainage structure. That is a better answer than anything the 3D terrain literature offers, because that literature is solving for appearance and this engine can solve for mechanism.

### The scale problem this creates

A river spanning 10,000 cells cannot be CA-simulated end to end — only loaded chunks run. This needs **coarse-level water routing**: which chunk drains to which, and at what throughput, with CA water only where it is watched. That is the catch-up model (`emergent-world-architecture.md` §8c) applied to water, and it is real work rather than a detail.

### One decision to make explicitly

Off-plane flux (§0) can be **real** — the coarse map genuinely computes upstream and downstream and the slice honours it — or **plausible**, where water simply appears at the boundary at a believable rate. The lazy version works for a long time and then is wrong in a way that is very hard to trace. Decide which is being built rather than discovering it later.

**And their data structure is directly useful.** <cite index="67-1">Terrain is stored as a construction tree whose internal nodes are operations and whose leaves are terrain features; the representation is analytic and continuous, and renderable at varying level of detail.</cite> An analytic construction tree can be **evaluated at any point without generating its neighbours** — which is exactly the chunk-local determinism constraint. A tree is also serialisable and cheap to persist, so it fits the coarse-layer role in §5 better than a rasterised heightmap would.

### 5b. Erosion, and a finding about `field.rs` **[revised]**

Worldgen erosion and runtime erosion are the same process at different time scales — the runtime version is the stigmergic loop from `emergent-world-architecture.md` §0 (water carves a channel, the channel carries more water). Pre-erode the coarse network for plausible valley shapes; let runtime water carve local detail.

The standard method is **Mei et al. (2007), the virtual pipes model**. <cite index="86-1">It works on a 2D grid where virtual pipes dictate flow between cells; each cell holds terrain height, water height, suspended sediment, four outflow flux components, and a velocity vector.</cite> <cite index="87-1">Water moves through the pipes by hydrostatic pressure difference between neighbouring cells, and the pipe model is an explicit method for the shallow-water equations.</cite>

**Caveat on orientation.** Mei's model is a *heightmap* model — a top-down grid of columns each holding a water height. It does not transfer directly to a side-view world that simulates water as actual cells. **It applies to the coarse layer** (§0), where terrain genuinely is a heightmap over `(x, z)` and pre-erosion is exactly the job. In the play world, erosion is whatever real water does to real material.

That said, **the machinery is structurally almost identical to what `field.rs` already does.** The existing field solve accumulates velocity divergence into pressure and the negative pressure gradient into velocity — the same coupling, on the same kind of grid, already Jacobi and already wall-aware. **Erosion should very likely reuse the field grid rather than being a separate system**: add sediment as a channel, and the transport machinery already exists.

Two constraints that come with it:

- <cite index="87-1">The pipe model has a CFL stability condition — the time step multiplied by velocity must not exceed the cell size, so as grid resolution increases the time step must decrease proportionally.</cite> **This directly constrains the fast-forward plans**: erosion cannot be accelerated by simply taking larger time steps. It has to be run at more steps, or at a coarser grid.
- A follow-up line of work extends virtual pipes to multi-layered heightmaps that produce overhangs, arches, and to some extent caves, and pairs it with **an iterative bedrock support check that propagates support to prevent floating terrain and unrealistic overhangs**. That support check is M17's structural integrity under a different name — erosion and support-checking are a known, necessary pairing, not two independent systems.

---

## 6. Two hard constraints that will bite

### 6a. Generated terrain must be *at rest* — unique to falling sand

Most worldgen produces static geometry. This engine produces material that obeys physics. Generate a sand slope at 50° against sand's 34° repose angle and the slope slumps the instant the chunk wakes.

**This is worse than cosmetic.** `World::set` creates chunks on demand, so a slump propagates across chunk boundaries **into chunks that do not exist yet** — that chunk now has content before worldgen ever ran there. When worldgen later runs, it either overwrites material or must merge. The bug is intermittent because it depends on visit order.

Three options:

1. **Generate only `Solid`.** Stone does not move. Loose material appears only where the player creates it or erosion produces it at runtime. Safest — and probably how Noita avoids this, its generated terrain being overwhelmingly static rock.
2. **Generate physically valid configurations.** Worldgen must know each material's repose angle and never exceed it. Couples worldgen to material data; one bad `.ron` edit silently breaks terrain.
3. **Settle at generation time.** Run the sweep before revealing the chunk. But settling needs neighbours, and neighbours may not exist — circular.

**Recommended: option 1**, with loose material allowed only in trivially stable configurations (flat-bottomed or fully enclosed pockets).

### 6b. The structural-integrity landmine

`structural.rs` deliberately never checks pre-placed terrain — its module doc explains why: the sandbox floor is 8 cells thick against stone's `max_unsupported_span: 3`, and the ledges float with no anchor path. Under a literal reading both would crumble immediately.

The current escape is that untouched terrain keeps `aux = 0`, **which is indistinguishable from "anchored."** That works for hand-placed terrain. **At worldgen scale it is a landmine**: the entire world becomes structurally invalid but reading as anchored, and any change to when checks are scheduled causes global collapse.

**Worldgen should compute real anchor distances** — a cheap BFS from bedrock, once per chunk. Then a mountain is genuinely anchored rather than accidentally exempt, and digging into it behaves correctly for the first time.

Anchor distance is a global property, so it belongs on the coarse layer (§5).

**Ordering — but see §7.** If caves are *carved*, the order must be: carve → compute anchor distances → place material, or distances are wrong wherever a cave exists. **If caves are an absence of placement in a density function (§7, the recommended approach), this ordering problem disappears entirely** — there is one final density field and anchor distances are computed against it once.

---

## 7. Caves

### Why they earn their cost in *this* engine specifically

Not decoration — caves are the first content that makes existing systems matter:

- **Flooded caves are free.** Implement caves, a water table, and water-as-material; caves below the table fill, caves above stay dry. The boundary is where springs happen. Nothing implements "flooded cave."
- **Caves are dark, and dark matters.** Once light has a writer (`emergent-world-architecture.md` §2), a cave is a real darkness gradient. Moss grows near entrances and stops deeper in — actual cave ecology, zero cave-specific code.
- **Caves weaken mountains.** M17's anchor distance must route *around* a void, so a cave genuinely reduces the support above it. Dig badly and the ceiling comes down. **This is the mechanic M17 was built for and has never had a real test case.**
- **Caves are the pre-existing void ants would actually use** — real ants exploit existing cavities rather than excavating everything.
- **Caves exercise the field grid's sealed-room case**, which `field.rs` already fought three rounds of boundary-condition bugs over.

### Generation: additive density, not subtractive carving **[revised — the original recommendation was wrong]**

The first draft recommended worm-carving as primary with noise as a modifier, and framed the whole operation as *carving voids out of rock*. **Minecraft's 1.18 Caves & Cliffs rewrite went the other way, and the reframing is the important part.**

<cite index="101-1">The update replaced cave generation with an additive system based on 3D noise. Rather than carving out existing stone, the generator uses noise to decide where stone should be placed at all: above a threshold, stone; below it, air — which is the cave.</cite>

That inversion removes an entire class of problem. There is no carve pass, no ordering question between carving and placement, and no possibility of a carve landing somewhere generation hasn't reached. **It also resolves §6b's ordering constraint**: if caves are an absence of placement rather than a subtraction, anchor distances can be computed once on the final density field, with no carve-then-anchor sequencing to get backwards.

<cite index="99-1">Minecraft's noise caves come in three named forms — cheese (pocket cavities), spaghetti (long winding tunnels), and noodle (a thinner variant) — controlled by three noise maps: frequency, hollowness, and thickness.</cite> Different noise parameterisations, one mechanism.

**But do not discard worms entirely.** Two of the original concerns survive contact with the literature, and one is specific to this engine:

- **Reachability.** Noise gives no guarantee that a cave system connects to the surface. Minecraft handles this with an explicit cave-entrance term in the density graph rather than by hoping. Whatever the mechanism, surface connectivity has to be *made* to happen.
- **Structural spans — and Minecraft does not have this problem, so its solution does not cover it.** `stone.ron` sets `max_unsupported_span: 3`. A noise-defined ceiling has no bounded thickness; a thin one exceeds the span and collapses the moment anchor distances are computed. Minecraft has no structural integrity system and therefore never had to care. **This remains an open problem for a density-function approach here**, and it is the strongest surviving argument for keeping a controllable-radius mechanism (worms, or a span-aware post-pass) somewhere in the pipeline.

There is also a cost signal worth noting: Minecraft shipped 1.18 with a known performance bug titled *"Inefficient generation of aquifers, noise caves and ore veins."* Density-function generation is not free at scale.

### Aquifers: local water bodies beat one global table **[new]**

§2 proposed a single global water table. Minecraft's model is better and costs little more.

<cite index="101-1">1.18 introduced aquifers: localised bodies of water or lava that generate with their own level, independent of the global sea level, producing underground lakes.</cite> <cite index="96-1">If aquifers are disabled, almost all caves below sea level simply fill with water</cite> — the single-table behaviour, which they explicitly chose against. Their noise router exposes `fluid_level_floodedness`, controlling how likely a given cave is to contain liquid, and a `barrier` term governing whether adjacent water bodies stay separated.

For this engine: a perched aquifer high in a rock face is a spring; a deep one is a flooded cavern; a dry cave next to a wet one is a moisture gradient a creature can navigate. All of that from making the water level a *field* rather than a constant.

Also worth stealing: <cite index="104-1">near the bottom of the world the density function is gradually forced toward a fixed value so caves cannot expose or penetrate the bedrock layer.</cite> Bedrock has to be defended explicitly, or caves will breach the world floor — and in this engine that means material falling out of the world.

### Noise definitions, corrected **[revised]**

The original draft defined ridged noise as `1 − |noise|` applied to fBm. **That is the common simplification, not Musgrave's construction, and the difference is what makes it look like terrain.**

Musgrave's ridged multifractal is a **multiplicative cascade**: each octave's contribution is weighted by the *previous octave's local value* (`weight *= signal` in his reference implementation), with an `offset` added per octave and a `gain` multiplied in. <cite index="78-1">Weight for a given frequency is derived from lower-frequency samples, creating a feedback loop that makes rough areas rougher and smooth areas smoother.</cite> <cite index="77-1">Multiplying each successive octave by the current value of the function means that near "sea level" the higher frequencies are heavily damped and the terrain stays smooth.</cite>

That heterogeneity — mountains rough, plains smooth, within one function — is the whole point, and plain `1 − |fBm|` does not produce it. Musgrave's suggested starting parameters: `H = 1.0`, `offset = 1.0`, `gain = 2.0`.

**Worley** (cellular/Voronoi) is unchanged from the original draft: value = distance to the *n*th nearest feature point; F1 gives blobs, **F2 − F1** is near zero along cell boundaries and thresholding it yields chambers linked by passages.

**The 2D advantage still holds.** In 3D the zero-set of a noise function is a surface, so thresholded ridged noise carves thin sheets — the usual fix is intersecting two independent ridged fields. In 2D the zero-set is already a curve, so channels come from one field. This world is 2D; the problem is skipped.

Remaining downsides, all still valid:

| Issue | Detail |
|---|---|
| Worley evaluation cost | Naive: 3×3 feature-cell neighbourhood, ~9 distance computations per sample vs Perlin's 4 gradient dots. Paid once per cell at generation, not per frame — generation latency, not frame budget. Still matters for hitch-free streaming. |
| Worley cell size is visible | Voronoi cells are roughly uniform by construction; real cave systems vary wildly. Jittering density, octaves, and domain warping all help and all multiply cost. Distance metric is a tell: Euclidean gives round chambers, Manhattan/Chebyshev obviously axis-aligned ones. |
| **Ridged has a derivative discontinuity** | The `abs()` creates a genuine kink; the gradient is discontinuous along the ridge. Irrelevant for a threshold decision. **A trap if that field is ever sampled as a channel** — and this architecture is gradient-following throughout. |
| No connectivity guarantee | Either can produce a beautiful system sealed in rock with no route to the surface. |
| No structural-span awareness | See above. The one Minecraft's approach does not solve for this engine. |

### Wave Function Collapse: examined and ruled out **[new]**

WFC was named as a blind spot in the original draft — constraint-based generation is the obvious answer to "noise gives no authorial control or connectivity guarantee." It was investigated and **it does not survive this engine's determinism requirement.**

Kleineberg's infinite-WFC city is the reference attempt. His own list of limitations is decisive:

<cite index="107-1">The result of generation depends on the order in which parts of the map are generated, and therefore on the path the player takes. Memory cannot be released when the player leaves an area, because there is no way to know when distant slots stop affecting local generation. The longer you walk, the higher the chance the algorithm hits a dead end. And because there are no chunks, all operations on the map structure must be sequential and cannot be threaded.</cite>

<cite index="106-1">His mitigation is backtracking over a stored history of collapses — which mostly works, but errors are sometimes recognised very late, causing many steps to be undone, and in rare cases the slot the player is standing in gets regenerated.</cite> He concludes the approach is unsuitable for commercial infinite worlds, and the follow-up literature agrees.

**"Result depends on player path" is precisely what `worldgen(seed, coord, age)` forbids** (§4), and it breaks catch-up, regeneration-on-reload, and replay simultaneously. His first attempt — generate per chunk, constrain against neighbours — failed for the reason that generalises: <cite index="106-1">collapsing one slot propagates constraints several slots away, so chunk-local generation picks modules that turn out to be illegal once neighbours are considered, and the next chunk has no solution at all.</cite>

**Verdict: not applicable to terrain here.** It remains plausible for bounded, non-streamed content generated once — a fixed-size ruin or a structure template placed as a unit — where the constraint region has an edge.

## 8. Channel persistence: derived vs accumulated

The owner's answer was "persist it." A taxonomy shrinks that substantially:

| Channel | Kind | On reload |
|---|---|---|
| Light | Derived from geometry | **Regenerate** — a function of what blocks it |
| Temperature | Mostly derived (depth, light) | **Regenerate**, let it re-settle |
| Pressure / velocity | Transient | **Discard** — re-settles in a few frames |
| Moisture | **Hybrid** — water table derived, puddles accumulated | Regenerate base, **persist deviation** |
| Pheromone | Purely accumulated | **Persist** — no alternative |

**Two of the five current channels never need saving at all.**

**Per slice.** Everything in this table is per-route/per-slice state (§0). One slice is one set; N visited slices multiply it. Derived channels are unaffected — they regenerate wherever you are — so this is another argument for keeping the derived column as large as possible.

The hybrid row argues for storing moisture as **deviation from the derived baseline** rather than absolute value. Undisturbed terrain then stores zeros, which RLE-compresses to nothing — which is exactly what M10's plan already assumes about pixel worlds.

---

## 9. Do not hardcode the depth split

**The ~100–200 cell "biologically active zone" is a current calibration, not an architectural constant.**

It derives from today's parameters (`SCATTER_HEIGHT = 56`, so trees are ~56 cells; roots ~30–60 below; ant nests plausibly 50–150 deep). Those parameters are deliberately realistic **as a starting point**. The owner's stated intent is to then tune them freely — same physics rules, different constraints — to find what is most interesting to play in. Trees might become 500 cells tall.

**So if "top 200 cells" gets baked into the persistence boundary, tick-rate tiering, water table depth, or LOD selection, all of it is silently wrong the day a parameter changes, and it becomes a hunt across five modules.**

**The fix already exists in the codebase: a chunk is biologically active if it contains active sites.** `scheduler.rs` tracks exactly this and `active_site_count()` already reports it. Persistence, tick tiering, and detail level should key on *"does this chunk have active sites or accumulated deviation from worldgen"* — never on a depth constant.

That is strictly better than a depth rule anyway: it correctly classifies a deep cave containing a nest, which a depth threshold would wrongly call geology.

**Exception:** water table depth is a genuine worldgen parameter rather than a consequence, so it stays authored — but as a *world profile* value (§10), not a source constant.

---

## 10. The parameters-as-data gap

Current tuning surface, hardcoded in Rust:

| Module | `const` count |
|---|---|
| `plant.rs` | 34 |
| `field.rs` | 12 |
| `creature.rs` | 8 |
| `structural.rs` | 2 |

Only **materials** are data-driven.

`PLAN.md` locked "Materials: data-driven from the start," with this reasoning: hot-reloadable files sidestep Rust's slow compile times exactly where iteration speed matters most — tuning.

**That reasoning applies verbatim to all 56 of those constants, and the stated plan is now to do a great deal of tuning.** "Mess with parameters to find what is most interesting" is currently a recompile-per-experiment loop.

The existing split is historical, not principled: `worm.ron` carries the worm's *thermal* numbers as data, while its energy budget, burrow cost, and heat threshold are `const`s in `creature.rs`.

**Proposal:** a `species/` directory alongside `materials/`, reusing the existing hot-reload machinery (`notify` is already a dependency; `MaterialRegistry::reload` is the pattern). Then:

- `tree.ron` carries `segment_length`, `influence_radius`, `branch_angle`, `max_tips`, `growth_cost`, tick intervals
- `worm.ron` gains its behavioural numbers alongside its thermal ones
- a **world profile** carries water table depth, geothermal compression factor, diffusion rates, decay rates

Tuning becomes save-file-and-watch, which is exactly the loop the scene-based verification protocol wants (`emergent-world-architecture.md` §10).

**And it makes the "different physics, different constraints" worlds into data rather than forks.** A world where trees are 500 cells tall is a `.ron` file, not a branch. That is the difference between exploring the parameter space and maintaining variants.

---

### Convergent evidence for this from the literature **[new]**

Two independent shipped/published systems arrived at the same answer:

- Génevaux et al. store terrain as a **construction tree** — a composable graph of operations and feature leaves, not a fixed algorithm (§5a).
- Minecraft 1.18 exposes worldgen as a **noise router: a graph of composable density functions**, defined in data, computing a value per position and used for terrain, biome layout, aquifers, and ore veins alike. <cite index="98-1">Density functions compute a value for each block position and are used for terrain generation, biome layout, aquifers, ore veins, and more.</cite>

Both are the same idea: **worldgen as a composable data-defined graph rather than compiled procedure.** That is the same call as §10's species/world profiles, and it argues for making the worldgen pipeline itself data-driven from the start rather than hardcoding a generation sequence in Rust.

---

## 11. Suggested sequencing

Worldgen depends on channels existing, so most of this follows the architecture doc's ordering rather than preceding it.

1. **Species/world parameters as data** (§10). Do this **before** heavy tuning, not after — every experiment run before it costs a recompile, and the migration gets harder as constant count grows.
2. **Replace `build_terrain` with a noise heightmap** in the *fixed* world. Testable today, needs no streaming, and immediately exercises §6a (generated terrain at rest).
3. **Water table + moisture baseline** (§2), once the moisture channel lands. Highest-value single structure; roots and ants both become meaningful.
4. **Coarse map layer** (§0, §5) — **planar over (x, z)**, carrying elevation, drainage network, water table depth, anchor distance. Prerequisite for streaming and for caves. This is where all the planar terrain literature applies; nothing here touches `sim/`.
4b. **Water cycle** (§5a) — rain and evaporation. Without both, rivers are one-shot puddles. Cheaper than it sounds and it closes the second matter cycle.
5. **Caves** (§7) — **density-function based, not carved.** Cheese/spaghetti-style noise terms defining where stone is *placed*; explicit surface-connectivity term; explicit bedrock protection; and a span-aware mechanism for ceiling thickness, which is the one problem Minecraft's approach does not solve here.
6. **Age-parameterised generation** (§4), alongside or after the catch-up implementation, since they share a model.
7. **Streaming** (M10) last — it needs 4, 6, and the persistence taxonomy in §8 settled first.

---

## 12. Sources consulted

Primary sources this revision is grounded in. The first draft cited none of these.

- [Génevaux et al., *Terrain Generation Using Procedural Models Based on Hydrology*, SIGGRAPH 2013](https://hal.science/hal-01339224) — rivers first, construction tree representation
- [Mei, Decaudin & Hu, *Fast Hydraulic Erosion Simulation and Visualization on GPU*, PG 2007](http://www-evasion.imag.fr/Publications/2007/MDH07/FastErosion_PG07.pdf) — virtual pipes model, CFL condition
- [Musgrave's reference implementation of ridged and hybrid multifractals](https://engineering.purdue.edu/~ebertd/texture/1stEdition/musgrave/musgrave.c) — the multiplicative cascade, correcting this document's original definition
- [Minecraft noise settings and noise router (wiki)](https://minecraft.wiki/w/Noise_settings) and [world generation](https://minecraft.wiki/w/World_generation) — density functions, noise caves, aquifers, bedrock protection
- [Kleineberg, *Generating an infinite world with Wave Function Collapse*](https://marian42.de/article/infinite-wfc/) and [the earlier chunked attempt](https://marian42.de/article/wfc/) — why WFC does not survive streaming plus determinism
- [Boris the Brave, *Wave Function Collapse Explained*](https://www.boristhebrave.com/2020/04/13/wave-function-collapse-explained/) — WFC as constraint solving

**Not yet consulted, and worth a pass before committing to the coarse layer:** *Procedural Content Generation in Games* (Shaker, Togelius, Nelson — free at pcgbook.com); *Texturing & Modeling: A Procedural Approach* (Ebert, Perlin, Musgrave et al.); Musgrave's 1993 thesis *Methods for Realistic Landscape Imaging*; Red Blob Games on polygonal map generation; Inigo Quilez on noise and domain warping.

---

## 13. Open questions

- **How deep is bedrock?** Bounded is settled; the actual number is a world profile value, and it interacts with how much rock is worth generating at all.
- **Does the coarse map get eroded iteratively, and how expensive is that at world-creation time?** A one-off cost is acceptable; a per-session cost is not.
- **Do caves need guaranteed surface connectivity?** Worms anchored at the surface give it. Whether *every* cave system needs it, or whether sealed pockets are a feature, is a design call.
- **Is off-plane flux real or plausible?** (§5a) Real means the coarse map computes upstream/downstream and the slice honours it. Plausible means water appears at the boundary at a believable rate. The second works for a long time then fails opaquely.
- **How does coarse-level water routing work?** (§5a) A 10,000-cell river cannot be CA-simulated end to end. Throughput between chunk-columns, with CA water only where watched — this is catch-up applied to water and it is unworked.
- **Density functions or heightmap-plus-carve?** §7 recommends the former on Minecraft's evidence, but this world is 2D side-view, not 3D — the density-function argument was made for a 3D game and its cost profile here is unmeasured.
- **How is ceiling span bounded under a density-function approach?** The one problem Minecraft's method does not solve for this engine (§7). A span-aware post-pass, a worm hybrid, or a density term keyed on local thickness are all plausible and none is worked out.
- **Should erosion reuse `field.rs` outright?** §5b argues the virtual pipes model and the existing field solve are structurally the same. If true, sediment becomes a channel and erosion is nearly free. If the difference matters more than it looks, it is a separate system.
- **What is the geothermal compression factor?** Real is ~25 °C/km — meaningless at this scale. The number chosen determines whether depth is a real thermal axis creatures navigate or set dressing.
