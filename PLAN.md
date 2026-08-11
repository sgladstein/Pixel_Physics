# Pixel Physics Engine — Build Plan

> **Status:** M1–M4 are built and working (80 tests, 60 fps, chunk sleeping,
> seven hot-reloadable materials, angle of repose from friction angle). M3 was
> only half-completed — flammability, melting and reactions were specified and
> never built. This plan now extends to **fire, explosions, plants, creatures
> and destructible structures**, and reorders the remaining milestones around
> the infrastructure they share. Original milestone numbers are kept as stable
> identifiers; see [Execution order](#execution-order) for the actual sequence.

> **Research reports:** the `research/` directory alongside this file holds
> full, uncondensed research passes this plan's milestone sections only
> summarize — [`research/m16-plant-biology.md`](research/m16-plant-biology.md),
> [`research/m18-creature-biology.md`](research/m18-creature-biology.md),
> [`research/m19-visual-polish.md`](research/m19-visual-polish.md). Written
> out in full deliberately, separate from this document, so the source
> material (citations, mechanisms, concrete algorithms) survives even if a
> future session's context window doesn't retain the conversation that
> produced it. Read the relevant report before implementing the milestone it
> backs, not just this file's condensed version.

> **Direction reports:** the `Reports/` directory holds four documents an
> external review pass produced against this codebase, all with **direction
> agreed with the repo owner** —
> [`Reports/emergent-world-architecture.md`](Reports/emergent-world-architecture.md)
> (the "thin agents, rich world" architecture decision — see
> [Third phase](#third-phase--emergent-world-architecture) below),
> [`Reports/worldgen-design.md`](Reports/worldgen-design.md) (the M10 redesign:
> 2D side-view play through a 3D coarse worldgen layer, vertical zones, caves,
> species-as-data),
> [`Reports/stigmergy-research.md`](Reports/stigmergy-research.md) (ant/termite
> stigmergy as the general deposit→diffuse→decay→follow primitive, for
> whatever the first colony-forming creature work turns out to be), and
> [`Reports/pixel-physics-issues.md`](Reports/pixel-physics-issues.md) (eleven
> concrete performance/correctness/housekeeping issues against the codebase as
> it stood then). These are **not milestone research** the way `research/` is
> — they're a direction-setting pass that reshapes near-term priority order.
> Read the relevant report before touching anything it covers; this file's
> condensed version is not a substitute for the reasoning behind it.

## Context

The goal is a falling-sand style pixel physics engine — every cell in the world is a simulated pixel — intended as a foundation for building games, not as a one-off tech demo.

Target depth, confirmed with the user, is the **full Noita stack**: cellular-automata materials core, destructible rigid bodies, and character physics. The world should eventually be **infinite and streaming**, and the engine should eventually support sandbox toys, 2D platformers, top-down roguelikes, and puzzle games. The **first shippable milestone is a sandbox toy** — no win condition, just materials and reactions — which doubles as the engine's test harness.

Decisions locked during planning:

| Decision | Choice | Rationale |
|---|---|---|
| Language | **Rust** | User has no deep experience in Rust/C++/C#. Cargo eliminates a whole category of build/dependency pain on a long solo project, and the compiler catches the concurrency mistakes this architecture invites. |
| Simulation device | **CPU** (GPU renders only) | Shaders have no `rand()`, so identical cells clump into symmetry artifacts; falling sand is order-dependent, which parallel-per-pixel can't express. Noita's author made the same call. |
| Materials | **Data-driven from the start** | Hot-reloadable material files sidestep Rust's slow compile times exactly where iteration speed matters most (tuning). |
| Determinism | **REVERSED — same-build deterministic replay is now required** | Was "not required." Reversed with the owner (`Reports/emergent-world-architecture.md` §8) because off-camera state persisting, slow processes running in unloaded chunks, and time being fast-forwardable **all require catch-up**, and catch-up is only sound if outcome = f(state at unload, elapsed time, seed) — a pure function needs determinism to exist as a shortcut at all. Scoped narrowly: same-build only (not cross-platform bit-identical), and worldgen is reproducible *within one world's lifetime*, not from a shared seed across worlds/versions (§8h). The engine turned out to already be ~90% there (§8a) — the one real violation is `scheduler::step`'s `HashMap` drain order (§8b, tracked as part of issue #7's rewrite). |
| Browser | **Nice to have** | Native-first, but avoid platform-specific APIs so the WASM door stays open. |
| Puzzle-game target | **Dropped** | `Reports/emergent-world-architecture.md` intro. Emergence needs no solvability guarantee; the sandbox toy is still the first shippable milestone, where unpredictability is the product. Revisit only if a puzzle-game target is pursued later. |

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

**Scientific accuracy directive**, added mid-session by explicit user
request: when actually building this milestone, research real plant biology
and try to make the mechanisms as scientifically grounded as the engine's
material physics already is, not just visually plausible. Space colonization
above is already a real botanical growth model, not an art trick — that
standard should extend to the rest of M16, not stop at tree shape.

**Full research reports: [`research/m16-plant-biology.md`](research/m16-plant-biology.md)**
— the condensed version below; the full report has the deep-dive follow-up
pass (gravitropism/hydrotropism antagonism, root branching via auxin
priming oscillators, and auxin-canalization apical dominance — the single
biggest lever for realistic tree shape) that this summary compresses.

**Research findings, to build the rest of M16 against:**

- **Root water uptake is a gradient-following process, not active pumping** —
  real uptake follows a water-potential gradient (soil wetter than root
  interior; roots actively pump mineral ions to *steepen* that gradient, water
  itself moves passively), and root hairs only ever access water within
  microns of them. Simulatable version: give a root cell a scalar "local
  water deficit" that adjacent water satisfies directly and that otherwise
  propagates one step per tick toward the plant's base (a cheap stand-in for
  the real cohesion-tension column, not a full transport-network solve) —
  closer to the real mechanism than "touch water, gain energy," and cheap
  enough for a scheduler that only ticks a handful of active cells.
  Roots should also grow preferentially toward cells with more neighbouring
  water (real hydropatterning), not spread uniformly.
- **Trees competing for light has a direct, citable sequel to the already-committed
  space colonization paper**: Palubicki, Horel, Longay, Runions, Lane, Měch
  &amp; Prusinkiewicz, ["Self-organizing tree models for image
  synthesis"](https://algorithmicbotany.org/papers/selforg.sig2009.html)
  (SIGGRAPH/ACM TOG 2009) adds light competition via **shadow propagation
  into a coarse voxel grid** — each branch casts shadow into the grid, local
  light value drives growth-direction weighting, shaded branches get
  starved. This maps almost exactly onto the engine's *existing* M13 light
  field rather than requiring a separate light model — a tree casting shadow
  into that same grid, and growing toward locally brighter cells, is the
  natural fit. (A 2025 Eurographics survey of light-model variants for this
  algorithm family exists too — [Nauber, CGF
  2025](https://onlinelibrary.wiley.com/doi/10.1111/cgf.15268) — abstract
  accessible, full text paywalled.)
- **Moss/lichen substrate rules should be moisture-and-shade-driven, not a
  fixed "north side" rule.** Moss has no waterproof cuticle and can't
  regulate internal water (poikilohydric) — it favours shaded surfaces
  because shade slows evaporation and preserves dampness, not from any
  directional pull. The correct simulatable rule is: spread probability as a
  function of (local light-field value — lower is better, adjacent water or
  humidity, temperature — lower favours less evaporation), which is both
  more accurate and barely more expensive than the flat "damp stone" check
  already planned.
- **Real relative growth rates**, for tuning constants rather than guessing:
  lichen ~0.5–8 mm/yr (some species far slower), moss ~0.5–4 cm/yr, trees
  tens of cm/yr+ — roughly lichen ≪ moss ≪ tree by 1–2 orders of magnitude
  at each step. A ratio around 1:10:100 is a reasonable anchor.

**Deep-dive findings (root architecture and plant signaling), from the
follow-up research pass — this is the part that actually answers "root
growth" and "plant signaling":**

- **Auxin canalization is the single biggest lever for realistic tree
  shape**, and it's a directly implementable, already-formalized algorithm,
  not an invention: Prusinkiewicz, Mündermann, Karwowski & Lane, ["Control
  of bud activation by an auxin transport
  switch"](https://www.pnas.org/doi/10.1073/pnas.0906696106) (PNAS 2009).
  Each bud competes to establish a self-reinforcing auxin transport channel
  toward the trunk (positive feedback, saturating, hysteretic — hard to
  reverse once established); whichever channel wins suppresses the others.
  This is the real mechanism behind a tree having one dominant leader with
  suppressed side branches instead of an evenly bushy form — and it's the
  real reason cutting a plant's top off makes it bush out (removing the
  apical auxin source releases nearby buds from suppression). Simulatable
  as two small scalars per node (auxin channel strength, cytokinin level
  diffusing up from roots) updated with simple positive feedback each tick
  — no PDE needed.
- **Root growth direction is gravity vs. water, not water alone, and the
  two genuinely fight rather than blend.** Root tips sense gravity via
  amyloplasts sedimenting in columella cells, redirecting PIN auxin
  carriers to bias growth downward; when water availability conflicts,
  **MIZ1** actively suppresses the gravity response so the moisture
  gradient wins instead — a real antagonism switch, not a weighted average.
  Simulatable as a `gravity_bias` vector, a `water_bias` vector, and a
  `miz_active` flag that zeroes gravity_bias when the local moisture
  gradient crosses a threshold. Lateral roots grow at a genetically fixed
  angle offset from their parent (a real "gravitropic setpoint angle") —
  cheap to encode as a per-node constant rather than continuous flux math.
- **Root branching should be periodic, not a flat per-tick probability.**
  Real lateral roots are primed by an internal oscillator in the root tip
  that marks roughly evenly-spaced sites as it grows, and only later do
  local resource conditions decide whether a primed site actually branches.
  Simulatable as a growth-tick counter per root tip: mark a "primed" site
  every N ticks, branch only if local resource signal clears a threshold —
  gives naturally regular spacing instead of noisy random branching.

Full citation list (roots, phototropism mechanism, moss/lichen ecology,
growth-rate sources, gravitropism, root branching, apical dominance,
cytokinin, and an optional systemic stress-signaling mechanic not needed for
the core build) is in
[`research/m16-plant-biology.md`](research/m16-plant-biology.md); the
points above (both blocks) are the load-bearing findings to actually build
from.

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

**Scientific accuracy directive**, same standard as M16's, added mid-session
by explicit user request: research real animal behaviour and physiology
rather than inventing plausible-looking rules from scratch. Reynolds
steering above is a real, citable model for *movement*, but it says nothing
about *why* a creature moves where it does.

**Full research report: [`research/m18-creature-biology.md`](research/m18-creature-biology.md)**
— full citations and detail behind the condensed points below.

**Research findings, to build the rest of M18 against:**

- **Burrowing should be gated by substrate mechanics, not a material
  whitelist.** Real peristaltic burrowing (earthworms) only works within a
  narrow band of substrate resistance — displaceable/compactable ahead of
  the animal, and able to flow back in behind it; too loose and there's
  nothing to anchor against, too resistant (compacted/solid) and the animal
  can't generate enough pressure to deform it (Kurth et al., *J. R. Soc.
  Interface* 2018). "Can enter loose sand, can't enter stone" is *already*
  roughly this, since sand is genuinely displaceable and stone isn't — the
  refinement worth making is tying burrow cost/speed to the target
  material's own physical properties (density, friction angle — both
  already tracked per material) rather than a hardcoded material-kind check,
  so the rule generalizes to future granular materials for free. Real energy
  numbers are stark enough to justify a cost model: burrowing through loose
  sand costs on the order of **26x more energy per metre than moving across
  open ground** (Namib golden mole measurements) — worth reflecting as a
  real movement-cost multiplier, not a flat "slower" tweak.
- **Heat/fire sensing: the simplest well-studied mechanism is a direct fit.**
  *C. elegans* thermotaxis — a single thermosensory neuron compares current
  temperature against a remembered set-point and drives movement down the
  gradient once above it — maps almost exactly onto "read the local
  ambient-temperature field, flee down-gradient once above a threshold,"
  which is both the scientifically grounded version *and* the cheap one; no
  need to invent something more complex than what a real 302-neuron animal
  actually uses for this exact behaviour.
- **Give the worm an actual reason to move: foraging, not wandering.** The
  Marginal Value Theorem (optimal foraging theory) predicts patch-leaving
  behaviour from a simple rule: leave a patch once its local intake rate
  drops below the environment's average. Simulatable as an internal
  energy/hunger stat that depletes over time, is satisfied by consuming
  material as the worm burrows, and triggers directed movement toward higher
  local resource density with a leave-threshold — replaces "wander
  randomly" with a real behavioural-ecology model at negligible extra cost.
- **If multiple creature kinds interact, the Wa-Tor model is close to exact
  prior art.** Dewdney's Wa-Tor (*Scientific American*, 1984) is a toroidal
  grid where prey move/breed on timers and predators move toward prey, eat,
  gain energy, and starve without food — a discretized Lotka-Volterra system
  built for grid cells on timers, i.e. this engine's own active-site
  scheduler shape almost exactly. Later CA variants (Cattaneo et al.) show
  these local grid rules reproduce real predator-prey population oscillations.
- **For the slime/fungus creature specifically**, *Physarum polycephalum*
  foraging-algorithm models (Jeff Jones, 2010) and fungal-mycelium CA growth
  models (nutrient uptake and translocation on a lattice) are direct
  grounding — both are literally grid/network growth-and-pruning driven by
  local nutrient gradients, the same shape as the plant root mechanic above,
  which suggests a shared "consume local resource, propagate deficit,
  grow/prune toward gradient" primitive could serve roots, fungus, and slime
  creatures with one mechanism doing triple duty rather than three bespoke
  ones.

Full citation list (fire-sensing ranges by species, Braitenberg vehicles as a
design philosophy for keeping each creature's rule-set small, Lenia/SmoothLife
as adjacent-but-not-applicable continuous-field alife) is in this session's
research notes; the five points above are the load-bearing findings to
actually build from.

---

## Third phase — emergent world architecture

An external review pass produced four documents against the codebase at
`b2ebea8` (right after M8's connected-component-labeling start), with
**direction agreed with the repo owner**. Full text in `Reports/`; this
section is the condensed, actionable version — read the originals before
implementing anything below, the same relationship this file has with
`research/`.

### The core decision: thin agents, rich world

> If two systems need to interact, they do it by reading and writing a
> shared channel, not by calling each other.

Two places state can live: **in the world** (cells, field channels — cost
O(world), roughly independent of population, ceiling is high) or **in the
organism** (`TreeState`, `CreatureState` — cost O(population × complexity),
scales with N). The target population (a reseeding forest, a Wa-Tor worm
ecology) is hundreds to thousands of agents, which is the regime where
thin-organism/rich-world wins outright — and rich-world state can always be
thickened later with nothing thrown away, where thick-organism state tends
to never get generalized into a channel because each organism has already
solved its own private version.

**The general primitive underneath nearly every "complex behavior from
simple rules" system in the literature**: deposit → diffuse → decay →
follow. Ant trails, Physarum, erosion, desire paths, predator scent, and
`plant.rs`'s own auxin `channel` (built without the word for it) are all
the same loop. **Behavior count scales with closed loops between channels,
not with systems written** — every channel with both a reader and a writer
is a feedback loop nobody explicitly coded. The engine currently has
exactly one closed loop (fire → heat → ignition → more fire); everything
else is one-directional (plants *read* light, nothing writes it; roots
*read* water, nothing depletes it). **The generative move is closing
loops, not adding systems.**

**The world currently runs down.** Every process is monotonic — sand
settles, wood burns to ash, ash is terminal, trees exhaust their
attractors and stop. A world that feels alive needs matter to cycle
(plant → dies → decays → nutrient → plant; water evaporates → rains →
flows → evaporates), not merely run to a terminal state.

**Standing review questions for any new system** (`Reports/
emergent-world-architecture.md` §12): does it read/write shared world
state or hold private state and query the world? Does the channel it uses
have a writer, not just a reader? Would any test fail if the mechanism
were deleted entirely? Could it produce different results on two runs of
the same build? Do two agents ever talk to each other directly (they must
not — only through the world)?

**Verification protocol, formalized from what M16 already did twice by
accident**: for any change to a channel, growth rule, or creature
behaviour — write an `examples/ascii.rs` scene first, look at it (this is
the step that finds real bugs; both real M16 bugs were caught this way,
not by assertions), then add the weakest assertion that would fail if the
mechanism were deleted entirely. Scenes judge quality; assertions catch
total failure. Neither substitutes for the other. Both stay in the repo
permanently.

### Settled with the owner, this phase

- **Moisture is a field channel**, not a per-cell `Cell` property — diffuses
  for free on existing machinery, and gradient (not raw value) is what
  organisms actually navigate by.
- **Tuning constants are in scope for whoever implements a channel** — tune,
  don't stop and ask. If a new channel blows the frame budget, tune first,
  revert second, flag either way; don't press on silently.
- **Determinism reversed to required** — see the decisions table above and
  `Reports/emergent-world-architecture.md` §8 in full for the audit, the one
  real violation, and what it enables (catch-up, testable emergent outcomes,
  a reachable multiplayer architecture).
- The light field was never wired up (`add_light` has exactly one caller, a
  test) — **this is unbuilt work, not a regression.**

### Priority order (`Reports/emergent-world-architecture.md` §11, folded with the issues backlog below)

1. **Field sleeping** + `rebuild_blocked` fix + loop-invariant hoisting
   (issues #4, #5, #6) — enabling work; buys the frame budget every channel
   below needs. **Fold in the scheduler `BinaryHeap` rewrite (#7) here** —
   it is simultaneously the performance fix and the fix for the one real
   determinism violation (§8b); add the deterministic tiebreak while
   rewriting it, not after.
2. **Light writer** (fire emits light + a sky ambient source) — resurrects
   two already-implemented, currently-inert mechanisms (moss shade,
   tree phototropism), retires a documented M16 simplification, closes two
   emergent loops for free. Small; can go first regardless of field
   sleeping since it adds no new pass.
3. **Bilinear field sampler** — `sample_bilinear` already exists
   (`field.rs`) and is private; expose it and route every gradient-follower
   through it. Fixes the worm's thermotaxis and the tree's phototropism,
   both of which currently read a block-nearest field that's flat almost
   everywhere a single cell moves (see "the resolution problem" below).
4. **Moisture channel** — four waiting consumers (roots, moss, worm
   burrowing, fire resistance), deletes two hand-rolled O(r²) scans
   (`is_damp`, `strongest_water_pull`), and is the first real inter-organism
   resource competition once roots deplete it locally.
5. **Plants write the channels they read** — two one-line writes (light
   occlusion is already free once `Plant` blocks the field per M16; a root
   depletes moisture where `ROOT_WATER_ENERGY` is granted). Turns two
   read-only channels into loops — the precondition for any patterning
   (tiger-bush-style lateral inhibition falls out free of light + moisture
   once both are loops).
6. **Day/night oscillator** — one writer, global rhythm across every system
   that already reads light or temperature at once. Highest
   aliveness-per-line once the sky exists.
7. **Ash → soil decay cycle, with reseeding** — the first closed matter
   cycle. Completes M16's own verify criterion ("a forest burns and
   regrows" — only the burns half exists today) and gives succession (a
   burned patch regrows *differently*). Reseeding is also the population
   gate on Wa-Tor-style predator/prey dynamics.
8. **Structural collapse writes a pressure impulse** — `break_free` swaps
   the cell and returns silently today; one `add_pressure_impulse` call
   gives dust/shock/wind for a collapsing structure the same way
   `explosion::trigger` already does for a blast.
9. Lower priority, in whatever order suits: plants read the velocity field
   (wind bends canopy/grass), structural integrity extended to `Plant`
   (blocked today on the same `aux` slot conflict M16's growth stage
   already occupies — resolve that first), tree attractors replaced by
   "grow toward the light gradient" once light exists (inter-tree canopy
   competition becomes automatic, no new code).

**Before creature work beyond the worm**: read `Reports/
stigmergy-research.md` in full. Condensed: stigmergy is two mechanisms this
engine already has substrate for — marker-based (pheromone → a field
channel) and sematectonic (structure-as-stimulus → cells); movement must be
*probabilistic* gradient-following with noise, never `min_by`/`max_by`
(deterministic selection kills the exploration the mechanism depends on);
evaporation drives path *selection* but a separate crowding/negative-feedback
term is what prevents a trail from ossifying on the first path found
(evaporation alone will not fix that); **the moisture channel already
planned above is very plausibly the termite construction channel too** —
deposition probability ∝ local moisture-gradient magnitude, no separate
cement-pheromone channel needed (Facchini et al., eLife 2024); nest shape
(chamber → branching tunnels) falls out of worker density alone, no new
channel; a colony has a minimum viable population (~50 in the source
literature, order-of-magnitude only in a side-view strip, not a number to
trust) — a 3-agent test scene will look broken when the code is correct;
sensor offset needs ≥3 cells of real gradient, making the bilinear-sampling
work above a hard prerequisite, not a nicety, for anything trail-following.

### The resolution problem (elevate this — it recurs for every gradient-follower)

`field.rs`'s `FIELD_SCALE = 8`, and `World::field_at` is block-nearest — any
two positions within the same 8×8 block return byte-identical values. A
worm's four neighbours at ±1 land in the same field cell ~7 times in 8, so
`min_by` degenerates to "always pick the first candidate," which reads as
"the worm always flees left" rather than real thermotaxis. Bilinear
sampling (priority 3 above) fixes this for any single-cell gradient read.
It does **not** fix trail *width* — a one-cell-wide pheromone trail smeared
across an 8-cell field block stays smeared no matter how it's sampled.
Pressure/temperature genuinely want to stay coarse (bulk properties, and
the coarseness is a real performance feature); pheromone genuinely wants a
finer grid, possibly `FIELD_SCALE = 1` for that one channel. **Do not add
pheromone to `FieldCell` assuming 8 will do** — that is the decision that
would be expensive to undo. `FieldTile` currently hardcodes one resolution
for every channel; splitting resolution per channel is a real, not-yet-designed
change to that struct and every pass in `field.rs`. Test cheaply first
(seed a synthetic trail at `FIELD_SCALE = 8`, run a gradient-follower with
a sensor offset around 8, see whether it actually tracks) before
redesigning anything.

### M10 redesign: worldgen, in full in `Reports/worldgen-design.md`

`PLAN.md`'s old M10 line — "seeded noise-based chunk generation... LRU
unload... RLE-compress" — hid essentially every real problem. Settled with
the owner, replacing that line:

- **The play world stays 2D side-view. Worldgen is 3D.** A coarse layer is
  planar over `(x, z)` — elevation, drainage, climate, one value per
  chunk-column. The play world is a *vertical slice* through it. **Nothing
  in `sim/` ever learns `z` exists** — no invariant in `world.rs`,
  `chunk.rs`, or `parallel.rs` changes. Arithmetic alone rules out true 3D
  (2048³ cells is 68 GB; 512³ CA cells is 819× the sandbox's own worst
  frame) and rules out top-down 2D (gravity leaves the simulated plane,
  taking M17, most of M16, and the angle-of-repose mechanic with it).
- **Slice topology (straight cut vs. a curved route following the drainage
  network) is deliberately open and free to defer** — the curve lives
  entirely behind a `terrain_at(play_x)` interface `sim/` never sees past.
  **But reserve a slice-identifier field on `ChunkCoord` now** (issue #11)
  — `ChunkCoord` is constructed in 42 places and will hit the save format;
  adding a third field later is a 42-site migration plus a save-format
  break, adding one now (always zero) is mechanical. Make it a generic
  slice id (a bare `u32`), not specifically `z` — a route needs a route id,
  not a coordinate.
- **Six vertical zones**, defined not homogeneous-with-caves: sky → surface/
  canopy → soil/regolith (deepens with world age) → saturated zone (below
  the water table) → rock (caves, geothermal gradient begins) → deep/
  bedrock (M17's anchor). **The water table is the single highest-value
  structure** — visible as a real line in any cut face, gives roots
  something to grow toward instead of scanning for raw `Liquid` cells,
  gives springs where it meets the surface, valleys wet because they're
  low. Revised toward **local aquifers** (their own independent level, not
  one global table) once the design was checked against Minecraft 1.18's
  aquifer model — a global table makes every cave below sea level flood,
  which is the weaker behavior.
- **Deep structure is field-defined** (moisture/temperature/compaction as
  functions of depth — cheap, static, regenerable, needs no persistence);
  **shallow structure is accumulated history** (soil is where decay has
  built up, persisted, age-dependent) — meaning **the biologically active
  zone is the only thing ever saved**, and the depth of "biologically
  active" should key on `scheduler.rs`'s existing `active_site_count()`,
  never a hardcoded depth constant (today's zone depths are a calibration
  of today's tuning parameters, not an architectural fact, and the owner's
  stated intent is to tune them freely).
- **Caves: yes, additive density-function generation, not carving** —
  reversed from this document's own first draft after checking Minecraft
  1.18's Caves & Cliffs rewrite: decide where stone *is placed* (noise
  above a threshold → stone, below → cave) rather than carving voids out of
  placed rock, which removes the entire carve-then-anchor ordering problem
  M17's structural checks would otherwise hit. Two things Minecraft's
  approach does not have to solve that this engine does: **surface
  connectivity** (needs an explicit term, not hope) and **structural span**
  (a noise-defined cave ceiling has no bounded thickness against
  `stone.ron`'s `max_unsupported_span: 3` — genuinely unsolved, the
  strongest surviving case for keeping a controllable-radius worm-carve or
  a span-aware post-pass somewhere in the pipeline).
- **Rivers are not a worldgen feature — they're a consequence of the water
  cycle plus terrain designed to have drainage structure**, and this
  engine can build that mechanism rather than fake the appearance the way
  3D terrain generators without a real water sim have to: generate `h(x)`
  with real local minima (valleys) and an outlet, and let the existing
  `Liquid` water (already conserved, already flows downhill) find it. A
  river needs a real source (rain, or a spring where an aquifer meets the
  surface) and a real sink (ocean/world edge/evaporation) or it's a puddle
  that formed once, which forces **the closed water cycle** to exist before
  rivers can.
- **`worldgen(seed, coord, world_age)`, not just `(seed, coord)`.** A chunk
  generated on day 400 must generate a 400-day-old ecology directly (mature
  trees, accumulated soil) — otherwise walking into fresh territory shows a
  hard seam between "just generated" and "lived in." This makes worldgen
  and succession the same function evaluated at different times, and
  catch-up (advancing an unloaded chunk by 400,000 frames on reload) the
  same function evaluated a third way.
- **Generated terrain must be at rest.** Unique to a falling-sand engine —
  generate a 50° sand slope against a 34° repose angle and it slumps the
  instant the chunk wakes, and the slump can propagate into chunks that
  don't exist yet since `World::set` creates them on demand.
  **Recommended: generate only `Solid`** (stone never moves); loose
  material appears only where the player or runtime erosion produces it.
- **Species/world parameters become data, before heavy tuning starts** — the
  same reasoning that put materials in `.ron` files applies verbatim to the
  **56 currently-hardcoded `const`s** across `plant.rs` (34), `field.rs`
  (12), `creature.rs` (8), `structural.rs` (2). A `species/` directory
  alongside `materials/`, reusing the already-dependency `notify` +
  `MaterialRegistry::reload`'s exact pattern. "A world where trees are 500
  cells tall" should be a `.ron` file, not a fork.
- **Suggested sequencing** (worldgen depends on channels existing, so this
  mostly follows the priority order above rather than preceding it):
  species/world-params-as-data → replace `build_terrain` with a noise
  heightmap in the *fixed* world (testable today, exercises "terrain at
  rest" immediately) → water table + moisture baseline once the moisture
  channel lands → the coarse `(x, z)` map (elevation, drainage, water
  table depth, anchor distance) → the water cycle (rain + evaporation) →
  caves (density-function, explicit surface connectivity, explicit bedrock
  protection, a span-aware mechanism) → age-parameterized generation
  alongside catch-up → **streaming (M10 proper) last**, since it needs
  everything above plus the persistence taxonomy (`Reports/
  worldgen-design.md` §8: light/temperature/pressure regenerate on reload
  and never need saving at all; moisture is hybrid — persist only the
  deviation from the derived baseline, which RLE-compresses to nothing over
  undisturbed terrain; pheromone is the one channel with no alternative to
  persisting in full).

### Issues backlog (`Reports/pixel-physics-issues.md`, eleven items, full detail in the file)

Suggested order from the source doc: **#2 → #3** (one shared root cause,
largest perf win) → **#5 → #6 → #4** (field grid) → **#9 → #8 → #7** (M16
correctness before the forest gets bigger) → #1 and #10 as housekeeping
whenever. Folded into the priority-order list above where they overlap.

| # | Title | Kind |
|---|---|---|
| 1 | Commit `Cargo.lock` — gitignored on a binary crate, so builds aren't reproducible | chore |
| 2 | `touch_neighbours`'s fast-path guard is unreachable (`MAX_REACH..CHUNK_SIZE-MAX_REACH` is an empty range at today's constants) — every `World::set` pays the full neighbour-wake loop | perf |
| 3 | `SURFACE_SEARCH == MAX_REACH` (32) sets the sweep-region widening for *every* material via one read-only liquid lookahead, though real movement reach tops out at 5 — decouple them, track actual reach per chunk | perf |
| 4 | `field::step` runs 5 whole-world passes every frame with no sleeping, contradicting `README.md`'s own "a quiet field costs almost nothing" claim | perf, blocks M10 |
| 5 | `rebuild_blocked` does ~164k hashed `World::get` calls per frame (open air is the worst case and the common case) — index the chunk directly instead | perf |
| 6 | Seven un-hoisted loop-invariant `next.get(&coord)`/`get_mut` calls inside `field.rs`'s inner loops | perf, good first issue |
| 7 | `scheduler::step` is O(all pending sites), reallocates the whole schedule every frame, and its `HashMap` drain order is the engine's one real determinism violation (§8b) — rewrite onto a `BinaryHeap` with a deterministic tiebreak | perf, correctness, M16 |
| 8 | `World::trees` never shrinks — a fully-dead tree's `TreeState` (attractors, tips, roots) leaks for the process lifetime | bug, M16 |
| 9 | Tree/root tips check only their own `alive` flag, never whether their cell still exists — burn a tree or erase its trunk and orphaned tips keep extending wood from open air forever | bug, M16 |
| 10 | Housekeeping: default branch is `main` (a 15-byte stub — the project lives on `master`), no LICENSE, no CI, no `rustfmt.toml`/clippy config/`[lints]`, no `rust-version` (real MSRV is ≥1.87 for `u64::is_multiple_of`) | chore |
| 11 | Reserve a slice-identifier field on `ChunkCoord` before it reaches the save format (see the worldgen redesign above) | chore, architecture, blocks M10 |

A note from the same document worth keeping as a standing rule: two of its
findings (#4, #7) were cases where `README.md` or a module doc claimed a
property the code didn't actually have — both were prose claims with no
regression test behind them, while every claim backed by a *named* test
(`same_group_chunks_are_never_within_reach_of_each_other`,
`a_connected_mass_of_cooling_cells_actually_settles`,
`a_settled_world_with_a_growing_tree_still_sleeps_between_growth_ticks`)
held up. **A performance or cost claim in a doc needs either a test or a
measurement command next to it, or it should be written as an intention,
not a fact.**

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
| 14 | **M10** Streaming world | Large | Additive; the `get`/`set` seam already exists — **now redesigned, see [Third phase](#third-phase--emergent-world-architecture)** |
| 15 | **M11** Lua scripting | Moderate | Last, on top of a stable content model |

**Reprioritized (added mid-session, after M8's CCL start): the [Third
phase](#third-phase--emergent-world-architecture) backlog above — field
sleeping/perf/determinism fixes, the light writer, bilinear sampling,
moisture — now runs interleaved with, and largely ahead of, continuing M8
and before M10 proper.** M10 specifically is now blocked on that section's
worldgen redesign rather than the old one-line plan, and several of the
perf/correctness items (issues #4–#7) are explicit prerequisites the
architecture doc calls "enabling work; nothing else is safe to do at scale
without it." M8's remaining pipeline stages (marching squares onward) and
M9 continue to wait behind M8's own risk-deferral reasoning below, which
this reprioritization doesn't change.

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
- **M5** (multithreading): **done**, including an independent adversarial
  review that found no data-race or corruption bugs (it specifically tried
  to construct one across the two-active-chunks-sandwiching-a-passive-chunk
  geometry, then disproved it by exact arithmetic on `MAX_REACH ==
  CHUNK_SIZE / 2`) — out of the plan's stated order, moved up ahead of
  M16/17/18 by explicit user decision once they were available to weigh in
  on the design. Shipped with **no `unsafe` code**, contrary to what this
  plan originally sketched (a single `unsafe` function handing out
  overlapping mutable 3×3 chunk neighbourhoods) — a `CellSurface` trait plus
  a per-pass exclusive-ownership-and-deferred-queue design turned out to
  cover the same ground safely. ~3.6x speedup on the CA sweep alone (4
  cores); the combined CA+field worst case dropped from ~28ms to ~11.5ms,
  comfortably back under the 16.6ms/frame budget. Found and fixed one
  pre-existing M14 bug along the way (a connected mass of cooling cells
  could oscillate forever near — not at — ambient), plus a test-coverage
  gap the review flagged (now closed: a test isolating the exact sandwiching
  geometry at the cell level, which itself needed a second fix once written
  — the first version's single-frame assertion didn't account for the
  `moved`-flag deferral interacting with scan-direction parity). See
  `README.md`'s M5 status section for the full writeup, including the proof
  the design leans on and the subtler within-worker ordering bug that proof
  alone didn't catch.
- **M19** (visual polish) and the **M16/M18 scientific-accuracy research**:
  added mid-session by explicit user request, all research complete (3
  parallel agents for M19, 2 passes — an initial one plus a requested
  deeper follow-up on root architecture and plant signaling — for M16, 1
  pass for M18) and folded into this document, both as condensed summaries
  inline (M16/M18's own "Scientific accuracy directive" text, M19's own
  section above) and as full uncondensed reports in `research/` —
  [`research/m16-plant-biology.md`](research/m16-plant-biology.md),
  [`research/m18-creature-biology.md`](research/m18-creature-biology.md),
  [`research/m19-visual-polish.md`](research/m19-visual-polish.md) — written
  to disk specifically so the source material survives context loss between
  sessions. M16 (below) is built against its research; M18's and M19's are
  still queued.
- **M16** (active sites + plants): **done**. Scheduler (`scheduler.rs`) plus
  moss and trees-with-roots (`plant.rs`), built against the deep-dive
  research above rather than a placeholder version of it — auxin
  canalization for tree branching/apical dominance, MIZ1-style
  gravitropism/hydrotropism antagonism for root direction, oscillator-based
  lateral root priming, and moisture-and-shade-driven moss spread. Two real
  bugs found and fixed, both caught by tests that expected growth and got
  almost none: moss originally required a candidate cell to have a *solid*
  neighbour specifically, so every growth front dead-ended one step after
  starting instead of thickening into a patch (fixed by also counting
  existing moss as growable); roots originally could only advance into
  `Empty`/`Powder` ground, so a root approaching water — the entire point of
  root growth — died at its edge without ever drinking (fixed by giving
  `Liquid` targets their own absorbed-on-contact case). Independent review
  (following the standing practice of a review pass after every milestone,
  not just large ones like M5) found six more real issues before commit,
  all fixed: moss and starved roots could both become permanently-scheduled
  "immortal" active sites (the exact unbounded cost the scheduler exists to
  avoid — both fixed with stale-tick dormancy counters); `MaterialKind::Plant`
  didn't block the M13 field grid or the paint brush the way `Solid` does,
  undermining moss's own shade mechanic; tree tips could tunnel through any
  tree's already-grown wood since `wood` is one shared `MaterialId`; roots
  grew for free despite `TreeState::energy`'s own doc claiming a shared
  competitive pool; and the "auxin canalization" doc comment claimed more
  cross-tip competition than the code actually implemented, fixed by adding
  a real (if modest) mechanism — a branch's starting channel is now debited
  from its parent's — and correcting the doc to be precise about what's
  genuine competition versus what's actually plain space-colonization/
  shared-energy effects wearing the same name. See `README.md`'s M16 status
  section for the full writeup, including a tuning bug the new branching
  test surfaced (channel decayed on temporary energy waits as well as
  genuine dead ends, so it could almost never cross the branch threshold).
- **M17** (structural integrity): **done**. `structural.rs` gives every
  `Solid` cell a distance-to-anchor in `Cell::aux`, recomputed incrementally
  through the M16 active-site scheduler (a third `ActiveKind`,
  `StructuralCheck`, alongside moss and tree/root growth) and only
  propagated to a cell's solid neighbours when its own value actually
  changes. A cell whose distance exceeds its material's
  `max_unsupported_span` converts to `breaks_into` (stone → gravel) and
  falls under ordinary gravity. The one design decision the milestone
  hinges on: checks are scheduled *reactively* — from `World::paint_capsule`
  (the player's brush) and `explosion::trigger` — and never at world-gen
  time, so the sandbox's own pre-placed floor (8 cells thick, deeper than
  stone's span of 3) and floating decorative ledges stay put by default
  rather than crumbling the instant this shipped. A `cargo run --release
  --example ascii` scene (`structural_scene`) makes the mechanic visible
  directly: a 7-cell bridge anchored at both world edges stands whole, then
  erasing the right anchor collapses everything beyond reach of the
  surviving left anchor into gravel while the near stub stands — the same
  geometry `cutting_a_bridges_support_makes_the_far_side_collapse` checks by
  assertion. Independent review (same standing practice as M5/M13/M16) found
  one real bug before commit: the neighbour-relaxation loop read a burning
  `Solid` neighbour's `aux()` (its burn-timer countdown) as if it were a
  distance, reachable via `explosion::trigger`'s fireball step, which
  force-ignites nearby material — including stone — regardless of
  flammability. Fixed by excluding burning neighbours from the relaxation
  and deferring rather than reading their timers. One property found rather
  than designed in: a structure with no
  path to any anchor at all doesn't read as falsely "anchored" at its
  default aux value of 0 — once any part of it enters the scheduler, its
  cells relax upward every round-trip with no true zero source to converge
  toward (the same shape as the "count-to-infinity" problem from
  distance-vector routing), climbing without bound until every cell exceeds
  its span and the whole thing collapses, which is the physically correct
  outcome for something with nothing holding it up. See `README.md`'s M17
  status section for the full writeup, including the burn-timer guard
  (`Cell::aux` is a tagged union; a structural check on a burning cell
  defers rather than clobbering the burn countdown).
- **M18 Phase 1** (cell-based creatures): **done**. A burrowing worm
  (`creature.rs`), a `MaterialKind::Creature` cell dispatched from the M16
  scheduler exactly like a plant tip — new `ActiveKind::Creature { creature
  }`, indexing a per-creature energy-budget state (`CreatureState`), ticked
  every 6 frames. Built directly against the research (three mechanisms:
  burrow cost tied to a target `Powder`'s own `density` rather than a
  material-kind whitelist, per Kurth et al. 2018 and the Namib golden mole's
  measured ~26x sand-vs-surface energy cost; *C. elegans*-style thermotaxis
  reading the M13 ambient-temperature field to flee down-gradient once a
  threshold is crossed; an energy budget replacing random wandering, with
  starvation itself — no separate dormancy counter — being what stops a
  permanently-trapped worm from being rescheduled forever). Fire needed zero
  creature-specific code: `fire.rs` already applies uniformly to every
  material kind from `.ron` data, so `worm.ron`'s own flammability numbers
  are the entire mechanism behind "a creature catches fire and dies." Two
  real test-quality bugs caught and fixed while writing this milestone's own
  tests (not by external review): three tests filled their terrain with sand
  *before* planting a worm at a position already inside that fill, so the
  worm was silently never created and the tests passed vacuously; and a
  fire/corpse test's floor blocked a newly-formed corpse's straight fall but
  not the multi-cell *roll* a `Powder` also tries, found via a throwaway
  diagnostic print. Independent review (same standing practice as
  M5/M13/M16/M17) then found one critical bug before commit, confirmed by
  the reviewer's own reproduction: a moving worm's cell was always rebuilt
  from scratch, silently clearing `FLAG_BURNING` and the burn timer the
  instant a burning worm's next scheduled move came due — since the
  movement interval (6 frames) is far shorter than a burn's duration (60),
  this fired in the ordinary case, and a worm effectively survived every
  fire it caught by moving. Fixed by applying the same defer-while-burning
  guard `structural.rs` already established for `Solid` cells, plus a
  related fix (a worm could burrow directly into an actively-burning
  neighbour, never having checked the target's own burning state) and two
  smaller hardening items (an index-overflow debug_assert, a vacuous-test
  gap closed in the burrowing test). See `README.md`'s M18 status section
  for the full writeup, including the deliberate simplifications (no full
  Marginal Value Theorem patch-leaving bookkeeping, no aquatic worms, no
  multi-creature-kind interaction yet).
- **M8** (rigid bodies): **started, not complete** — deliberately narrow,
  per this plan's own warning that M8 is "the largest single milestone" and
  "the most exciting item and the one most likely to consume months without
  a playable result." `rigid.rs` implements the pipeline's first two
  stages: connected-component labeling (a 4-connected flood fill over
  `Solid` cells, capped by `max_cells`) and boundary/contour extraction
  (directed-edge stitching, the unambiguous equivalent of marching squares
  for a binary occupancy grid — no interpolation, no saddle case). Douglas-
  Peucker simplification, `earcutr` triangulation, the `rapier2d` collider,
  and the erase/step/re-rasterize frame loop are not started, and no new
  dependency has been added to `Cargo.toml` yet. Two real bugs caught: (1)
  while writing this module's own tests — `Cell::OUT_OF_BOUNDS` reads as
  `BEDROCK`, whose `MaterialKind` is `Solid`, so a naive flood fill treated
  the entire world boundary as one connected wall; fixed with the same
  "exclude bedrock, one check covers both literal bedrock and the world
  edge" trick `structural.rs`'s anchor detection already established. (2)
  by independent review — a "pinch point" input (two cells touching only at
  a shared corner) made the contour walk loop forever rather than degrade
  to the documented "wrong-but-closed" contour, confirmed by the reviewer's
  own reproduction; fixed by breaking the walk on any revisited point, not
  just its own start, with a timeout-guarded regression test. See
  `README.md`'s M8 status section for the full writeup.
- **M18 Phase 2** (Reynolds-steering entities): not started yet — explicitly
  waits on the rest of M8 per the plan's own reasoning.
- **Issue #1** (commit `Cargo.lock`): **done.**
- **Issue #10** (housekeeping): **partially done, honestly.** LICENSE (MIT,
  chosen by the owner over the dual MIT/Apache-2.0 Rust-ecosystem convention
  and GPL-3.0), `rust-version = "1.87"` (the real MSRV, from
  `u64::is_multiple_of`), `rustfmt.toml` (`max_width = 120`, chosen against
  a survey of actual current line lengths, not the default 100), and a CI
  workflow (`cargo test --release`, `cargo clippy -- -D warnings`,
  `cargo run --release --example ascii` as a headless smoke test — all
  gating; `cargo fmt --check` included but non-blocking) are all in place.
  **Not done**: the default branch is still `main` (a stub) rather than
  `master` — no `gh` CLI or API token was available in this session to
  change that GitHub repo setting, and it needs the owner's action via
  Settings → Branches (or `gh repo edit --default-branch master`) — and no
  actual `cargo fmt` pass has been run against the codebase (`rustfmt.toml`
  alone surfaces ~1550 lines of diff against the existing hand-formatted
  style; running it is a large, separate, reviewable change deliberately
  not bundled into housekeeping).
- **Issue #2** (dead `touch_neighbours` guard): **done, Option 1 (safe
  cleanup, zero behaviour change)** — the guard is genuinely a no-op at
  today's constants (`MAX_REACH..CHUNK_SIZE-MAX_REACH` is `32..32`, empty),
  and the comment on both copies (`world.rs`, `parallel.rs`) now says so
  explicitly instead of reading as though a fast path exists. **Issue #3**
  (decoupling `SURFACE_SEARCH` from `MAX_REACH`, which is what would make
  this guard live again) is **deliberately not attempted this session** —
  it requires re-deriving `parallel.rs`'s concurrency-safety proof from an
  equality (`MAX_REACH == CHUNK_SIZE/2`) to an inequality, and reasoning
  through whether that proof still holds when neighbouring chunks have
  *different* per-material reach values, not just a uniformly smaller
  constant. The same judgment call as M8's own scoping: real, and worth
  doing, but deserving dedicated attention rather than a pass at the tail
  of an already large batch of changes.
- **Issues #5 and #6** (field-grid lookup cost): **done.**
  `rebuild_blocked` now fetches the owning `Chunk` once per field tile
  (`world.chunk(coord)`, guaranteed resident since `coords` comes from
  `world.chunks()`) and indexes into it directly via `Chunk::get_world`,
  instead of a `World::get` — bounds check plus `HashMap` lookup — for
  every one of up to 4096 CA cells scanned per tile in the open-air worst
  case. Also hoisted seven loop-invariant `next.get(&coord)`/`get_mut`
  calls out of the `ly`/`lx` inner loops across `rebuild_blocked`,
  `step_pressure`, `step_velocity`, `step_diffusion`, and `step_advection`
  — each pass now fetches its tile pointer once per chunk. Measured via
  `cargo run --release --example ascii`'s combined CA+field stress scene:
  worst frame **28 ms → 24.8 ms serial, 11.5 ms → 7.8 ms parallel**. See
  `README.md`'s Performance section for the full numbers, including a
  correction to a claim that section used to make ("a quiet field costs
  almost nothing") that was never true of the actual implementation —
  issue #4 (field sleeping) is what would make it true, and is not done yet.
  Independent review caught one real bug before commit: the
  `World::get` → `Chunk::get_world` swap in `rebuild_blocked` dropped the
  world-bounds check along with it, so the out-of-world sliver of a chunk
  whose span extends past a non-64-aligned world size (the sandbox's own
  512×320 divides evenly; the 200×200 test worlds elsewhere in the codebase
  don't) silently stopped reading as blocked. Currently inert (every real
  consumer of field data re-checks bounds itself before consulting the
  stored value) but exactly the class of bug this file has hit before
  (three prior rounds of boundary-condition bugs, per its own README
  section) — fixed with an explicit `world.in_bounds` check, with a
  regression test that deliberately reaches past the masking layer via
  `World::fields_ref` to check the actual stored value.
- **Issue #9** (orphaned tree/root tips): **done.** `tree_tip_tick` now
  checks whether its own last-written cell still holds this tree's wood
  before doing anything else, mirroring `moss_tick`'s existing check —
  `alive` was previously only ever set by the tip's own logic, never by
  anything happening *to* it, so burning a tree or erasing its trunk left
  every tip extending wood from open air forever. `root_tip_tick` needed a
  real wrinkle handled, not just the same check copied over: a root's own
  cell is only *sometimes* wood — draining an adjacent water cell absorbs
  it and advances into the now-legitimately-empty space with no wood left
  behind, so checking for wood unconditionally would kill a perfectly
  healthy root the tick after it drinks. Added `RootTip::resting_on_wood`
  (set each tick depending on which branch of the growth match fired) so
  the validity check only fires when wood is actually expected there. Two
  regression tests confirmed to fail without the fix.
- **Issue #8** (`TreeState` leak): **interim fix, done** — not the full
  generational-index rewrite the issue's own "Direction" recommends as the
  complete fix (deferred; it's a real architecture change, not a quick
  pass). `attractors` (up to `ATTRACTOR_COUNT` = 50 points, by far the
  largest part of `TreeState`) is now dropped the moment every tip and root
  of a tree has died, checked inline at all six death sites via
  `reclaim_if_tree_is_fully_dead`. `TreeState` itself still never shrinks —
  tips/roots index into it by position, and the id-stability guarantee that
  buys is exactly why the full fix needs a free list, not attempted here.

---

## M19 — Visual polish: make the engine beautiful

Added mid-session by explicit user request, deliberately open-ended rather
than scoped like the numbered milestones above: **explore how to improve the
graphics and put deep effort into making the engine beautiful**, using
multiple research agents where that helps rather than working through it
alone. Everything up through M15 was built to be *correct* — every screenshot
taken so far was to verify behaviour, not to judge whether it looks good.
Nothing in the engine has yet been built or tuned with visual quality as the
actual goal, and this milestone is where that starts.

This is deliberately positioned **after** M5 and layered on top of, not
instead of, the work already queued (M16–M18, M8) — it does not block them,
and they do not block it. Concretely it overlaps with and very likely
subsumes **M6** (dirty-region uploads, emissive lighting, bloom), which was
deferred purely for lack of a human able to judge the result live, not
because the work itself was in question; this milestone is the natural home
for that work once it's unblocked, rather than a separate pass after it.

### Research findings

**Full research report: [`research/m19-visual-polish.md`](research/m19-visual-polish.md)**
— full detail (concrete algorithms, code-level specifics, complete citation
lists) behind the condensed summary below.

Three parallel research passes: how other falling-sand engines actually
render (not simulate) their materials, what `pixels`/wgpu concretely support
for custom rendering, and pixel-art palette/colour theory. All three landed
in the same place — **the highest-leverage wins here are CPU-side, need no
shader pipeline at all, and are cheap enough to be weekend-scale work, not a
rewrite** — which reframes M6 from "one big GPU pipeline" into a small set
of independent, individually-shippable techniques.

**Palette (Lospec community practice, and specifically how Resurrect 64 /
Endesga 32 are built):** organize colours as **ramps in HSL**, one ramp per
material family, not picked freehand per material as today. The rule that
actually unifies a palette: shift hue *while* shifting value — darks rotate
toward blue/purple and desaturate, lights rotate toward yellow and desaturate
slightly — rather than just scaling brightness. Cap total distinct hues
game-wide to a handful; distinguish adjacent materials (sand vs. water vs.
gravel) by **hue**, not just value, since value-only differences vanish at
small pixel sizes. Reserve peak saturation *and* peak lightness together
exclusively for hot/emissive materials — that specific combination is what
reads as "glowing" to the eye even with zero lighting engine involved, pure
palette trick. [Resurrect 64](https://lospec.com/palette-list/resurrect-64)
and [Endesga 32](https://lospec.com/palette-list/endesga-32) are both
directly adoptable starting points that already span earth/fire/water/gas
families in one cohesive grade, rather than designing 7+ ad hoc palettes from
scratch.

**Grain and glow, cheaper than expected (Sandspiel, The Powder Toy):**
Sandspiel's whole "looks good despite being simple" reputation traces to one
trick — an 8-bit per-particle register reused as a brightness jitter, so
same-material pixels get organic ±brightness variation for free. This
engine already has almost exactly that slot (`Cell::shade`, currently only
used to pick a fixed palette entry) — modulating brightness *from* shade
rather than only indexing a palette with it is nearly free. The Powder Toy's
actual renderer (`Renderer.cpp`, real shipped C++, not a talk) has three
techniques worth stealing directly, all CPU-side, no shader: `PMODE_GLOW`
(draw a hot/lit pixel at full intensity, add the same colour at reduced
alpha to its near neighbours, decaying over ~5px — a hand-rolled tiny radial
blur per glowing pixel, not a full-screen pass); `PROP_HOT_GLOW`
(temperature-driven colour modulation — above a threshold, shift RGB by a
function of temperature rather than a flat lookup, which this engine's
existing per-cell temperature already has everything needed to drive); and
its heat/pressure debug visualizations (dark-blue-to-pink gradient by
temperature), directly reusable for the M13 field grid's own display.

**Noita's actual technique is not a lighting engine.** Public sources (the
GDC talk and the community wiki) don't document per-pixel dynamic lighting —
"darkness" is a particle-based fog/visibility layer, holes of soft radial
falloff punched into it by light sources, additively stacked. That's a
second buffer and a short list of active light sources, not a shader —
squarely in reach without M6's originally-assumed custom pipeline.

**A concrete, cheap path to real light propagation, reusing work already
done:** the M13 field grid already carries a `light` channel at 1/8
resolution, currently only lit by debug tools. A flood-fill/BFS light
propagation (seed emitters — fire, lava — at full intensity, propagate to
neighbours subtracting falloff per step, attenuate crossing solid material)
populates it cheaply at that coarse resolution (documented prior art:
[0fps.net's voxel/flood-fill lighting writeup](https://0fps.net/2018/02/21/voxel-lighting/)).
Upsample and multiply over the frame the way Noita's fog layer does, and the
existing field grid is doing double duty it was already positioned for.

**If a custom GPU pass is still wanted later (true bloom, not the CPU
radial-glow approximation):** confirmed this is a first-class, documented
extension point, not a hack — `Pixels::render_with` hands over the raw
`wgpu::CommandEncoder` and render target, and the crate ships a working
`custom-shader` example (texture, sampler, bind group, WGSL pipeline,
chained into the same encoder as the default scale blit) that's a direct
template for a bloom pass fed from a separate emissive texture. This is real
work (~1 week estimate, mostly wgpu bind-group plumbing) and still needs the
live visual judgment M6 was deferred for — it stays the GPU-pipeline tier of
this milestone, not a blocker for the CPU-side tier below.

### Execution plan

Reframed into tiers by how much they need a human watching, cheapest and
most self-verifiable first:

1. **Palette overhaul + per-cell brightness jitter + temperature-driven
   colour shift + ordered (Bayer) dithering to kill flat-colour banding.**
   All CPU-side, all inside the existing `render.rs`/`material.rs` model, no
   new systems. Verifiable the same way M7/M15 were — the in-app framebuffer
   dump — since "does this look like it has grain/isn't flat" is a much
   lower judgment bar than "is this bloom kernel tuned well."
2. **Powder-Toy-style radial glow around hot/burning cells, and fake
   ambient-occlusion darkening for granular piles** (darken a solid cell by
   its local enclosed-neighbour fraction — cheap, no new data needed).
   Still CPU-side; still self-verifiable by screenshot comparison.
3. **Coarse light propagation on the M13 field grid's existing `light`
   channel**, Noita-fog-layer style. Bigger than 1–2 but still no shader
   pipeline required.
4. **A real GPU bloom pass via `Pixels::render_with`**, and any further
   custom lighting — this tier is what M6 actually is, stays deferred for
   the same reason it always was (needs a human watching it render, not a
   correctness question), and is the natural place any of tiers 1–3 that
   turn out to want GPU acceleration would move to later.

Tiers 1–2 are square with what's already been proven safe to do unattended
this session (self-verifiable via screenshot, no live judgment call); 3 is a
reasonable stretch; 4 stays with M6. See the progress log for how far this
actually got.

---

## Scientific accuracy for plants and creatures (M16, M18)

Added mid-session by explicit user request, applying to both halves of the
"life" work in this plan: when M16 (plants) and M18 (creatures) are actually
built, do real research and try to make the mechanisms as scientifically
accurate as possible, not just visually/behaviourally plausible. The
directive is recorded inline in each milestone's own section above (search
"Scientific accuracy directive") so it stays attached to the specific claims
it constrains rather than living only as a disconnected note here; this
entry exists so it's also visible without reading the full milestone text.
Research toward this is being gathered ahead of actually starting M16/M18 —
see the progress log for status.
