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

## Contents

Context · Stack · Non-negotiable architecture invariants · Milestones
(M1–M11) · Second phase (M12–M18) · Third phase — emergent world
architecture (priority order, issues backlog, M10 worldgen redesign) ·
Execution order · Overall verification · Progress log (split into
[`PLAN-log.md`](PLAN-log.md)) · M19 — Visual polish · Scientific accuracy
(M16, M18) · Code review findings · five session-handoff sections, each
carrying a *(State …)* line under its heading.

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

Versions were verified current as of Aug 2026, when this table was a plan.
The **State** column records what actually happened — this table used to
read as a list of what the project uses, and five of its rows were never
added to `Cargo.toml` (three of them declined on purpose; see README's M8
status for the reasoning).

| Concern | Crate | State | Notes |
|---|---|---|---|
| Window + pixel buffer | `pixels` 0.17 + `winit` | in use | Purpose-built: "a tiny library providing a GPU-powered pixel frame buffer." wgpu underneath, so raw wgpu is the escape hatch for custom shaders in M6. |
| Parallelism | `rayon` | in use | Drives the checkerboard chunk passes. |
| Rigid bodies | `rapier2d` 0.35 | declined so far | M8's chunk bodies shipped with no constraint solver, deliberately — gravity, a grid fit test and a settle rule needed none. Still the crate if a real solver is ever wanted; its `enhanced-determinism` cannot combine with `parallel` (`Reports/coupling-research.md` §0.2). |
| Material data | `serde` + `ron` | in use | RON is more readable than JSON for nested config and supports enums natively. |
| Hot reload | `notify` | in use | Filesystem watching for material files. |
| Scripting (M11) | `mlua` 0.12 | planned | Lua 5.4 via the `lua54` feature. (This row used to say M9 — an older numbering; M9 became the gnome.) |
| Triangulation (M8) | `earcutr` 0.5 | declined so far | Port of MapBox earcut. Same story as rapier2d: the pipeline stopped before triangulation. (Row used to say M7.) |
| Math | `glam` | declined so far | Standard; rapier uses it. Nothing shipped has needed it. |
| Profiling | `puffin` | never added | "Add at M4 — you cannot optimize threading you can't see." M4 and M5 shipped without it; worst-frame timing in `examples/ascii.rs` turned out to be the profiler this engine actually uses. |
| Image encoding | `image` 0.25 | in use, unplanned | PNG/GIF for the framebuffer dump and `filmstrip` — the capture path that exists because OS screen capture cannot see this app's swapchain (README M14 status). |

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

### M5 — Multithreading ✅ done
`rayon` over a **4-pass checkerboard**: chunk `(cx, cy)` belongs to group `(cx % 2, cy % 2)`. Two chunks in the same group are ≥2 apart on some axis, so they're never adjacent *including diagonally* — which matters because a chunk update writes into its 8 neighbors (sand falling across a boundary).

**This is the one `unsafe` seam in the codebase.** A single function hands out non-overlapping mutable 3×3 chunk neighborhoods. Requirements: document the invariant precisely, wrap it in a safe API, test under **Miri**, and never let `unsafe` leak elsewhere.

**Verify:** identical visual behavior single- vs multi-threaded. Near-linear scaling in `puffin` up to core count. Miri clean. Stress test: fill the screen with sand, confirm no corruption or lost cells.

---

### M6 — Rendering upgrade ⏸ deferred
Dirty-region-only texture uploads. Custom wgpu pipeline for emissive lighting (fire/lava) and bloom. Drop below `pixels`' simple blit here if needed.

**Verify:** upload bandwidth scales with activity, not world size. Fire visibly lights nearby terrain.

---

### M7 — Free particles ✅ done
Off-grid pixels with float position/velocity for explosions and splashes, converting back to grid cells on landing. Separate system from the CA grid — Noita does exactly this.

**Verify:** an explosion throws debris that re-integrates into terrain on impact.

---

### M8 — Rigid bodies *(largest single milestone — treat as its own project)* ⚠️ started: chunk bodies built
Pipeline per Noita: **connected-component labeling** on pixel clumps → **marching squares** contour → **Douglas-Peucker** simplification → **`earcutr`** triangulation → rapier2d collider. Each frame: erase the body's pixels from the grid, step rapier, re-rasterize at the new transform.

