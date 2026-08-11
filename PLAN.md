# Pixel Physics Engine — Build Plan

> **Status:** M1–M4 are built and working (80 tests, 60 fps, chunk sleeping,
> seven hot-reloadable materials, angle of repose from friction angle). M3 was
> only half-completed — flammability, melting and reactions were specified and
> never built. This plan now extends to **fire, explosions, plants, creatures
> and destructible structures**, and reorders the remaining milestones around
> the infrastructure they share. Original milestone numbers are kept as stable
> identifiers; see [Execution order](#execution-order) for the actual sequence.

## Context

The goal is a falling-sand style pixel physics engine — every cell in the world is a simulated pixel — intended as a foundation for building games, not as a one-off tech demo.

Target depth, confirmed with the user, is the **full Noita stack**: cellular-automata materials core, destructible rigid bodies, and character physics. The world should eventually be **infinite and streaming**, and the engine should eventually support sandbox toys, 2D platformers, top-down roguelikes, and puzzle games. The **first shippable milestone is a sandbox toy** — no win condition, just materials and reactions — which doubles as the engine's test harness.

Decisions locked during planning:

| Decision | Choice | Rationale |
|---|---|---|
| Language | **Rust** | User has no deep experience in Rust/C++/C#. Cargo eliminates a whole category of build/dependency pain on a long solo project, and the compiler catches the concurrency mistakes this architecture invites. |
| Simulation device | **CPU** (GPU renders only) | Shaders have no `rand()`, so identical cells clump into symmetry artifacts; falling sand is order-dependent, which parallel-per-pixel can't express. Noita's author made the same call. |
| Materials | **Data-driven from the start** | Hot-reloadable material files sidestep Rust's slow compile times exactly where iteration speed matters most (tuning). |
| Determinism | **Not required** | Free to use thread-local RNG and fastest update ordering. Worldgen still seeded (cheap, covers the practical need). |
| Browser | **Nice to have** | Native-first, but avoid platform-specific APIs so the WASM door stays open. |

### Research findings that shape this plan

- **Noita** ([GDC 2019](https://www.youtube.com/watch?v=prXuyMCgbTc)) is the only shipped game with this full stack. Its rigid-body pipeline — marching squares → Douglas-Peucker → triangulation → Box2D → re-rasterize — is the pattern to copy.
- **Dirty-rect chunk skipping is the single highest-leverage optimization.** Per [A Million Pixels of Falling Sand](https://hdyar.com/blog/posts/falling-sand/) (Unity+Burst, 2M cells @ ~30fps): *"the fastest running code is code that doesn't run."* This must land before threading.
- **Every implementation hits the same aliasing problem** (many threads mutating one grid while reading neighbors). [FallingSandEngine](https://github.com/PieKing1215/FallingSandEngine) (Rust, rapier2d) self-describes its threaded sim as *"uses unsafe, questionable soundness."* Mitigation: confine `unsafe` to **one** documented function, verify with Miri, keep everything else safe.
- **Scale is not the interesting part.** The million-pixel author hit his perf goal and concluded *"nothing I have made is surprising. There are no emergent behaviours."* Material interaction rules are where the value is — hence the emphasis on grounded material physics below.
- **The academic literature is selectively useful** (see Milestone 3).

### Research for the second phase (fire, explosions, plants, creatures, structures)

- **[The Powder Toy's `Air.cpp`](https://github.com/The-Powder-Toy/The-Powder-Toy/blob/master/src/simulation/Air.cpp) is the reference implementation for explosions.** It is the one falling-sand engine with a real air simulation: a **coarse grid at 1/CELL resolution** carrying pressure, velocity and temperature, updated as divergence→pressure, gradient→velocity, then semi-Lagrangian advection with wall-blocking and clamped pressure. This is a simplified stable-fluid solver, and it is directly reimplementable.
- **Fire is percolation.** The [Drossel–Schwabl forest fire model](https://link.springer.com/chapter/10.1007/978-3-662-04804-7_8) (1992) is a probabilistic CA — burning becomes empty, a tree beside a burning cell ignites — and is one of the best-studied cases of self-organised criticality, giving power-law fire sizes. The governing number is the **site percolation threshold on a square lattice, `p_c ≈ 0.59275`** ([exact results](https://iopscience.iop.org/article/10.1088/1751-8121/ac4195)): below that density of flammable material, fires die out; above it, they span the world. That single constant makes `flammability` predictable to tune instead of guesswork.
- **Heat diffusion has a hard stability limit.** Explicit finite-difference diffusion is unstable unless the Fourier number `α·Δt/Δx² < 0.5`, and **≤ 0.25 in 2D**. With `Δt = 1` frame and `Δx = 1` cell, that directly caps the per-step heat-conductivity coefficient. Exceed it and temperatures oscillate and blow up rather than merely looking wrong.
- **Trees should use space colonization, not L-systems.** [Runions, Lane & Prusinkiewicz, *Modeling Trees with a Space Colonization Algorithm*](https://algorithmicbotany.org/papers/colonization.egwnp2007.large.pdf) (2007): scatter attractor points in empty space, grow nodes toward the average direction of nearby attractors, remove attractors within a kill distance. Its whole point is that it **breaks the symmetry inherent in L-systems**, which is exactly what makes rule-generated trees look mechanical. Influence radius and kill distance map to visually meaningful tree characteristics.
- **Creature movement is a solved problem worth not reinventing.** [Reynolds, *Steering Behaviors For Autonomous Characters*](https://www.red3d.com/cwr/papers/1999/gdc99steer.pdf) (GDC 1999): seek, flee, pursue, evade, wander, obstacle avoidance, flocking — as simple forces that combine linearly. Cheap, tunable, and enough for everything short of planning.
- **Structural integrity has no canonical published algorithm.** Games that do it well (Red Faction: Guerrilla's Geo-Mod 2.0, Teardown) describe the *effects* rather than the method, and the reported cost is high — Volition's artists "had to become structural engineers." The pragmatic route is a support/anchor graph rather than finite elements, and it connects neatly to the **BTW sandpile toppling** already cited for M3: accumulated load exceeding a threshold and redistributing to neighbours is the same mechanism.

---

## Stack

All versions verified current as of Aug 2026.

| Concern | Crate | Notes |
|---|---|---|
| Window + pixel buffer | `pixels` 0.17 + `winit` | Purpose-built: "a tiny library providing a GPU-powered pixel frame buffer." wgpu underneath, so raw wgpu is the escape hatch for custom shaders in M6. |
| Parallelism | `rayon` | Drives the checkerboard chunk passes. |
| Rigid bodies | `rapier2d` 0.35 | Actively maintained (dimforge); the Box2D equivalent. |
| Material data | `serde` + `ron` | RON is more readable than JSON for nested config and supports enums natively. |
| Hot reload | `notify` | Filesystem watching for material files. |
| Scripting (M9) | `mlua` 0.12 | Lua 5.4 via the `lua54` feature. |
| Triangulation (M7) | `earcutr` 0.5 | Port of MapBox earcut. |
| Math | `glam` | Standard; rapier uses it. |
| Profiling | `puffin` | Add at M4 — you cannot optimize threading you can't see. |

---

## Non-negotiable architecture invariants

These are cheap now and extremely expensive to retrofit. Establish them in Milestone 1 and never violate them.

1. **World is `HashMap<ChunkCoord, Chunk>`, never a flat `Vec<Cell>`.** A flat array indexed `y * width + x` is the one decision that would force a full rewrite when the infinite world arrives.
2. **All coordinates are global signed `i32` world coordinates.** Never screen-space, never chunk-local outside a chunk's own internals.
3. **All access goes through `World::get(x, y)` / `World::set(x, y)`** from the first commit. This is the seam that later absorbs chunk load/unload/generate/serialize without touching call sites.
4. **Chunks are 64×64.** Small enough for fine-grained dirty rects, large enough to amortize dispatch.
5. **Simulation and rendering are separate.** Sim writes cells; a render pass translates cells → pixels. Never let material logic write colors directly.

With these in place, "start fixed-screen or start infinite?" dissolves — begin with a small fixed set of chunks using the infinite-world data model, and streaming becomes additive.

---

## Milestones

M1–M4 are complete. Their entries are kept for the invariants and reasoning
they record; new work starts at M12.

### M1 — Skeleton ✅ done
Window, fixed-timestep loop, `pixels` framebuffer, FPS overlay. Render noise to prove the pipeline.

**Verify:** window opens, stable 60fps, resizes cleanly.

---

### M2 — Cellular automata core ✅ done
> Built, with two corrections worth recording. The **frame-parity bit below was
> wrong** and caused sand to freeze in mid-air: parity only stays in step if
> every cell is visited every frame, which is precisely what dirty rectangles
> stop doing, so a cell skipped once aliases and is skipped forever. It was
> replaced by a self-clearing `FLAG_MOVED`. Separately, dirty rectangles must be
> widened by `MAX_REACH` horizontally, not one cell, or any rule that scans
> sideways acts on cells that never wake it.

- `Cell`: material id + flags, packed to 4 bytes. Use a **frame-parity bit** for "already updated" so you never pay to clear it.
- `Chunk`: `[Cell; 64*64]` + dirty rect + active flag.
- `World`: chunk map + `get`/`set` per invariants above.
- Single-threaded update: **bottom-to-top** (so falling material moves one cell/frame instead of teleporting to the floor), **alternating left/right scan direction per frame** (otherwise sand visibly drifts one way).
- Hardcode four materials to start: sand (powder), water (liquid), stone (static), smoke (gas).
- Mouse painting + material picker.

**Verify:** sand forms a pile with a believable slope; water finds its level; smoke rises; stone is inert.

---

### M3 — Grounded material model ⚠️ half done
> **Built:** data-driven `.ron` materials with hot reload, angle of repose from
> a friction angle, density-driven displacement and layering.
> **Not built:** flammability, melting and boiling points, and reactions — the
> whole thermal half of the schema below. **M14 finishes it.** BTW toppling and
> hole-propagation granular flow were also skipped; the friction-angle roll
> already delivers tunable, irregular slopes, so those are now optional polish.


Replace ad-hoc "try down, then down-left/down-right" with rules that have physical meaning. This is where the academic work pays off:

- **Angle of repose from a friction angle.** Klár et al., [*Drucker-Prager Elastoplasticity for Sand Animation*](https://math.ucdavis.edu/~jteran/papers/KGPSJT16.pdf) (SIGGRAPH 2016) models sand via a yield criterion relating shear to normal stress. Do **not** implement MPM — far too slow. Steal the *parameterization*: give each powder a `friction_angle` instead of a magic "spread factor." It maps directly to pile slope and tunes predictably across every material you add.
- **Pile relaxation via BTW toppling.** The Bak–Tang–Wiesenfeld sandpile model ([overview](https://www.hiskp.uni-bonn.de/uploads/media/sandpiles.pdf)): when a cell exceeds critical height, it topples to neighbors and cascades until stable. Gives avalanches with power-law size distribution *for free* — precisely the emergent surprise the million-pixel project lacked.
- **Granular flow as upward hole propagation.** Baxter & Behringer, *Physica D* (1991); [Kozicki & Tejchman, *Granular Matter* (2005)](https://link.springer.com/article/10.1007/s10035-004-0190-x). Modeling voids diffusing *up* rather than grains falling *down* is cheap and produces correct funnel/mass-flow behavior. Also see [friction in lattice CA granular models](https://www.researchgate.net/publication/274430628_The_Inclusion_of_Friction_in_Lattice-Based_Cellular_Automata_Modeling_of_Granular_Flows).
- **Density-driven displacement:** heavier materials sink through lighter ones. One rule, enormous behavioral payoff.

Move definitions into `assets/materials/*.ron`, loaded into a `MaterialRegistry` at startup, hot-reloaded via `notify`. Schema: `name`, `category` (Powder/Liquid/Gas/Solid/Fire), `density`, `friction_angle`, `dispersion`, `color_palette`, `flammability`, `melting_point`, `boiling_point`, `reactions[]`. Update functions dispatch on category and are parameterized entirely by data.

**Verify:** edit a `.ron` file with the app running → behavior changes without restart. Different `friction_angle` values produce visibly different pile slopes. Oil floats on water; sand sinks through both.

---

### M4 — Dirty rects and sleeping ✅ done
> Landed inside M2 — writing the sweep correctly was inseparable from it. The
> `F1` chunk overlay exists; `puffin` profiling does not, and should be added
> before M5.

Per-chunk dirty rect; skip chunks with no activity; **wake neighbor chunks when writing near a boundary** (the classic bug source — miss this and material freezes at chunk edges). Debug overlay drawing chunk bounds, dirty rects, and sleep state.

**Verify:** a settled world drops to near-zero CPU. Overlay confirms chunks sleeping. Material crossing a chunk boundary wakes the neighbor — watch a single sand grain fall across a boundary.

---

### M5 — Multithreading
`rayon` over a **4-pass checkerboard**: chunk `(cx, cy)` belongs to group `(cx % 2, cy % 2)`. Two chunks in the same group are ≥2 apart on some axis, so they're never adjacent *including diagonally* — which matters because a chunk update writes into its 8 neighbors (sand falling across a boundary).

**This is the one `unsafe` seam in the codebase.** A single function hands out non-overlapping mutable 3×3 chunk neighborhoods. Requirements: document the invariant precisely, wrap it in a safe API, test under **Miri**, and never let `unsafe` leak elsewhere.

**Verify:** identical visual behavior single- vs multi-threaded. Near-linear scaling in `puffin` up to core count. Miri clean. Stress test: fill the screen with sand, confirm no corruption or lost cells.

---

### M6 — Rendering upgrade
Dirty-region-only texture uploads. Custom wgpu pipeline for emissive lighting (fire/lava) and bloom. Drop below `pixels`' simple blit here if needed.

**Verify:** upload bandwidth scales with activity, not world size. Fire visibly lights nearby terrain.

---

### M7 — Free particles
Off-grid pixels with float position/velocity for explosions and splashes, converting back to grid cells on landing. Separate system from the CA grid — Noita does exactly this.

**Verify:** an explosion throws debris that re-integrates into terrain on impact.

---

### M8 — Rigid bodies *(largest single milestone — treat as its own project)*
Pipeline per Noita: **connected-component labeling** on pixel clumps → **marching squares** contour → **Douglas-Peucker** simplification → **`earcutr`** triangulation → rapier2d collider. Each frame: erase the body's pixels from the grid, step rapier, re-rasterize at the new transform.

**Known pitfall to design for up front:** rotated bodies no longer align to the grid and leave gaps that sand leaks through (raised explicitly in [FallingSandSurvival#4](https://github.com/PieKing1215/FallingSandSurvival/issues/4)). Rasterize by *inverse-mapping* each destination pixel into the body's local space rather than forward-mapping source pixels, and dilate slightly.

**Verify:** cut a chunk of terrain free → it detaches, falls, tumbles, and sand piles on top of it correctly. No leaking at any rotation.

---

### M9 — Character physics
Player as a kinematic body with sand-aware movement: walking on debris, being buried, swimming in liquid.

**Verify:** player can be buried by a sand dump and dig out; swims in water; stands on a tumbling rigid body.

---

### M10 — Infinite streaming world
Seeded noise-based chunk generation on a background thread; LRU unload with serialization to disk (RLE-compress — pixel worlds compress extremely well).

**Verify:** walk 10,000 cells in one direction and back; terrain is unchanged; memory stays flat; no hitching at chunk boundaries.

---

### M11 — Lua gameplay scripting
`mlua` for spells, entities, and scripted reactions on top of the data-driven material layer.

---

## Second phase — fire, explosions, life and structures

These five features look independent and are not: three of them need the same
two pieces of infrastructure, and building that first turns three hard features
into three moderate ones.

### M12 — Widen `Cell` to 8 bytes *(prerequisite for almost everything below)*

`Cell` is currently exactly 4 bytes and full — `material: u16`, `shade: u8`,
`flags: u8` with 7 spare bits. Fire needs a temperature, structures need an
anchor distance, plants need a growth stage, creatures need an owner id. Seven
bits will not stretch to any one of them.

```rust
material: u16   shade: u8   flags: u8   temperature: i16   aux: u16
```

**Temperature is universal** — everything has one, and per-cell granularity is
what lets one wooden beam ignite while its neighbour does not. The coarse
ambient field in M13 is *not* a substitute; The Powder Toy carries both for
exactly this reason. **`aux` is kind-specific**: burn timer, anchor distance,
growth stage, creature id. A tagged union is not elegant, but the alternative is
four parallel side tables, and this is what these engines do. Write down which
kind owns which interpretation and keep it honest.

Cost: a 2048² world goes 16 MB → 32 MB. Irrelevant.

**Verify:** `cell_is_eight_bytes` replaces the current size test; existing
behaviour is unchanged; no measurable frame-time regression.

---

### M13 — Coarse field grid *(the biggest new system, and it pays for four features)*

One grid at **1/8 the cell resolution** — a 64×64 chunk maps to an 8×8 field
tile — carrying **pressure, velocity (x, y), ambient temperature and light**.
Modelled on [The Powder Toy's `Air.cpp`](https://github.com/The-Powder-Toy/The-Powder-Toy/blob/master/src/simulation/Air.cpp):

1. Solid occupancy per field cell, derived from the CA grid — solids block air.
2. `pressure += divergence(velocity) * k_p`
3. `velocity += -gradient(pressure) * k_v`, zeroed into blocked cells
4. Semi-Lagrangian advection of all fields along the velocity
5. Diffusion, with the coefficient held under the **Fourier limit of 0.25 in 2D**
6. Clamp pressure; blend edges toward ambient

Cells then read their local field: gases and light powders get advected by wind,
fire heats the ambient temperature, plants read light.

**The architectural payoff:** the field grid is a whole-grid pass, not a per-cell
CA rule, so **it is not bound by `MAX_REACH`**. Long-range effects — a shockwave
crossing the screen, light falling from the sky — become possible without
violating the staleness invariant that governs the CA sweep. Cheap, too: at 1/8
resolution the sandbox's world is 64×40 per field.

**Verify:** an impulse produces a shock that reflects off walls; smoke drifts on
wind rather than only rising; a sealed chamber holds pressure; fields stay stable
over 10,000 frames (no blow-up from exceeding the diffusion limit).

---

### M14 — Fire, heat and reactions *(finishes M3)*

Per-cell temperature diffuses to neighbours by explicit finite difference, with
the conductivity coefficient **clamped to keep the Fourier number ≤ 0.25** —
above that the scheme is unstable, not merely inaccurate. Cells also exchange
heat with the M13 ambient field.

New material properties, extending `assets/materials/*.ron`:

```ron
flammability: 0.4,            // chance to ignite per burning neighbour per step
ignition_temperature: 300.0,
burn_temperature: 900.0,      // what it emits while burning
burn_duration: 180,           // frames, stored in `aux`
heat_conductivity: 0.15,      // clamped by the stability limit above
melting_point: 1200.0, melts_into: "lava",
boiling_point: 100.0, boils_into: "steam",
reactions: [(with: "lava", produces: ("stone", "steam"), chance: 0.8)],
```

Fire is a burning cell with a timer in `aux` that ignites flammable neighbours
probabilistically, raises local temperature, and leaves ash or smoke — the
[Drossel–Schwabl](https://link.springer.com/chapter/10.1007/978-3-662-04804-7_8)
model with material parameters attached.

**Tune against percolation.** Fire spans a region only when flammable material
exceeds the site percolation threshold `p_c ≈ 0.59275`. That is why a sparse
scattering of wood will not carry a fire and a dense one will, and it turns
`flammability` from a magic number into something predictable.

**Verify:** oil ignites and burns out to ash; water quenches lava into stone and
steam; a wooden structure burns from one corner and the fire spreads or dies
depending on how densely it is built; temperature is stable over long runs.

---

### M15 — Explosions *(needs M7 free particles)*

An explosion writes three things: a **pressure impulse** into the M13 field, a
**temperature spike**, and a radius of cells converted to free particles or
vacuum. The shock then propagates and reflects through the field for free —
that is the whole reason for building it.

Debris takes its initial velocity from the local pressure gradient, so blasts
throw material *away* from the centre and around corners rather than in a naive
radial burst. The physical reference for the waveform is the Friedlander /
Rankine–Hugoniot description of a shock front, but an impulse plus field
propagation is what The Powder Toy does and is sufficient.

**Verify:** an explosion in a corridor vents along it rather than through the
walls; debris re-integrates into terrain on landing; a blast behind cover does
less damage than one in the open.

---

### M16 — Active sites, then plants

**The scheduler first, and it is the highest-leverage piece in this phase.**
Dirty rectangles mean *"something moved"* and deliberately skip settled cells —
that is the entire performance strategy. But a plant that is not moving still
needs to grow, and a structure that is not moving still needs its supports
checked. Waking chunks to do it would destroy sleeping across any world with
vegetation or buildings in it.

The resolution is that **plants only change at their tips** — a trunk is inert.
So keep a per-chunk list of *active sites* (position, kind, next-update frame),
separate from the dirty rect. Cost becomes proportional to the number of
interesting cells, not to the size of the world. Structural integrity and
creatures both reuse it unchanged.

Then plants:
- **Moss, grass, vines** — local rules; a growth direction fits in spare flag bits.
- **Trees** via [space colonization](https://algorithmicbotany.org/papers/colonization.egwnp2007.large.pdf)
  adapted to 2D: attractor points scattered in empty space above, nodes growing
  toward the average direction of attractors within an influence radius, and
  attractors removed within a kill distance. Chosen over L-systems specifically
  because it breaks the symmetry that makes rule-generated trees look mechanical.
- **Roots** consume adjacent water cells and credit an energy counter — the first
  mechanic tying plants to the existing material physics, and nearly free.
- **Light-seeking** reads the M13 light field.
- Plants are flammable, so M14 turns a grown forest into an actual
  Drossel–Schwabl forest fire.

**Verify:** a settled world with a forest in it still sleeps between growth
ticks; moss spreads over damp stone and not over dry; two trees grown from the
same seed differ; a forest burns and regrows.

---

### M17 — Structural integrity *(destructible building with no solver)*

Each solid cell stores in `aux` its **distance to an anchor** — bedrock, the
world edge, or a foundation material. Distances relax from neighbours
(`d = 1 + min(neighbours)`), and a cell whose distance exceeds its material's
tolerance is unsupported and breaks free, falling as loose material.

One new material property carries the entire structural feel of the game, and it
hot-reloads:

```ron
max_unsupported_span: 8,   // stone 3, wood 8, steel 20
```

This buys collapsing buildings, span limits, and materials that differ
structurally **without polygons, connected-component labelling or a physics
solver** — roughly a week against several for M8. Debris falls as loose material
rather than tumbling as coherent chunks; M8 upgrades that later without
replacing this. Recomputation is incremental and driven by the M16 active-site
list, bounded by the size of the affected structure.

**Verify:** cut the supports from under a bridge and it collapses progressively
rather than all at once; a stone arch spans less than a steel one; editing
`max_unsupported_span` while running changes what stands up.

---

### M18 — Creatures

**Phase 1 — cell-based (now).** A creature is a cell with a `Creature` kind and
a named behaviour, scheduled on the M16 active-site list: worms that burrow
through powder, slimes, spreading fungus. Fits the data-driven material model
almost exactly, is destructible for free, and adds no new systems. The
interesting part is material interaction — a creature that dies out of water,
burrows only through loose powder, or eats sand and excretes it gets enormous
mileage out of what already exists.

**Phase 2 — entities (after M8).** Proper entities with
[Reynolds steering](https://www.red3d.com/cwr/papers/1999/gdc99steer.pdf) —
seek, flee, wander, obstacle avoidance combined as linear forces — whose bodies
are rasterized into the grid so they stay destructible and physical. That
erase-transform-rasterize loop **is** the M8 rigid-body pipeline, which is why
this waits for it: nearly free afterwards, expensive before.

Two things to get right early:

- **Entities must not write cells during the CA sweep.** The bottom-up ordering
  and the moved flag both assume nothing else is mutating the grid. Fix the frame
  order now: **`entities → CA sweep → rigid bodies → render`**.
- **Perception is cheap and unconstrained.** `MAX_REACH` binds CA rules because
  of how waking works; entities are outside the sweep and may read anywhere.

**Verify:** a worm burrows through sand and cannot enter stone; a creature flees
a fire it senses through the temperature field; killing one leaves a destructible
corpse.

---

## Execution order

Milestone numbers are stable identifiers, not an order. This is the order.

| # | Milestone | Size | Why here |
|---|---|---|---|
| 1 | **M12** Widen `Cell` | Small | Unblocks fire, structures, plants and creatures at once |
| 2 | **M13** Coarse field grid | Large | Pays for fire, explosions, plant light and M6 lighting together |
| 3 | **M14** Fire, heat, reactions | Moderate | Finishes M3; highest payoff per unit of work in the whole plan |
| 4 | **M7** Free particles | Moderate | Explosion debris needs it |
| 5 | **M15** Explosions | Moderate | Shock propagation falls out of M13 nearly free |
| 6 | **M6** Rendering upgrade | Moderate | Fire and explosions look wrong without emissive light and bloom |
| 7 | **M5** Multithreading | Large | The budget is already spent — see below |
| 8 | **M16** Active sites + plants | Moderate | Scheduler first; it unblocks 9 and 10 |
| 9 | **M17** Structural integrity | Moderate | Destructible building without a solver |
| 10 | **M18** Creatures, cell-based | Moderate | Reuses the M16 scheduler directly |
| 11 | **M8** Rigid bodies | Large | The risk concentration; deferred as far as it sensibly can be |
| 12 | **M18** Creatures, entities | Moderate | Nearly free once M8 exists |
| 13 | **M9** Character physics | Moderate | Shares M8's collision work |
| 14 | **M10** Streaming world | Large | Additive; the `get`/`set` seam already exists |
| 15 | **M11** Lua scripting | Moderate | Last, on top of a stable content model |

### Why this order

- **Fire lands third.** It is the cheapest large win available and completes work
  M3 already promised.
- **The CA rules must stop churning before M5.** Threading a moving target is
  painful, and M14 rewrites material behaviour substantially. Everything after
  M5 adds *separate systems* in their own frame phases rather than changing the
  sweep, so it threads independently.
- **M5 can no longer be deferred indefinitely.** A full screen of moving
  material already costs ~16 ms of a 16.6 ms budget, single-threaded. Fire,
  explosions and the field grid all add to that. Add `puffin` before starting.
- **M16's scheduler comes before its plants.** Active sites are what make plants,
  integrity and creatures cost proportional to interesting cells rather than
  world size. Build it deliberately, not as a plant feature.
- **Resist starting at M8.** It is the most exciting item and the one most likely
  to consume months without a playable result. Steps 8–10 between them deliver
  growing, burnable, buildable, collapsible worlds on machinery that already
  works.

### Standing invariants for all new work

1. **No CA rule may read further than `MAX_REACH`.** Sweep regions are widened
   by exactly that much; a rule reading further acts on cells that never wake it.
   This caused both the frozen-sand and unlevel-water bugs. Field-grid passes are
   exempt — they are whole-grid, not per-cell.
2. **Decisions that gate movement must be stable, not re-rolled per frame.**
   Drawing fresh randomness each call lets a chunk sleep on a frame the dice said
   no, freezing material permanently. Key such draws on position.
3. **Nothing outside the sweep may write cells during it.** Entities and rigid
   bodies get their own frame phases.

## Overall verification

`cargo run` launches the sandbox. Paint sand/water/oil/fire; confirm piles,
levelling, buoyancy by density, and combustion. Edit a material `.ron` while
running and see it take effect. Toggle the debug overlay to confirm chunks sleep
when settled.

`cargo test` covers the material registry, chunk coordinate maths, `World::get`
/`set` boundary cases, and — most importantly — the two invariant tests that
catch staleness: `settled_sand_is_never_left_unsupported` and
`every_unstable_cell_is_scheduled_for_examination`. **Extend the second to each
new system**; it is what caught the dirty-rectangle width bug, and the weaker
version of it passed happily while the bug was live.

`cargo run --release --example ascii` gives headless scenes with worst-frame
timings — the fastest way to judge whether a rule looks right and whether it
still fits the budget. `cargo +nightly miri test` validates the chunk-splitting
`unsafe` from M5 onward.

---

## Progress log

Kept here so the plan and the actual build stay honest against each other —
updated at each milestone commit, not just when something is added.

- **M1–M4**: done, see status notes inline above.
- **M12** (widen `Cell` to 8 bytes): done.
- **M13** (coarse field grid): done. Independent code review caught 3 real
  bugs before they shipped — see `README.md`'s M12/M13 status section.
- **M14** (fire, heat, reactions): done, finishes M3.
- **M7** (free particles): done.
- **M15** (explosions): done.
- **M6** (rendering upgrade — bloom/emissive lighting): **deferred**. Needs
  live visual judgment a screenshot-and-reason-about-it loop can't substitute
  for; parked for a session where that's available, not abandoned.
- **M5** (multithreading): in progress, out of the plan's stated order —
  moved up ahead of M16/17/18 by explicit user decision once they were
  available to weigh in on the design. See `README.md`'s M5 status section
  for the safety design (no `unsafe`, contrary to what this plan originally
  sketched) once it lands.
- **M16/M17/M18/M8**: not started yet.
