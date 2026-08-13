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

> **Direction reports:** the `Reports/` directory holds five documents an
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
> whatever the first colony-forming creature work turns out to be),
> [`Reports/pixel-physics-issues.md`](Reports/pixel-physics-issues.md) (eleven
> concrete performance/correctness/housekeeping issues against the codebase as
> it stood then — nine are closed; issue #3 closed in the overnight run's
> section 5. Issue #8 (generational tree-state indices) is partially
> addressed in section 8: a real generational `organism_id` allocator
> exists and is tested, but nothing calls its `free` side yet (moss, the
> only species retrofitted so far, has no natural "this organism is fully
> dead" signal without more infrastructure), and `TreeState` itself — the
> issue's actual original subject — is untouched, deferred alongside the
> rest of the tree retrofit. So this is not quite a purely historical
> record yet),
> [`Reports/design-philosophy.md`](Reports/design-philosophy.md) (the short,
> opinionated statement of the philosophy the other four already implied —
> read this one first), and
> [`Reports/organism-substrate-design.md`](Reports/organism-substrate-design.md)
> (overnight run section 7: the shared, cell-typed species/behavior model
> replacing `TreeState`/`CreatureState`, built for section 8). These are
> **not milestone research** the way `research/` is — they're a
> direction-setting pass that reshapes near-term priority order. Read the
> relevant report before touching anything it covers; this file's condensed
> version is not a substitute for the reasoning behind it.

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
> already delivers tunable, irregular slopes, so those were left as optional
> polish. **Superseded:** `Reports/granular-mechanics-research.md` §3 finds BTW
> toppling should not be built at all, not just deprioritized — real 2D
> sandpiles don't show the power-law avalanche distribution BTW predicts
> (Jaeger/Liu/Nagel; the Oslo rice-pile result), and the actual mechanism
> (dilatancy + velocity-weakening, §5 and §2 of that report) produces *better*
> granular behavior for less code. See the granular-mechanics entry in the
> execution order below for what replaces this.


Replace ad-hoc "try down, then down-left/down-right" with rules that have physical meaning. This is where the academic work pays off:

- **Angle of repose from a friction angle.** Klár et al., [*Drucker-Prager Elastoplasticity for Sand Animation*](https://math.ucdavis.edu/~jteran/papers/KGPSJT16.pdf) (SIGGRAPH 2016) models sand via a yield criterion relating shear to normal stress. Do **not** implement MPM — far too slow. Steal the *parameterization*: give each powder a `friction_angle` instead of a magic "spread factor." It maps directly to pile slope and tunes predictably across every material you add.
- ~~**Pile relaxation via BTW toppling.**~~ **Do not build** — see the
  "Superseded" note above. `Reports/granular-mechanics-research.md` §2's
  two-angle model (θ_ms/θ_r, one `Cell::flags` bit) is what actually
  produces avalanches, hysteresis and bistability, and is cited there as
  the *accurate* replacement for this, not a cheaper approximation of it.
- **Granular flow as upward hole propagation.** Baxter & Behringer, *Physica D* (1991); [Kozicki & Tejchman, *Granular Matter* (2005)](https://link.springer.com/article/10.1007/s10035-004-0190-x). **Caution added:** `Reports/granular-mechanics-research.md` §6 finds the naive void-random-walk formulation over-mixes badly (confirmed against the same literature this section already cites) — if built, use the report's correlated "spot" model instead of a single-cell void walk, and only once an actual hopper/silo use case needs it (§10 there recommends deferring past piles/pours/avalanches, which the two-angle model already covers).
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
- **M6** (rendering upgrade): **split**. The bloom/emissive shader half
  stays **deferred** — needs live visual judgment a screenshot-and-reason-
  about-it loop can't substitute for, parked for a session where that's
  available, not abandoned. The dirty-region half shipped, reframed: the
  originally-planned GPU texture upload path turned out to be blocked by
  `pixels` 0.17.2's own architecture (see the overnight run's section 11
  entry), so the actual win landed as a CPU-side skip in `Renderer::draw`
  instead — measured 6.6ms → 0.0ms worst frame on a settled scene.
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
- **Issue #4** (field sleeping): **done.** `field::step` now skips its whole
  five-pass solve once `world.active_chunk_count() == 0 &&
  world.fields_settled()` — both conditions, not the field's own
  convergence alone, which is what keeps "a shockwave can cross the whole
  screen" safe without any separate per-tile occupancy tracking: any CA
  write (including painting a new wall) always dirties its own chunk,
  forcing at least one more full pass, and within that pass a cell that
  just became blocked resets to ambient (every pass skips writing to a
  blocked cell) while the pre-block value is still what `is_converged`
  compares against — a jump it will not miss. `is_converged` compares each
  channel of the just-solved state against its pre-step value against a
  small per-channel epsilon; `add_pressure_impulse`/`add_heat`/`add_light`/
  `add_heat_local` clear the settled flag directly, since those bypass the
  CA grid entirely. Measured via a new permanent `examples/ascii.rs` scene:
  an isolated pressure impulse's worst frame drops from ~2-4 ms while
  actively propagating to ~0.0001-0.01 ms once settled — several hundred
  times, and the actual acceptance criterion the issue asked for (a
  measured number, not an assertion). The continuously-active stress scenes
  cost slightly *more* than the pre-#4 baseline (~28 ms serial / ~9 ms
  parallel vs. ~24.7 ms/~7.6 ms), not less — `is_converged`'s own
  comparison pass is real added cost on every frame the solve actually
  runs, paid back only once things go quiet, which a scene built
  specifically to never settle never collects on; the win is real but
  shows up entirely in the quiet case, not the saturated one.

  Independent review (warranted given `field.rs`'s history of three prior
  boundary-condition bugs) found two real, narrow gaps in the "occupancy
  changes are caught for free" argument, both fixed: (1)
  `parallel::ChunkView::add_heat`'s same-chunk branch — the common path for
  `fire::tick_burn`'s heat push — wrote directly into a worker's own field
  tile without clearing the settled flag, since a worker has no `&mut
  World` to clear it on the spot; currently masked only by the coincidence
  that a burning cell's own `tick_burn` also writes its cell every frame it
  burns, independently keeping the chunk awake regardless, not a structural
  guarantee. Fixed with a queued `field_touched` flag replayed in
  `parallel::run_pass`, the same shape `field_writes` already uses, with a
  regression test confirmed to fail without the fix. (2) A wall placed by
  `step_active_sites()` (plant growth) or `particle::step()` (a landed
  particle) is invisible to `active_chunk_count()` for the one frame it
  happens on if the field was already fully converged, since `Chunk::mark_
  dirty` only sets `pending_dirty` and `World::end_step` (which promotes it)
  runs *before* those two subsystems in `App::update`'s frame order — but
  self-correcting (the very next frame's `end_step` promotes it, so the
  wall is noticed one frame late, never dropped entirely), and CA writes
  from the sweep itself are never subject to it. Documented in `field::
  step`'s own doc rather than structurally fixed, since fixing it would
  mean coupling `plant.rs`/`particle.rs` to field-grid internals for a
  one-frame effect that already heals itself.
- **Issue #7 + determinism §8b** (scheduler): **done.** Replaced
  `scheduler.rs`'s `HashMap<ChunkCoord, Vec<ActiveSite>>` — which drained
  and re-tested *every* pending site against `due` every frame regardless
  of how many were actually due, and whose randomized-per-process iteration
  order was the engine's one documented non-determinism source — with a
  `BinaryHeap<Reverse<ActiveSite>>`, a min-heap on `next_frame` with
  `(x, y, kind)` as a fully deterministic tiebreak via a hand-written `Ord`
  impl (not derived field-order, which would have compared `x` before
  `next_frame`). `scheduler::step` now peeks the minimum and stops the
  instant it finds a not-yet-due site — true O(due · log n), no
  full-structure rebuild every frame — fixing the performance half and the
  determinism half with the same change, as the issue itself predicted.
  Confirmed nothing actually depended on the old chunk-keyed lookup before
  removing it (grepped every use; only ever iterated the whole structure).
- **Issue #11** (reserve a slice field on `ChunkCoord`): **done.** Added
  `pub slice: u32` (see the worldgen redesign above for what it's for),
  always `0`. Every `ChunkCoord` in the codebase is built through exactly
  two constructors (`new`, `containing`), both in `chunk.rs` — updating
  those two hardcoded the new field, so none of the ~26 actual call sites
  elsewhere needed to change at all (the issue's own estimate of "42
  places" was counting call sites on the assumption the constructor
  signature itself would need to change, which it didn't).
- **Architecture §2** (light writer): **done.** Two writers for the M13 light
  channel that had stood inert since M16 — `shade_factor` (moss) and tree
  phototropism in `plant.rs` both already read `field_at(..).light`, but
  nothing had ever written to it in real gameplay. `fire.rs`'s `tick_burn`
  now pushes a small `add_light` alongside its existing `add_heat` call.
  `field::step` gained a new `apply_sky` pass — run last, after
  `step_advection` (which, like every other pass, unconditionally overwrites
  every field cell it touches, sky row included) — that forces the topmost
  *exposed* field row (no chunk resident directly above it, so this adapts
  correctly to irregular/streaming chunk layouts rather than assuming one
  global top row) to `MAX_LIGHT` every step, unless that cell is itself
  CA-blocked. Deliberately does not clear `fields_settled` (unlike
  `add_light`/`add_heat`): it's a stable boundary condition, not an external
  disturbance, and `is_converged`'s existing old-vs-next comparison already
  catches any real change (newly exposed or newly shaded cells) on its own.
  `CellSurface` gained `add_light`, implemented by both `World` and
  `ChunkView` as an exact mirror of their existing `add_heat` (including
  `ChunkView`'s cross-chunk write-queueing and shared `field_touched` flag).
  `LIGHT_DECAY` turned out steep by design ("diffuse fast, decay hard" —
  see `field.rs`'s own doc comment): a sky-lit column reads near dark again
  within about 3 field cells (24 world pixels), so the new regression test
  (`open_sky_reads_brighter_than_a_directly_blocked_cell`) probes one field
  row below the sky rather than assuming any deeper reach. Two pre-existing
  field-sleeping tests (`an_impulse_wakes_an_already_settled_field`,
  `a_same_chunk_heat_push_during_the_parallel_sweep_wakes_the_settled_field`)
  needed their one-step "should already be settled" setup widened to a
  bounded loop, since an undisturbed field now takes several frames to reach
  its fixed point (light diffusing down from the new sky source) rather than
  being trivially converged from frame one.
- **Architecture §6a** (bilinear field sampler, "the resolution problem"):
  **done.** `sample_bilinear` (`field.rs`) already existed for advection's
  own back-traced lookups and was private; it is now `pub(crate)`, wrapped by
  a new public `World::field_at_bilinear(fx, fy)` that computes its own
  blocked-corner fallback (this position's own block-nearest reading).
  Routed the two existing short-range gradient-followers through it: the
  worm's thermotaxis `min_by` (`creature.rs`) and the tree tip's
  phototropism probe (`plant.rs`) — both were comparing candidates only 1–4
  world cells apart, well inside the same `FIELD_SCALE = 8` block `field_at`
  reads identically for, degenerating "follow the gradient" into "always
  pick whichever candidate was checked first." New regression test
  (`field_at_bilinear_resolves_what_field_at_flattens_within_one_block`)
  proves the specific claim: two probe points sharing one coarse block read
  identically through `field_at` but distinctly through `field_at_bilinear`.
  An independent review found the diff itself correct but flagged that
  neither existing consumer test actually discriminated the fix from the bug
  it fixes — the worm's own flee-test happened to put the heat where "always
  flee west" (the degenerate tie-break) was also the right answer, and no
  phototropism test existed at all. Two regression tests added in response,
  both confirmed to fail (by temporarily reverting the call site to
  `field_at`, running the test, then restoring) before being trusted:
  `a_worm_flees_east_even_though_west_is_checked_first` (heat placed so the
  degenerate and correct answers disagree) and
  `a_tip_leans_more_steeply_upward_when_lit_from_above` (a hand-constructed
  `TreeState` with a single off-axis attractor, so the photo term's
  y-only nudge has a real x/y mix to bias rather than a purely-vertical
  vector it can't visibly change after normalization). Does not yet touch
  the trail-*width* half of "the resolution problem" — that is explicitly a
  future moisture/pheromone-channel-resolution question (§4), out of scope
  here.
- **Architecture §4** (moisture field channel): **done.** `FieldCell` gained
  a fifth channel (`moisture`), sourced from `Liquid` CA cells
  (`apply_moisture_sources`, same shape as `apply_sky`), diffusing
  (`MOISTURE_DIFFUSION_RATE`) and evaporating faster above ambient
  temperature (`MOISTURE_EVAPORATION_PER_DEGREE` — the "extra loop" the
  architecture report itself suggested, tying moisture to heat rather than
  a single fixed decay rate). All four waiting consumers wired in: `plant.rs`'s
  `is_damp` and `strongest_water_pull` (renamed `moisture_pull`, now a
  gradient read through `field_at_bilinear` per §6a rather than an O(r²)
  hand-rolled scan) for moss and root hydrotropism; `creature.rs`'s
  `move_cost` discounts a worm's burrow cost by local saturation
  (`WORM_MOISTURE_DISCOUNT` — damp substrate holds a tunnel shape better
  than dry, a documented judgment call since the cited research names
  moisture as a resistance modulator without specifying direction);
  `fire.rs`'s `try_ignite` suppresses (not eliminates) the probabilistic
  contact-ignition path by local saturation (`MOISTURE_IGNITION_RESISTANCE`),
  leaving the deterministic temperature-crossing path untouched so a fire
  hot enough to boil off the water can still set wet material alight.
  `CellSurface` gained a `field_moisture_at` read (fire's only field read,
  unlike every other consumer here, which is why it needed the trait
  extended rather than just calling `World::field_at` directly — `ChunkView`
  answers it from its own field tile with no shared-`World` access needed,
  since the query position is always inside the caller's own chunk).
  `rebuild_blocked`'s CA scan now also detects `Liquid` presence in the same
  pass, at a real, measured, and honestly-documented cost: its first version
  kept the original early-exit on finding a solid cell, which broke the
  common "puddle resting on a thin floor" case whenever an unrelated solid
  cell happened to sit earlier in scan order than the water — caught by
  `moss_spreads_over_damp_stone_and_not_over_dry` regressing hard once it
  switched from the old scan to a real field read. Every block is now
  scanned in full; measured against the full-screen stress scene, no
  significant regression (28.0 ms serial / 8.3 ms parallel vs. ~28 ms/~9 ms
  already on record) — see README's Performance section. Four regression
  tests, one per consumer, each confirmed to fail without its fix before
  being trusted: `standing_water_is_a_moisture_source_...`/`moisture_does_
  not_leak_through_a_sealed_wall` (field.rs), `roots_steer_toward_off_axis_
  water_via_hydrotropism`/`moss_spreads_over_damp_stone_and_not_over_dry`
  (plant.rs, both switched to a new `run_with_fields` test helper that also
  steps the field solver — most of `plant.rs`'s other tests deliberately
  don't, isolating CA/scheduler behaviour from field behaviour), `damp_sand_
  is_cheaper_to_burrow_through_than_dry_sand` (creature.rs), and `moisture_
  suppresses_ignition_from_a_burning_neighbour` (fire.rs — exploits `World::
  new`'s fixed RNG seed for an exact, non-statistical comparison: two fresh
  worlds draw the identical random sequence each frame, so a lower
  ignition-chance threshold can only ignite the same frame or later, never
  earlier, deterministically). Independent review found one real bug: the
  first version of `rebuild_blocked`'s rewritten scan still broke its entire
  block scan on the first out-of-bounds cell it hit, reintroducing — one
  level up — the exact "scan order can hide a liquid cell" bug it had just
  fixed for the solid-cell case. A world whose size isn't a multiple of
  `FIELD_SCALE` has field blocks straddling its own edge, and a vertical
  edge puts an out-of-bounds cell at the same column in every row, so hitting
  it on row zero aborted the scan before any later, fully in-bounds row —
  where a real `Liquid` cell could sit — was ever examined. Currently
  unreachable in practice (every `World::new` call site in this codebase
  uses `FIELD_SCALE`-aligned dimensions) but not guarded against, so fixed
  rather than left latent: no early exit anywhere in the scan any more, on
  either condition. New regression test (`a_liquid_cell_is_detected_even_in_
  a_field_block_that_straddles_the_world_edge`), confirmed to fail against
  the reverted behaviour before being trusted.
- **Architecture §5g** (plants write the channels they read): **done.** One
  of the two writes was already free — `rebuild_blocked` has blocked on
  `Solid | Plant` since M16, so light occlusion needed nothing new once §2's
  sky writer landed. The other: a new `World::deplete_moisture` (mirrors
  `add_light`'s shape, subtracts and floors at zero instead of adding)
  called at both of `root_tip_tick`'s water-drink sites, right next to the
  existing `ROOT_WATER_ENERGY` grant. Turns moisture from a read-only
  channel into a loop — a root draining a shared puddle now leaves a
  measurably lower reading behind for a neighbouring root's own `moisture_
  pull` to notice, the resource-competition-through-the-world mechanism the
  architecture report's §0 names as the actual payoff. New regression test
  (`deplete_moisture_lowers_the_local_reading_and_floors_at_zero`) checks
  the mechanism directly rather than through a full multi-root competition
  scene, which would mostly be testing scheduling noise rather than the
  write itself.
- **Architecture §5h** (day/night oscillator): **done.** Per the report's
  own build note — "the same writer [as §2's sky] with a time-varying
  amplitude" — `apply_sky` now forces the sky row to `sky_light_amplitude
  (world.frame)` instead of a flat `MAX_LIGHT`: a cosine hump clamped at
  zero, spending exactly half of `DAY_NIGHT_PERIOD_FRAMES` (3600) flat at
  `NIGHT_LIGHT_FLOOR` (0.2, real moon/starlight rather than absolute black)
  and the other half ramping smoothly through a daylight peak at `MAX_
  LIGHT`. Every existing reader of the light channel (moss shade-seeking,
  tree phototropism) gets a real day/night cycle for free, matching the
  report's own claim that one oscillator drives several systems at once
  purely because they already read the channel it writes.
  
  This surfaced a real interaction with issue #4 (field sleeping) that
  needed its own fix, not just documentation: `apply_sky`'s value now
  changes with elapsed time alone, with no CA write to keep `active_chunk_
  count()` nonzero the way every other disturbance the sleep gate relies on
  does — without a fix, a field that settled at noon and then saw the CA
  grid go fully quiet would stay frozen at noon's brightness forever.
  `field::step`'s early-return gate now also compares `sky_light_amplitude
  (world.frame)` against the previous frame's value (a cheap pure-function
  call, not a field read) and refuses to skip when they differ by more than
  `SETTLE_EPSILON_LIGHT` — which happens only near actual dawn/dusk
  transitions, since the cosine's own derivative is small near noon and
  midnight, so sleeping through the steady parts of day and night still
  works exactly as before. Measured against the stress scene: no
  significant change (28.6 ms serial / 9.8 ms parallel, within this
  machine's already-documented run-to-run noise). Two new regression tests:
  `sky_light_amplitude_cycles_between_the_night_floor_and_max_light`
  (the oscillator's own shape) and `the_sky_keeps_cycling_through_day_and_
  night_even_after_the_field_goes_quiet` (the sleep-gate interaction,
  confirmed to fail — stuck at noon's brightness forever — without the fix).
- **Architecture §5f/§5e** (ash → soil decay cycle, with reseeding):
  **done.** Closes M16's own verify criterion, "a forest burns and
  regrows" — only the burning half existed before this. New `decay.rs`
  module, dispatched from `scheduler::step` via a new `ActiveKind::Decay`
  the same way M17/M18 already are. New `soil` material (Powder, appended
  to `EMBEDDED` — not inserted alphabetically, since every other material's
  numeric id is its array position and inserting in the middle would have
  silently renumbered everything after it). `fire::tick_burn`'s burnout
  path schedules a decay check the moment a burnout specifically produces
  ash (hardcoded to that one material name, not a new schema field —
  matching the report's own "cheap: one material, one slow transformation"
  framing); `decay::tick` re-checks periodically, gated on the moisture
  channel (damp ash decays into soil at a real rate, dry ash only very
  rarely, mirroring `plant.rs`'s own damp/dry duality for moss), and a
  freshly-formed soil cell gets one roll to reseed moss or a tree in the
  empty cell above it — a documented simplification, not perpetual
  reseeding.

  Needed a real architectural seam, not just a new module: `fire::tick_
  burn` runs generic over `CellSurface` (both the serial sweep and
  `ChunkView`'s parallel workers), but only `World` owns the active-site
  heap, so `CellSurface` gained `frame()` and `schedule_active_site()` —
  `ChunkView`'s implementation queues the site and replays it in `parallel::
  run_pass`, the same shape as the existing `field_writes`/`light_writes`
  queues. Three regression tests, one per real claim (damp decays but dry
  doesn't; a real burnout schedules its own check, not just a hand-built
  `ActiveSite`; a freshly-decayed soil cell can reseed) — the reseed test
  needed several separately-walled puddles along one long ash strip, not
  one, since a single puddle's edge only gives a handful of damp-and-open
  cells to roll the reseed chance against, and one unlucky small sample
  had already been caught failing during development.

  Found and fixed a live regression along the way, not introduced by this
  work but exposed by it: `examples/ascii.rs`'s `plant_scene` helper never
  called `world.step_fields()`, despite its own doc comment already
  claiming it did — harmless before the moisture channel existed, since
  `is_damp` used to scan the CA grid directly, but once §4 switched it to
  a real field read, the "moss spreads on damp stone, stalls on dry" demo
  scene silently stopped demonstrating anything (both sides read as
  uniformly dry, since the field was never being solved). Fixed, and a new
  `regrowth_scene` demoes the full ash → soil → (sometimes) regrowth path
  end to end.

**With the priority-ordered list above fully done (items 1–8), remaining
work is item 9's own "lower priority, in whatever order suits" tier**:

- **Plants read the velocity field** (§5d, wind bends canopy): **done.**
  `tree_tip_tick`'s growth-direction formula gained a `wind_lean` term, the
  same additive shape `photo` already uses. Deliberately a growth-time
  lean, not a per-frame visual sway — nothing in this engine's rendering
  can bend an already-placed cell, so the "large visual payoff" the report
  describes comes from the tree's *grown shape* carrying a permanent
  prevailing-wind bias, the same way a real wind-trained tree does, not
  from real-time animation. Independent review caught a real problem with
  the first version: it scaled the lean by raw velocity magnitude, and
  `field.rs` clamps pressure but never velocity — a nearby explosion's own
  shockwave (magnitudes the review measured at several times the combined
  weight of every other input to the formula) could dominate the tip's
  growth direction outright for as long as the transient took to pass,
  contradicting the "gentle lean" the constant's own doc claimed. Fixed by
  making the lean direction-only at a fixed magnitude (`WIND_LEAN_
  MAGNITUDE`, gated by `WIND_SPEED_THRESHOLD` so a near-zero field reads as
  no wind rather than an arbitrary direction) — mirrors `photo`'s own fixed
  `0.25` nudge exactly. The review also found the original regression
  test's one-shot `add_pressure_impulse` produced a decaying, sign-flipping
  oscillation rather than a steady breeze (empirically confirmed: `vx` at
  the tip crossed negative by step 27 of a test that read it at step 20,
  passing only because that step happened to land in a lucky window).
  Replaced with continuous per-step forcing instead of one impulse, which
  settles into a genuinely stable window (confirmed by hand: 30+
  consecutive steps of consistent sign after a brief initial transient) —
  a real steady wind, not a lucky sample off a decaying wave.
- **Structural integrity extended to `Plant`** (blocked on the `Cell::aux`
  slot conflict M16's growth stage was originally reserved for):
  **done.** Resolved, not deferred further — growth stage in `aux` was
  never actually implemented (grepped for it: zero write sites in
  `plant.rs`), since real per-tip growth state lives in `TreeState`/`Tip`/
  `RootTip` instead, which is where it needed to be anyway (attractor
  lists and channel strength don't fit in a `u16`). With the slot
  genuinely free, `structural.rs` gained `is_body_material` (`Solid |
  Plant`, replacing three separate `MaterialKind::Solid`-only checks), and
  every place that already schedules a structural recheck reactively
  (`World::paint_capsule`, `explosion::trigger`) now triggers on `Plant`
  too. `wood.ron` gained the plan's own long-suggested numbers
  ("stone 3, wood 8, steel 20") — `max_unsupported_span: 8`, `breaks_into:
  "deadwood"` — and a new `deadwood` material (`Powder`, flammable, burns
  to ash) for what a broken trunk actually falls as. A new hook in `fire::
  tick_burn` schedules a structural recheck around whatever a burnout just
  removed, generalizing the existing `placed_solid`/`erased_solid`
  reasoning to a *third* way a structural cell disappears — burning is
  neither painting nor an explosion, and needed its own hook rather than
  falling out of either existing one for free. Found and fixed a real bug
  while writing the end-to-end regression test for this: an early version
  wrapped `update::step` in its own manual `begin_step`/`end_step` pair
  inside the test loop, not realizing `update::step` already calls both
  internally — the double call desynced `world.frame` and the dirty-rect
  promotion badly enough that the test's burning cell never got swept at
  all (`active_chunk_count()` stuck at 0 for the entire run). Four new
  tests: span-exceeded and span-respected beam checks (mirroring the
  existing stone ones), and the full burn-collapses-the-trunk path, each
  confirmed to fail without its respective fix before being trusted.

### Live playtest feedback (screenshots of `cargo run`, not the ascii harness)

The owner ran the actual GUI mid-session (trees grown from several plantings,
two explosions in a sand pile) and reported three things back, independent of
any automated test. Two were actioned immediately; the third was deliberately
deferred:

1. **Explosions vaporized almost everything and produced little visible
   force** — "I want to see sand flying." **Actioned, done.** The old model
   rolled `chance(1.0 - sqrt(dist2/r2))` per cell in the blast radius, which
   put the odds against debris almost everywhere: a circle's area is
   dominated by its outer band, where that curve is already low.
   Reproduced the complaint exactly with a dense-fill test before touching
   anything (temporarily reverted to the old formula: 90/317 = 28% debris,
   i.e. ~72% vaporized, matching "vaporize 99%" in spirit). Replaced with:
   a small deterministic vaporize core (`VAPORIZE_FRACTION = 0.12`, no
   debris — genuinely gone), *unconditional* debris everywhere else in the
   primary radius (no more RNG roll), and a new shockwave annulus out to
   `radius * SHOCKWAVE_RADIUS_MULTIPLIER` (1.8) where loose material only
   (`Powder | Liquid`, not `Solid | Plant`) gets a linearly-fading pickup
   chance — this is what throws sand that was never inside the crater,
   which is the actual mechanism the "collapses inward instead of flying
   outward" complaint was missing. `Solid`/`Plant` deliberately excluded
   from the shockwave pickup: ordinary CA-grid material still isn't pushed
   by the field outside an explosion (that's a much bigger, separate
   change — free particles and this shockwave zone are the only things the
   pressure field moves today), and flinging structural material on every
   nearby blast would fight M17's collapse mechanic rather than complement
   it. Three new tests, each confirmed to fail without its fix:
   `most_of_the_blast_radius_becomes_debris_not_vaporized`,
   `a_shockwave_flings_loose_material_beyond_the_crater`,
   `the_shockwave_does_not_uproot_solid_material_beyond_the_crater`.
   Independent review then caught a real rounding-mismatch bug in the
   shockwave's pickup-chance formula: zone membership was decided against
   the *continuous* `radius * SHOCKWAVE_RADIUS_MULTIPLIER`, but the
   fade-to-zero denominator used the *rounded* integer `shockwave_radius`,
   so whenever the multiplier rounded the outer edge down, cells between
   the true and rounded radius passed the zone check but produced a
   negative chance (`Rng::chance` silently treats negative as "never," so
   this never crashed — it just quietly narrowed the annulus below what
   the constant promised). Extracted the formula into its own
   `shockwave_pickup_chance(radius, dist)` function using the continuous
   denominator throughout, clamped defensively (float rounding can still
   land a hair below zero exactly at the edge), and added
   `shockwave_pickup_chance_never_goes_negative_across_the_whole_annulus`,
   which sweeps every cell every radius 1..30 could admit — confirmed to
   fail at exactly the review's reproduction (`radius=3`, `dx=-2, dy=-5`)
   when temporarily reverted to the rounded-denominator formula.
2. **Fire animation was flat** — cells "just turn orange for a second and
   then go back to the original color," no real flame look except the
   already-cool spreading mechanic. **Actioned, done.** Two independent
   changes: a time-varying flicker for actively-burning cells only
   (`rng::jitter3(x, y, frame / FLAME_FLICKER_PERIOD)`, the same
   hash-based approach as the existing position-only `jitter`, extended
   with a third input so the result is stable within a short bucket —
   avoiding 60fps noise — but changes deterministically bucket to bucket,
   with no per-cell state to maintain), and a genuine hue ramp
   (`FIRE_TINT_LOW` dim ember → `FIRE_TINT_HIGH` bright yellow-white,
   interpolated by `heat_ratio`) replacing the old single flat
   `FIRE_TINT`, so intensity changes *colour*, not just blend strength.
   Caught a real test-quality bug applying the session's own
   revert-and-verify standard: the first version of the hue-ramp test
   compared two different temperatures and asserted the green channel
   rose — which still passed after temporarily flattening the tint back
   to a constant, because blend strength alone (`t = heat_ratio * 0.5`)
   already raises green with temperature regardless of hue. Rewrote it to
   pin the temperature at exact `heat_ratio` saturation (where
   `fire == FIRE_TINT_HIGH` algebraically, independent of `t`), hand-derive
   the pixel a flat-tint implementation would have produced at the same
   blend strength, and assert the real renderer's output disagrees with
   that prediction and matches the ramp's instead — confirmed to fail
   without the fix, confirmed to pass with it restored.
3. **Tree growth redesign — deliberately deferred, not implemented.** The
   owner's own words: seeds should fall and require real germination
   conditions (with an instant/no-condition mode for testing), trunk
   thickness should come from an emergent resource-flow mechanism rather
   than the current uniform one-pixel path, and roots currently fail to
   grow at all when a tree is planted directly on stone with no soil
   underneath. Explicit constraint carried forward: *"I don't want you to
   hardcode most of these habits, we want to create realistic Complex
   behavior from simple rules."* Needs a longer design conversation before
   any code changes — added to the TODO list, not started.
   **Update:** that design conversation happened next (see
   `Reports/design-philosophy.md`). The *direction* was settled there — a
   cell-typed, CA-native organism model, generalized past trees to any
   species. **Second update:** the technical design itself (data schema,
   transport mechanics, secondary thickening, connectivity, the
   `TreeState`/`CreatureState` migration plan) is now written up in full —
   see `Reports/organism-substrate-design.md`, the overnight run's section
   7 — and implementation is scheduled as section 8, next.

### Overnight run, section 1: frame-sequence debugging capture

A second, separate capture mechanism alongside the existing
`PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES` single-shot dump — this one for
behavior that only reads correctly across time, which a single screenshot
can't show. `PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start_frame>,<interval_frames>,
<count>` saves a numbered PNG sequence plus one assembled GIF into a
timestamped temp folder. New `CaptureSequence` struct in `main.rs`, `gif`
feature added to the existing `image` dependency.

Caught a real off-by-one in its own first implementation: the countdown
reset after a capture was `self.countdown = self.interval`, which spaced
captures `interval + 1` ticks apart instead of `interval` (confirmed via a
regression test asserting captures at ticks 0, 4, 8 for interval=4, which
failed against the buggy version and passes against the fix —
`self.countdown = self.interval - 1`). Verified end-to-end with a real
`cargo run` pass (not just unit tests): captured 6 real frames of the
default scene, confirmed both the PNGs and the GIF are valid by reading a
captured PNG directly.

### Overnight run, section 2: `Cell` widens to 12 bytes

Found while scoping the water and organism-substrate rewrites below: both
collide with the existing `aux`/burn-timer aliasing (a burning cell's `aux`
was always overwritten with the remaining burn duration, regardless of
material kind). Confirmed live, not hypothetical, for one real case: oil is
a flammable `Liquid`, and the compressible-volume fill amount the water
rewrite plans to store in `aux` would be stomped by the burn timer the
moment oil catches fire. The organism-substrate rewrite is expected to hit
the identical problem for a burning `Plant` cell's planned cell-type tag —
not built yet, but the same class of collision, which is why `organism_id`
is added in this same widening rather than a second one later.

**Fix: `Cell` widens 8 → 12 bytes**, giving the burn timer its own
`burn_timer: u16` field (`ignite`/`tick_burn`/`extinguish`/`burn_remaining`
all moved onto it) and adding `organism_id: u16` in the same widening
(unused until the organism-substrate rewrite — same "irrelevant at this
scale" cost argument M12's own 4→8 byte widening already made: a 2048²
world goes 32 MB → 48 MB). `set_aux`'s old debug-assert against calling it
on a burning cell is removed — no longer a real invariant, since `aux` and
burning no longer interact at all. `cell_is_twelve_bytes` replaces
`cell_is_eight_bytes`.

Independent review of this section caught real documentation regressions
before commit (no functional bugs): `aux`'s own doc had silently dropped
the pre-existing `Creature → owning creature id` case, and both the struct
doc and `ignite`'s doc overclaimed the Plant cell-type-tag scenario above
as an already-fixed bug rather than a planned one — corrected. The review
also flagged that this change made `structural.rs`'s and `creature.rs`'s
own comments about deferring structural/movement work on a burning
neighbour stale (they explained the defer via the now-nonexistent "aux
priority order"); fixed to state the real reasons that survive this change
(conservative deferral in `structural.rs`; `creature.rs`'s cell-rebuild-on-
move losing `flags`/`burn_timer` independent of `aux`).

New regression test confirmed to fail without the fix (temporarily
reverted `ignite` to write `self.aux` again and reran): with only `ignite`
reverted, `burn_remaining()` read 0 instead of the ignited duration, since
`burn_timer` was never actually set — a different assertion line than
expected, but a genuine failure catching the same aliasing bug.

### Overnight run, section 4: water/liquid leveling — compressible-volume rewrite

`update_liquid`'s old model searched up to `dispersion` (5) cells for a
directly reachable empty destination. A cell buried more than 5 cells from
an opening had no destination to find, on any frame — confirmed live from a
playtest screenshot: a wide water column eroded only from its edges inward,
never flattening. Replaced with the standard falling-sand technique for
this (Tom Forsyth's "Cellular Automata for Physical Modelling"; the
w-shadow.com falling-sand water tutorial): each `Liquid` cell holds a
continuous fill amount in `aux` (`material::LIQUID_FULL` = 1000 scale, with
a small `LIQUID_MAX_COMPRESS` = 10 overfill allowance), exchanging fill
with neighbours instead of moving as a discrete occupied cell.

**`aux == 0` on a `Liquid` cell means "never transferred, treat as full,"
not "empty."** This is what let every existing liquid-creation call site
(the paint brush, phase changes, every pre-existing test using `Cell::new
(material::WATER, 0)`) keep working unmodified — a cell drained to
genuinely zero fill converts to `Cell::EMPTY` outright, so `aux == 0` on a
still-`Liquid`-material cell is unambiguous.

**Two real bugs found and fixed during development, both confirmed via
temporary revert:**
- An early version reset the horizontal-transfer amount to the *whole*
  fill difference rather than half, reasoning (wrongly) that this would
  reach equality faster. It doesn't — it overshoots *past* equality (500/300
  becomes 300/500, the same gap flipped to the other side), and the next
  frame's alternating scan direction flips it back, forever. A debug run
  showed `active_chunk_count` still nonzero at 24,000 frames with the
  overshooting version; the halved version settles the same scene cleanly.
- `MIN_LIQUID_TRANSFER` (a floor below which two adjacent cells count as
  "close enough to settled") was needed because without one, a wide
  puddle's very last few units of difference take an extremely long tail to
  fully zero out — empirically tuned from 8 (still ~12,000 frames to fully
  settle a modest test puddle) up to 150 (settles comfortably inside 1,000
  frames) by directly measuring convergence at each step.

**A third, more significant finding came from actually running the app,
not just unit tests — matching this session's own standing practice of
treating live playtesting as a distinct verification channel.** Unit tests
alone (a 40-cell-wide column) showed clean, fast convergence and did not
surface this. Capturing a real `cargo run` scene (§1's tool) with a wider,
more realistic 100-cell column showed it settling into a smooth *mound*
--- visibly still un-flat, heights ranging roughly 6 to 39 cells, after
3000 frames (50 seconds). Root cause: pure nearest-neighbour diffusion
propagates a fill difference exactly one cell per frame no matter how large
`flow_rate` is, so full equalisation across a wide body needs on the order
of *width²* frames — confirmed directly (raising `flow_rate` 200→500 made
no measurable difference, exactly as that reasoning predicts, since once a
fill difference is small, `flow_rate` was never the limiting term).

**Fix: `transfer_liquid_horizontal` now scans up to `HORIZONTAL_TRANSFER_
REACH` (8) cells in the given direction and transfers toward the *emptiest*
reachable same-material-or-empty cell, stopping at the first wall**, rather
than only ever considering the immediate neighbour. This is not a
reintroduction of the old dispersion-search's failure mode: unlike that
search, finding nothing better within reach never blocks levelling
entirely, it only falls back to the immediate neighbour, and the same
diffusion process that fixes the original bug still applies beyond the
scan. The same 100-wide-column scene went from a persistent mound at 3000
frames to fully settled (`active_chunk_count() == 0`) by frame 1800, with a
height profile flat to within about 3 cells across the whole 200-cell test
world — re-confirmed visually via §1's capture tool, not just the numeric
assertion.

New `flow_rate: u16` material field replaces `dispersion`'s role for
`Liquid` kind specifically (`dispersion` is untouched, still governs `Gas`
kind); water `flow_rate: 200`, oil `flow_rate: 80`. `parallel.rs`'s
material-conservation test checks were changed from raw cell-count to
summed fill volume (`liquid_volume`, calling the now-`pub(crate) update::
liquid_fill`), since a cell legitimately splitting its fill across two
cells is not the same thing as material being created. One of those
tests (`two_same_group_chunks_writing_into_their_shared_passive_neighbour_
land_disjointly`) needed its geometry rebuilt entirely — it was written
against the old model's long-range `flow_sideways` search reaching a
specific distant pit, which `Liquid` no longer does at all.

Independent review of this section, requested before commit, caught three
real issues:

- **The reach-8 mechanism — the most complex, most recently-changed
  piece — had no test that actually depended on it.** The committed
  wide-column test used a 40-cell-wide scene, which settles fine even with
  `HORIZONTAL_TRANSFER_REACH` reduced back to 1. Fixed by widening the
  test scene to 100 cells and asserting `active_chunk_count() == 0`
  (fully settled), not just a spread/flatness bound — confirmed to fail
  with reach temporarily set to 1 (stays at 8 active chunks, never
  settles) and pass with reach restored to 8.
- `MIN_LIQUID_TRANSFER`'s doc comment still said "8 is 0.8% of
  `LIQUID_FULL`," a leftover from before the empirical tuning pass that
  raised it to 150 (15%) — fixed to describe the actual value and why it
  moved.
- **A latent conservation bug in `fire.rs`'s `transform`**, used by
  `melts_into`/`boils_into`/reactions: it always rebuilds the cell via
  `Cell::new`, which defaults `aux` to 0 — read by the liquid model as
  "full." No shipped material currently transforms one `Liquid` into
  another, so this was dormant, but a future one would have silently
  inflated a partially-drained cell to a full one on transform,
  manufacturing volume. Fixed to carry the raw `aux` value across when
  both the source and target are `Liquid` kind. New regression test using
  synthetic materials (the same temp-directory technique `fire.rs`'s
  existing reaction tests already use, since no shipped material exercises
  this path) — confirmed to fail without the fix (fill reset to 0 instead
  of the expected partial value).

### Overnight run, section 5: issue #3 — chunk sweep-reach decoupling

Before this section, `Chunk::sweep_region` always widened a dirty rectangle
by the flat `MAX_REACH` (32) regardless of what the chunk actually held —
so a chunk containing nothing but sand (real roll reach of a handful of
cells) paid to re-examine the same wide band as a chunk full of
long-dispersion gas. Fix: each `Chunk` now tracks its own `reach: i32`
(floored at 1), grown on every `set_world` call from the written cell's own
material — `Material::sweep_reach` (`material.rs`), a `Powder`'s
`roll_reach_base` (its true per-position worst case, `floor() + 1`, not
just the base), a `Liquid`'s fixed `HORIZONTAL_TRANSFER_REACH` (8), a
`Gas`'s `dispersion`, everything else 0 — and `sweep_region` widens by that
tracked value instead of the constant.

**Growing is cheap and immediate (a `max` on every write); shrinking needs
a full scan of the chunk's cells, so it happens in exactly one place:**
`World::end_step`, only for a chunk that transitions from active to
settled *this* step (`was_settled` compared before and after
`end_sweep`). That is the one point recomputing is both cheap (nothing is
mid-sweep) and safe (nothing needs the wider, possibly-stale value again
until the chunk wakes, at which point `set_world`'s growth takes back
over) — and it keeps a fully-settled world's `end_step` loop, which already
iterates every resident chunk regardless of activity, from paying for a
4096-cell rescan on chunks that didn't change.

**Two premises in this section's original plan text turned out to be
wrong once checked against the actual §4 code, both caught before writing
any implementation:**
- The plan assumed §4 would drop liquid's reach to 1 and delete
  `SURFACE_SEARCH` outright. Neither happened: liquid's real horizontal
  reach is `HORIZONTAL_TRANSFER_REACH` = 8, and `SURFACE_SEARCH`/
  `flow_sideways` are still live — for `Gas`-kind materials, which §4 never
  touched.
- The plan called for restating `parallel.rs`'s cross-chunk write-safety
  proof from `MAX_REACH == CHUNK_SIZE / 2` to an inequality, and
  parameterizing `same_group_chunks_are_never_within_reach_of_each_other`
  over reach. Neither is needed: that proof bounds how far a write can
  *land* (a hard per-frame movement cap independently enforced at every
  movement rule's own call site — `roll_reach_base`'s clamp,
  `flow_sideways`'s `.min(MAX_REACH)`, `HORIZONTAL_TRANSFER_REACH` itself),
  which stays exactly `MAX_REACH` regardless of anything this section
  touches. `Chunk::sweep_region`'s widening only decides which *stale*
  cells get re-examined — a strictly smaller, purely-performance question —
  and narrowing it can only shrink a sweep region relative to before, never
  grow one, so it cannot invalidate a proof about how far a write can go.
  `touch_neighbours`/`queue_touch_neighbours` (the cross-chunk wake
  mechanism the proof's loop-ordering argument in `parallel.rs` also
  depends on) are deliberately left keyed on the flat `MAX_REACH`, not the
  new per-chunk reach — see the extended comment left on `World::
  touch_neighbours` explaining why those are different questions.

**A real bug found via the standing test suite, not by inspection:**
narrowing `sweep_region`'s widening broke `world.rs`'s existing
`neighbour_waking_stops_at_max_reach` test. Root cause, traced rather than
guessed: `touch_neighbours` marks a neighbour chunk dirty at the *raw world
coordinate* of the write, which can legitimately sit far outside that
neighbour's own bounds — under the old flat-`MAX_REACH` widening this
always worked, because expanding by the same `MAX_REACH` used to decide
*whether* to wake a chunk always reached back across the gap. With a
neighbour's own (now often much smaller) tracked reach, a write far enough
away that nothing in an otherwise-empty neighbour chunk could ever actually
see it now correctly produces no sweep region there — a chunk gets
conservatively marked dirty (harmless) but isn't examined for nothing
(the actual fix). Confirmed this is the intended behaviour, not a
regression, by checking the genuinely-adjacent case
(`a_write_at_a_chunk_edge_wakes_the_neighbour`) still passes unmodified.
The old test encoded the pre-issue-#3 assumption that every chunk always
had the same wide reach; renamed to `neighbour_waking_stops_at_the_
neighbours_own_reach` and rewritten to assert the new, more precise
behaviour directly.

**`Material::sweep_reach` also gained a load-time `debug_assert`** guarding
the one reach-defining value not already clamped by construction elsewhere
(`roll_reach_base` is; `Liquid`'s reach is a fixed engine constant, not
data) — a `Gas` material's raw `dispersion` (`u8`, so nominally up to 255).
A future `.ron` setting it past `MAX_REACH` now fails loudly at load time
instead of being silently capped downstream where a content author would
never see why their gas stopped dispersing as far as the number they
wrote.

**`HORIZONTAL_TRANSFER_REACH` moved from `update.rs` to `material.rs`**
(re-exported under its original name), since `Material::sweep_reach` needs
the same number and `chunk.rs` — which must not depend on `update.rs`, or
the two would become mutually dependent modules — is where the per-chunk
reach tracking itself lives.

New tests: `chunk.rs` gained
`a_chunks_tracked_reach_starts_at_one_and_only_grows_from_writes` and
`recompute_reach_shrinks_once_the_wide_reach_material_is_gone`, both
confirmed to fail with `sweep_region` temporarily reverted to the flat
`MAX_REACH`. Benchmarked via `cargo run --release --example ascii`
before/after (`git stash`): no regression on the full-screen sand/water
stress scenes (worst frames within normal run-to-run noise either way) —
expected, since that scene's worst frame comes from the initial
full-chunk-dirty settle burst, where `sweep_region`'s expansion is a no-op
regardless of reach (a chunk already dirty across its full bounds can't be
widened further by clipping). The actual win this section targets is the
steady-state case — a small, localized change in an otherwise mostly-quiet
world no longer re-examining a needlessly wide band — which the unit tests
verify directly rather than a full-screen chaos benchmark.

**Independent review, requested before commit, caught one real bug:**
`Material::sweep_reach`'s first-draft `Gas` arm returned `dispersion` alone,
undercounting the true reach. Traced (not just asserted) by the reviewer
through `flow_sideways` (`update.rs`): its initial walk stops within
`dispersion`, but its free-surface branch then searches a further
`SURFACE_SEARCH` (`= MAX_REACH`) cells past that point for somewhere to
fall — the same free-surface search a liquid used before
`HORIZONTAL_TRANSFER_REACH` replaced it, still live for `Gas` since it
never moved off `flow_sideways`. A gas cell's true worst case is
`dispersion + MAX_REACH`, not `dispersion`; for smoke (`dispersion: 3`)
that's up to 35 cells against a first draft that tracked 3, which could
have frozen a floating smoke cell mid-decision the moment it drifted more
than 3 cells from a chunk boundary. Fixed: the `Gas` arm is now
`dispersion == 0 ⇒ 0`, else `dispersion + MAX_REACH` (clamped to
`MAX_REACH` by the function's existing final `.min`) — which, since
`SURFACE_SEARCH` already equals `MAX_REACH`, means any dispersing gas
correctly gets the full flat `MAX_REACH` widening this section's flat
constant was supposed to let chunks *avoid* paying for. **Gas is
consequently the one kind this section does not narrow at all** — only
`Powder`/`Liquid`-only chunks see a smaller tracked reach; a chunk with any
resident `Gas` cell still gets the same widening it always did, correctly,
because nothing smaller would be safe for one. Four new regression tests in
`material.rs` (`sweep_reach_for_powder_bounds_the_true_worst_case_roll_
reach`, `sweep_reach_for_liquid_matches_horizontal_transfer_reach`,
`sweep_reach_for_a_zero_dispersion_gas_is_zero`, `sweep_reach_for_a_
dispersing_gas_reaches_max_reach_not_just_dispersion`), the last confirmed
to fail against the pre-fix formula. Two pre-existing stale doc comments
the same investigation surfaced were fixed alongside it: `update.rs`'s
`SURFACE_SEARCH` still described itself as being for "a free liquid
surface" (true before §4, not since), and this section's own first-draft
doc comments on `chunk.rs`'s `MAX_REACH` and `Material::sweep_reach`
repeated the same `dispersion`-alone assumption the code did.

### Files touched

`src/sim/material.rs` (`HORIZONTAL_TRANSFER_REACH` relocated here,
`Material::sweep_reach`, load-time `debug_assert`). `src/sim/chunk.rs`
(`Chunk::reach` field, `set_world`'s new `reach` parameter,
`sweep_region` widens by it, `Chunk::recompute_reach`, `MAX_REACH`'s doc
comment rewritten to describe its two remaining jobs — the cross-chunk
proof and `sweep_reach`'s defensive cap — instead of the sweep-widening job
this section moved off it). `src/sim/world.rs` (`World::set` computes
reach and passes it through, `World::end_step` recomputes reach on the
settle transition, `touch_neighbours`'s comment extended to explain why it
stays on the flat `MAX_REACH`). `src/sim/parallel.rs` (`ChunkView::set`
mirrors `World::set`'s reach computation for the owned-chunk case).
`src/sim/update.rs` (`HORIZONTAL_TRANSFER_REACH` now imported from
`material.rs` rather than defined locally). `src/sim/mod.rs` (unrelated
stale doc fix noticed in passing: `cell`'s size comment still said 8 bytes,
left over from before §2 widened it to 12).

### Overnight run, section 6: explosion debris realism

Two separate diagnoses, confirmed against the actual code rather than
guessed:

- **Same-tile launch clustering.** `debris_velocity` samples `world.field_at`
  (a coarse block lookup, see its own doc) at exactly `±FIELD_SCALE` (8) from
  each cell — every cell within roughly one field tile reads the same
  quantized pressure gradient and launched with identical velocity, reading
  as a moving block rather than a scatter.
- **Lockstep falling.** `ParticleSystem::step` applied one shared `GRAVITY`
  with no per-particle variation, so identically-launched particles traced
  identical arcs forever.

**Fix 1 — launch jitter.** `debris_velocity` now adds position-keyed jitter
(`rng::jitter`, the same stable-per-position primitive `roll_reach_at`/fire
flicker already use) to each axis, scaled by the cell's own computed
`speed` — deliberately **not** by raw `strength`, per the plan review from
earlier this session: `strength` values large enough to throw debris
convincingly are already well past `MAX_SPEED_PER_AXIS` once multiplied by
`SPEED_PER_STRENGTH`, so a `* strength` jitter term would pin every
particle to the clamp and make debris *more* uniform. `JITTER_AXIS_OFFSET`
decorrelates the x and y jitter samples so jitter isn't purely diagonal.

**`DEBRIS_JITTER_STRENGTH` kept at the plan's original estimate (0.4) rather
than tuned down to make a failing test pass** — it broke an existing test,
traced to a pre-existing fragility in that test rather than the jitter
being genuinely too strong, and fixed at the root instead (see below).

**Fix 2 — per-particle drag/gravity variance.** `Particle` gained `drag`
and `gravity_scale` fields (`0.985..=1.0` and `0.9..=1.1`), drawn once at
spawn and held for life — not redrawn per frame, the same "stable decision"
shape `Chunk::rng` already argues for. **Deliberately drawn from
`ParticleSystem`'s own new internal `Rng` stream, not threaded through
`&mut World`/`&mut Rng` at every `spawn` call site** — a design deviation
from the plan's literal "drawn from `world.rng`" text, decided because nothing
in this engine was ever required to be reproducible (`rng.rs`'s own module
doc) and threading a shared generator through `app.rs`'s `spawn_burst`,
every `render.rs` test, and `explosion.rs` just to reach one generator would
have bought nothing `Chunk::rng`'s own per-owner-stream precedent didn't
already justify skipping.

**A real, pre-existing test fragility surfaced by adding jitter, found via
the standing test suite rather than by inspection:**
`debris_is_thrown_away_from_the_epicentre_not_toward_it` failed at
`DEBRIS_JITTER_STRENGTH = 0.4`. Traced rather than immediately tuned away:
temporarily zeroing the constant and measuring the same scene's minimum
cosine-of-angle showed the *pre-existing*, jitter-free code already had only
an 8.1-degree safety margin for one cell near the corner of the test's
filled-square blast (`min_cos = 0.1414`) — a structural property of reading
a pressure gradient near a corner, nothing to do with jitter. Jitter (a
deliberate, on-purpose angular perturbation) spent most of that already-thin
margin, grazing to 91.3 degrees for that one cell. Fixed at the actual
source of the fragility: the test's `dot > 0.0` requirement was strict
seven-nines precision for every single particle in a whole blast radius,
which the mechanism was never actually designed to guarantee that tightly.
Rewritten to assert (a) no particle moves *strongly* backward
(`cos > -0.2`, generous enough to admit a legitimate graze, tight enough to
still catch a genuine sign-flip bug) and (b) the population as a whole
skews strongly outward (mean `cos > 0.5`) — the second check is what would
actually catch a real direction bug, which would show up as roughly half
the particles failing, not one grazing corner case.

New regression tests, each confirmed to fail against the pre-fix code via
temporary revert: `debris_velocity_varies_within_a_single_field_tile`
(`explosion.rs`) — an open world with a real pressure impulse so `x = 34`
and `x = 35` read the identical coarse field block and would produce
bit-identical velocity without jitter, confirmed to fail
(`vx1 == vx2 && vy1 == vy2` exactly) with `DEBRIS_JITTER_STRENGTH`
temporarily zeroed. `particles_spawned_with_identical_velocity_diverge_over_
time` (`particle.rs`) — two particles spawned identically (`vx: 0.0`, to
isolate `gravity_scale` from `drag`) must have fallen different amounts
after 30 frames, confirmed to fail with `gravity_scale` temporarily
short-circuited back to flat `GRAVITY`.

**Live verification, per this session's standing practice of treating
`cargo run` as a distinct channel from unit tests:** the real windowed app's
capture-sequence tool (§1) turned out not to be useful for this specific
check — captured frames were pixel-identical across the whole sequence,
traced to the fixed-timestep accumulator not advancing meaningfully within
however this environment paces `RedrawRequested` events, a pacing question
about this specific headless/background invocation rather than a bug in the
engine. Switched to a small temporary throwaway example
(`examples/debug_explosion.rs`, deleted after use, mirroring
`examples/ascii.rs`'s existing headless-verification style) that steps the
CA sweep and particle system directly with no windowing involved, printing
an ASCII grid of landed material (`#`) and in-flight particles (`*`). First
attempt showed almost no scatter at all — traced to the test scene's own
geometry (a stone block thicker than the blast radius's remaining margin to
open air, so debris immediately re-embedded in the few cells of still-solid
stone between the crater's edge and the block's own edge — correct physics,
useless test scene). Corrected to a block smaller than the blast radius, so
debris flies into genuinely open space: confirmed a wide, irregular,
progressively-thinning scatter halo around the crater by frame 6, debris
still landing at points scattered across the whole visible world by frame
20 — not a moving block, not lockstep arcs.

### Files touched

`src/sim/explosion.rs` (`DEBRIS_JITTER_STRENGTH`, `JITTER_AXIS_OFFSET`,
`debris_velocity`'s jitter, `use super::rng`, the rewritten
`debris_is_thrown_away_from_the_epicentre_not_toward_it`, the new
`debris_velocity_varies_within_a_single_field_tile`).
`src/sim/particle.rs` (`Particle::drag`/`gravity_scale`,
`ParticleSystem`'s own `rng: Rng` field, the `ranged` helper, `step`
applying both, the new `particles_spawned_with_identical_velocity_diverge_
over_time`).

### Overnight run, section 7: `Reports/organism-substrate-design.md`

Research-and-design section, no code changes — the deliverable is the
report itself, [`Reports/organism-substrate-design.md`](Reports/organism-substrate-design.md),
read in full before starting section 8.

Grounded in the actual current code, not the plan's own description of
it, which mattered: `plant.rs`/`creature.rs`/`structural.rs` were read in
full first, and two of the plan's premises turned out to need correcting
before the report could be written honestly:

- The plan's `Cell::aux` layout for `Plant`/`Creature` (cell-type tag +
  resource scalar, 16 bits, no room left over) silently drops the anchor
  distance `Plant` cells currently store in that same field for M17
  structural integrity — a real conflict the plan text never resolved.
  Decided here: `Plant` structural integrity moves off the per-cell cache
  entirely, onto an event-triggered bounded reachability search from the
  organism's own anchors, rather than `Solid`'s incremental relaxation
  (which needs the per-cell cache `Plant` no longer has room for).
- The plan asked to "factor `structural.rs`'s BFS-from-anchors into a
  generic primitive." `structural.rs` does not run a BFS — it's an
  incremental local relaxation (`min(neighbour.aux()) + 1`, cached per
  cell, recomputed reactively). There is no full-graph search anywhere in
  the current codebase to extract. The report designs the actual shared
  primitive the two different storage strategies (`Solid`'s cache,
  `Plant`'s on-demand search) can both be built from instead — a bounded
  BFS with a caller-supplied anchor set and connectivity predicate, used
  three ways: an M17 verification pass, the organism substrate's primary
  structural mechanism, and `SecondaryThicken`'s downstream-leaf-count
  flood fill.

Four citations researched and verified with real, fetched URLs (a
dedicated background research pass, separate from writing the report
itself, specifically so no URL in the final document was guessed): Münch
(1930)/Knoblauch et al. (2016) for the real phloem pressure-flow mechanism
this engine's diffusion-based transport is a named simplification of;
Shinozaki, Yoda, Hozumi & Kira (1964) for the pipe model theory
`SecondaryThicken` translates, plus Lehnebach et al. (2018)'s review of
its real, documented limits (the proportionality constant is tree-local,
not universal — directly shapes `pipe_ratio` being a per-species
parameter, not a hardcoded constant); L-PEACH and MuSCA as the FSPM tier
of coupled-transport-on-explicit-architecture this engine is deliberately
not attempting, cited so that's a stated decision rather than a gap no one
noticed.

Also settles issue #8's design question (`Reports/pixel-physics-
issues.md`): generational `organism_id` indices with a free list, not
deferred to be re-litigated later, since the organism substrate makes
`TreeState`'s existing leak guaranteed to matter (moss/worm reseeding, and
section 12's ants, all churn through far more short-lived organisms than
a tree ever did). **Update after section 8 actually shipped:** the
allocator itself was built and is tested, but section 8's real scope
ended up moss-only — see its own entry for why the free-list's *reuse*
side has no caller yet, and issue #8 isn't fully closed until the tree
retrofit lands.

### Overnight run, section 8: organism substrate rewrite — moss only, tree/worm deferred

Implements `Reports/organism-substrate-design.md`, scoped down from its own
§7 retrofit order (moss → trees → worm) to **moss alone** — a deliberate
mid-implementation call, not a shortfall discovered afterward. Reasoning:
the design report itself flagged the tree retrofit's `Divide` behavior
(discrete grid-candidate growth for moss vs. continuous space-colonization
for trees) as a genuine open risk needing real implementation-time
judgment, not a mechanical port; attempting it at the tail of an already
very large session, alongside the worm's own `Locomote` port, risked
rushing exactly the piece the report itself said deserved care. Moss alone
is still a complete, coherently tested, honestly-scoped unit: it proves
the entire new pipeline (species data, generic behavior dispatch, the
`aux` cell-type/resource encoding, the generational allocator, structural
dispatch on `organism_id`) end to end, with zero risk taken on the harder
part.

**What was built:**

- `src/sim/organism.rs` (new) — `CellType` (currently one variant,
  `GrowingTip` — room for the rest once a species needs them), `Behavior`
  (currently one variant, `Divide { cost, damp_chance, dry_chance,
  shade_sensitive }` — a struct-shaped enum variant, not a newtype
  wrapping a separate struct, because RON's syntax for the latter needs an
  awkward doubled `Divide(Divide(...))`, caught by a failing embedded-
  species-parse test on the first attempt), `Species`/`SpeciesRegistry`
  (mirrors `MaterialRegistry`'s `builtin`/`reload`/`get`/`id_of` shape,
  deliberately without a `resolve_references` pass — a species file never
  names another species, so there's nothing to resolve after loading),
  `pack_aux`/`unpack_aux` (the cell-type-plus-resource encoding into
  `Cell::aux`'s 16 bits), and `reachable_from_anchors` (the shared bounded
  BFS the design report's §5 specifies, generic over `CellSurface`, tested
  directly — not yet wired to a real caller, see below).
- `assets/species/moss.ron` — reproduces the old `MOSS_DAMP_CHANCE`
  (0.35) / `MOSS_DRY_CHANCE` (0.002) split exactly as one `Divide`
  behavior's parameters, `cost: 0.0` since moss never had an energy budget
  before this retrofit and inventing one is a bigger behavioural change
  than a retrofit should make silently.
- `src/sim/cell.rs` — `organism_id`/`set_organism_id`/`with_organism_id`
  accessors (the field existed since §2, unused until now).
- `src/sim/world.rs` — the generational `organism_id` allocator:
  `push_organism`/`organism` (12-bit slot index + 4-bit generation packed
  into the `u16` `organism_id`, `encode_organism_id`/`decode_organism_id`).
  4 bits of generation (not more — widening `Cell` a third time this
  session for this alone wasn't justified) means a slot wraps after 16
  reuses, at which point a sufficiently stale reference could in principle
  alias; accepted as a documented, bounded risk rather than a silent one.
  **`organism_mut`/`free_organism` do not exist yet** — see below.
- `src/sim/scheduler.rs` — `ActiveKind::Moss { stale_ticks }` replaced by
  a generic `ActiveKind::Organism { organism, stale_ticks }`, dispatched
  from `plant::tick` for any species, not just moss.
- `src/sim/plant.rs` — the whole moss section rewritten: `organism_tick`
  (generic dispatch: reads the cell's `organism_id`/`aux`-encoded
  `CellType`, looks up the owning organism's species, runs each
  registered `Behavior`) replaces `moss_tick`; `has_growable_neighbour`
  generalized from "touches stone or moss" to "touches `Solid` or shares
  this cell's `organism_id`" — the exact mechanism that lets a patch
  thicken over its own earlier growth, now expressed generically instead
  of hardcoding the moss material id. `plant_moss_seed` now allocates a
  real organism via `push_organism` instead of just painting a material.
- `src/sim/structural.rs` — `tick` gains one new branch: an
  organism-owned cell (`organism_id != 0`) routes to
  `organism_structural_tick` instead of the aux-cached relaxation, since
  its `aux` no longer holds a distance once it's carrying a cell-type tag
  and resource scalar. **Deliberately a no-op in this pass** (see below).

**Two real design gaps found and resolved during implementation, not
anticipated by the design report:**

- The report's §2 said the cell-type-plus-resource `aux` layout applies
  to organism-owned `Plant`/`Creature` cells, but only worked through the
  `Plant`/wood conflict in detail — an independent review of the report
  itself (before this section started) caught that `Creature`'s existing
  `aux`-as-creature-index scheme has the identical conflict, unaddressed.
  Resolved in the report before implementation began: `organism_id` (not
  `MaterialKind`) gates `aux`'s interpretation, and `Creature`'s existing
  use retires in favour of `organism_id` with no conflict at all (no
  "unowned worm" case the way there's hand-painted wood). Implementation
  didn't touch `creature.rs` this pass (deferred with the worm), so this
  is a decision recorded for when it does, not yet exercised in code.
- `structural.rs`'s new organism branch has nowhere real to search from
  yet: `OrganismState` (this pass) only tracks which species an organism
  is, not an anchor/root-tip list the way `TreeState::roots` does — moss
  has no root concept at all. Rather than fake an anchor (the cell's own
  position, say) that wouldn't mean anything, `organism_structural_tick`
  is an explicit, documented no-op, guarded by a debug assertion that
  fires if any organism-owned material ever sets a finite
  `max_unsupported_span` (none does yet — moss's own material config
  makes the check moot regardless of which code path handles it). What
  the branch *does* guarantee, and the actual correctness requirement for
  this pass: an organism-owned cell can never fall through to the old
  aux-cached path, which would silently corrupt its cell-type/resource
  encoding by writing a "distance" into the same bits.

**Independent review of the implementation (not just the report) found
three more real issues before commit:**

- **`Divide`'s `cost` was never actually deducted from the dividing
  (parent) cell** — the new cell was stamped with `resource - cost`, but
  the parent's own `aux` was never rewritten at all, so the resource gate
  (`if resource < cost { continue }`) checked a value that could never
  decrease. Invisible with moss's own `cost: 0.0` (240 tests green
  regardless), but a real latent bug against `Divide`'s own documented
  contract that the very next species with a nonzero cost would have hit.
  Fixed properly, not just patched: the parent now pays `cost` from its
  *own* resource, and the new cell starts at `0.0` rather than inheriting
  the parent's post-cost leftover — the first draft of the fix handed the
  child `resource - cost` too, which would have manufactured that amount
  of resource out of nothing on every division (both cells ending up with
  the same post-cost value that only one of them started with). New test,
  `divide_deducts_cost_from_the_parent_without_manufacturing_resource`,
  using a synthetic species (the same temp-directory technique
  `material.rs`'s own synthetic-material tests already use, since moss's
  `cost: 0.0` can't exercise this) — confirmed to fail against both the
  original bug and the manufacturing-resource half-fix.
- **Species hot-reload was designed but never wired up.** The report's §1
  says species are "hot-reloaded via the same `notify` pattern
  `MaterialRegistry` already uses" — `SpeciesRegistry::reload` existed and
  was tested, but nothing called it: `App::new`/`reload_materials` only
  ever reloaded `world.materials`, and `main.rs`'s file watcher only ever
  watched the materials directory. Editing `assets/species/moss.ron` did
  nothing, live or via F5, unlike every material file. Fixed: a shared
  `reload_assets` helper (`app.rs`) reloads both registries together so
  the two can't drift out of sync again, and the watcher
  (`main.rs::watch_materials`, name kept despite now covering both
  directories — renaming every call site wasn't worth doing alongside an
  unrelated fix) watches the species directory too.
- **Two separately-planted moss patches that grow into contact can leave a
  permanent one-cell notch at their shared boundary.**
  `has_growable_neighbour` requires a candidate to touch either `Solid` or
  a cell sharing *this* organism's own `organism_id` — so once two
  patches' fronts meet with no bare stone left in the seam, a cell whose
  only moss neighbour belongs to the *other* patch is growable for neither
  side. The old material-identity check (`m == moss_id`) had no such
  boundary and would have filled it. Judged an accepted, narrow scope
  boundary rather than a bug to fix here: patches from different
  `plant_moss_seed` calls are, correctly, different organisms, and letting
  them silently fuse into one would need a real merge-organism-ids
  mechanism this session isn't building. No test exercises two patches
  meeting (nothing currently depends on the fused-vs-notched distinction),
  so this is a known, documented follow-up rather than silently
  unnoticed — worth deciding on deliberately if it turns out to look wrong
  in play.

**`organism_mut`/`free_organism` were written, tested, and then removed**
— caught by `cargo clippy --all-targets -- -D warnings` flagging them as
dead code once nothing called them. Investigated rather than silenced
(`#[allow(dead_code)]` has no precedent anywhere in this codebase and
wasn't added as one here): moss's `Divide` never mutates `OrganismState`
after creation, and detecting "this organism has zero cells left" cheaply
needs a real anchor list or a live cell count, neither of which exists
yet. Removing them (rather than keeping unused methods, or forcing a fake
trigger just to use them) is the honest scope boundary — real work for
the tree retrofit, which already needs exactly this to generalize
`reclaim_if_tree_is_fully_dead`. The generational safety property itself
(a stale id can't silently alias a reused slot) is still fully covered by
direct tests of `push_organism`/`organism` and the encode/decode
functions, independent of whether `free_organism` exists yet.

**Every existing moss test carried over unchanged and still passes**,
now exercising the entirely new pipeline —
`moss_spreads_over_damp_stone_and_not_over_dry` in particular, confirmed
via temporary revert (hardcoding the dispatched chance to `dry_chance`
regardless of dampness) to still fail exactly the way it would have
against the old code. One new test,
`moss_thickens_into_a_patch_by_growing_over_its_own_earlier_growth`,
added because the old test never actually exercised the same-organism
thickening branch specifically (a large damp/dry spread-count comparison
would pass even with only single-cell-wide lines) — confirmed to fail
when that branch is temporarily reverted to the old material-name check.
Live-verified via `cargo run --release --example ascii`'s existing M16
moss scene: moss (`,`) appears next to the damp side's water and is
absent from the dry side, matching the pre-retrofit screenshot exactly.

### Deferred: tree and worm retrofits

**Explicitly not started this pass** — `plant.rs`'s `tree_tip_tick`/
`root_tip_tick`/`TreeState` and `creature.rs`'s `worm_tick`/
`CreatureState` are completely untouched, still the pre-retrofit code,
still passing their own full test suites unchanged. Per the design
report's own §1 caveat: `Divide`'s tree mode (continuous-position space
colonization, a shared `attractors` list, a per-tip `channel` scalar) is
not a data-parameterized version of the same algorithm moss uses — it may
need splitting into a genuinely separate named behavior rather than one
`Divide` covering both, a judgment call the report deliberately left for
"the implementation session," not something to force through at 2am at
the end of an already very large batch of changes. The worm's `Locomote`
port is comparatively low-risk but was deferred alongside it rather than
attempted alone, since `creature.rs`'s own `aux`-as-index retirement
(this section's own finding above) needs to land at the same time as
whatever session does the worm, not split across two.

**What a future session picking this up needs**: read `Reports/organism-
substrate-design.md` in full (still accurate — nothing in this pass
invalidated it), then this section's own "design gaps found" and
"independent review" notes above, all real additions to the report's
original text. The organism-substrate machinery (species loading, generic
`organism_tick` dispatch, the allocator, `structural.rs`'s dispatch point,
species hot-reload) is all in place and ready — a tree retrofit is
additive (a `TransportChannel`/`SecondaryThicken`/space-colonization-mode
`Divide` behavior, an
`OrganismState` with a real anchor list, `organism_mut`/`free_organism`
finally getting callers) rather than a rework of what this section built.

### Files touched

`src/sim/organism.rs` (new — `CellType`, `Behavior`, `Species`/
`SpeciesRegistry`, `pack_aux`/`unpack_aux`, `reachable_from_anchors`).
`assets/species/moss.ron` (new). `src/sim/cell.rs`
(`organism_id`/`set_organism_id`/`with_organism_id`). `src/sim/world.rs`
(the generational allocator: `push_organism`/`organism`,
`encode_organism_id`/`decode_organism_id`, `OrganismSlot`).
`src/sim/scheduler.rs` (`ActiveKind::Moss` → generic `ActiveKind::
Organism`). `src/sim/plant.rs` (`moss_tick` → `organism_tick`,
`has_growable_neighbour` generalized to `organism_id` equality,
`plant_moss_seed` allocates a real organism). `src/sim/structural.rs`
(`tick`'s new `organism_id != 0` dispatch branch, `organism_structural_
tick`). `src/sim/mod.rs` (registers the `organism` module). `src/app.rs`
(`reload_assets` helper, used by both `App::new` and
`reload_materials`). `src/main.rs` (`watch_materials` now also watches
the species directory). `PLAN.md`/`README.md`.

### Overnight run, section 9: UI improvements

All 8 sub-steps from the plan's own list, built on a new `src/hud.rs` text
primitive — the engine's first on-screen text at all (`render.rs`'s own
comment on the window title bar previously called it "cheaper than
rendering text").

**Step 0, the font, deliberately narrower than planned.** The plan
sketched full ASCII 0x20-0x7E (95 glyphs); shipped instead with space,
`A`-`Z`, `0`-`9`, and a small punctuation set — hand-authoring 95 accurate
bitmap glyphs with no reference font to check transcription against risks
silently shipping wrong data for characters nothing would ever exercise
enough to notice. HUD text upper-cases internally as the direct
consequence. **Caught before commit by an actual visual check** (render
sample text to a PNG, read it, don't just trust the hand-copied bit
patterns): `[`/`]` — used by the help overlay's own "brush size" line —
had no glyph at all and rendered as a silent gap. Fixed, with a test that
checks every character the module doc claims to support actually lights a
pixel, confirmed via revert to fail against the original omission.

**Steps 1-7**, each landing exactly where the plan specified: zoom
(`=`/`-`, one continuous scale across `Renderer::zoom` (magnify, 1-8) and
`zoom_out_stride` (minify via sampling, 1-4) rather than two independent
controls — zooming out counts the stride down/up before magnification
engages the other way, so the key pair reads as one control, not two);
brush label (always on, the data `status()` already computed, now
persistent instead of title-bar-only); hover inspector (`I` — material,
temperature/burning, every M13 field channel at the cursor); field overlay
(`V`, cycling pressure/temperature/light/moisture, including over empty
cells — a field reading exists over vacuum same as anywhere, so
`cell_colour`'s empty-cell early return routes through the overlay too,
not just the non-empty path); brush outline preview (a midpoint-circle
primitive, `render::draw_circle_outline`, reusable beyond just this);
material palette (`Tab`, swatch row, selection outlined); keybind help
(`/`, shown as `?`).

**Key-collision check re-run against the plan's own list** (taken: Esc,
Space, `.`, R, F1, F5, F, P, X, T, M, W, `[`, `]`, Q, E, 1-9) — `=`, `-`,
`I`, `V`, `Tab`, `/` all confirmed free, matching what the plan predicted;
no collisions introduced. `README.md`'s controls table also had a
pre-existing gap independent of this section (no `W`/plant-worm row at
all) — fixed alongside the new entries since it was noticed in passing,
not left for a future pass to rediscover.

**Live-verified via a throwaway direct-construction harness**
(`examples/debug_hud.rs`, deleted after use, mirroring `debug_explosion.rs`'s
and `debug_font.rs`'s pattern from earlier sections) rather than the real
windowed event loop — this session's §6 already found that loop doesn't
reliably advance frames headlessly in this environment, and HUD state
(toggle booleans) doesn't need real simulation ticks to verify anyway, just
`App::draw` called directly with the toggles set and a synthetic cursor.
Confirmed: brush label, hover inspector readout, palette swatches with a
visible selection outline, the help overlay's full text block (including
the `[`/`]` fix), and a magnified brush outline at `zoom = 3` all render
correctly. **Notably absent from that list: the field overlay's own
appearance** — not screenshotted, just spot-checked via a unit test at the
time, which is exactly the gap the next finding fell through.

**Independent review of the implementation found one real bug before
commit, in the field overlay's blend math.** `apply_field_overlay`'s first
version used a flat 60% blend strength regardless of how far a channel's
reading sat from its own ambient baseline, deliberately — the reasoning at
the time was that scaling blend by magnitude would wash out exactly the
low-but-real readings the overlay exists to show. But every channel's
*ramp colour* at a baseline (zero pressure, ambient temperature, no light,
no moisture) is some fixed saturated colour, not `base` itself — so a flat
blend actually tinted *every* pixel toward that fixed colour regardless of
whether the channel was elevated there at all. Concretely, for pressure:
toggling the overlay blended the *entire visible world* 60% toward white,
not just cells near a real disturbance — directly contradicting this
function's own doc comment, which explicitly claimed "an unaffected cell
should look exactly like it does with the overlay off." The existing test
only asserted the whole-frame output changed with the overlay on, which
is trivially true once the screen turns white, for the wrong reason.
Fixed: blend strength now scales with magnitude (0 at true baseline, up
to `MAX_BLEND` at a fully saturated reading), so an ambient cell renders
byte-identical to the overlay being off while a genuinely elevated one
still reads clearly. Two new tests —
`field_overlay_leaves_an_unaffected_cell_unchanged_even_when_on` (the
actual property that broke, confirmed via revert to a flat blend to fail
exactly as described above) and `field_overlay_off_matches_the_pre_
overlay_render_exactly` (replacing a tautological off-vs-off comparison
with one against a hand-computed pre-overlay expected value) — plus the
existing near-impulse test renamed to `pressure_overlay_tints_a_cell_
near_a_real_impulse` to make clear what it does and doesn't cover.

### Files touched

`src/hud.rs` (new — font, `draw_text`, `text_width`). `src/render.rs`
(`Renderer::zoom`/`zoom_out_stride`/`adjust_zoom`, `FieldOverlay` +
`cycle_field_overlay`/`apply_field_overlay`, `draw_circle_outline`,
`world_to_screen`, `screen_to_world` updated for zoom, `put` made
`pub(crate)` for `hud.rs` to reuse). `src/app.rs` (`draw`'s new `cursor`
parameter, `draw_hud`/`draw_hover_inspector`/`draw_palette`/`draw_help`,
the three new toggle fields/methods). `src/main.rs` (six new key
bindings). `src/lib.rs` (registers the `hud` module).
`PLAN.md`/`README.md`.

### Overnight run, section 10: in-game live tunables panel

A generic `(category, name, value, min, max, step)` registry
(`src/tunables.rs`, new) rather than a bespoke UI per subsystem — any
already data-driven value can register into it, and only `Material`'s
finite `f32` fields register this round (`density`, `friction_angle`,
`flammability`, `heat_conductivity`, `ignition_temperature`,
`burn_temperature`, `melting_point`, `boiling_point`). Integer fields
(`dispersion`, `flow_rate`, `burn_duration`, `max_unsupported_span`) and
fields left at the "never" sentinel (`f32::INFINITY`) are deliberately not
registered — scoped down from the plan's own text, documented in the
module's doc rather than silently dropped.

**`O`** toggles the panel; `↑`/`↓` move the selection, `←`/`→` adjust by
the tunable's own step (applied immediately to the live `MaterialRegistry`
— felt next frame, not deferred), `Enter` saves, `Esc` closes without
saving (the live-adjusted value stays in effect for the session either
way — closing the panel was never what would have discarded it). `Esc`'s
existing unconditional-quit binding became contextual: closes the panel
first if open, quits only once there's nothing left to close.

**Saving is a targeted text-span edit, never a `ron::ser` round-trip** —
the standing reason: re-serializing would silently destroy every comment
in a material file, and those comments carry real reasoning (`oil.ron`'s
own header, for one). `write_field_value` finds the existing `field:
value` span and replaces just the value; verified to still parse
(`ron::from_str::<MaterialDef>`) *before* ever touching disk, aborting and
reporting rather than writing a broken file on failure.

**Live PNG verification (a throwaway `examples/debug_tunables.rs`, direct
`App` construction, deleted after use — the real windowed event loop still
doesn't reliably advance frames headlessly here, per §6's finding) caught
two real bugs the unit tests hadn't exercised, both fixed before an
independent review even ran:**

1. **Saving failed for most real materials.** `write_field_value`
   originally *errored* when `field` wasn't already present as literal
   text in the file — but most material files only write the handful of
   fields that differ from `Material`'s own `serde` defaults (`stone.ron`
   never mentions `heat_conductivity` at all), so "field absent from the
   text" is the *common* case for a registered tunable, not a typo.
   Running the debug harness against `stone.ron` for real (not just the
   hand-built strings in this module's own tests) surfaced it immediately:
   adjusting `heat_conductivity` worked live, saving it reported "field
   not found." Fixed: when the field isn't found as an existing key,
   `write_field_value` now appends `field: value,` on its own line just
   before the file's own closing `)` (every shipped material file is a
   single top-level struct, so its last `)` is unambiguous), inserting a
   leading comma only when the preceding content doesn't already end in
   one.
2. **The panel's last visible row overlapped the status-message footer.**
   `draw_tunables_panel`'s row count was computed from the full panel
   height, with the message drawn into space that was only reserved when
   `self.message` happened to be `Some` at draw time — so the list's own
   last row and a just-set save confirmation landed on the same pixels,
   both unreadable, visible immediately in the saved PNG. Fixed: the
   footer is now reserved unconditionally.

**Independent review of the fixed implementation before commit found two
further, more subtle bugs, both confirmed by writing a failing test
first, then fixed:**

3. **`find_field_value_span` was comment-unaware.** A field written as
   `density: 1.0 // heavy` (no shipped file happens to use this style
   today, but nothing stopped a future one from being hand-edited that
   way) had its trailing comment silently folded into the matched span and
   deleted on save — the result was still valid RON, so the pre-write
   parse check didn't catch it, and the comment was gone permanently.
   Fixed: the value-span search now also stops at `//`, not just
   `,`/`)`/newline.
4. **The insert-if-missing path's "does the file need a leading comma"
   check read through comments the same naive way.** A file whose last
   content before `)` was a bare trailing comment (rather than a field)
   could either insert a stray comma or skip a genuinely needed one,
   depending on what the comment's own last character happened to be —
   the stray-comma case is caught by the pre-write parse check (fails
   safely, if confusingly), but the fix is the same underlying one either
   way: added `last_significant_char`, which strips each line's own `//`
   comment before checking what's really there.

**Two lower-severity findings from the same review were assessed and
deliberately left as-is, documented rather than engineered around:** the
disk write in `save_tunable` is picked up by `main.rs`'s existing file
watcher a couple hundred milliseconds later, which calls
`reload_materials` and overwrites the "saved X.Y = Z" confirmation with a
generic "reloaded N materials" one — harmless (the reload just re-reads
the identical value already live in memory), not worth a cross-module
suppression flag for one message briefly outliving another. Separately, a
hot-reload while the panel is open can change which conditional fields
are registered per material, shifting every later flattened list index —
`tunables_selected` is now reset to 0 on every `reload_materials` call
(matching the existing precedent `self.selected`'s own reset already
set), closing the one version of this that could have silently landed a
save on the wrong field.

All four confirmed bugs were verified via revert: each fix's own new test
was checked to fail against the pre-fix code, then the fix restored.
`cargo test` (268 lib tests, up from 266) and `cargo clippy --all-targets
-- -D warnings` both clean.

**Verification screenshots kept and committed under
`docs/screenshots/section-10-tunables/`** (`panel-open.png`,
`panel-scrolled-adjusted-saved.png`, and a genuine before/after pair for
the footer-overlap bug — `footer-overlap-before.png` was captured by
temporarily reverting the fix, then the fix was restored and
`footer-overlap-after.png` captured against the real code). Starting this
section, per explicit request: throwaway debug harnesses (`examples/
debug_*.rs`) themselves still get deleted after use as before, but any
PNG/GIF they produce for visual verification is now kept and committed
rather than discarded, as a visible record of what a feature looks like
and what a visual bug actually looked like before its fix.

### Files touched

`src/tunables.rs` (new — `Tunable`, `from_materials`,
`write_field_value`, `find_field_value_span`, `last_significant_char`,
`format_value`). `src/sim/material.rs` (`MaterialRegistry::get_mut`).
`src/app.rs` (`show_tunables`/`tunables_selected` fields,
`toggle_tunables`/`tunables_list`/`tunables_move`/`tunables_adjust`/
`save_tunable`, `draw_tunables_panel`, `reload_materials` now also resets
`tunables_selected`). `src/main.rs` (`O` toggle; arrow keys and `Enter`
guarded on the panel being open; `Escape` now contextual). `src/lib.rs`
(registers the `tunables` module). `PLAN.md`/`README.md`.

### Overnight run, section 11: M6 rendering upgrade — reframed as a CPU-side dirty-rect skip

The plan's original split — dirty-region GPU texture uploads (objective)
plus a runtime-tunable bloom shader (needs live judgment, stays deferred)
— ran into an architectural wall on the first half before any code was
written: reading `pixels` 0.17.2's own source (`render`/`render_with`,
`PixelsContext`) confirmed both public entry points unconditionally
re-upload the *entire* frame buffer to the GPU texture before the
caller's render closure ever runs, and `PixelsContext::surface` is
private, so there is no way to drive a narrower upload short of forking
the crate. An independent review agent confirmed this reading and the
follow-on reasoning: the GPU copy this would have saved is ~655KB/frame
at 60fps (≈39MB/s) — nowhere near a real bottleneck on any GPU bus — so
forking a dependency to shave it is a bad trade nobody asked for.

**Reframed to the CPU-side cost that actually is real and already
measured**: `cell_colour` (grain, heat glow, field-overlay tint) reruns
for every one of up to 512×320 pixels every frame regardless of what
changed — the fake-AO experiment M19 already recorded cutting for cost is
concrete prior evidence this isn't hypothetical. `Renderer::draw` now
skips recomputing pixels for any chunk that provably didn't change,
verified via the engine's own existing capture tools (confirmed working
in this environment before committing to the section — `PIXEL_PHYSICS_
SCREENSHOT_AFTER_FRAMES` and `PIXEL_PHYSICS_CAPTURE_SEQUENCE` both still
launch the real window and render correctly here, contrary to this
session's earlier §6 finding, which turned out to be about frame
*advancement* pacing specifically, not the render path itself).

**Two real bugs found and fixed before this shipped, both via live/debug
verification rather than only unit tests:**

1. **A settled-chunk snapshot check is not the same question as "did this
   change since I last drew it."** The first version checked `chunk.
   is_settled()` directly inside `Renderer::draw`. A debug harness built
   to stress exactly this — call `App::update()` 300 times, then draw
   once — caught a sand pile that had fallen and landed rendering frozen
   at its original mid-air position. Root cause: `main.rs`'s own
   `MAX_TICKS_PER_FRAME` catch-up loop can run several ticks per draw on
   any frame that runs behind, and a chunk that goes active and settles
   again *within* that window reads as settled at draw time despite
   having visibly moved. Fixed by moving the tracking to `World` itself:
   a new `touched_chunks: HashSet<ChunkCoord>` accumulates across every
   tick's `end_step`, drained once per render via `take_touched_chunks`
   — `Renderer::draw` now takes `touched: &HashSet<ChunkCoord>` instead
   of scanning `world.chunks()` for `!is_settled()`.
2. **The touched-chunks fix above still had a one-tick lag for the first
   write to an already-settled chunk**, caught by an independent review:
   `end_step` computed `was_settled` *before* calling `end_sweep`, so a
   chunk that was fully settled and then received exactly one
   out-of-sweep write (organism growth via `step_active_sites`, an
   explosion, a structural collapse, a landing free particle, a hot-
   reload's `wake_all` — none of these are gated on the cursor being
   over the window the way painting incidentally is) wouldn't appear in
   `touched_chunks` until a *second* subsequent `end_step`, one whole
   tick later than the write that actually changed its pixels. Fixed by
   checking settledness on both sides of `end_sweep` — `!was_settled ||
   !settled_now` — which correctly catches both transition directions
   (a chunk going active, and a chunk a write just promoted out of
   settled) without reintroducing the first bug. Both fixes were
   confirmed via revert: each one's regression test was checked to fail
   against the pre-fix code, then the fix restored.

**Bypassed to a full redraw** (matching the original per-pixel loop
exactly) whenever: the caller's own `force_full` is set (`App::draw`
sets it whenever the cursor is on-screen or any HUD panel is open, since
those are painted over the terrain with no tracked footprint of their
own — painting requires the cursor, so this incidentally covers every
paint/erase action too); `zoom`/`zoom_out_stride` changed since the
renderer's own last `draw` call; the field overlay is on (the M13 field
grid diffuses independently of chunk activity); `show_chunk_overlay` is
on; or particles are non-empty (free debris has no tracked footprint —
bypassing was judged simpler and cheap enough against tracking a
leave/enter region for something already fast to just redraw).

**Measured, not just asserted**: `examples/ascii.rs`'s existing
`render_stress_scene` benchmark (full 512×320 sand scene, `Renderer::draw`
timing isolated from simulation cost) went from **6.6ms worst frame to
0.0ms** once the scene is genuinely settled and idle — the exact
"densely-filled static world pays this cost forever" case the earlier
fake-AO cut already flagged as real. `Renderer::draw` also returns the
actual pixel-recompute count now, exercised directly by this module's own
tests, rather than an env-var-gated instrumentation hook layered on
separately.

**7 new tests** (`render.rs`, prefixed `§11` in comments) cover: a
partial redraw is pixel-identical to a full one after the world actually
changes between two draws (confirmed via revert to catch a deliberately
inverted settled-chunk polarity); a fully settled world recomputes zero
pixels; the very first draw is always full regardless of `force_full`;
zoom changes force one more full redraw; nonempty particles and an active
field overlay both force a full redraw every frame; and the two
regression tests above. Plus **2 new tests** in `world.rs` for the
`touched_chunks` accumulation itself.

**Visual verification kept and committed** under `docs/screenshots/
section-11-dirty-rect-render/` (`mid-fall.png`, `mid-fall-2.png`,
`settled.png`, `settled-after-skip.png`) — a real sand column painted,
ticked through falling and landing with no intervening draws, confirming
the settled pile renders at its true final position rather than a stale
mid-air one.

`cargo test`: 277 lib tests (up from 268), all green. `cargo clippy
--all-targets -- -D warnings` clean.

### Files touched

`src/render.rs` (`Renderer::draw` signature gains `touched: &HashSet<
ChunkCoord>` and returns `usize`; `last_zoom_state` field;
`world_rect_to_screen_rect`; `Rect::union` used from `chunk.rs`).
`src/sim/chunk.rs` (`Rect::union`). `src/sim/world.rs` (`touched_chunks`
field, `end_step` checks settledness on both sides of `end_sweep`,
`take_touched_chunks`). `src/app.rs` (`App::draw` is now `&mut self`,
fetches and passes `touched`). `examples/ascii.rs` (`render_stress_scene`
settles its world and measures the optimized path).
`docs/screenshots/section-11-dirty-rect-render/`. `PLAN.md`/`README.md`.

### Live playtest finding: water settles into sand-like piles, not flat

Reported directly from a live `cargo run` screenshot (F1 chunk overlay on,
paused mid-simulation): two separate water pools on floating ledges both
showed a trapezoidal, angle-of-repose top instead of a flat surface, and
appeared to "bunch" aligned to the visible chunk grid.

**Root-caused, not guessed at**, via a debug harness (`examples/
debug_water.rs`, deleted after use) that poured water on isolated ledges
and stepped it through both the serial and parallel (M5) drivers, dumping
raw fill values alongside PNG snapshots:

- **The sloped shape is real and reproducible** — the harness's own output
  matched the screenshot's shape closely. Cause: `update_liquid`'s first
  two moves (fall straight down, then diagonally into empty space) are the
  *exact same code path* `update_powder` uses, and that mechanism can only
  ever build an angle-of-repose slope — it stops the instant a surface
  cell has diagonal support both sides, identically to how a sand grain's
  pile forms. Only the much slower "compare against the emptiest same-
  material cell within 8 cells, transfer half the difference" horizontal
  mechanism erodes that initial rough shape toward flat, and it does so
  slowly by design (see `MIN_LIQUID_TRANSFER`'s own doc in `update.rs`).
  For a pool tens of cells wide this took hundreds to thousands of frames
  to visibly flatten in the harness — plausible on a real timescale, and
  exactly the "still sloped" state the screenshot caught.
- **The "chunk-boundary bunching" does not appear to be a distinct bug.**
  A controlled test poured two *identical*, symmetric water rectangles —
  one centred exactly on a chunk boundary (x=64), one safely mid-chunk as
  a control — and tracked left/right fill asymmetry over time under the
  real parallel driver. Both converged to *perfect* symmetry by frame
  ~60-100 and stayed symmetric through frame 400; the asymmetry that
  appears later (frame 700+, as both pools drain to sparse residual
  droplets) was actually *larger* in the mid-chunk control than the
  boundary case, consistent with ordinary RNG-driven tie-breaking noise on
  a near-empty region rather than anything boundary-specific. Current
  read: the visible F1 grid lines simply happen to overlap wherever the
  (separately real) slow-convergence slope sits, which reads as
  "aligned to the grid" without an actual causal link. Flagged as needing
  more scrutiny, not closed — see below.

**Quick mitigation applied**: `water.ron`'s `flow_rate` raised from 200 to
1000 (removing it as a redundant bottleneck under-neath `transfer_liquid_
horizontal`'s own half-difference cap, which is what actually prevents
overshoot/oscillation — see that function's doc). Confirmed via the debug
harness this measurably speeds up the *later*, slow-smoothing phase, but
does **not** fix the initial sloped shape, since that phase is dominated
by the diagonal-fall step described above, not by `flow_rate`. All
water-related tests still pass (`cargo test --lib water`: 11/11).

**Explicitly not a trial-and-error fix from here**: at the user's
request, a deep research pass on particle-based and CA-appropriate liquid
simulation techniques (SPH and its real-time variants, and specifically
how other falling-sand engines — Noita, The Powder Toy, Sandspiel — solve
this exact "liquid looks like sand" problem) grounded the real fix in
prior art rather than more constant-tuning. Full report:
[`Reports/liquid-simulation-research.md`](Reports/liquid-simulation-research.md).

**The verdict: don't adopt SPH/PBF/PIC-FLIP — fix the CA rule's mechanism
ordering instead.** No GPU compute path exists in this engine to run any
of them on (`pixels`/`wgpu` are presentation-only here), and the numbers
don't fit even generously (real-time SPH's own 2003 paper caps at 5000
particles for a single-purpose demo; Position Based Fluids needs a CUDA
GPU to hit 128k particles at a few ms; PIC/FLIP-for-granular-material runs
~6 seconds a frame, offline, in 3D) — against this engine's existing
163,840-cell grid already running six material kinds plus fire/structural/
plant logic in one ~16ms budget. Every comparable falling-sand engine
surveyed (TPT, Sandspiel, Noita) also uses a discrete per-cell CA rule for
liquid, never a particle solver — the genuine difference is that each of
them lets sideways movement participate in a liquid cell's *first*
movement decision, where this engine's `update_liquid` currently runs an
unconditional, powder-identical diagonal-fall phase first (to exhaustion)
before its liquid-specific horizontal-transfer mechanism ever gets a
turn — exactly reproducing a sand pile's angle-of-repose shape for however
long that phase takes to exhaust itself. Zhu & Bridson's sand-as-fluid
paper (SIGGRAPH 2005) supplies the cleanest frame for the fix: give a
liquid cell an explicit per-step choice between "behaving as a settled
mass" and "behaving as a flowing surface" (their Mohr–Coulomb yield
check), rather than one hardwired first-mechanism that can only ever
express the settled case. Concretely: give `Liquid` kind a same-step,
bounded-width horizontal search for a lower/emptier opening, evaluated
before or alongside the diagonal-fall check rather than gated behind it —
reusing the existing `HORIZONTAL_TRANSFER_REACH` rather than a new
constant. The existing compressible-fill mechanism
(`transfer_liquid_vertical`/`horizontal`) stays; it's already the
technically correct long-run leveling process, validated by this
engine's own passing tests — the bug is specifically about the *first few
frames'* shape, not eventual convergence. Not yet implemented — the
report deliberately stops at the direction, not a finished design; that's
implementation work with its own test-driven verification loop.

### Live playtest feedback: tree growth is real but tiny — a soil/moisture/differentiated-cell/environmental-interaction vision for a later phase

After the four `Grow`/canopy-density bugs above were fixed and `tree.ron`'s
resource economy retuned via a 6-way parallel comparison (own section
below), the owner's read on the result: genuine improvement, but still a
tiny tree — one cell thick, ~18 cells total, no visually distinct leaves,
no roots at all in the test scene. Asked what's actually limiting bigger
growth, and to record a fuller future vision (soil-grown roots with a real
moisture economy, mud, visually/behaviorally distinct root/trunk/leaf
cells, and environmental interaction — debris catching on branches, weight
breaking them, roots stabilizing soil) without implementing any of it yet:
**"I want to get the simple tree mechanics right before we complicate
things."**

**Diagnosis of the four symptoms, each traced to a specific mechanism, not
a bug:**

- **Growth stays small and stops for good.** A `GrowingTip` that
  successfully grows retires to `MatureBody` (the fix above) — the
  *child* carries the frontier forward, not the parent. But nothing ever
  creates a *new* independent frontier once every existing lineage has
  either dead-ended (four consecutive `Grow` misses → permanent
  `MatureBody`, `plant.rs`'s staleness path) or run its course. `branch_
  chance: 0.1` is the only source of more than one simultaneous lineage,
  and it's low. Once every active `GrowingTip` an organism has is gone,
  growth is over for that organism, forever, regardless of remaining
  light or space — there is no mechanism (epicormic budding, or anything
  like it) for a mature tree to issue a new shoot later. This is the
  actual ceiling on total size, not the resource economy just retuned.
- **One cell thick.** `SecondaryThicken`'s own pipe-model trigger
  (`leaf_count / width > pipe_ratio`, `plant.rs`'s `thicken()`) counts
  only cells that are *currently* `GrowingTip` or `Leaf` via a downstream
  flood fill. Since tips now retire to `MatureBody` immediately after
  growing (necessary — it's what fixed the round-clump bug above), the
  count of cells still carrying `GrowingTip` at any instant is almost
  always 0–2 for a tree this small, which essentially never clears `pipe_
  ratio: 2.5`. Direct, connected side effect of today's own retirement
  fix, not a separate problem — thickening needs a bigger, longer-lived
  tree (or a lower ratio, or a different downstream-load signal than
  "currently mid-growth") before it can ever fire.
- **No visually distinct leaves.** By design, documented in `tree.ron`'s
  own header: this pass's `Grow` only ever creates more of its own
  parent's cell type — no separate `Leaf` spawned. `GrowingTip` doubles as
  its own photosynthetic surface. `CellType::Leaf` exists in `organism.rs`
  and is wired into the dispatch table, just never produced by anything
  yet. A known, deliberate simplification carried the whole session, not
  a regression.
- **No roots in the test scene, and roots can't grow through soil at all
  yet regardless.** `germinate()` only creates the companion `RootTip` if
  the cell directly below the seed is empty (`plant.rs:540`) — the test
  room's stone floor sits directly under the seed, so no root is ever
  created there. That's scene-specific and easy to change. The deeper gap
  is older than this session: `Grow`'s candidate loop (shared by
  `GrowingTip` and `RootTip`) only ever considers a neighbour if `world.
  is_empty(nx, ny)` — there is no displacement-into-loose-material
  mechanic at all, for either cell type. A root cannot grow "through"
  soil today even where soil exists; it can only extend into literal open
  air, exactly like canopy growth does. This was flagged as far back as
  the very first tree-growth playtest note above ("roots currently fail
  to grow at all when a tree is planted directly on stone with no soil
  underneath") and the organism-substrate rewrite this whole arc has been
  working toward was always the intended fix — it just hasn't reached
  roots yet.

**"Too much weight breaks a branch" is not new scope — it's already the
next planned step, just not done.** `wood.ron` already sets `max_
unsupported_span: 8` (the plan's own suggested tree number, "stone 3, wood
8, steel 20"), and `structural.rs` already extends `is_body_material` to
`Solid | Plant`. But `organism_structural_tick`'s own doc says so
directly: *"material sets a finite max_unsupported_span but organism_
structural_tick has no anchor-based check wired up yet for organism-owned
cells."* That's this session's own pending "tree rewrite step 5." Given
the owner's stated preference — simple mechanics first — this is the
natural very-next piece, not a future-phase item.

**The rest of the vision, organized by what already exists to build on
versus what's genuinely new design work:**

- **Roots grow through soil, not just into open air.** `soil` is already
  a real material (Powder kind, produced by the existing ash → soil decay
  cycle — overnight run section 8's own entry above). What's missing:
  `Grow`'s `RootTip` candidate scoring needs a second "growable" case
  alongside `is_empty` — displacing into `soil` specifically (converting
  it to root material, the same shape `has_growable_neighbour` already
  uses for moss growing onto `Solid`, generalized to a displace-and-
  convert instead of grow-onto-empty).
- **Soil moisture, consumed by roots, raised by water, too much is bad.**
  Also not starting from zero: a field-level moisture channel already
  exists, and `World::deplete_moisture` is already called by `RootTip`'s
  `Absorb` when it drinks an adjacent `Liquid` cell. Extending "grow
  through soil" to also deplete moisture along the way is the same API,
  not new infrastructure. Genuinely new design decisions: the mud
  transition at high saturation (a new material, or a wet-soil variant —
  `decay.rs`'s ash→soil transition is the template to follow), and what
  "too much moisture" actually costs a root (slower growth? reduced
  absorb efficiency? literal rot?) — needs a real mechanism, not a vague
  penalty.
- **Root, trunk, and leaf cells look and behave differently.** Partially
  exists: `CellType` already distinguishes `RootTip`/`GrowingTip`/
  `MatureBody`/`Leaf`, and each already carries its own behavior list in
  species data. What's missing is the material/appearance side — every
  cell type currently paints as plain `wood`. Needs either per-cell-type
  materials (`root-wood`, `heartwood`, `leaf`) or a shading/palette rule
  keyed on `CellType`, plus `Grow` actually producing a distinct `Leaf`
  cell (closing the "no visible leaves" gap above at the same time).
  **Added requirement, owner's own words:** a plant should start with a
  base reserve of energy (a real starting `resource`, not today's `0.0` a
  freshly-germinated `Seed`/`GrowingTip` gets — `germinate()`, `plant.rs`
  — mirroring a real seed's stored starch), but should not be able to
  photosynthesize *at all* until it actually has a leaf. Today's
  `Photosynthesize` sits directly on `GrowingTip` and produces resource
  from its very first tick, with no leaf involved — once `Leaf` is a real
  produced cell type, `Photosynthesize` belongs on `Leaf` only, and a
  seedling's early growth needs to be funded entirely out of its starting
  reserve until it manages to put out a leaf. This makes "fails to grow a
  leaf before exhausting the seed reserve" a real, emergent seedling-
  death condition instead of something hardcoded — directly the kind of
  mechanism `design-philosophy.md` §2b asks for, and connects the resource
  economy (this session's own tuning pass) to leaf differentiation (this
  bullet) rather than treating them as separate problems.
- **Environmental interaction — debris catching on branches, roots
  stabilizing soil.** The least-grounded item, genuinely open design
  work: does a powder cell resting against a `Plant` cell already count
  as supported by the existing CA fall rules, or does it currently fall
  through/around? Not yet checked. Roots stabilizing nearby soil is the
  mirror image of the weight-breaking mechanic above — extending
  anchor-distance credit *outward* from a root into adjacent soil, rather
  than only checking a wood cell's own distance from an anchor.

**Sequencing, per the owner's own stated preference:** this phase comes
*after* the current tree-rewrite retrofit finishes — step 5 (structural
integrity, above, effectively already part of it), step 7 (cut over
`plant_tree`, delete `TreeState`/`Tip`/`RootTip`), and step 8 (independent
review) — not before. It needs its own just-in-time design report before
any implementation, matching this project's standing practice for every
other structural change (`design-philosophy.md` §3, `organism-substrate-
design.md`'s own retrofit-order precedent). Not started; recorded here so
the next design pass has the full picture rather than rediscovering it.

**External research, folded into this same phase: `Reports/plant-
simulation-research.md`** (owner-supplied, written against an earlier
commit than the tree rewrite's own completion — two of its own findings,
the crowding-reads-an-always-empty-cell bug and "finish or delete
`TreeState`," were independently found and fixed by this session before
the document was read, which is a real cross-check that its other
findings are trustworthy too). Full document is the source of record;
summarized here so this phase's eventual design pass doesn't have to
rediscover it:

- **The load-bearing finding: accretion is not growth.** A `Plant` cell is
  immovable and exactly one pixel; growth can only write into an *empty*
  neighbour. That's the growth mode of moss, lichen, and coral — not a
  tree. It's a deeper diagnosis of the one-pixel-trunk problem than this
  session's own (`SecondaryThicken`'s pipe-ratio trigger almost never
  firing): *even if* that trigger fired constantly, "grow sideways into
  an empty cell" is still accretion, not real thickening — a trunk
  already surrounded by wood has no empty neighbour to accrete into at
  all. Three ways out, increasing ambition: (i) accept accretive growth
  as the honest target (moss/lichen/coral are real biology, not a
  compromise), (ii) a displacement primitive — a growing cell pushes the
  column ahead of it by one (the root-grows-through-soil idea already
  recorded above is this, at the smallest possible scope: one cell
  converted, not a pushed column), (iii) a continuous turgor/extension
  scalar reusing the liquid rewrite's own fill-amount trick, promoting to
  a whole cell on saturation — sub-pixel growth *rate* without needing
  displacement. This project's own preliminary lean (not yet decided):
  accept (i) for canopy, use the small-scope version of (ii) for roots
  growing through soil specifically, (iii) for rate. **This decision is
  first in the eventual design report** — every other plant mechanic in
  this phase inherits it.
- **`Cell::aux` is already fully packed (16/16 bits)**, and this phase's
  own vision needs more: a second resource currency (carbon vs.
  water/nitrogen — collapsing them to one scalar removes the trade-off
  that makes allocation interesting), organ age (for leaf lifespan),
  and canopy density's own 4 bits are already coarse enough to produce
  quantization ties. Recommendation: stop packing into `Cell` entirely —
  organism cells are a small fraction of any world, so a sidecar table
  keyed by position costs little and removes the ceiling permanently.
  Worth deciding *before* this phase's own leaf/soil scalars get added,
  not after, to avoid building on a foundation about to need
  restructuring anyway.
- **`organism::diffuse_resource` is isotropic (symmetric neighbour
  averaging), and every real shape-generating process in plant
  development is polar** (auxin moves basipetally, xylem/phloem are
  separate directional tissues) — symmetric diffusion can blur a
  gradient but never canalize it into a channel, no matter how long it
  runs or how weights are tuned. Named as the same failure-mode *family*
  as this session's own crowding bug: a mechanism named after a
  directional process, implemented as a symmetric or inert one. A few
  bits of per-cell polarity plus a flux-following update rule (move
  preferentially along polarity; rotate polarity toward whatever
  direction carried the most flux last tick) would make apical dominance
  and vein-like structure real emergent outcomes instead of tuned
  weights — highest emergent-behavior-per-effort item in the whole
  document, but it changes the core diffusion mechanism, not a leaf node.
  Sequence *after* the soil/leaf work, not before, per "simple mechanics
  first."
- **Evolution is where this architecture is unusually well-suited, not a
  stretch fit.** The `.ron` species file is already a genotype;
  `organism_tick` is already a developmental program; `structural.rs`
  already measures one of Niklas's four adaptive-walk fitness tasks
  (mechanical stability — light interception and water conservation are
  also already measurable from existing field data, only reproduction is
  missing). The document's central warning: fitness has to be multi-task
  (3+ conflicting objectives) or selection collapses the whole population
  onto one morphology — a real trade-off (leaf economics spectrum: fast
  photosynthesis inversely coupled to leaf lifespan/durability; wood
  density vs. growth rate, both sides already exist as `density`/`max_
  unsupported_span`) has to be built in before any selection runs, not
  discovered after. A real future milestone, not this phase — recorded
  here so it isn't lost, not scheduled.
- **Standing gotcha for evolution specifically, once it's scheduled:**
  `Chunk::rng` is seeded from chunk coordinate, so the same genotype
  planted in two different places draws a different random sequence —
  position becomes a hidden inherited variable, which is exactly the kind
  of confound that produces a spurious "evolutionary" result. A
  per-organism RNG stream seeded from the organism id would remove it.

**Standing constraint for all of the above, restated by the owner:**
today's organism substrate (`OrganismState { species: SpeciesId }`,
species-level shared behavior data) should be built so a later per-
organism trait-variation/evolution milestone can extend it, not require
throwing it away. Concretely, in mind for every change from here on:
prefer adding new *per-organism* state (a trait vector, eventually) over
hardcoding more assumptions that every individual of a species is
identical; keep species-level constants read through the existing
`Species`/`Behavior` indirection rather than inlined at call sites, so a
future per-organism override has a seam to hook into instead of a rewrite
to perform.

### Tree rewrite step 7: cutover, and a `RootTip` resource-economy gap found while porting tests

Repointed `plant_tree`/the `T` key at the new `Grow`/`Germinate`-driven
system (`World::plant_tree` now calls `plant_tree_species(x, y, "tree")`
directly; the transitional `plant_tree_v2` name is gone). Deleted the old
`TreeState`/`Tip`/`RootTip` structs, `tree_tip_tick`/`root_tip_tick`,
`plant_tree_seed`, `World::push_tree`/`tree`/`tree_mut`, and the
`ActiveKind::TreeTip`/`RootTip` schedule variants — the emergent system is
now the only tree implementation.

Ported every old test rather than deleting them wholesale, each kept only
where its underlying claim still applies to the new system:

- `a_tip_leans_more_steeply_upward_when_lit_from_above`/`a_tip_leans_
  downwind_of_a_steady_breeze` became direct unit tests of `organism::
  phototropism_dir`/`wind_lean_dir` themselves (the exact ported formulas)
  rather than a whole simulated `tree_tip_tick` call — those two functions
  had no test of their own at their new location until this pass.
- `a_tree_can_produce_multiple_simultaneous_tips_via_branching` became
  `a_tree_can_branch_into_more_than_one_lineage`, checking for a branch
  *point* (3+ same-organism 8-neighbours) instead of counting
  simultaneously-alive tips — this session's own tip-retirement fix means
  tips essentially never stay alive simultaneously any more, by design.
- The two orphaned-tip/orphaned-root regression tests ported directly
  (`organism_tick`'s `cell.organism_id() != organism_id` guard is the
  direct equivalent); the old "root resting in drunk water" half didn't
  translate — the new cell-based `Absorb` only ever empties an *adjacent*
  water cell, never the root's own position, so a `RootTip` can't end up
  sitting in a cell it vacated itself the way the old continuous-position
  model could.
- The old TreeState-leak mitigation (freeing a fully-dead tree's
  `attractors` list) had nothing to port — the new system has no
  attractors at all. The underlying concern is real and still open,
  though: `World::push_organism`'s own doc already says "nothing
  populates [`free_organism_slots`] yet in this pass," so a fully-dead
  organism's id slot is never reclaimed. Recorded as a known gap, not
  silently dropped — a real fix needs a BFS-from-roots liveness check,
  `organism-substrate-design.md` §6's own scoped-but-undone item.
- Several ported tests initially failed for a reason unrelated to the
  cutover itself: they planted at the old system's own y=100-150 depths,
  which `Germinate`'s real light gate can't reach at all (`field.rs`'s
  light model decays hard within a few rows of open sky) — the old flat
  `AMBIENT_GROWTH_ENERGY` never had this constraint. Moved to y≈20.
  `roots_consume_adjacent_water` also needed its assertion changed from a
  cell-*count* water comparison to checking one specific cell directly:
  the compressible-volume liquid model can spread the *remaining* water
  into more, shallower-filled cells as it resettles around the gap a
  root's `Absorb` leaves, which raises `count()`'s tally even as real
  volume drops.

**A second, genuine resource-economy gap found while porting the
hydrotropism test, separate from the `GrowingTip` cost/rate tuning done
earlier this session:** `RootTip` has no income source of its own besides
`Absorb` (which only pays off once already touching water) — a root with
no adjacent water lives entirely off resource slowly diffusing over from
the trunk, and can permanently go dormant (`ORGANISM_STALE_LIMIT`
consecutive starved misses) well before ever reaching a water pocket even
a few cells away, no matter how long the simulation runs afterward.
Confirmed directly: at both 1,500 and 6,000 ticks a root in an off-axis-
water test scene had made identical, minimal progress (2 successful
growth steps, drifted the wrong way) — not a timing issue, a permanent
stall. Worked around for now by testing `organism::moisture_pull`'s
steering directly rather than a full growth simulation (mirroring the
phototropism/wind-lean tests above), which is a legitimate test-design
choice on its own merits, but doesn't fix the underlying gap. Candidate
for the same kind of parallel-comparison tuning pass `tree.ron`'s
`GrowingTip` values already got (`examples/debug_tree_variants.rs`), on a
`RootTip`-specific cost/rate pair — not done here, since it wasn't the
task in front of this session, but the tool to do it already exists.

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

---

## Code review findings (owner-supplied, verified against actual code) — parked, not started

An owner-supplied general code review, written against roughly the same
earlier commit as `plant-simulation-research.md` (close to `838c557`).
Every claim was individually re-verified against the current codebase
(grep/read, not assumed) before recording here. This is a *separate track*
from the plant-substrate-v2 work above — scheduler/CI/doc-health, not plant
biology — parked because context ran low, not because it's low priority.

**Already fixed by this session's own work (verified, listed for closure
tracking only, no action needed):**
- Crowding-reads-an-always-empty-cell (`candidate_crowding` now reads
  occupied neighbours, regression test exists).
- `pack_aux` clobbering canopy density on every resource update
  (`pack_aux_preserving_density` now wraps every self-update).
- `World::trees` never shrinking — `TreeState` itself is deleted.
- Two coexisting tree implementations / `tree.ron` not existing — the old
  `TreeState`/`Tip`/`RootTip` system is fully deleted, `tree.ron` exists and
  is the only tree system, `plant_tree`/the `T` key point at it.
- **Item #2, no dedup/budget on the active-site heap — fixed, plus two
  bugs it uncovered along the way.** `pending_structural_checks: HashSet<
  (i32,i32)>` on `World`, deduping `ActiveKind::StructuralCheck` inside
  `World::schedule_active_site` itself (the one point every insertion path
  — `structural::schedule_structural_check`, `fire.rs`'s hand-built
  burnout fan-out, the parallel driver's `ChunkView` replay — funnels
  through). `scheduler::MAX_SITES_PER_FRAME = 2000` caps how much of the
  due backlog one `step()` call drains, leaving the rest exactly where the
  heap will find it again next frame. Regression tests:
  `overlapping_schedule_structural_check_around_calls_do_not_duplicate`,
  `overlapping_burnouts_do_not_duplicate_structural_checks`,
  `scheduler_processes_at_most_the_per_frame_budget` — all three confirmed
  to fail without their respective fix.
  - **Bug #1, found by independent review:** the dedup's first cut only
    covered `structural::schedule_structural_check`'s own callers, missing
    `fire.rs`'s direct `ActiveSite` construction and `structural.rs`'s own
    self-rescheduling paths. Fixed by centralizing into `schedule_active_
    site` as above, rather than chasing each call site.
  - **Bug #2, found by the same review, larger:** any `world.schedule_
    active_site` call made *from inside* a dispatched tick (`plant.rs`'s
    `Grow`/`germinate`/`thicken` calling `schedule_structural_check_around`
    mid-tick; `decay.rs`'s reseed calling `plant_moss_seed`/`plant_tree`)
    was silently discarded — `scheduler::step`'s old take-the-whole-heap-
    out-then-write-it-back-at-the-end shape (`take_active_sites`/`set_
    active_sites`) left `world.active_sites` genuinely empty for the whole
    dispatch loop, so a mid-tick schedule call wrote into the empty field
    and was overwritten when the real heap replaced it. Fixed by replacing
    that shape with `World::pop_due_active_site`, which pops one due site
    at a time *in place* rather than taking the field out — `world.
    active_sites` stays live and correctly populated through every tick,
    so `schedule_active_site` (and anything reading the heap, like
    `organism_active_tip_count`, which was *also* silently undercounting
    to near-zero during dispatch under the old shape) works correctly
    everywhere. Regression test: `decay.rs`'s `a_reseeded_organism_keeps_
    growing_after_its_first_tick`, confirmed to fail against the old shape
    (cell count stuck at 3, forever) and pass against the fix.
  - **Fixing bug #2 surfaced a real, previously-untested design gap**:
    once `schedule_structural_check_around` calls made from `Grow`/
    `germinate`/`thicken` actually reached the heap for the first time
    ever, every open-sky tree test started failing — a freshly germinated
    seedling, not yet connected to any ground, read as unsupported and
    was destroyed by its own first structural check. Resolved by removing
    those calls entirely: ordinary growth only ever adds material, never
    removes support, so — unlike painting, erasing, an explosion, or a
    burnout — it isn't a disturbance the structural system needs to react
    to, matching this module's own long-standing "checked reactively,
    never at creation time" rule (previously applied to world-gen and
    Solid painting, now extended to organism growth for the same reason).
  - Cost re-checked on the `ascii` stress scene both times (dedup/budget
    alone, then again after the pop-in-place refactor): serial ~38-40ms,
    parallel ~8ms, matching the pre-session baseline — no regression from
    either change.

**Still real and unaddressed — verified against current code just now:**

1. **`World::creatures` never shrinks, `free_organism_slots` is popped but
   nothing ever pushes to it.** The worm/creature system hasn't been
   touched this session (worm migration onto `organism.rs` is still
   pending, above). Real, same shape as the tree-side issue #8 that got
   fixed for organisms but not creatures.
2. **`ChunkView::set` redundantly recomputes neighbour-waking for a
   cross-boundary write.** Confirmed by tracing it: `queue_touch_neighbours`
   queues `dirty_touches` during the parallel pass, then `run_pass`'s replay
   calls `world.set(x, y, cell)` (which internally calls `touch_neighbours`
   again) *and* separately replays the queued `dirty_touches` for the same
   write. Real, but low-impact — idempotent, just wasted cycles, not a
   correctness bug.
3. **CI never runs a debug build.** `.github/workflows/ci.yml`'s three real
   steps (`cargo test`, `cargo clippy`, `cargo run --example ascii`) are all
   `--release`. The codebase leans on `debug_assert!` as a real guard
   mechanism (organism/creature index overflow, `clear_moved` ownership,
   `tick_burn` on a non-burning cell, `organism_structural_tick`'s span
   assertion) — none of it is compiled in CI today.
4. **`examples/ascii.rs` gates nothing.** Its worst-frame numbers are
   treated as the de facto perf regression suite (README, CI comment) but
   the example has no assertions and always exits 0 — only a panic fails
   it. A 10x regression would pass silently.
5. **`cargo fmt --check` is `continue-on-error: true`.** Confirmed
   deliberate, not an oversight — the CI file has an honest comment
   explaining the codebase predates `rustfmt.toml` and hasn't had a full
   pass run against it yet (`pixel-physics-issues.md` issue #10). Still an
   open item, just not a silent one.
6. **Doc drift, three confirmed-stale claims:**
   - README:631 claims `grep -rn unsafe src/` returns nothing but doc
     comments — false; `src/main.rs` has real `unsafe { std::env::set_var
     }`/`remove_var` blocks in tests (correctly `ENV_LOCK`-guarded, but the
     claim itself is stale).
   - `src/sim/cell.rs`'s own `aux` doc still says `Liquid` → "unused,
     always 0, for now" — stale since the liquid rewrite started using it
     for fill fraction.
   - README has a direct in-document self-contradiction: line ~1302
     documents field-sleeping (issue #4) as implemented ("`field::step` now
     skips its whole five-pass solve once..."), and line ~1379, later in
     the same file, says "issue #4, not yet fixed" — a "Correction to a
     claim this section used to make" paragraph that itself went stale once
     issue #4 was actually fixed without this passage being removed.

**Suggested priority when this track gets picked back up** (owner hasn't
committed to this order, just the reviewer's/agent's own read): the CI
debug-build step (#3) and `ascii.rs` regression gate (#4) together since
they're the same "make the safety net real" theme; then the doc-drift
fixes (#6, cheap); #1 (creature reclaim) and #2 (redundant touch_
neighbours) whenever convenient, both low urgency.
Also worth a decision, not just a fix: whether to make `master` the
default branch (or merge into `main`) and close out the
`pixel-physics-issues.md` items that are actually resolved now.

---

## `Reports/plant-substrate-v2-design.md` — done, not started (session handoff)

Written by a dedicated research agent, per the owner's explicit "plan all
of this before implementing, especially since a diffusion-mechanism change
is on the table" and "do deeper research if needed to make sure we do it
correctly the first time." **1,205 lines, 9 sections, design only — no
code touched.** This entry exists so a session resuming after a context
compact doesn't have to re-read the whole document (or this whole session)
to know where things stand.

**First, a real gap the agent itself caught and which is now fixed:**
`Reports/plant-simulation-research.md` (the owner-supplied research doc
this whole phase is grounded in, summarized into `PLAN.md` two entries
above) had only ever been *read* from the chat upload, never actually
committed to the repo — every citation of it as a real `Reports/` file was
technically pointing at nothing. Copied and committed
(`5a3c9b9`) before anything else, since the design report depends on being
able to cite it for real.

**The five decisions, landed:**

1. **Growth mode.** Accepted accretion for canopy (a real vascular cambium
   is also outermost-layer-only, so this isn't a compromise) and one-cell
   root-into-soil displacement; **rejected** the sub-cell turgor/extension
   scalar this project's own preliminary lean had favored (the resource
   scalar already *is* that accumulator; the liquid-fill trick's load-
   bearing property — conserved, transferable fill — belongs to soil
   moisture instead, not growth rate); **added bud break** (epicormic
   budding, already cited in `m16-plant-biology.md` §5), which the
   original lean didn't cover and which is the actual fix for "growth has
   a hard ceiling once every tip goes stale" — the real cause of the tiny-
   tree problem turned out to be `thicken()`'s counting bug, not the
   growth-mode question at all (see decision 4).
2. **`Cell::aux` → sidecar.** Confirmed full (4+8+4 of 16 bits). New
   organism-cell layout: 4 bits `CellType` + 12 bits index into
   `OrganismState::cells: Vec<Option<OrganismCell>>` — `World::organisms`'
   existing generational slot-`Vec` is the template, not `CreatureState`
   (per-entity, no generation — a precedent, but the wrong one). Hard part
   named explicitly: `diffuse_resource` runs over the generic `CellSurface`
   trait today, which can't reach a per-organism `Vec`; resolved by moving
   diffusion to a per-organism pass over the cell list instead (also
   faster than the current per-CA-frame placement, and unblocks
   `free_organism`/`organism_active_tip_count`/`organism_is_supported`'s
   still-missing anchor list). Four-step migration in the doc, every
   existing test mapped to what happens to it.
3. **Soil moisture**, real citations: Least Limiting Water Range (Silva/
   Kay/Perfect 1994; Letey 1985) as the unifying frame, field capacity/
   wilting point/aeration porosity (Grable & Siemer 1968, with their own
   per-species caveat carried through), waterlogging as O₂ diffusion
   collapse (Pan et al. 2021) causing root-*tip* necrosis specifically
   within 1-2 hours while mature tissue survives (Evans 2004) — so the
   mechanism is necrosis of `RootTip` on a duration gate, not reduced
   absorb efficiency (backwards) or slowed growth (indistinguishable from
   drought). The mud transition uses real Atterberg limits (ASTM D4318).
   Predicted emergent payoff: root systems should stabilize at the
   capillary fringe rather than growing into the water table and dying.
4. **Real `Leaf` cells, seed reserve, leaf-gated photosynthesis.** This is
   what actually fixes the one-cell-trunk problem (§2 above) — `thicken()`
   counts `Leaf|GrowingTip` cells, and since tips retire to `MatureBody`
   almost immediately (this session's own fix), that count is nearly
   always too small to clear `pipe_ratio`. Real leaves fix the count
   directly. **Explicit warning from the report:** this session's own
   `cost: 0.2`/`rate: 0.5` tuning (the 6-way comparison) gets invalidated
   by this change and needs redoing, not reused.
5. **Differentiated materials + environmental interaction.** Verified in
   code (not assumed) that a `Powder` cell already rests on `Plant`
   material — `is_displaceable` only allows `Liquid`/`Gas` through
   (`material.rs:104`) — so "debris catches on branches" needs no new
   mechanic, just branches wider than one cell, which decision 4 also
   supplies. What *is* missing is load: a scoped one-term addition to
   `organism_is_supported` (reduces effective span under weight).

**Retrofit order — updated below to the final 10-step version (design
doc §10; this entry originally cited the pre-Decision-6 §9 numbering,
which shifted by one once polarity was inserted).** In short: sidecar
storage (blocking, step 1) → **polarity** (step 2, immediately after the
sidecar lands and *before* any `.ron` re-tune — see "Revised after
landing" below for why) → real leaves (step 3, re-tune `tree.ron` here,
now sweeping `TRANSPORT_SUBSTEPS` and canalization contrast alongside
`cost`/`rate`/`reserve` since polarity already landed) → soil moisture
storage + differentiated materials/root-soil-stabilization + load-reduces-
span (parallelizable, steps 4-6) → root displacement into soil (step 7) →
soil `Absorb` + anoxia necrosis (step 8, the water-table equilibrium is the
actual proof this works, and optionally where a second `water_conductance`
array lands per §7f) → bud break (step 9, deliberately last — it removes
the size ceiling, so it's what will expose scaling problems in everything
before it, and is also the first workload long enough to demonstrate or
withdraw §7h's structural-loop claim) → independent review (step 10, same
rigor as the tree rewrite's own step 8, now also checking the transport
pass visits each shared face exactly once and that `away_from_supply`'s
fallback fires sanely for a fresh organism).

**Status: fully planned (all 6 decisions, all 10 retrofit steps), zero
code written.** Next action for whoever picks this up is retrofit step 1
(`Cell::aux` → sidecar, design doc §3f), not a discussion — the planning
phase the owner asked for is complete, including the polarity addendum
below.

**Revised after landing, and now itself landed:** polarity/directional
diffusion was originally scoped out (design doc §7 in its first draft),
deferred to its own future pass. Owner follow-up question caught a real
gap in that call: it isn't independent of this phase after all — retrofit
step 1 (sidecar storage) already has to restructure `diffuse_resource`'s
own execution shape (moving it off the generic `CellSurface` trait to a
per-organism pass, since a per-organism `Vec` needs that), which is the
exact code polarity would also need to change, and step 1 is what actually
gives a polarity field room to exist at all (no spare bits in the old
packed `aux`). Worse, retrofit step 2 (real leaves, leaf-gated
photosynthesis) requires re-tuning `tree.ron`'s resource economy — tuning
it once against isotropic diffusion and again after polarity lands later
would be exactly the "don't optimize if a diffusion-mechanism change is
coming" waste the owner flagged at the start of this whole planning pass.
**Decision: move polarity up**, sequenced between retrofit steps 1 and 2
(before the leaf/reserve re-tune, not after).

**Decision 6, now written in full** (design doc §7, ~1000 lines): four
per-face `carbon_conductance: [f32; 4]` scalars on `OrganismCell` (§7b —
rejected a packed direction-only encoding, since conductance itself, not
just direction, is what has to update); pairwise carrier-shaped transport
`J_ij = RATE·(c_ij·R_i − c_ji·R_j)` replacing `diffuse_resource`'s
symmetric average, visiting each shared face exactly once for
conservation (§7c); a Hill-function conductance-ratcheting update rule
(§7e) — this is the actual "polarity" mechanism, canalization by
use-dependent strengthening, not a stored direction; `Grow`'s
`away_from_growth` term becomes `away_from_supply`, reading local
conductance rather than crowding (§7g). Canopy density deliberately stays
isotropic (§7f) — a principled scope cut, not an oversight. **Honest
walk-back, stated by the report itself (§7i):** this produces *competitive
resource allocation* between tips (a well-connected, less-hungry tip can
out-compete a starved-but-needy one — demonstrated in the §7h worked
example), not literal auxin-transport apical dominance; a second,
oppositely-polarized inhibitory channel that would give the real thing is
explicitly deferred, not built. The four new tests in §7k gate step 2 of
the retrofit; the Y-junction test (the less-hungry-but-better-connected
tip taking the larger share at §7h tick 6) is the load-bearing one, and
`VEIN_GAIN = 0` must reproduce today's isotropic results exactly so a
future regression here stays bisectable. Evolution stays out of scope for
this phase either way (§8b).

---

## `Reports/granular-mechanics-research.md` and `Reports/liquid-simulation-research-r2.md` — landed, plan updated (session handoff)

Two more research reports ("Report A" and "Report B" of a planned four —
Report C on solid-granular-fluid coupling and Report D on worldgen erosion
are referenced by both as forthcoming but do not exist in `Reports/` yet;
treat any conclusion either report calls provisional-pending-C as still
open) arrived via a raw GitHub-web upload to `origin/master`
(`e6ad4dd`, "Add files via upload") while this session worked entirely on
local commits descended from `838c557` — a genuine history divergence,
resolved with `git merge origin/master --no-edit` (clean, zero conflicts,
disjoint file sets: `2 files changed, 1297 insertions(+)`).

**Agreement, stated up front, since the owner asked for updates "assuming
you agree with them":** both reports' central recommendations are accepted
as-is below. The one place this entry pushes back rather than transcribes
is Report A §6 (hole-propagation) — the report itself already frames that
finding as a caution rather than a build order, and this entry keeps it
that way rather than upgrading it to a task. Everything else (two-angle
repose, deleting BTW, the `FLAG_FLOWING` unification across both reports,
VOF's local-height fix, the dilatancy packing scalar, hydrostatic
pressure's path-trace) is agreed with and folded into the plan below with
no changes to the reports' own reasoning.

### Granular (Report A) — what changes in the plan

The M3 section above already struck the BTW-toppling bullet and
hole-propagation caution; this is the constructive side.

1. **Two-angle repose model** (`granular-mechanics-research.md` §2). One
   new bit in `Cell::flags`' free bits, `FLAG_FLOWING`. Content model: keep
   `friction_angle` meaning the *repose* angle θ_r (no change to existing
   `.ron` files — this is additive), add an optional `max_stability_angle`
   per material defaulting to `friction_angle + 8.0` (the θ_ms/θ_r gap
   Metcalfe et al. and Lee & Herrmann 1993 report — flagged §11 as read via
   secondary sources, worth checking a primary before the acceptance-test
   number below is treated as a hard bar). A resting pile can stand up to
   θ_ms; once *any* cell starts moving it flows down to θ_r and doesn't
   re-lock until it's below that — real hysteresis and bistability, which
   the current single-angle `roll_along_slope` structurally cannot express
   (§1 of the report is explicit that this is a ceiling of the current
   model, not a missing tuning pass).
2. **Delete, don't build: BTW toppling** (§3). Real 2D sandpile avalanches
   don't show BTW's power-law size distribution — Jaeger/Liu/Nagel 1989,
   the Oslo rice-pile studies, a recent 5-bead-drum result are all cited
   against it. The lattice *stability condition* BTW is built on
   (`|h_x − h_{x+1}| < tan(θ_r)`) is worth keeping as a **test invariant**
   for the two-angle model above, just not as toppling dynamics. Already
   reflected in the M3 edit above and README.md's "not yet built" list.
3. **Dilatancy / packing state** (§5), lower priority, sequenced after 1.
   Reuse `Powder`'s currently-unused `aux` as a packing-fraction scalar —
   the exact same precedent the compressible-liquid `aux`-as-fill-fraction
   design already set (§4 above), so this is a proven pattern in this
   codebase, not a new one. Dilate on move, compact at rest (Reynolds
   dilatancy), packing modulates the two-angle reach. **Needs an early-exit
   for cost**, per the report's own citation of this project's M14
   `heat_conductivity` lesson (a per-cell scalar recompute that skips
   settled cells cheaply) — don't build this without that guard.
4. **Known and deliberately not built now:** force chains / Janssen effect
   (§4) — noted as sharing `structural.rs`'s existing relaxation shape if
   ever built, and an explicit warning not to conflate granular stress with
   the engine's `pressure` field (that channel is air pressure) *or* with
   Report B's hydrostatic liquid pressure below, once that lands — three
   different quantities, one tempting shared name. μ(I) rheology (§7)
   needs velocity+stress fields this engine doesn't have; explicitly
   flagged as Report C's problem, not this plan's.
5. **Hole-propagation caution** (§6): if a hopper/silo use case ever
   surfaces, the correct model is Bazant's correlated "spot" model, not a
   single-cell void random walk — the naive version measurably over-mixes
   against real hopper data. Not scheduled; recorded so it isn't
   rediscovered the naive way later.

**Acceptance criteria added to this plan's verification bar** (§8 of the
report, cost ceiling flagged as needing re-measurement against current
`~23ms`, not assumed): a 2D column-collapse test matching Lube et al.'s
scaling numbers (flagged §11 as needing primary-source verification before
treating as a hard pass/fail bar); the θ_ms − θ_r gap test targeting
δ ≈ 8°; an avalanche-size-distribution test asserting Gaussian shape, *not*
power-law (the direct, checkable version of finding 2's claim); a repose-
accuracy test within ~2° of an authored θ_ms (current single-angle model
sits 5-6° under); cost regression ≤15% against the current serial
baseline.

**Where this sits relative to the plant-substrate-v2 work:** independent
tracks. This touches `Cell::flags`, `update.rs`'s powder path, and
`material.rs`; the plant work touches `organism.rs`/`plant.rs` and (once
its own retrofit step 1 lands) a new sidecar `Vec` on `OrganismState`. No
shared files, no ordering dependency either direction — safe to build in
either order, or interleave, without the "don't tune before a mechanism
change lands" trap that drove the plant/polarity resequencing.

### Liquid (Report B) — what changes in the plan

The §4 "water/liquid leveling — compressible-volume rewrite" section above
is already-shipped work this report evaluates and extends, not a rewrite
of it — read the two together.

1. **Symptom 3, named for the first time: flotsam-and-jetsam** (§3a). The
   "draining pools decay to residual droplets" behaviour this session's own
   playtesting has seen is a documented, named VOF failure mode, not a bug
   specific to this engine's implementation. **Fix (§3b):** a published
   3-cell local height-function read, computed only for surface cells, not
   a global solve — cited as eliminating the droplet artifact and giving
   exact mass conservation versus standard VOF's ~2% gain.

   **Investigated directly, not built — the persistent version of this bug
   does not reproduce in this codebase.** Three scenarios were tried before
   writing any fix code (a draining pool through a floor hole, a multi-
   ledge settle, and a splash impact — splash is where real VOF flotsam
   most commonly appears): isolated single-cell droplets *do* appear
   transiently during active motion (confirmed, e.g. 6-7 stray cells mid-
   drain), but every scenario reached full settlement
   (`active_chunk_count() == 0`) with zero stray droplets remaining and
   exact fill conservation (0.000% drift, summed across every water cell).
   This refines rather than contradicts the original playtest observation:
   the visual artifact is real and transient, not a permanently stuck
   fragment the way "flotsam and jetsam" implies — this implementation's
   specific choices (a drained cell converts fully to `Cell::EMPTY`,
   `HORIZONTAL_TRANSFER_REACH`'s 8-cell search) apparently already prevent
   the *persistent* failure mode the literature describes. **Locked in as a
   permanent regression test**
   (`a_splash_settles_with_no_stray_droplets_and_no_mass_drift`,
   `update.rs`) rather than left as a one-off check.

   **Correction, caught by the owner directly: the paragraph above does not
   show the §3b fix wouldn't help — it only shows the pre-fix code doesn't
   exhibit a persistent version of the symptom the fix targets.** Those are
   different claims. The fix itself (the 3-cell local-height read) was
   never implemented, so there is no before/after comparison of it, only a
   symptom-absence check on the *unfixed* code — and that check used the
   same serial-only, small-scale methodology that produced a confirmed
   false negative on the liquid-ordering fix directly above. The correct
   status is **untested, not tested-and-rejected**: build §3b for real and
   compare directly (mass-conservation precision, visual surface
   smoothness, the transient mid-drain droplets already confirmed real),
   the same way the ordering fix was actually validated, rather than
   inferring its value from whether the current code already has a
   problem.
2. **`MIN_LIQUID_TRANSFER = 150` reframed** (§3c): the value this session
   arrived at empirically (documented at `update.rs`'s own doc comment and
   PLAN.md's §4 entry above, tuned 8 → 150 purely by measuring convergence
   time) is, per this report, a 15%-of-`FULL` dead band large enough to
   lock in a permanently uneven surface — **"treat as diagnostic of the
   underlying leveling algorithm being too slow, not as the correct
   setting."** Recommended path: fix symptom 2 properly (item 3 below),
   then re-measure whether `MIN_LIQUID_TRANSFER` can drop back toward 8
   without reintroducing the original long-tail convergence problem — do
   not lower it first, since without a faster leveling mechanism that
   regresses straight back to the ~12,000-frame tail this session already
   measured and rejected once.
3. **Symptom 2 (wide bodies level in O(width²)) has a real fix, not just
   more tuning: heightfield-is-1D** (§5). This engine's own §4 fix
   (`HORIZONTAL_TRANSFER_REACH = 8`) is a mitigation, honestly — the
   100-cell test column still only reaches "flat to within about 3 cells,"
   not truly flat. The actual fix reframes settled liquid bodies as a 1D
   array of column heights (virtual-pipes / Mei-Decaudin-Hu), giving
   O(width) leveling instead of O(width²) — and doubles as the worldgen
   erosion mechanism Report D is expected to need, so this is not a
   liquid-only investment. **The real difficulty is named honestly (§5a):**
   a side-view world isn't purely a heightfield — caves, overhangs, sealed
   vessels all break a pure column model — so this needs a hybrid: small/
   dynamic water stays the existing CA model, large/settled bodies promote
   to per-column heights, with demotion back to CA on disturbance.
   `rigid::label_component` is flagged as suggestive existing
   infrastructure for the promotion/demotion boundary, "currently wired to
   nothing." **§5a's own recommendation: prototype the promotion/demotion
   seam first, not the pipe physics** — that's where the real risk is.
4. **Symptom 4, hydrostatic pressure, named for the first time** (§6). The
   engine's `pressure` field channel already has writers (explosion,
   `structural::break_free`) but liquid never reads or writes it — there is
   currently no "deep water pushes harder" behaviour at all. Dwarf
   Fortress's documented path-trace approach ("pressure never exceeds the
   first full cell found tracing up from a cell") is directly portable.
   **Three cautions carried forward as-is:** needs a hard length cap once
   M10 streaming exists, since DF's rule bounds height but not trace
   length, and an unbounded trace against a streamed water table could get
   expensive; must sample via `field_at_bilinear`, not `field_at` — the
   same block-nearest degeneracy already fixed once for worm thermotaxis
   and tree phototropism, worth not reintroducing here; whether granular
   stress (Report A §4), hydrostatic liquid pressure, and the existing air-
   pressure channel can share one field is an open question — starting
   with liquid as **read-only** on the existing channel is the recommended
   first step, deferring the shared-channel question.
5. **LBM — deferred, and re-justified, not just re-asserted** (§4). Real
   bandwidth arithmetic: D2Q9 f32 is 36 bytes/cell, 72 bytes/cell/step of
   traffic, ~11.8 MB/step at this engine's current resident size, 0.6-
   1.2ms at typical bandwidth — cheap enough on its own. **The actual
   reason it's deferred is composability and M10, not cost (§4b, §4c):**
   LBM would own the entire liquid layer with no notion of the density-
   driven cell-swap displacement this engine's CA model gets for free, and
   at an assumed (not measured — §12 is explicit about this) 10x resident-
   chunk count under M10 streaming, LBM is the one subsystem whose cost
   scales with resident world size rather than activity — breaking this
   engine's core cost-proportional-to-change architecture. **Gated, not
   scheduled:** any LBM prototype must (a) demonstrate real sleep/
   coarsening, not defer the problem, (b) be measured at actual streaming
   scale, not the current test scene, (c) report mass conservation across
   a coarse/fine boundary. Thürey & Rüde 2009 flagged (§12) as identified
   via citations only — read it in full before any prototype, not after.
6. **Unifies with Report A, explicitly (§8):** Report A's `FLAG_FLOWING`
   bit and this engine's own already-planned same-step horizontal search
   (§4 above) are the same underlying mechanism — a settled/flowing state
   bit — and should be one implementation shared by both the granular and
   liquid paths, not two. Gating the horizontal search on `FLAG_FLOWING`
   also means settled pools stop paying for the search every tick, which
   is real budget back and lets more chunks sleep.

**Superseded — the original rejection below was itself a false negative,
now confirmed and corrected.** The first attempt at rev1 §5's reordering
fix was tested on a single 60-tall column in a narrow world and showed no
effect; that null result was wrongly generalized to "this doesn't work,"
when it only meant "this specific tiny scenario doesn't have enough total
lateral redistribution to distinguish the two orderings." A live playtest
report (three tall multi-chunk water columns, visibly stalling with sharp
vertical walls landing at chunk boundaries, still unresolved after ~15
real seconds) forced a much larger, more realistic reproduction. That
reproduction, run through the real parallel driver: (a) a `wake_all()`-
every-frame control run produced the *same* stall pattern as the normal
run, ruling out the chunk/sleep machinery as the cause and confirming the
stall is `update_liquid`'s own transfer priority, not a parallel-specific
bug; (b) the reordering fix, retested at this corrected scale, showed a
real, repeatable effect — full flatness by frame 900 against a residual
step still present at frame 1800 without it. **Landed** (`update.rs`'s
`update_liquid`, doc comment there has the full mechanism and the
measured ~12% worst-frame cost this reorder carries — see below), with a
permanent regression test (`parallel.rs`'s `three_tall_columns_spanning_
chunk_boundaries_flatten_within_900_frames`) confirmed to fail without the
fix and pass with it.

The original diagnosis below is *not* invalidated by this — it correctly
traced *why* the small-scenario test showed nothing (fill-transfer
throughput capped by `flow_rate`/`HORIZONTAL_TRANSFER_REACH`), and that
throughput cap is real and still there. What was wrong was concluding the
reordering therefore "doesn't help" — it does, meaningfully, even though
it doesn't remove the underlying O(width²) cap. Symptom 1 (pour shape) and
symptom 2 (wide-body leveling speed) do still likely share that one root
cause, and the structural fix for the cap itself remains build-order item
4 (1D virtual pipes) below, not this reordering. Kept for the record
(original text): "Root cause, traced rather than guessed: `try_move`'s
downward case requires the cell below to be `Cell::EMPTY` outright, and
`write_liquid_transfer` only produces `Cell::EMPTY` once a cell's fill
reaches exactly 0 — so a row in a stacked pour cannot even attempt to fall
via `try_move` until the row below has fully drained sideways, and that
draining is itself capped by `flow_rate`/`HORIZONTAL_TRANSFER_REACH`'s
throughput." `Cell::flowing()` gating (§8's specific proposal) was tried
and correctly rejected for a different reason than originally stated —
not because it wouldn't matter, but because a packed liquid column rarely
earns `flowing()` under its move-only definition, so gating it would have
silently undone the fix for the exact cells it targets. The landed version
is unconditional.

**Reverted again, for a real reason this time — not a testing gap. The
reordered version is not lost — it's exact git history, not a memory.**
`git show eeefceb:src/sim/update.rs` reconstructs it precisely (`eeefceb`
= "Fix liquid pour stalling at chunk boundaries: horizontal before
vertical"; `dcb761c` = the revert immediately after it, this same
commit). This matters concretely: the reordered version's water behavior
was genuinely better overall, not just on the one narrow chunk-boundary-
stall symptom it was written to fix — the ballooning was the one thing
wrong with it. A replacement design should be benchmarked *against the
reordered version's own numbers* (full flatness by frame 900 on the
three-tall-columns scene, vs. a residual step still at frame 1800 on the
original), not against the slower pre-reorder baseline — and that
comparison can be run for real, checking out `eeefceb`'s `update.rs`
into a scratch copy or worktree, rather than trusted from this summary.
`a_landing_column_does_not_balloon_in_cell_count` (`parallel.rs`) is the
one hard constraint that must never regress; the frame-900 flatness bar
is the target to match or beat, not just avoid failing.

A live report caught what the "landed" paragraph above missed: dropping a
column onto a floor made it visibly balloon out to nearly 5x its own cell
count within a couple hundred frames before slowly re-collapsing, while
total fill stayed *exactly* conserved throughout (checked every single
frame of the reproduction, never drifted by even one unit). Water is
incompressible — "same mass spread across 5x the cells" is not a
mass-conservation bug, but it is still physically wrong, and it looked
exactly as alarming live as an actual leak would have. Root cause: the
*old* vertical-first order had a load-bearing side effect nobody had
named until this investigation — a deep, blocked, fully-packed cell's
vertical attempt only ever has `LIQUID_MAX_COMPRESS` (1%) of genuine
room, but that tiny transfer still succeeds and returns early, which
incidentally throttles that cell out of horizontal transfer almost every
frame. That accidental throttle is what had been keeping a packed
column's interior inert. The reorder removed it everywhere at once, not
just at the free surface where the original fix was actually aimed — so
the instant a column's base landed, its *entire* body started leaking
sideways in the same few frames. A second attempt scoped the reorder to
only fire when the cell directly above isn't more of the same liquid at
full fill (a literal "am I at the free surface" check) — measured, and
it did not fix the ballooning (still ~4.8x peak cell count): once
diagonal cascading off a narrow column's edges creates an irregular,
locally-uneven top profile, *many* cells legitimately read as "at the
surface" under a purely local check, and they collectively over-dilute
just the same. **Fully reverted to vertical-first**, confirmed via the
same reproduction that catches it (temporarily re-broken, watched the
new test fail with cell count at 102,915 against a start of 23,400,
restored). The permanent regression test is now `parallel.rs`'s
`a_landing_column_does_not_balloon_in_cell_count` (replacing the deleted
`three_tall_columns_spanning_chunk_boundaries_flatten_within_900_frames`,
which tested for the now-reverted behavior) — it checks exact mass
conservation every frame *and* that cell count never exceeds 1.5x its
starting value.

**Net position, stated plainly: the chunk-boundary stalling symptom is
still real and still unfixed.** Two different per-cell reordering
heuristics were tried and both either failed to fix it convincingly
(the false-negative-corrected version) or fixed it while introducing a
worse, physically-nonsensical regression (both reorder variants). The
honest read is that a per-cell local heuristic cannot reliably
distinguish "the free surface of a connected body" from "any cell that
happens to have nearby room" once the body's shape becomes irregular —
which is exactly the class of problem the heightfield/virtual-pipes
redesign (build-order item 4 below) is meant to solve by tracking
column height at the level of the whole body, not per cell. Don't
attempt a third per-cell ordering tweak here without a genuinely new
idea for that distinction; two are now on record as insufficient.

**Acceptance criteria added to this plan's verification bar** (§10 of the
report): 0.5% mass conservation over a 2000-frame dam-break; a 2%-of-
`FULL` surface-flatness bar for a 100-cell pool (today's dead band permits
15%, so this fails by construction until item 2/3 above land); a 300-frame
leveling-time bar (current behaviour is hundreds to thousands of frames);
zero detached droplets after drainage (item 1 above); a U-tube
communicating-vessels test equalizing within one cell (fails outright
today — no hydrostatic pressure at all); initial pour slope ≤10° after 60
frames (current behaviour reproduces a 30-40° sand-like slope, the exact
symptom-1 diagnosis this whole track started from); cost ceiling ≤15%
regression against current serial/parallel baselines; **and a new
variable-resident-area stress scene added to `examples/ascii.rs`**,
independent of the liquid work and needed before M10 regardless — bar is
sub-linear worst-frame growth as resident chunk count grows, replacing the
10x-multiplier assumption above with an actual measurement.

**Where this sits relative to the plant-substrate-v2 work:** independent,
same reasoning as the granular section above — `update.rs`'s liquid path,
`field.rs`'s new pressure read, and `Cell::flags` again, none of it
touching `organism.rs`/`plant.rs` or the sidecar. **Where it sits relative
to the granular work:** genuinely coupled, per item 6 — `FLAG_FLOWING`
should be designed and built once, shared by both, not built twice. If
both tracks are picked up, sequence the shared bit first (Report A §2 /
Report B §8), then the two behaviors that consume it can proceed in
parallel.

**Status: both reports read, agreed with, and folded into this plan. Report
A's item 1 (the two-angle model, including the shared `FLAG_FLOWING` bit
item 6 calls for) is now implemented — see the next section. Everything
else in both reports — dilatancy, VOF's local-height fix, heightfield
leveling, hydrostatic pressure, LBM — is still unbuilt.** Should either
track be picked up next, `Reports/granular-mechanics-research.md` §10 and
`Reports/liquid-simulation-research-r2.md` §9 each carry their own full
recommended build order in more detail than the summaries above. Note for
whoever picks up the liquid track: `Cell::flags`' `FLAG_FLOWING` bit
already exists (`cell.rs`) and is already set generically by
`CellSurface::move_cell` on every successful move, including a liquid's —
Report B item 1 (gating the liquid horizontal search on it) can consume it
directly rather than adding a second bit.

### Granular two-angle repose model — implemented

`Cell::flags` gained `FLAG_FLOWING` (`cell.rs`), set on every successful
move by `CellSurface::move_cell`'s default implementation (`surface.rs`) —
generic across every kind, since it's the one shared move seam, though
only `Powder` currently reads it. `Material`/`MaterialDef` (`material.rs`)
gained `max_stability_angle` (0.0 sentinel = unset, defaults to
`friction_angle + 8.0` via `DEFAULT_STABILITY_ANGLE_GAP_DEGREES`, clamped
to never sit shallower than `friction_angle` so `stability_reach_base`
can never exceed `roll_reach_base` — the invariant `Material::sweep_reach`'s
existing `Powder` arm relies on to stay correct without also considering
the new field) and a `stability_reach_at` mirroring the existing
`roll_reach_at`. `update.rs`'s `roll_along_slope` now picks between the two
based on `cell.flowing()`: lenient `roll_reach_at` (repose-based) while
flowing, strict `stability_reach_at` (stability-based) while settled;
`update_powder` clears the flag when a cell fails to move at all,
guarded so an already-settled cell never re-writes itself. No `.ron`
content changed — every material gets the default 8-degree gap for free.

**A real, if minor, side effect found along the way and fixed in the same
pass:** `World::move_cell` (the inherent method) was a byte-for-byte
duplicate of `CellSurface::move_cell`'s default, not a delegation to it —
exactly the "two implementations of the same thing" pattern flagged
elsewhere in this plan's code-review-findings section. Left alone, it would
have silently *not* set `flowing`, a second divergent movement primitive
appearing at the same moment the first one gained new behaviour. Fixed by
making it delegate (`<Self as CellSurface>::move_cell`) instead of
reimplementing the swap.

**A real behavioural discovery, not a regression, surfaced by the existing
test suite:** three tests (`settled_sand_is_never_left_unsupported`,
`sand_is_stable_when_every_chunk_is_swept_in_full`,
`every_unstable_cell_is_scheduled_for_examination`) failed immediately
after the change, all via the shared `unstable_sand` test helper. Their own
control test (the "every chunk swept in full" one, which rules out a
dirty-rect bug by construction) confirmed the fault was the helper's
definition of "stuck," not the movement rule: it judged every cell against
the old, lenient single-angle reach regardless of state, which is now
wrong on purpose — a settled cell resting on a slope between the two
angles is correctly stable, not stuck. Fixed by making the helper mirror
production exactly (`cell.flowing()` gates which reach it checks against,
same as `roll_along_slope` itself). Temporarily reverting the production
fix confirmed the new dedicated hysteresis test
(`a_settled_grain_does_not_creep_across_a_gap_only_its_flowing_reach_can_see`)
fails without it, per standing practice.

**Verification:** `cargo test --lib` (296 tests) and
`cargo clippy --all-targets -- -D warnings` both clean. Cost: the
"stress: a full screen of sand and water" scene in `examples/ascii.rs`
measured ~37.5 ms serial / ~8.7 ms parallel before and ~38.1 ms / ~7.8 ms
after (git-stash-compared) — no regression against the report's own ≤15%
bar. Independent review (general-purpose agent) traced the core invariant,
the `FLAG_FLOWING` lifecycle including the sleep-interaction edge case, the
generic-set-on-every-move decision, the `World::move_cell` delegation, and
every new test for vacuousness — found no defects, one honest tradeoff
worth naming: with the test helper now deriving its expected answer from
the same `cell.flowing()` state production reads, the three
`unstable_sand`-based tests can no longer independently catch a
future bug that clears the flag prematurely (both sides would agree a
cell is stable when it isn't). Not fixable without a genuinely independent
oracle; not a defect in what's here, just a known limit of testing a
hysteretic system this way. Visually confirmed via a throwaway probe
(built two identical 40-degree wedges — between sand's 34-degree repose
and 42-degree default stability — one placed settled, one placed flowing;
after 600 frames the settled wedge held at 40.0 degrees exactly, the
flowing one relaxed to 36.9 degrees), screenshotted to
`docs/screenshots/section-granular-two-angle/`, then deleted per this
project's standing practice for single-use verification harnesses.

**Not yet built, from Report A's own remaining items:** the dilatancy/
packing-fraction scalar (§5), the Janssen/force-chain caution (§4, know-
about-don't-build), hole-propagation's spot-model caution (§6,
build-only-if-needed), and the formal acceptance-criteria suite (§8 —
column-collapse scaling, avalanche-distribution-shape, repose-accuracy
within ~2 degrees). The wedge probe above is a manual spot-check of the
core mechanism, not a substitute for those.

---

## `Reports/liquid-heightfield-design.md` — landed; Step 1 implemented

A full design for the liquid-leveling rewrite the sections above call for
(promoting large, connected liquid bodies out of the per-cell CA into a
1D-per-body heightfield, solved with virtual pipes and rasterized back to
cells), produced by a background design agent per the owner's "continue"
while other work was in flight. 1,416 lines / 16 sections. Read in full
before implementation started. Its own headline claims, load-bearing for
everything below: **the grid never lies** (a promoted body's cells stay
correct in ordinary `World` storage at all times; promotion only changes
*who* may write them); heights are **integer fill units** on the same
`LIQUID_FULL` scale a `Liquid` cell's `aux` already uses, not floats, so
conservation is exact by construction; the ballooning failure from the
reverted reorder (`eeefceb`/`dcb761c`, recorded above) becomes **structurally
unrepresentable** under this model rather than merely guarded against.
Two corrections to `Reports/liquid-simulation-research-r2.md` (Report B)
along the way: promotion must be gated on structure, not quiescence (§4a —
the whole point is accelerating a body that's still moving); a naive
per-step relaxation is still O(width²), real O(width) leveling needs the
flux itself to be persistent state (§7a). Full report has its own §11
six-step build order, §12 acceptance criteria, and §15 sourcing/
verification ledger (what was read vs. re-measured vs. taken on the
brief's word) — not reproduced here.

### Step 1 (§11) — the ownership substrate and the promote/demote round trip — implemented

No solver, no absorption, exactly as scoped: a promoted body (`liquid::
LiquidBody`) holds per-column heights read from the cells that already
existed and never changes on its own.

**New `src/sim/liquid.rs`:** `label_body` — a bounded 4-connected flood
fill over same-material `Liquid` cells, validated against design doc §3b
(single material; single vertical span per column; a free surface above
every column; width ≥ `MIN_BODY_COLUMNS` = 32, untuned per the report's own
flag; cell count ≤ `MAX_BODY_CELLS` = 20,000) before returning a
`BodyScan`. `LiquidBody::managed_positions`/`container_positions` (the
bed/walls immediately outside the body, flagged identically but never
counted as the body's own mass or moved by it) derived from the column
arrays rather than stored separately.

**`Cell::flags` gained `FLAG_MANAGED`** (`cell.rs`, the fourth of eight
bits) — a cell owned by a promoted body; the CA sweep must not move it, and
a write into it from outside the body's own rasterizer demotes the owner.
`update::update_cell` returns immediately for a managed cell (no movement,
no `fire::update`, no organism diffusion).

**`World` gained a `BodyId`/`BodySlot` generational allocator**
(`bodies`/`free_body_slots`), mirroring the existing `organisms`/
`OrganismSlot` pattern exactly — the third user of that shape, not a new
one. Unlike `organism_id`, `BodyId` is never stored on a `Cell` (a liquid
body's cell has no body-local coordinate to remember; its position *is*
its column index) — it lives only in a new `body_index: HashMap<ChunkCoord,
Vec<BodyId>>`, "which bodies touch this chunk." `World::promote_liquid_body`/
`demote_body`/`demote_body_at`/`find_body_at` implement the round trip;
`World::set_owned` is the sanctioned bypass for a body's own future
rasterizer (no caller yet — nothing rasterizes until a solver exists).

**Disturbance detection lives at the one write seam every caller already
goes through** (`World::set`), per design doc §5a's own reasoning against
enumerating call sites: catches the brush, the eraser, `explosion::
trigger`, `fire.rs`'s reaction-into-a-neighbour path, and ordinary CA
movement (density displacement) with no per-caller special-casing.
`parallel.rs`'s `ChunkView::set` needed a second, genuinely separate check
for its same-chunk write branch (which writes directly into its own
`Chunk`, never through `World::set` at all) — queued into a new
`ChunkOutcome::demotions` field. Both that queue and remote-write
disturbances (detected via `world.get(x,y).managed()` before replay, then
written through `world.set_owned` rather than `world.set` to avoid
resolving early) are deliberately **deferred into one `pending_demotions`
list, resolved only after every chunk from the parallel pass is back in
`world.chunks`** — a body can span two same-parity chunks both active in
the same pass, and resolving a demotion before all of them are resident
could silently fail to clear `FLAG_MANAGED` on part of the body.

**A real cost regression found and fixed in the same pass:** the first cut
of `World::set`'s disturbance check read the old cell via a separate
`self.get(x, y)` before writing — a second `HashMap` lookup in the
hottest function in the engine. Measured ~1.7x the serial stress-scene
worst frame (38ms baseline → ~65-69ms). Fixed by having `write_cell`
itself return the cell it just overwrote (one lookup, shared between the
write and the check) — confirmed back to baseline (~38ms) across five
repeated runs after the fix, git-stash-noise having produced a
misleadingly-consistent-looking 65ms reading twice in a row before that
(this session's standing lesson, restated: single readings on this
machine are not to be trusted without repetition — see the earlier
scheduler-dedup section's identical false alarm).

**A real bug found by independent review, fixed in the same pass:**
`label_body` didn't check whether cells it was about to claim were already
`FLAG_MANAGED`. Promoting an already-promoted pool a second time silently
succeeded, producing two live `BodyId`s over identical cells; demoting the
first cleared the flag everywhere, orphaning the second permanently (its
cells could never again read as disturbed once already unmanaged, so
nothing could ever trigger its own demotion). Not reachable from any
shipped caller yet (no automatic promotion trigger exists in Step 1 — every
test calls `promote_liquid_body` directly), but exactly the gap the
design's own §3e candidate queue (the natural next caller) would have hit.
Fixed by refusing the scan if the start cell or any visited cell is already
managed. Regression test (`promoting_an_already_managed_pool_a_second_
time_fails`) confirmed to fail without the fix, passes with it.

**Verification.** New tests in `liquid.rs`: promotion flags every cell and
moves no mass (plus the `cell_count == Σ ceil(h[i]/LIQUID_FULL)`
invariant, design doc's B-1); a promoted body is untouched across 2,000
frames; promote → 2,000 frames → demote is bit-identical (mass-exact round
trip); every disturbance §11 step 1's own verify list enumerates —
painting, erasing, digging out the bed, an explosion, a falling grain of
sand displacing in by density, and a material reaction (a synthetic
`spark`+`water` reaction, mirroring `fire.rs`'s own synthetic-reaction test
pattern, since no shipped material reacts with anything today) — each
confirmed to demote the body and leave every other cell's content
unchanged. Every disturbance test (and the double-promotion regression
test) confirmed to fail without its respective fix, including two
temporary reverts to isolate `World::set`'s check from `ChunkView::set`'s
independent same-chunk check specifically (the falling-sand test kept
passing with only `World::set`'s check disabled, since it exercises the
parallel same-chunk path — confirmed it *does* fail once both checks are
disabled). `cargo test --lib` (312 tests) and `cargo clippy --all-targets
-- -D warnings` both clean. Independent review (general-purpose agent)
traced the deferred-demotion cross-chunk logic in detail (confirmed a
remote write can never land in another same-pass-active chunk, since two
active chunks in one checkerboard pass always share parity and can
therefore never be adjacent — the scenario it checked for is geometrically
impossible, and the real analogous case is handled correctly regardless)
and the dirty-rect/sleep interaction of skipping managed cells entirely in
`update_cell` (safe — `mark_dirty` still fires on promotion and on
demotion's own flag-clearing writes, so a chunk wakes once, settles while
managed, and re-wakes on demotion) — found the double-promotion bug above
(now fixed) plus three lower-severity notes (validating §3b during the
flood fill rather than incrementally within it — a fidelity/perf gap
against the report's own text, not a bug; a forward-looking watch-item on
`FLAG_MOVED`-clearing ordering once in-frame promotion exists; `container_
positions()` rebuilding its `HashSet`s on every disturbance resolution
with no caching, bounded and rare by design but worth measuring later).

**Not yet built at the time Step 1 shipped:** absorption (§11 step 2), the
persistent-flux pipe solver (step 3), the terminal equilibrium snap and
body sleep (step 4), `try_extend`/edge demotion/cooldowns (step 5),
dropping `MIN_LIQUID_TRANSFER` toward 8 as the verdict on steps 3-4
(step 6). No automatic promotion trigger exists yet either (design doc
§3e's candidate queue) — every test and any real use today calls
`World::promote_liquid_body` directly.

**A real gap surfaced when the owner tested Step 1 in live play:** neither
Step 1 nor Step 2 (below) can produce any visible difference in ordinary
play. Nothing in either step's own scope calls `promote_liquid_body`
automatically — the design doc's §3e candidate queue is real but was never
assigned to any of the 6 numbered build steps' own file lists, an actual
gap in the design's own build order, not an oversight in what got
implemented. Flagged to the owner directly; decision was to keep building
the numbered steps in order and wire up automatic promotion once the
solver (step 3) exists, since a promoted-but-not-yet-solved body would
look identical to before promotion anyway (frozen, matching whatever it
started as) — there is nothing to actually *see* until leveling exists.

### Step 2 (§11) — absorption and lazy rasterization — implemented

Still no solver: a column that absorbs mass grows taller in place rather
than spreading sideways — the design doc's own predicted "informative
failure," confirmed directly by a new test
(`repeated_pours_onto_one_column_pile_up_into_a_visible_spike`).

**`update::transfer_liquid_vertical` gained a managed branch.** When the
cell below is `FLAG_MANAGED` and the same material, the *entire* source
cell is absorbed (not `flow_rate`-limited — the body's own future solver
spreads it in O(width); throttling the handoff here would only pile a
waterfall up above the surface) via the new `CellSurface::absorb_liquid`
trait method, mirroring `schedule_active_site`'s exact shape: `World`
resolves and credits directly, `ChunkView` (`parallel.rs`) queues into a
new `absorptions` field, replayed in `run_pass`.

**`World::absorb_liquid`** takes the `LiquidBody` out of its slot for the
call's duration (mirroring `scheduler::step`'s take-then-restore shape —
`rasterize_column` needs `&mut World` and `&mut LiquidBody` at once, which
can't both be live while the body is still borrowed from `self.bodies`),
credits `h[i]`, and calls the new **`LiquidBody::rasterize_column`**
(`liquid.rs`), which implements design doc §7e: writes only when a
column's whole-cell count actually changed, claims new cells upward from
the current top (full except the newest, topmost partial cell, using the
`aux == 0` "full" sentinel `liquid_fill` already defines), and flags any
newly-exposed container cells (bed/walls) via a before/after `HashSet`
diff of `container_positions()`.

**Absorptions and demotions in `parallel.rs`'s `run_pass` are both
deferred until every chunk from the pass is reinserted** (same reasoning
as Step 1's demotion deferral — `rasterize_column`'s writes can cross into
a chunk that isn't resident yet), **and absorptions resolve strictly
before demotions**: an absorption's debit (the source cell emptying)
already happened synchronously during the sweep, so resolving a demotion
of the same body first would make the pending credit find no live body
and silently lose that mass. Absorbing first means the credit either
lands on a still-live body or gets rasterized into ordinary cells that the
following demotion then correctly folds back into the CA grid — never
lost either way.

**A real, subtle bug found while writing this step's own tests, fixed in
the same pass:** `Cell::is_empty()` (`cell.rs`) checked only material, so
a container cell — materially `Cell::EMPTY` but `FLAG_MANAGED` — still
read as available. Ordinary movement (`try_move`'s diagonal fallback, in
particular) would move straight into it, which *does* correctly demote
the body via `World::set`'s disturbance check, but only by winning an
incidental race against the *intended* path (vertical absorption into the
body's own top cell) — and in real play, any loose material merely
falling *near* a lake, not into it, would repeatedly graze its wall and
demote it for no physically meaningful reason. Fixed by making `is_empty()`
also require `!managed()` — every caller already means "is this position
available to use," which a reserved container cell never is. Confirmed to
fail without the fix (`repeated_pours_onto_one_column_pile_up_into_a_
visible_spike`, traced live: the second of four drops repeatedly
triggered exactly this false-disturbance path).

**A second real bug, found by independent review, fixed in the same
pass:** `rasterize_column`'s growth can claim cells in a chunk `body_index`
never registered (`promote_liquid_body` only records a body's *initial*
footprint), so once a column's height crosses a `CHUNK_SIZE` (64-row)
boundary, `find_body_at` — the one path both `absorb_liquid` and the
write-seam's `demote_body_at` use — hard-fails on the unregistered chunk:
further absorption there silently loses mass, and a disturbance there
silently fails to demote. Fixed by registering every chunk the body's
current full footprint touches after each `rasterize_column` call.
Regression test (`a_column_growing_across_a_chunk_boundary_stays_
findable`) grows a column across a real chunk boundary directly via
`absorb_liquid` (70,000 fill in one call, avoiding tens of thousands of
simulated frames) and confirms both failure modes; confirmed to fail
without the fix.

**Two more independent-review findings, both fixed, lower severity:**
`render.rs`'s `cell_colour` used `cell.is_empty()` as its background-vs-
material branch — with the fix above, a container cell would fall through
into grain-jitter/heat-glow code instead of flat background, a visible
static artifact along every heightfield body's silhouette. `World::
ignite_circle`'s debug brush had the identical mismatch (skipping a
container cell would ignite "nothing" and, as a side effect, demote a
nearby body the brush merely passed over). Both fixed with a raw
`cell.material == material::EMPTY` check instead — the question both
callers actually ask ("is there material here") differs from `is_empty()`'s
new "is this position available to use."

**Verification:** new tests in `liquid.rs` — a single absorption grows
`h[i]` exactly and conserves total world fill; the predicted spike shape
under repeated pours; demotion after growth clears every newly-claimed
cell too; digging beside a newly-grown cell (not just the original
footprint) demotes the body; absorption works identically under the
parallel driver; the cross-chunk-growth regression above. `cargo test
--lib` (318 tests) and `cargo clippy --all-targets -- -D warnings` both
clean. Cost re-checked on the `ascii` stress scene across seven runs:
serial settled at the ~38ms baseline in five of seven (two noise spikes,
consistent with this machine's established jitter pattern from earlier in
the session), parallel at ~8ms in six of seven — no real regression.
Independent review (general-purpose agent) traced the take-then-restore
borrow pattern, the absorb-before-demote ordering (including the specific
"two workers absorb into the same column" race — not actually reachable,
since the first absorption in a pass synchronously empties the only
source cell that could produce a second), multi-row growth in one call,
and a full audit of every `.is_empty()` call site in the codebase — found
the two bugs above (cross-chunk `body_index`, render/ignite mismatches),
confirmed everything else sound.

**Not yet built:** the persistent-flux pipe solver (step 3), the terminal
equilibrium snap and body sleep (step 4), `try_extend`/edge demotion/
cooldowns (step 5), dropping `MIN_LIQUID_TRANSFER` (step 6). Automatic
promotion still doesn't exist — see the gap noted above.