**Known pitfall to design for up front:** rotated bodies no longer align to the grid and leave gaps that sand leaks through (raised explicitly in [FallingSandSurvival#4](https://github.com/PieKing1215/FallingSandSurvival/issues/4)). Rasterize by *inverse-mapping* each destination pixel into the body's local space rather than forward-mapping source pixels, and dilate slightly.

**Verify:** cut a chunk of terrain free → it detaches, falls, tumbles, and sand piles on top of it correctly. No leaking at any rotation.

---

### M9 — Character physics ✅ done — the gnome
Player as a kinematic body with sand-aware movement: walking on debris, being buried, swimming in liquid.

**Verify:** player can be buried by a sand dump and dig out; swims in water; stands on a tumbling rigid body.

---

### M10 — Infinite streaming world ⚠️ worldgen done, streaming not started
Seeded noise-based chunk generation on a background thread; LRU unload with serialization to disk (RLE-compress — pixel worlds compress extremely well).

**Verify:** walk 10,000 cells in one direction and back; terrain is unchanged; memory stays flat; no hitching at chunk boundaries.

---

### M11 — Lua gameplay scripting (not started)
`mlua` for spells, entities, and scripted reactions on top of the data-driven material layer.

---

## Second phase — fire, explosions, life and structures

These five features look independent and are not: three of them need the same
two pieces of infrastructure, and building that first turns three hard features
into three moderate ones.

### M12 — Widen `Cell` to 8 bytes *(prerequisite for almost everything below)* ✅ done (widened twice, to 12)

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

### M13 — Coarse field grid *(the biggest new system, and it pays for four features)* ✅ done

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

### M14 — Fire, heat and reactions *(finishes M3)* ✅ done

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

### M15 — Explosions *(needs M7 free particles)* ✅ done

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

### M16 — Active sites, then plants ✅ done

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

### M17 — Structural integrity *(destructible building with no solver)* ✅ done

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

### M18 — Creatures ✅ Phase 1 done

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
  **Correction, from `Reports/population-dynamics-research.md` §4a:** true as an
  engineering statement, dangerous as an ecological one. An entity that perceives
  arbitrarily far and moves toward what it perceives has *effective* mobility far
  above its step size, and mobility past a sharp critical threshold destroys the
  spatial structure that coexistence depends on (Reichenbach, Mobilia & Frey,
  *Nature* 448:1046). **Constrain perception range for ecological reasons even
  though nothing technical requires it**, and treat perception radius × movement
  rate as one combined stability parameter, measured rather than guessed.

### M18 Phase 2's species set: a cycle, not a chain

**Re-shaped from `Reports/population-dynamics-research.md` before any creature
`.ron` is written, because it determines what they contain (§12).** The original
sketch — a worm and something that eats worms — is the configuration Gause showed
goes extinct *regardless of starting population*, and Huffaker only rescued it by
engineering spatial structure **and** handicapping the predator's dispersal. A
linear food chain has a top predator checked by nothing but starvation; a
non-transitive cycle has every species checked by another, and coexists on a
lattice where a two-species chain does not (§6).

**Proposed cycle, using only mechanisms that already exist** — §6 notes a
material-mediated interaction is both more interesting than "eats" and cheaper,
since the substrate already does the work:

| species | does | loses to |
|---|---|---|
| **worm** | burrows loose powder, loosening compacted material behind it | *binder* — hardened substrate is unburrowable |
| **binder** | eats loose powder, excretes a compacted variant (higher `friction_angle`/`density`, which `roll_along_slope` and the two-angle repose model already turn into behaviour) | *borer* |
| **borer** | eats binders, but can only travel through compacted material | *worm* — loosened ground strands it |

A → B → C → A, expressed entirely through material properties the engine already
simulates. **The physical refuge falls out for free**: loose sand is the worm's
refuge from the borer, which is §3's preferred reservoir mechanism (a burrow the
predator cannot follow into) rather than an off-screen immigration hack.

**Species identity needs owner sign-off before implementation** — the cycle
*structure* is the report's recommendation; these three particular creatures are
this plan's proposal, not the report's.

**Design rules that come with it, all from the same report:**

- **Prey must disperse better than predators** — not equally, better. The single
  most load-bearing parameter in the system (§2), and it must be **asserted as a
  property of the `.ron` data** so a well-meaning tuning change cannot silently
  invert it (§9c).
- **Cap mobility at half the measured threshold.** Sweep combined mobility, find
  where persistence falls below 50%, ship at no more than half of it — the
  transition is sharp, so margin is cheap insurance (§9b).
- **Density-dependent predator mortality**, the cheapest defence against the
  enrichment problem below, and something the existing energy budget nearly
  expresses already (§5).
- **Acceptance is an ensemble, never a single run** (§8, §9a): all species alive
  at 100,000 frames in ≥80% of 20 seeds. Extinction is stochastic; a parameter set
  with a 30% extinction rate looks fine three times and then fails in front of a
  player. This is the same finding the plant work hit independently — twelve
  identical trees span a five-fold size range (`examples/plant_probe.rs
  -- trees=12`) — so **one persistence-testing harness should serve both**, per
  §12, rather than growing two.

### Standing note: everything else getting better makes the ecology less stable

`Reports/population-dynamics-research.md` §5, recorded here because it is exactly
the cross-system interaction that produces a week of misdirected debugging.
Rosenzweig's paradox of enrichment: raising the prey's carrying capacity raises
the amplitude of population cycles, and amplitude crossing zero is extinction —
with no atto-fox to rescue it, since an individual-based grid has extinction as a
genuine absorbing state (§3).

**Every improvement on this roadmap is an enrichment event.** Working plants mean
more prey food; fixed water levelling means more habitable area; worldgen with a
water table means a richer world everywhere. The plant work now in progress is
one. So the ecology will get *less* stable as the engine gets *better*, and the
failure will be attributed to whatever shipped most recently.

**Two corrections to the report, verified against this codebase:**

- **§7a (chunk sleeping) is already decided, not open.** The report asks whether
  creatures keep their chunk awake or have timers advanced on wake, warning that
  either silently creates a perfect refuge or a silent extinction. `scheduler.rs`'s
  own module doc settled it: the active-site schedule is *explicitly independent
  of chunk sleep state*, so creatures and plants tick in sleeping chunks. Record
  it; don't re-decide it.
- **§7d (per-chunk RNG) names the wrong generator.** `Chunk::rng` is seeded from
  chunk coordinates, but organisms and creatures never touch it — it is reached
  only by the CA sweep through `CellSurface::rng()`. Both `plant.rs` and
  `creature.rs` drew from the single shared `World::rng`, whose real defect is
  *order coupling*: every organism's sequence depends on how many draws every
  other caller made first. The recommendation (a per-organism stream) was right
  for the wrong reason. **Done for plants** (`rng::stream`); `creature.rs` still
  draws from `World::rng` and should move the same way before Phase 2 breeding.

**§7c is real and still open:** `World::push_creature` guards a `u16` overflow
with a `debug_assert`, and CI runs `--release` exclusively, so it is never checked
anywhere; in release the assert vanishes and `(len - 1) as u16` silently wraps, so
creature 65,536 *becomes* creature 0 — two creatures sharing one state slot,
presenting as a creature behaving erratically. `creatures` never shrinks, so with
breeding this is a hard limit on **cumulative births**, not live population. Fix
with the same free-list `free_organism` needs, and do them together (§7c) — the
plant work's Decision 2 is the pass that builds it.

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
- **Appearance should eventually be a readout of physical state, and
  heritable — later goal, stated by the owner.** Colour today is authored
  data: a species declares a palette band range and an individual draws one
  band inside it (`Reports/plant-appearance-design.md` §3). The end state is
  that what a plant *looks* like is derived from what is true of it —
  foliage hue from nitrogen/chlorophyll status and light history, autumn
  colour from the temperature channel, bark from age and thickness, pallor
  from drought — so a sick plant looks sick without a rule that says so.
  **Two concrete blockers, recorded now because they are cheap to fix early
  and expensive later:**
  1. ~~The individual's band is keyed on `(world seed, germination
     coordinate)`, so colour as it stands cannot evolve.~~ **Closed by the
     genome re-map** (below): both bands now derive from discrete alleles
     and are inherited and mutable. It rode along with the root-trait
     widening exactly as this entry asked. Only the *positional founding
     draw* survives, and only for a first generation — which is what keeps
     a fresh stand mixed rather than uniform.
  2. A derived colour has to stay *legible* — the same trap
     `CLAUDE.md`'s debug-overlay rule records. A hue that is a continuous
     function of four physical channels converges on mud across a stand;
     the band structure exists precisely so variation is visible, and a
     physical derivation should pick a band and modulate *within* it rather
     than replacing it with a free-floating colour.
  Not scheduled. Wants the light/temperature economy and a real heritable
  genome under it first.

- **The heritable genome's slot map is settled, and slots are positional
  forever.** Signed off 2026-08-18, four calls made:
  `Reports/plant-genome-design.md` §5 (the map) and §9 (the calls) are the
  contract; §8a records what has been measured against it. Nine continuous
  slots and six discrete loci:

  | slot | continuous trait | | locus | discrete gene |
  |---|---|---|---|---|
  | 0 | shoot branch chance | | 0 | leaf economy (2) |
  | 1 | root branch chance | | 1 | branch angle (3) |
  | 2 | shoot plastochron | | 2 | internode (3) |
  | 3 | turgor per cell | | 3 | sympodial (2) |
  | 4 | pipe ratio | | 4 | tropism (2) |
  | 5 | root tropism gain | | 5 | wood density (3) |
  | 6 | root:shoot allocation bias | | | |
  | 7 | stomatal closure point | | | |
  | 8 | root penetration force | | | |

  **The slot index selects which stored draw a trait reads, so renumbering
  one silently rewrites every genome ever measured.** Retire a dead trait
  by setting its width to `0.0`, never by removing its slot. The one
  exception, and it is spent: *a slot dead by measurement in every species
  may be re-purposed once, with the measurement record re-baselined* —
  slots 1 and 5 used it at this re-map (they were `upward_weight` and
  `light_weight`, both measured flat across 1,024 genomes). Neither may be
  re-purposed again.

  Two consequences that are not obvious from the table: **colour is now a
  readout of two of these genes** (foliage band = leaf economy, bark band =
  wood density), which closes blocker 1 above; and **slot 1 is known not to
  reach the world** — its consumer sits behind a carbon gate the root
  economy clears twice in twelve thousand frames — so it is measured, not
  assumed, and its disposition is open (`plant-genome-design.md` §8a).

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
- **What catch-up must contain, ecologically:
  `Reports/ecological-lod-design.md`** (recommendation, awaiting sign-off).
  Three tiers — cell, individual, patch — each a lossy projection of the one
  below, with conserved currencies (water, biomass, one nutrient) crossing
  every boundary. Answers `population-dynamics-research.md` §7b, which stood
  flagged-but-unanswered: **freeze individuals, advance the fields and the
  patch tier, quantize populations to integers.** It does not reorder the
  sequencing below — it says what the catch-up step has to *do*. Two
  consequences worth carrying into the roadmap: chunk sleeping becomes a
  **deliberate refuge tier** (the dispersal handicap the ecology needs, per
  that report's §2, rather than the accidental one its §7a describes), and
  field sleeping (issue #4) is the **diversity budget** — field channels are
  niche axes, and the axis count is what decides whether the world supports
  several plant strategies or exactly one winner.
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

Split into [`PLAN-log.md`](PLAN-log.md) (2026-08-21) — the append-only
session record, kept under its own roof so this file stays readable whole.
**Append new progress entries there, not here.**

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
*(State 2026-08-21: still parked.)*

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
*(State 2026-08-22: overtaken — the `plant-substrate-v2` branch built it, and it is **merged** now. See `Reports/README.md`'s Plants section, and `Reports/open-bugs-handoff.md` §A for what the merge left unsettled.)*

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

## Plant substrate v2 — started, on branch `plant-substrate-v2` (session handoff)
*(State 2026-08-22: **merged.** `plant-substrate-v2` / `plant-genome` and `plant-ecology-design` both landed on an integration branch off `origin/main`; `plant-branch-angle` has not. The plans of record are `Reports/tree-architecture-implementation-plan.md` and `Reports/plant-implementation-plan.md`, both merged and indexed. The merge left one test red and three unmeasured cross-line inconsistencies — `Reports/open-bugs-handoff.md` §A–§D, which should be read before touching any plant constant.)*

The design above was "fully planned, zero code written" for several sessions.
Implementation started on a worktree branch off `master` at `a39da4e`, isolated
because explosion work was live in the main working tree at the time.

**Re-ordered against the design doc's own §10, for reasons that are recorded
where they bite rather than only here.** The doc sequences sidecar → polarity →
leaves, and instructs that neither of the first two be screenshot-verified. That
is two large invisible refactors before anything reaches the screen, which is the
shape `CLAUDE.md` says has repeatedly failed here. The revised order puts a
lookable-at result at the end of every step.

**Landed so far:**

1. **Tooling first (`fdb7a0c`).** `render.rs` gains `OrganismOverlay` (`B`
   cycles it), the organism-side parallel to `FieldOverlay`: cell type, resource,
   canopy density. `filmstrip` gains `tree`/`forest` scenes and `channel=`;
   `examples/plant_probe.rs` dumps the per-cell numbers. Baseline committed under
   `docs/screenshots/plant-v2-baseline/` with a README of what each sheet shows.
2. **Real leaves (`4ff9f52`).** A plastochron counter on `ActiveKind::Organism`
   retires every N-th parent to `Leaf` instead of `MatureBody`. **This is the one
   place the design doc is wrong:** §3a lists a plastochron counter among the
   scalars that make the sidecar migration a prerequisite for leaves. It is not a
   per-cell scalar — it is lineage state, which `ActiveSite` already hands
   parent→child for `stale_ticks` — so it costs no `aux` bits and the visible half
   of Decision 4 did not have to wait behind the migration.
3. **Per-organism RNG (`f9ab577`)** and **open bug #3 reproduced (`dcb9c0d`)`**.

**Measured, on the standard scene at 8,000 frames:**

| | before | after |
|---|---|---|
| organism cells | 18 | 69 |
| `Leaf` cells | 0 | 18 |
| thickest contiguous run | 1 | 4 |
| height | 6 rows | 33 rows |

`SecondaryThicken` had **never fired on anything, ever** — it counts downstream
`Leaf | GrowingTip` cells and tips retire the instant they grow, so the count sat
at 0–2 against `pipe_ratio: 2.5` for a tree's whole life. Persistent leaves are
what give it a real signal.

**Four findings that change the remaining plan:**

- **Transport is starved, not runaway.** `diffuse_resource` is dispatched from the
  CA sweep, which skips settled chunks: measured awake on 22.8% of frames, then 0
  once the tree settles, while decay runs every organism tick regardless.
  Invisible today because `GrowingTip` carries its own `Photosynthesize` and each
  tip funds itself; it becomes a correctness problem the moment Decision 4 moves
  `Photosynthesize` to `Leaf` only. **Decision 2 is a correctness prerequisite,
  not a storage tidy-up**, and its gate is this duty cycle.
- **Growth is frontier-limited, not income-limited.** The tree stops while still
  holding mid-range resource (and post-leaves, saturated at 4.0/4.0). No amount of
  economy tuning lifts the ceiling; only bud break does.
- **Single-run tuning is unsound here.** Twelve identical trees in one scene span
  **31–153 cells and 10–33 leaves** (`plant_probe -- trees=12`). Swapping the RNG
  alone moved the standard scene 69→19 cells. `examples/debug_tree_variants.rs`
  compares six variants at n=1 each and is the harness the whole economy gets
  tuned with — it needs to become an ensemble before Decision 4's re-tune, and it
  is the same harness `population-dynamics-research.md` §12 asks the creature work
  to share.
- **Canopy density is *not* inert** (deposit and decay both work: 1.600 → 0.800 →
  0.533 → 0.267 → 0), but its whole live range is ~6 quantization steps and its
  decay constant was already tuned around the quantization half-step. That is the
  measured version of §3a's "tail wagging the dog."

**Phase 2 landed** (`5a842ba`, `31ddcb4`, `e8b91a9`, `00a84c5`) — pulled *ahead*
of the tuning pass, because the design doc's §10 sequences its "single tuning
pass" before step 8 gives `RootTip` an entirely new income source, which would
guarantee the double tuning §8a exists to prevent.

- **Roots grow into soil.** `Material::penetration_resistance` (MPa) against a
  per-species `Grow.penetration_force`, calibrated to the 2–3 MPa bound where
  real root elongation stops (Da Silva/Kay/Perfect 1994). soil 0.8, sand 1.4,
  gravel 3.5, tree roots 1.2 — so a tree roots through soil, is refused by
  gravel, and never touches stone. Chose a real field over the doc's cheaper
  "use `density`": conflating them breaks wet sand and pumice.
- **Soil holds water and roots drink it.** Per-cell moisture in a `Powder`'s
  `aux` — **`0` means dry, the inverse of a liquid's fill**. Field capacity and
  wilting point from the same LLWR framework, so `Absorb` credits exactly zero
  below the wilting point and drought is terminal. Closes PLAN.md's recorded
  `RootTip` stall, which was a missing mechanism no tuning could have fixed.
- **`leaf` and `rootwood` are real materials**, on differing physics. `rootwood`
  exists mainly so `update_powder` can ask "is this a root" from a bare `Cell`.
- **Roots hold soil, weight breaks branches** — Wu–Waldron apparent cohesion,
  and `effective_span = max_span − load / LOAD_PER_SPAN_UNIT`.

**Three bugs the work exposed rather than caused**, all older than it:

- `reachable_from_anchors` traversed **4** neighbours while `Grow` places
  children at **8**, so any diagonally-grown tree read back as disconnected
  fragments — and `thicken()`, its only production caller, had been counting a
  fragment of the canopy rather than the canopy. Found by a test written for a
  different bug.
- `thicken()`'s "downstream" was never downstream: the flood spreads across
  every connected cell with no direction. 4-connectivity had masked it. Now
  filtered to cells above, which is what the pipe model specifies, and
  `pipe_ratio` recalibrated 2.5 → 10.0 because its input changed by an order of
  magnitude.
- `RootTip` never retired, so once roots could grow, every root cell stayed an
  eligible tip and the frontier multiplied into mats spanning the soil bed.

**Decision 2 started, and it was driven by performance** (`1e7b30f`,
`fe426ff`, plus `696630f`). Measured on six trees over 6,000 frames:

| | active sites | elapsed |
|---|---|---|
| before | 2,698 | 41 s |
| cheap fixes only | 2,698 | 22 s |
| after the per-organism pass | **18** | **16 s** |

- `OrganismState::cells` is hooked at **`World::set`**, not at the design
  doc's dozen creation/removal sites. Every one of those paths writes
  through `set`, so hooking the write is complete by construction — the
  lesson already recorded in that function for `FLAG_MANAGED`. Gated by a
  test checking the list against a full grid scan after growth, after
  erasing through a canopy, and after fire.
- Mature cells left the active-site schedule entirely; their upkeep runs in
  one pass per organism. `thicken()`'s per-cell whole-organism flood fill
  became one row histogram per organism, producing the identical number.
- The dispatch no longer heap-allocates per cell per tick.

**A finding worth generalising, now in `CLAUDE.md`: a performance limit
standing in for a design one hides the design one.** `MAX_SITES_PER_FRAME`
(2,000) against 2,698 due sites had been *accidentally throttling plant
growth*. Removing the backlog exposed that nothing bounded root growth at
all — roots then converted essentially a whole soil bed to root tissue.
Fixed with real allometry (a conserved root:shoot ratio, `MAX_ROOT_FRACTION`),
using the whole-organism totals the cell list makes cheap and §6 sanctions.

**Step 0, before any new plant work: rebase onto `master`.**

This branch was cut from `master` at `a39da4e` while explosion work was live
in the main working tree, which is why it is on its own worktree at
`.claude/worktrees/plant-v2`. That work has since landed and `master` has
moved on, so the divergence is real and only gets more expensive to carry —
21 commits is already past the point where deferring is the cheaper option.

It is not merely hygiene. The explosion work touched `parallel.rs` and
`surface.rs`, and the per-organism transport pass sits directly downstream
of both. Rebasing *after* building polarity on top would mean resolving a
transport-mechanism conflict inside a transport-mechanism change.

Expect mechanical conflicts in `render.rs`, `app.rs` and
`examples/filmstrip.rs` — every plant change to those three is additive (an
overlay enum beside the existing one, a key binding, a new scene). Re-run
the full suite *and* `examples/ascii` afterwards before starting anything:
a rebase that compiles is not evidence the sweep still behaves.

**Step 0 done, with one correction to the entry above.** The rebase landed on
`abe9c2f`, not `5cb856e`: **`master`'s tip does not compile.** `5cb856e`
("Give sand drag when it sinks into a liquid") calls
`surface.field_wind_at(x, y)`, and that method exists in no commit — it is
declared on `CellSurface` and implemented in `parallel.rs`/`world.rs` only in
the *uncommitted* explosion work still live in the main working tree. So the
explosion work has **not** landed, contrary to what this entry said, and the
`parallel.rs`/`surface.rs` conflict it was written to get ahead of is still
ahead of us. Rebasing onto `abe9c2f` picks up everything that builds; redo the
last step once the explosion work is committed. Only `CLAUDE.md` conflicted
(both sides appended method entries; both kept).

**Decision 2 step 2c landed — and it was a prerequisite for polarity, not a
tidy-up.** The handoff said the per-organism transport pass already existed.
It did not: `step_organisms` did *upkeep*, transport was still the per-cell
`CellSurface` rule on the CA sweep, `OrganismCell` did not exist, and the
resource scalar was still 8-bit fixed point in `aux`. Polarity cannot be built
on that, and the reason is arithmetic rather than taste: at §7e's constants the
§7h Y-junction gate — which §10 calls "the real gate" — turns on a share
difference of 0.0048, and one quantum of the packed representation is 0.0157.
**The gate resolves 0.31 of a step.** Per-substep rounding also exceeds the
per-substep signal at `c_min` in both directions, so §7c's exact conservation
is unreachable. Worth noting the trap: the *chain* canalization test passes
under packing either way (flux saturates Φ), so only the discriminating gate
dies — a guard suite that looks green while the mechanism is unmeasurable.

**Polarity landed** (`12739bc`), and the economy pass is under way. **Bud
break is the remaining item.**

### Polarity (Decision 6) — landed

`carbon_conductance: [f32; 4]` on `OrganismCell`, the pairwise carrier rule
with substeps, the Hill-function conductance update, and `Grow`'s
`away_from_growth` → `away_from_supply` swap. Gate was a picture as well as
unit tests: `render.rs` gained a VEIN CONDUCTANCE channel (`B` in-app,
`channel=vein` in filmstrip) and `plant_probe` prints the distribution
beside it. Sheets under `docs/screenshots/plant-v2-polarity/`.

Deviated from §7f in one place, recorded in `transport`: density cannot run
through the general pairwise form and stay "bit-for-bit" the symmetric
average, because the mean rule and the pairwise rule differ by the
neighbour count. Density keeps its tested rule.

The §7h Y-junction test asserts the *claim*, not the illustration. §7h pins
the stem at 1.0 for hunger and caps delivery at Q=0.3, which a running
simulation cannot do both of; pinning drives flux to ~19× J_REF, saturates
Φ on both faces and collapses the conductance ratio to 1.00. Supply-limited
instead, the ratio lands at 1.26 against §7h's predicted 1.38.

### The economy pass — three things landed, in this order

**1. The harness had to become an ensemble first, and it immediately
justified itself** (`f4ce696`). `debug_tree_variants` compared six variants
at one tree each. Its n=1 output read as "highrate is 3× baseline"; at n=8
that variant spans 15 to 488 and every range overlaps every other. **Any
tuning decision taken on the old harness would have been wrong.** It now
prints the full value list and refuses to rank when the leader's median
sits inside the runner-up's range.

Two side findings: growth converges by **4,000** frames (counts identical
at 4k/10k/20k), so the old 20,000 was 3× longer than needed; and replicates
must be different planting *positions*, since `rng::stream` is seeded from
position and re-running one scene draws identical numbers.

**The finding that matters is that the outcome is bimodal** — 13–21 cells
(germinated then stopped) or 100–500, almost nothing between. A mean
describes a population that does not exist. Report the **establishment
rate** instead.

**2. Canalization contrast 30:1 → 10:1** (`11c2b1e`). §7e names the
contrast as the first knob when seedlings stall, ahead of
`CARBON_SUBSTEPS`; measured, and it is right.

| contrast | established | cells | strand contrast achieved |
|---|---|---|---|
| 30:1 | 54/96 (56%) | 4,906 | 20.7× of 30 |
| **10:1** | **70/96 (73%)** | **6,321** | 6.1× of 10 |

`TRANSPORT_RATE` re-derives with it (0.008 → 0.024) and that coupling *is*
the mechanism: lowering contrast raises the rate the stability bound
permits, and unpolarized tissue conducts at `RATE · CONDUCTANCE_MIN`, so a
seedling with no strand gets 3× the transport. 5:1 tried and rejected.

**3. `thicken()` measures the trunk's real cross-section** (`c0e278f`) —
and this was the big one.

`width` was a count of immediate left/right neighbours, so the *growing end
of a run* always read 2 however wide the trunk had become. It passed
`leaf_count / width > pipe_ratio` forever and spread sideways without
limit. Now the full contiguous run, measured perpendicular to the stem
(axis from `supply_direction` — the first consumer of the conductance field
outside transport).

| | before | after |
|---|---|---|
| thickest run on one row | 105 | 51 |
| stem thickness above base, max | 103 | 32 |
| stem thickness above base, median | 5 | 6 |
| rows >1 cell wide | 38% | 44% |
| leaves, max | 31 | 55 |

**And it roughly doubled establishment** — baseline 5/12 → 11/12 at n=12,
best variant 75% → 100%. Not predicted; measured, then explained.
`thicken` grows *through* its own leaves, so an unbounded trunk ate the
seedling's foliage faster than it could be replaced, starving the plant
building it. The visible symptom (a slab) was two steps removed from the
damage (seedlings quietly failing).

### AUDIT: what the scene discovery invalidates, and what survives

**The environment every plant judgement was made in is not fit for plants**,
and this was found only after the polarity and economy work. See
`Reports/tree-architecture-research.md` §6. Headroom above ground, by
harness:

| harness | rows of sky | used for |
|---|---|---|
| `debug_tree_variants` | **20** | the canalization contrast tuning |
| `plant_probe` (default `ground=40`) | **40** | every ensemble number below |
| `filmstrip` `tree` / `forest` | **40** | every screenshot |

Trees reach 41-47 rows in those scenes, so **they are pressed against the
ceiling and can only spread sideways**. Measured directly: widest
above-ground row is **56 cells at 40 rows of sky and 7 cells at 70**. The
"canopies merge into a slab" symptom that drove two sessions of work is
substantially this.

And there is no depth that fixes it, because `field.rs` seeds light only on
the topmost chunk's top row and diffuses it down with `LIGHT_DECAY` —
**light attenuates through empty air**, so headroom costs illumination.
Same trees, `ground=70`, decay alone: median height **21 → 65** and biomass
**1,271 → 15,362**. Light is the binding constraint on tree size; no plant
mechanism was ever the limit.

**Stands — correctness, established by synthetic unit tests or by
run-to-run comparison, with no dependence on scene shape:**

- The Decision 2 sidecar migration, and transport moving off the CA sweep.
- The `ChunkView` cell-list bug (a falling seed vanished from its own
  organism), both halves — same-chunk and remote.
- The determinism fix (sorted iteration; 5877/5872/5881 → 5806 ×3).
- `RootTip` retiring instead of becoming a phantom.
- A root growing into soil displacing its water instead of deleting it.
- The capillary capacity clamp.
- Transport honouring `RESOURCE_SCALE` (a cell held 92.0 against a cap of 4).
- The polarity mechanism itself and its six tests — all synthetic organisms.
- `thicken()`'s width bug. The *defect* is real and scene-free: the growing
  end of a run always read `width = 2`, so the gate never terminated.
- The ensemble harness, and all three metric corrections. **These are worth
  more now, not less** — the audit was only possible because the metrics
  had been cleaned up.

**Must be re-measured — tuned or judged against a ceiling:**

- **The canalization contrast 30:1 → 10:1.** Tuned on establishment rate in
  the 20-row scene. The *direction* may well hold; the numbers do not.
- **`thicken()`'s measured magnitudes** (slab 105 → 51, establishment
  doubled). The fix stands, the effect sizes were measured against a
  ceiling.
- **The bud break verdict.** "Fills the world as a solid mass" was measured
  against a 40-row ceiling, and the mass is exactly what a ceiling
  produces. **That revert may have been wrong**, and re-testing it in a
  proper scene is the first thing to do after the scene exists.
- **`tree-architecture-research.md` §1** (maintenance respiration and
  self-pruning). Reasoned from the mass symptom. The biology is sound and
  the citations stand; whether it is *this engine's* missing mechanism is
  now unproven.
- The respiration and gravitysense sweeps, both run in the bad scene.

**The environment is now fixed, and it corrected the audit above.**
`LIGHT_DECAY` 0.997 → 0.9997 (air is near-transparent; attenuation comes
from occlusion, where it belongs), plus a `grove` scene with ~96 rows of
sky. Frame cost unchanged, paired: 10.027 → 10.061 ms.

**And with light and headroom both fixed, the mass is real.** Trees now
reach 94 rows tall instead of 21 — and then grow into a large branching
blob anyway. So the ceiling was *hiding* the problem by starving growth,
not causing it. Two consequences:

- The audit above overstated. "The slab is substantially a ceiling
  artifact" is **wrong**; the ceiling was a confound, and removing it makes
  the mass more visible, not less.
- **`Reports/tree-architecture-research.md` §1 is back on.** Maintenance
  respiration and self-pruning remain the best candidate for what is
  missing, and can now be tested somewhere the answer means something.

What still stands from the audit is the *list* — every correctness fix
survives, and every tuned number (the canalization contrast above all,
measured in a 20-row scene) still needs re-deriving in `grove`.

### The blob is almost pure wood, and `thicken()` is why

Measured in `grove` (96 rows of sky, light fixed), 8 trees:

| frame | total | Leaf | MatureBody | widest run above ground |
|---|---|---|---|---|
| 3,000 | 868 | 92 | 768 | 10 |
| 5,000 | 2,711 | 184 | 2,512 | 34 |
| 9,000 | 8,611 | 255 | 8,355 | 103 |
| 14,000 | 12,292 | **253** | **12,039** | 105 |

**Foliage plateaus at ~255 while wood grows to 12,039.** From frame 5,000
on, leaves grow 38% and wood grows 379%; the wood:leaf ratio goes 8:1 →
**48:1**. The blob is not a canopy at all, it is secondary thickening —
and `SecondaryThicken`'s whole justification is Shinozaki's pipe model,
*wood in proportion to the foliage it supplies*. At 48:1 that gate is
plainly not binding.

**The growth trajectory passes through a good tree and keeps going.** The
time series is unambiguous: frames 3,300–5,100 read as a genuine tree —
vertical trunk, branching crown, thickened base — and everything after is
the same tree filling in. So the *shape* mechanisms are broadly right and
what is missing is anything that stops the filling. That is a much better
position than "trees are the wrong shape".

**Prime suspect, and it is in code this branch already touched.**
`thicken()` measures `width` along the axis perpendicular to
`supply_direction`. In a slender stem that axis is meaningful. Inside a
blob the supply direction is near-arbitrary, so the "perpendicular" is
arbitrary too, and the run it measures is not the cell's actual thickness —
a horizontal lobe measured vertically reads as thin and keeps widening.
The fix that made the *end of a run* terminate correctly (`c0e278f`) does
not help once the axis itself is meaningless. Check this before reaching
for self-pruning: it is cheaper, and it is a defect rather than a missing
mechanism.

**Order from here:** re-measure in `grove`, in this order — the
canalization contrast, then self-pruning, then the bud break verdict. Do
not re-tune anything against `forest` or the default `plant_probe` ground
again; both are 40-row scenes and are now for root and soil work only.

### Bud break — built, measured, and **reverted**. Read this before rebuilding it.

`Reports/plant-substrate-v2-design.md` §2e specifies `BudBreak {
resource_threshold, crowding_threshold, chance }` on `MatureBody`, argues
it is "self-limiting without a cap", and calls it the only thing that can
lift the size ceiling. It was implemented as written, wired into
`tree.ron`, measured, and taken back out. **Three of its claims are false in
this engine, each falsified by measurement rather than argument.**

**The ceiling is real.** Without budding, a 24-tree ensemble reaches 0
active sites by frame 16,000 and is flat from there (4,682 → 7,265 →
7,466 → 7,484 at 4k/8k/16k/24k). So §2e's diagnosis is right even though
its mechanism is not.

**1. "Surplus resource" carries no information.** The premise is that a
mature cell holding surplus has nothing downstream consuming what it is
fed. It does not: carbon fills *every* cell to `RESOURCE_SCALE` exactly
(that is what `transport`'s headroom clamp guarantees), so once a tree
stops growing every mature cell is at the cap simultaneously. Raising
`resource_threshold` from 0.75 to **0.99** — a near-impossible surplus —
moved the ensemble from 17,181 to 17,365 cells. It is not a gate.

**2. "Self-limiting without a cap" is false.** Local crowding closes for
about two ticks (`GROW_CANOPY_DEPOSIT` 1.5, halved every tick), and
conductance is no better: with growth stopped there is no flux anywhere,
so every face decays to basal together. **When a tree stops growing, every
local signal equalizes at once**, so any purely local "am I idle" test
fires on every mature cell simultaneously. A per-cell chance then makes
budding proportional to *volume* — every new cell is another bud site —
and the tree fills in as a solid mass. The time series is unambiguous:
recognisable tree to frame 2,000, then a mound expanding from the base
with leaves buried inside it.

**3. A rate cap is not enough, and neither is allometry.** Moving the roll
to one per organism per tick converts exponential growth into linear
growth, and linear growth still fills the world — the canopy went lumpy at
frame 12,500 instead of 3,000. Adding the shoot-side allometry bound this
file asks for (`MAX_SHOOT_FRACTION`, the mirror of `MAX_ROOT_FRACTION`)
does **not** fix it either, and the reason generalises: **a ratio bound
does not bound size.** Roots and shoot can both grow without limit while
staying in band. Measured with budding off, the shoot bound cost ~13%
biomass (7,484 → 6,529 at frame 24,000) and changed nothing about
saturation, so it is a tax with no benefit unless something else is
producing frontier. Both reverted.

**What is *not* the problem: self-shading.** It works — `field.rs` blocks
light on `MaterialKind::Plant`, so a buried cell really is dark. The canopy
grows at its *lit surface*, which is correct; the slab is the mass filling
in behind that surface.

**What a future attempt needs.** Not a better threshold — a real bound on
absolute size, which this engine currently has nowhere. Two candidates
neither of which was tried: making growth cost rise with distance from the
roots (a hydraulic limit, which is what actually bounds real trees), or
triggering budding on **disturbance** rather than idleness, which is both
the cited biology (`research/m16-plant-biology.md` §5's fire-resprouting)
and self-limiting by construction. The second is also the more satisfying
one in the hand — cut a limb and the tree resprouts near the wound — and
it needs no size bound at all, because the trigger is an event rather than
a state.

**Do not tune against `rows >1 cell wide`.** It is dominated by the basal
slab; eight A/B runs were spent chasing a "regression" in it that turned
out to be the metric, not the trees. Judge on **stem thickness above the
base**, on the **establishment rate**, and on the picture.

**Two bugs step 2c exposed, both older than it, both recorded where they
bite:**

- **The organism cell-list hook was incomplete, and its guard test could not
  see it.** `World::set` maintains `OrganismState::cells`, and step 2a
  recorded that hooking that one seam was "complete by construction". It is
  complete over every *caller* — but `parallel::ChunkView::set` writes a
  same-chunk cell straight into its own `Chunk` and never calls `World::set`.
  A falling seed therefore dropped out of its own organism's cell list while
  staying in the grid. Latent while the list was behaviour-free; **fatal the
  moment carbon moved into it** — the shoot read 0 carbon forever, `write_
  carbon` was a silent no-op, and every seed that fell germinated and then
  never grew. `filmstrip scene=forest`: 253 → 5468 organism cells before,
  **8 → 8** after, i.e. four germinated seedlings and nothing else, for 8,000
  frames. Fixed by queueing the membership change in `ChunkView` and replaying
  it after the pass, exactly the shape its `demotions` queue already had.
  `every_organism_cell_list_agrees_with_the_grid` runs `update::step` — the
  *serial* driver — so it never built a `ChunkView` and could not fail. This
  is `CLAUDE.md`'s "test both drivers" costing a session.
- **`organism_upkeep` was non-deterministic.** It iterated the cell `HashSet`
  directly, and Rust seeds its hasher per process, so the same binary on the
  same scene gave 5877 / 5872 / 5881 organism cells on three consecutive runs.
  Sorting row-major gives 5806 three times. `PLAN.md` requires same-build
  determinism and it was not being met.

**And one constant the migration invalidated, deliberately left for the economy
pass.** Canopy density was 4 bits, `CANOPY_DENSITY_DECAY_PER_TICK` is a
halving, and 0.267 × 0.5 = 0.133 rounds straight back to 0.267 — so density had
a **permanent floor of one quantum on every cell that ever received a
deposit**, and the mechanism whose whole purpose is letting later growth
reclaim space near mature wood could never release it. As `f32` it now reaches
zero.

**`rows >1 cell wide` mostly measures the basal pancake. Do not tune against
it.** This is the metric trap `CLAUDE.md` warns about — ask what a metric counts
when nothing is wrong — found the expensive way, after eight A/B runs chasing a
"regression" in it.

Paired 24-tree ensembles across the migration:

| | baseline | after 2c |
|---|---|---|
| total organism cells | 5872–5881 | 5806 |
| rows >1 cell wide, mean | 42% | 35% |
| **thickest contiguous run on one row** | **61** | **121** |
| **stem thickness above the base**, mean | **11.8** | **13.3** |
| stem thickness above the base, median | 5 | 4 |

The headline metric dropped 42 → 35, which reads as "trees got thinner". They
did not: *above the base*, where the trunk actually is, they are slightly
**thicker** on the mean. The entire gap sits in the basal region, where
`thicken()` spreads sideways along open ground — the pancake this file already
lists as known-open. The migration widened the outcome distribution: more pure
whips *and* a bigger maximum slab (121 cells vs 61), at flat mean biomass.

**Lowering `pipe_ratio` "fixes" the metric by making the pancake worse.**
Measured: 10.0 → 6.0 takes the metric from 35% to 43%, matching baseline — and
takes biomass from 5,806 to 13,795 and draws a slab spreading the full width of
the ground. Reverted; `pipe_ratio` stays 10.0. **The real fix is the `thicken()`
change already on the books** (grow around the stem rather than left/right, and
stop `width` reading 1 on a diagonal stem), not a tuning knob.

**Eight candidate causes for the 42 → 35 shift were A/B'd; all were ruled out**
— every variant lands at 35–37 and none reaches 42:

| variant | rows >1 cell wide, mean |
|---|---|
| baseline (4 runs, stable) | 42 |
| migrated, as shipped | 35 |
| + canopy density re-quantized | 36 |
| + resource re-quantized | 36 |
| + transport per-frame at 1 substep | 36 |
| + transport gated on chunk-awake (old duty cycle) | 35 |
| + iteration order reversed | 35 |
| + scalar writes dirty the chunk again | 35 |
| + Gauss-Seidel instead of Jacobi | 37 |
| + Gauss-Seidel and both quantizations | 35 |

Not cumulative — the combination is *worse* than Gauss-Seidel alone, so these
interact rather than adding. Given the metric turned out to be pancake-dominated,
the shift is no longer worth chasing as a defect; **judge the next pass on stem
thickness above the base and on the picture, not on this number.**

**A metric caveat worth carrying into that pass:** `plant_probe` plants its
seeds *on the ground*, so its trees never fall. It therefore reported a healthy
−1% biomass change while `filmstrip`'s `forest` scene — which drops seeds 25
cells, like real play — was totally dead. The ensemble is the right tool for
tuning and was structurally blind to a total growth failure. Shoot a picture.

**The economy pass now has concrete, visible targets** — all consequences of
the same removed throttle, and all shape/tuning rather than missing
mechanism, which is why they wait for the single pass after polarity rather
than being tuned twice:

- **Canopies merge into a slab.** Roots are bounded by allometry; the shoot
  is bounded by nothing equivalent. The mechanisms that should bound it
  (self-shading, the resource gate) exist and are mis-parameterised.
- **`thicken()` makes a pancake, not a trunk.** It only grows left/right, so
  a trunk base carrying the whole canopy spreads sideways along open ground.
  `width` is also still "same-organism cells immediately left/right on this
  row", which on a diagonal stem is 1 almost everywhere. Fix both when
  `thicken()` is next touched.
- Tree-to-tree variance was 5x before any of this; `examples/
  debug_tree_variants.rs` still compares six variants at n=1 each and must
  become an ensemble before the pass.

**Known-open, carried forward:**

- **Trees still read as whips.** ~75% of a tree's rows are one cell wide.
  `thicken()`'s `width` term counts same-organism cells immediately left/right on
  one row, which on a diagonal stem is 1 almost everywhere — fix it when
  `thicken()` is next touched (Decision 2 §3e already bounds its scan).
- **Tree-to-tree variance is 5x** (31–153 cells from one genome, `plant_probe --
  trees=12`). `examples/debug_tree_variants.rs` still compares six variants at
  n=1 each and must become an ensemble before the economy pass.
- **`water_capacity` is opt-in per material** (soil only). Real sand holds water;
  widening it means teaching the engine's liquid-conservation tallies about held
  water first.
- Anoxia necrosis deferred to the economy pass — it needs the sidecar's
  `anoxia_ticks`.

**`germinate()`'s root gate is deferred to Phase 2 deliberately.** Refusing a root
on bare stone is *correct* — trees should not root in rock. It refuses on soil
too, and that is the real bug, but it cannot be fixed honestly until roots can
enter soil.

---

## `Reports/granular-mechanics-research.md` and `Reports/liquid-simulation-research-r2.md` — landed, plan updated (session handoff)
*(State 2026-08-21: still accurate — the two-angle granular model remains unbuilt; see README.md's not-yet list.)*

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
*(State 2026-08-21: promotion was implemented and reverted; heightfield bodies remain test-only until it lands.)*

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

**Not yet built at the time Step 2 shipped:** the persistent-flux pipe
solver (step 3), the terminal equilibrium snap and body sleep (step 4),
`try_extend`/edge demotion/cooldowns (step 5), dropping `MIN_LIQUID_
TRANSFER` (step 6). Automatic promotion still doesn't exist — see the gap
noted above.

### Step 3 (§11) — the persistent-flux pipe solver — implemented

**`LiquidBody` gained `flux: Vec<i32>`**, one signed entry per interface
between adjacent columns (`columns() - 1` long, never resized after
promotion in any step built so far), and **`LiquidBody::step`**, run every
frame for every live body from `World::step_liquid_bodies` (wired as a
no-op back in step 1; now does the actual work). Three full passes, matching
design doc §7b's pseudocode exactly rather than interleaving them:

1. **Flux update** (§7a/§7b step 1) — each interface's flux is `damp(flux)
   + gain * (level[i] - level[i+1])`, where `level[i]` is a per-column
   surface elevation on the `LIQUID_FULL` scale, computed fresh each call
   from a locally-chosen reference row (only differences ever matter, so
   nothing needs to persist). **The persistence of `flux` itself across
   frames is the entire point** (§7a's own correction to the research
   report this design is built from) — a naive per-step recompute
   degenerates to O(width²) diffusion; the persistent term makes leveling
   a real O(width) travelling wave.
2. **K clamp** (§7b step 2) — a single forward pass scaling down any
   column's total outflow (across both interfaces) if it would exceed that
   column's own current `h[i]`, exact-rational via `i64` intermediates.
3. **Apply** (§7b step 3) — debit/credit `h[i]`/`h[i+1]` by the clamped
   flux, exactly conservative by construction regardless of what the first
   two passes computed.

`SOLVER_GAIN = 0.4` / `SOLVER_DAMP = 0.9`, the design doc's own recommended
starting values — **both explicitly untuned against real play.** Design doc
§7f says `SOLVER_GAIN` should eventually become the `flow_rate` a material
already carries in its own `.ron` (rescaled), replacing a second parallel
viscosity knob with a real job for the existing one — deliberately deferred:
`flow_rate` is also read by the ordinary CA path, and retuning a shared
value blind (no live-play feedback available in an unattended pass) risks a
regression in already-tuned CA behaviour that can't be verified without
watching it played.

**`LiquidBody::rasterize_column` (step 2) rewritten to handle three cases,
not one** — growing (already existed), **shrinking** (newly reachable: the
solver's flux is the first thing that can ever reduce `h[i]`, clearing
vacated rows to `Cell::EMPTY` and unflagging container cells no longer
adjacent to any remaining body cell), and **same whole-cell-count but the
topmost cell's own partial fill changed** (a small solver step that
doesn't cross a whole-cell boundary — absorption's whole-cell-at-a-time
credits never exercised this case, so step 2's version of the function
silently handled it wrong; see the bug below).

**Two real bugs found by independent review, both fixed in the same pass:**

1. **Every live body's chunk stayed dirty on every single solver frame,
   forever — the sleep mechanism step 4 needs was already broken before
   step 4 was even built.** `rasterize_column`'s "same cell count" branch
   called `write_liquid_cell` unconditionally, and `Chunk::set_world`/
   `mark_dirty` don't compare bytes — so even a perfectly flat body at true
   equilibrium (flux settled near zero, nothing left to redistribute) kept
   marking its chunk dirty every frame purely from being rewritten with
   identical content. Fixed by making `write_liquid_cell` itself a genuine
   no-op (no `set_owned` call at all) when the cell is already exactly
   correct — material, `managed`, and `aux` all already match. Regression
   test (`a_flat_body_at_equilibrium_does_not_keep_dirtying_its_chunk`,
   using the real per-frame order — `parallel::step` then `step_liquid_
   bodies`, matching `app.rs` — since `touched_chunks` is only promoted
   from `pending_dirty` by the CA sweep's own `end_step`) confirmed to fail
   without the fix (found 2 chunks still dirty after 20 quiescent frames).
2. **The exact cross-chunk `body_index` gap Step 2's review already found
   and fixed for `absorb_liquid` alone was reintroduced by the new solver
   call path** — `LiquidBody::step`'s own redistribution can grow a column
   across a `CHUNK_SIZE` boundary exactly like absorption's growth can, and
   `World::step_liquid_bodies` never registered anything. Fixed by
   factoring the registration into a shared `World::register_body_chunks`,
   called from both `absorb_liquid` and `step_liquid_bodies` — specifically
   so a third future caller can't reintroduce the same gap a third time.
   Independent review verified this live (grew a body only via the
   solver's own redistribution, confirmed a disturbance in the newly-
   crossed chunk silently failed to demote before the fix); the regression
   test added here (extending `the_solver_levels_correctly_across_a_
   chunk_boundary`) confirms the registration survives 3000 solver frames
   but does not itself force a *fresh* mid-solve crossing — a known,
   accepted test-coverage gap given the review's own direct verification
   already covers that exact mechanism.

One more overflow-safety fix, low likelihood but free to make correct: the
apply step's `-flux` negation used `.unsigned_abs()` instead of `-f as u32`,
which would panic on `f == i32::MIN` (not reachable today given `MAX_BODY_
CELLS` bounds every realistic value far below either limit, but there's no
reason to leave a trap for whenever that bound changes).

**Verification:** new tests in `liquid.rs` — the solver substantially
levels a large single-column imbalance within a fixed frame budget; mass
is exact every single frame, not just at the end; the same starting state
levels to bit-identical column heights across two independent runs
(determinism); leveling still converges correctly across a real chunk
boundary; the two bug-specific regression tests above. `cargo test --lib`
(323 tests) and `cargo clippy --all-targets -- -D warnings` both clean.
Cost re-checked on the `ascii` stress scene (no promoted bodies exist in
that scene, so this is really confirming zero incidental overhead from the
new code paths, not solver cost itself) — held at the ~38ms/~8ms baseline
across three runs. Independent review (general-purpose agent) hand-verified
the flux/K-clamp/apply math against the design doc's pseudocode line by
line (confirmed correct — and found the K clamp's single-forward-pass
scaling is actually *fully* conservative, not merely "conservative but not
optimal" as its own code comment suggested: each interface's flux is
touched by at most one column's clamp iteration, so the `.min()` calls in
apply are genuinely redundant safety nets, not masking a real gap), found
the two bugs above by writing and running temporary reproduction tests
(fully reverted afterward), and confirmed the take-then-restore pattern in
`step_liquid_bodies` is sound across multiple bodies in one call.

**A B-8-style formal cost bound for the solver itself (≥4 bodies, ≥1,000
columns total, serial body phase under 0.5ms) was not measured this
pass** — no `examples/ascii.rs` scene exists yet with multiple promoted
bodies to measure against, and building one is infrastructure work
orthogonal to the solver's own correctness. Flagged as deferred, not
skipped silently.

**Not yet built at the time Step 3 shipped:** the terminal equilibrium snap
and body sleep (step 4), `try_extend`/edge demotion/cooldowns (step 5),
dropping `MIN_LIQUID_TRANSFER` (step 6). Automatic promotion still doesn't
exist.

### Step 4 (§11) — quiescence, the terminal snap, and body sleep — implemented

**`LiquidBody` gained `asleep: bool`.** `step` returns immediately — no
computation, no cell writes — once `asleep` (design doc §8c: a sleeping
body costs nothing per frame). Set by a new `terminal_snap`; cleared by
`World::absorb_liquid` (new mass to redistribute has to wake the body).

**Quiescence** (§4a/§7d) is checked inside `step`, from the same `level`/
`flux` arrays the flux-update and K-clamp passes already computed this
frame — a whole-body measurement in O(width), never a per-cell test, and
explicitly *not* a promotion gate (§4a: a still-moving body is the entire
reason to promote it). If every interface's level difference and flux both
sit under `SNAP_EPSILON`, `step` calls **`terminal_snap`** instead of its
usual incremental rasterization: solves for the exact integer equilibrium
surface elevation via monotone binary search
(`contribution(L) = Σ max(0, L - bed_level[i])`, smallest `L` with
`contribution(L) >= total`), assigns each column its exact share, trims the
resulting overshoot by subtracting 1 from the lowest-indexed wet columns
(deterministic remainder distribution, never hash/iteration order —
issue #7's standing rule), zeroes `flux`, rasterizes, and sleeps.

**`SNAP_EPSILON = 30`, not a small value, for a real measured reason, not
a guess:** the solver's own integer rounding (`f64::round` in the flux
update) leaves a genuine non-decaying limit cycle — `SOLVER_DAMP = 0.9`
rounds small flux values back to themselves (`round(0.9 * n) == n` for
`n` in `1..=5`), so a small residual can only ever cancel via the gain
term hitting it exactly, which integer rounding of `0.4 * d` doesn't
reliably do. Measured directly on the test scene this session already
uses (40 columns, single-column spike): settles to a persistent
oscillation around 11-21 units, never reaching zero. `30` clears that
floor with real margin. **Still flagged as untuned against real play** —
this is "confirmed to actually trigger," not "confirmed to look right."

**Verification:** new tests in `liquid.rs` — a leveling body eventually
snaps flat (adjacent columns within 1 fill unit — design doc 10b's own
bar) and sleeps, with mass exact through the snap; a slept body lets
`active_chunk_count()` reach zero (design doc B-6); absorbing new mass
wakes a sleeping body. `cargo test --lib` (326 tests) and `cargo clippy
--all-targets -- -D warnings` both clean (one clippy fix along the way:
`needless_range_loop` in the snap's own write-back). Cost re-checked on
the `ascii` stress scene (no bodies exist in that scene, so this again
confirms zero incidental overhead, not solver/snap cost) — held at
baseline across three runs. All three sleep-specific tests confirmed to
fail without the quiescence check (temporarily forced `quiescent = false`).

Independent review (general-purpose agent) hand-traced the binary
search for off-by-one errors (none — proved the smallest-`L` search is
correct and the overshoot is always strictly less than the number of wet
columns, so the trimming loop's `debug_assert_eq!(overshoot, 0, ...)`
can never fail), verified mass conservation through the trim
algebraically, confirmed waking a sleeping body is safe (the only path to
`asleep = true` already zeroes `flux` immediately before, so the
"sleeping implies zero flux" invariant holds structurally with nothing
extra needed at wake time), and **independently reproduced the
`SNAP_EPSILON = 30` measurement in a standalone probe outside the crate**
(20,000 frames, same math, converged to the same 11-21 residual) rather
than taking the code comment's claim on faith — confirmed it isn't
masking a sign error or off-by-one elsewhere. One real, fixed finding:
`register_body_chunks` was being called unconditionally every frame for
every body regardless of sleep state, rebuilding a `HashSet` over the
body's whole footprint — real allocation-bearing work `step`'s own no-op
didn't cover, undercutting §8c's "costs nothing" claim for as long as a
body stayed asleep. Fixed by checking `asleep` before calling `step` and
skipping the registration call for a body that was already asleep coming
into the frame (still registers once, correctly, on the exact frame a
body newly falls asleep).

**Not yet built:** `try_extend`/edge demotion/cooldowns (step 5), dropping
`MIN_LIQUID_TRANSFER` (step 6). Automatic promotion still doesn't exist —
see the gap noted several sections up. A formal B-8 solver-cost bound
(≥4 bodies, ≥1,000 columns, body phase under 0.5ms) and the full B-2
leveling-speed acceptance bar (matching `eeefceb`'s exact three-tall-
columns scene) both remain deferred — B-2 in particular needs either
automatic promotion or a hand-built multi-thousand-frame scene to be a
faithful reproduction, and is the natural thing to verify once automatic
promotion exists rather than before.

### Step 5 (§11) — `try_extend`, edge demotion, and post-demotion cooldowns — implemented

**`overloaded_edge`/`demote_edge_column`** (§6c): if an edge column's fill
exceeds `EDGE_OVERFLOW_RATIO` (2.0) times the body's own average, and the
position just outside it has room, that column demotes back to ordinary CA
cells — array entries removed, `x0` adjusted, `flux` trimmed by one entry,
container cells no longer needed by any surviving column unflagged via a
before/after `container_positions()` diff. "Has room" is checked by raw
material/kind, not `managed()` — the position just outside an edge is
always the body's own pre-flagged container wall, so `managed()` there is
always `true` and the real check has to see through it.

**`try_extend`** (§3d): the reverse operation, claims one more edge column
if the cells just outside hold unmanaged same-material liquid with a free
surface above. Scans the *candidate's own* actual vertical extent
independently (anchored at the neighbouring edge column's bed row, walking
upward through contiguous same-material unmanaged cells, capped by
`MAX_EXTEND_SCAN`) rather than assuming it matches the surviving
neighbour's height — a demoted column is routinely much taller than the
flat neighbour it demoted from, so validating against the neighbour's own
row range made the "free surface above" check fail by reading more of the
candidate's own water as if it were open air.

**`extend_cooldown_until: u64`** (§4c): gates both operations on the same
edge. Set by a demotion (stopping an instant re-claim of what was just
shed) *and* by a successful claim (stopping the newly-claimed column from
being judged overloaded and demoted right back before it's had any real
frames to solve). `step`'s structure changed to check `try_extend` *before*
the `asleep` early return — a settled body still needs to periodically
check for a reclaimable neighbour — with a successful claim clearing
`asleep` and returning immediately, deferring the solver/edge-demotion
check to the next frame.

**Three real bugs found and fixed during implementation** (beyond the
thrash-control design itself, which needed two attempts — see below):

- **Mass creation**: `rasterize_column`'s grow branch, when growing a
  column from genuinely zero cells (`old_count == 0`, so `old_top ==
  bed_y`), wrote through `bed_y` itself — the container/bed cell, not a
  body cell — via an inclusive-range off-by-one. Found via debug
  instrumentation that located a stray managed liquid cell exactly at a
  column's own bed position, inflating `total_liquid_fill` by exactly one
  `LIQUID_FULL` unit at frame 2 of an otherwise-idle 500-frame test. Fixed
  with a conditional upper bound: `old_top` if `old_count > 0`, else
  `old_top - 1`.
- **Wrong-candidate validation**: covered above under `try_extend`.
- **Extend/demote thrash, two layers**: (a) a successful claim falling
  through to the same frame's `overloaded_edge` check could re-demote the
  just-reclaimed column instantly — fixed by returning immediately after a
  claim; (b) that alone only spread the cycle across two frames instead of
  one — a column reclaimed while still genuinely overloaded relative to the
  new average got re-demoted a frame or two later, resetting a fresh
  cooldown each time and thrashing forever on a slower cycle. Confirmed via
  a background test process that hung indefinitely (had to be killed with
  `taskkill` — it held a lock on the test `.exe`, breaking subsequent
  `cargo test` runs with a linker error until found and killed) and via
  debug output showing `cooldown_until` growing unboundedly. Fixed by
  making a successful claim *also* set `extend_cooldown_until`, and gating
  `overloaded_edge` itself on that same field, not just `try_extend`'s own
  trigger.

**Verification:** four new tests (`an_overloaded_edge_column_demotes_
when_it_has_somewhere_to_spill`, `try_extend_claims_an_adjacent_puddle`,
`try_extend_refuses_a_puddle_of_a_different_material`,
`extend_is_suppressed_during_the_post_demotion_cooldown_then_resumes` —
the last with a hard 10,000-iteration guard as a permanent regression net
against the thrash bug above) plus a shared `build_pool_with_a_real_edge_
demotion` helper. Two Step 3 tests (`the_solver_conserves_mass_every_
frame`, `a_leveling_body_eventually_snaps_flat_and_sleeps`) updated to
check whole-world `total_liquid_fill` instead of `body.total_fill()` alone,
since edge demotion now legitimately moves mass out of the body's own
accounting onto the ordinary grid — a real conservation, not a loss.
`cargo test --lib` (330 tests, then 331 after the review fix below) and
`cargo clippy --all-targets -- -D warnings` both clean. Cost re-checked on
the `ascii` stress scene (still no bodies in that scene, so still zero
incidental overhead, not solver cost) — held at baseline (~37-38ms serial,
~7.5ms parallel) across three runs both before and after the review fix.

Independent review (general-purpose agent) verified the scan bounds (no
off-by-one — the `top..bed` range matches `top_y`/`bed_y` semantics
exactly), stress-tested the thrash-prevention logic beyond the existing
regression test (simultaneous double-edge overload and an extreme
single-edge overload, both run 20,000 frames, both converged after one or
two changes and stayed stable, mass exact throughout — no oscillation
found), verified `flux` sizing stays correct under repeated grow/shrink
alternation, and confirmed the container-flagging/unflagging diff pattern
is safe under overlap. One real, fixed finding: `step_liquid_bodies` gates
`register_body_chunks` on the body's *pre-step* sleep state (`was_asleep`)
to avoid rebuilding a `HashSet` over the whole footprint every frame a
body stays asleep (§8c: sleeping costs nothing) — but `try_extend` runs
even while asleep specifically so a sleeping body can reclaim a neighbour,
and a successful claim can grow the footprint into a chunk never touched
before. A body asleep going into the exact frame it reclaims a
chunk-crossing column skipped registration entirely, silently desyncing
disturbance/demotion handling in that chunk from then on — this is the
same "unregistered chunk" bug class `register_body_chunks` was factored
out to prevent a third time (Step 3's own doc), recurring a fourth time via
a new growth path. Fixed by registering whenever the body wasn't asleep on
*both* sides of `step` (`!(was_asleep && body.asleep)`) rather than just
before it — skips only the true steady-state case where nothing could have
changed. New regression test
(`a_body_that_wakes_via_try_extend_from_asleep_registers_its_new_chunk`)
builds a real edge demotion, piles extra water on the demoted column past
a genuine chunk boundary, drives the body to sleep before the cooldown
expires, advances frames without stepping to land exactly on the
reclaiming frame while still asleep going in, and confirms a disturbance
in the newly-claimed chunk still demotes the body — confirmed to fail
against the pre-fix gate (reverted it locally, reran, watched it fail with
`demote_body_at` silently no-op'ing) before restoring the fix.

**Untuned constants, flagged, not guessed:** `EXTEND_INTERVAL = 30`,
`MAX_EXTEND_SCAN = 10_000`, `EDGE_OVERFLOW_RATIO = 2.0`, `DEMOTE_COOLDOWN_
FRAMES = 120`. **Known non-issue, noted for completeness:** at exactly 2
columns, `overloaded_edge` can never fire (`h[edge] > avg * 2` reduces to
requiring the other column's fill be negative) — not a thrash risk, acts
as a floor against demoting below 2 columns; a very skewed 2-column body
can only shed mass by growing a third via `try_extend` first.

**Not yet built:** dropping `MIN_LIQUID_TRANSFER` (step 6). Automatic
promotion still doesn't exist. The same deferred B-8/B-2 verification bars
noted in Step 4 remain deferred for the same reason.

### Step 6 (§11) — drop `MIN_LIQUID_TRANSFER` toward 8, confirm settling stays in budget — implemented, landed at 16 not 8

**The design doc's own framing: "this step is the verdict on steps 3-4."**
Report B §3c set the standard back when the dead band was first tuned *up*
to 150 to hide slow per-cell convergence: "treat `MIN_LIQUID_TRANSFER` as
the diagnostic, not the setting — if a change to the leveling mechanism
doesn't let this number go down, it didn't fix the underlying problem."
B-9 turns that into a bar: the constant must drop to ≤ 16 (2% of
`LIQUID_FULL`) with 10b/10c (a 100-column pool flat within 2% inside 300
frames) still passing.

**New test, `a_wide_shallow_pool_levels_within_budget`** (§12/B-9, 10b/10c):
a single shallow row, 100 columns wide, deliberately uneven (one spike
against a flat plateau) — at this depth, fill unevenness *within* the row
is exactly the "adjacent-column height difference" 10b measures, without
needing multiple layers to express it. Asserts `MIN_LIQUID_TRANSFER <= 16`
via `const { assert!(...) }` (a compile-time check, per clippy's own
`assertions_on_constants` suggestion — stronger than a runtime check, since
a regression fails the build, not just this one test) and that max adjacent
fill difference is ≤ 20 after 300 frames.

**Tried 8 first, the design doc's literal target — it didn't clear the
bar.** Measured directly on this exact scene: 8 leaves 31 units of residual
difference after 300 frames (bar is 20); 16 clears it. Landed at exactly
16, the B-9 ceiling, not a rounder or more conservative number — it is the
tightest value this scene actually measured as sufficient, re-derived
rather than assumed from the doc's "toward 8" phrasing.

**One real, expected regression, fixed by raising a test's own frame
budget rather than the constant:** `a_wide_deep_water_column_levels_out_
instead_of_only_eroding_at_the_edges` (a 100-wide, 49-deep column, Step
4's own §4 acceptance scene) stopped reaching `active_chunk_count() == 0`
within its old 3000-frame budget at the tighter dead band. Checkpointed
directly rather than guessing: still 6 chunks active at 3000 frames, 4 at
4000, fully asleep by 5000 — and settling *flatter* than before
(`height_spread` reaches exactly 0.0, where 150 left visible residual
unevenness). Budget raised to 6000 for real margin. This is exactly the
cost the design doc's own framing predicts: a column this large is precisely
what automatic body promotion (still not wired up — next on the list) is
meant to take off this per-cell path entirely; until then, a scene this
size pays for the tighter dead band in frames, not in final flatness. Not
treated as a reason to keep the dead band wide — B-9 exists specifically so
this kind of regression gets found and characterized instead of hidden
behind a loose bound.

**Verification:** `cargo test --lib` (332 tests) and `cargo clippy
--all-targets -- -D warnings` both clean. Cost re-checked on the `ascii`
stress scene, a full-screen mixed sand-and-water scene that *does* exercise
ordinary per-cell liquid transfer (unlike the liquid-body steps' cost
checks, which measured zero incidental overhead on a scene with no
promoted bodies) — held at baseline (serial ~37-38ms, parallel ~7.5-9ms)
across 13 runs, with 3 isolated single-run spikes (up to 71ms) that did not
recur on the very next run and appeared on both serial and parallel at
different times — read as OS scheduling noise on this dev box, not a
directional shift, since a real regression from a tighter transfer
threshold would show as a consistently elevated median, not sporadic
one-off spikes surrounded by baseline runs.

**Not yet built:** automatic promotion (design doc §3e) — still the one
gap standing between this design and bodies actually promoting during real
play. The deferred B-8/B-2/B-6/B-7 verification bars noted in Steps 4-5
remain deferred for the same reason.

### Automatic promotion (§3e): attempted, reverted — a real architectural gap found, not a tuning problem

**Attempted a full implementation** of design doc §3e/§4c: a per-chunk
`Chunk::liquid_hint: Option<(i32, i32)>` seed position, set for free at the
existing write seam (`World::write_cell`, `ChunkView::set`) whenever an
unmanaged `Liquid`-kind cell is written (naturally excluding a promoted
body's own `set_owned` rasterizer writes, since those are always already
`managed`, and naturally *including* `demote_body`'s own `set_owned` call
that clears the flag — the exact moment a demoted body's cells become
promotable candidates again); a deterministic round-robin
`World::try_promote_one`, called once per `end_step` (so it applies
uniformly to the serial driver, the parallel driver, and every existing
test harness without touching `app.rs`), collecting every chunk's hint,
sorting by `ChunkCoord` (never raw `HashMap` order — issue #7's standing
rule, the same fix `chunks_to_sweep()` already applies) and picking one via
a persistent cursor for fairness across frames; a per-chunk
`demote_cooldown` map, populated by `demote_body`, reusing
`liquid::DEMOTE_COOLDOWN_FRAMES` (a flat cooldown, not §4c's full
exponential-backoff shape — that constant's own doc already flagged this
simplification back when it was written for Step 5's edge-demotion
cooldown, anticipating this exact reuse).

**It compiled clean, and broke five previously-passing tests the moment it
actually fired** — not just liquid tests: `smoke_rises_through_water`,
both parallel-driver fire/settling tests, and this section's own
`a_wide_deep_water_column_levels_out_instead_of_only_eroding_at_the_edges`
(see that test's own comment for the full writeup, copied in outline
here). Isolated the cause directly (temporarily stubbed `try_promote_one`
to a no-op — all five passed again; restored it — all five failed again)
rather than guessing, then diagnosed the water-column case specifically
since it has no fire/gas involved, via frame-checkpointed instrumentation:

```
frame 1:    bodies=1 active_chunks=7 spread=106
frame 10:   bodies=1 active_chunks=0 spread=106
frame 6000: bodies=1 active_chunks=0 spread=106
```

**The body promotes almost immediately (its starting shape — a uniform
rectangular block — is already internally flat, so it satisfies
quiescence on its very first real solver pass), snaps, sleeps, and then
never moves again for the rest of the run.** This is not a bug in any one
mechanism; it is a structural gap in what the persistent-flux solver (Step
3) and edge demotion (Step 5) can express *together*. The solver only
redistributes mass among a body's already-claimed columns. The only
mechanism that could put a column back onto the open CA grid where it
could spread freely — `overloaded_edge` — triggers on
`h[edge] > avg * EDGE_OVERFLOW_RATIO`, a check defined entirely in terms
of imbalance *among the body's own columns*. A uniformly-full body has
`h[edge] == avg` by definition, so this can never fire, no matter how much
open floor sits beside it or how physically wrong the body's current shape
is (a real fluid poured as a tall block does not stay a tall block once
promoted to "already flat" — it spreads until it is much shallower and
wider). Nothing in the design tracks "taller than the open floor beside me
would let a real fluid settle to" as a driving force at all — that quantity
lives outside a single body's own column array entirely.

**Why quiescence-based promotion criteria (§4a) didn't already predict
this.** §4a is correct that quiescence must not *gate* promotion — the
reasoning there (a body that's still moving is exactly what needs
accelerating) is sound. But every existing test that exercises the
solver's leveling behaviour (`the_solver_levels_an_uneven_column_over_many_
frames` and neighbours) promotes a body that already has real per-column
*height* variance to level — none of them promote an initially-flat block
sitting on open floor with room to spread, because no prior step's tests
had a reason to build that scene: manual `promote_liquid_body` calls in
`liquid.rs`'s own test suite always target scenes built to exercise a
specific mechanism, never scenes built to ask "should this have promoted
at all, given what's beside it." Automatic promotion is the first thing in
this whole build order that promotes *whatever the CA happens to hand it*,
which is exactly what exposed the gap — matching the design doc's own
"step 6 is the verdict on steps 3-4" framing better than step 6 itself did
in the end: automatic promotion turned out to be the real verdict on the
whole design, not the dead-band drop.

**Reverted in full** (`src/sim/chunk.rs`, `src/sim/world.rs`,
`src/sim/parallel.rs`, `src/sim/liquid.rs`'s `DEMOTE_COOLDOWN_FRAMES`
visibility change — confirmed via `git diff --stat` that only this
section's PLAN.md entry and the water-column test's updated comment
survive) rather than landed disabled-by-default: leaving broken,
unreachable plumbing in the tree is worse than no plumbing, since it would
need re-auditing for correctness *again* whenever someone eventually wires
it back in, on top of designing the actual fix. `cargo test --lib` (332)
and `cargo clippy --all-targets -- -D warnings` both clean after the
revert, confirming the working tree is back to exactly Step 6's own
validated state.

**Deferred, per standing instruction, rather than attempted further
tonight** — this needs real design work, not another implementation
attempt: candidates sketched here for whoever picks this up, **not
decided**. (a) Give the solver a pressure term relative to bed depth
*outside* the body's own column range, so an overfull body facing open
floor genuinely wants to spread even while internally flat — the largest
change, closest to what real hydrostatics would do. (b) Refuse promotion
(§3b) for a component whose shape is "suspiciously uniform" relative to
its surroundings (e.g., every column at the same height *and* open floor
adjacent to an edge) until the CA has had a chance to erode it toward its
natural footprint first — cheaper, but reintroduces exactly the kind of
per-cell local heuristic §1a(3) already proved insufficient once this
session, just relocated to a promotion gate instead of a leveling rule.
(c) Give `try_extend` (or a new sibling mechanism) the ability to claim
*open floor*, not just existing unmanaged liquid — extending the body's
own column range into `Empty` space when doing so would lower the body's
own potential energy, which is closer to (a) in spirit but scoped to the
existing extend/demote machinery rather than the solver's core math.
Whichever direction is chosen, the fix needs its own acceptance scene
(this water-column test, or one like it) run all the way through
promotion, not just through the solver in isolation the way every existing
Step 3-5 test does.
