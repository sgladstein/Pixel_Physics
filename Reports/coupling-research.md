# Solid–granular–fluid coupling: the part of M8 that isn't a cookbook

**Report C of four.** Depends on Report A (granular mechanics) and Report B
(liquid, revision 2); both are referenced throughout and neither is re-derived.

Scope: what happens at the boundary between a rigid body and the cell grid. The
M8 *pipeline* — connected components → contour → simplify → triangulate →
collider → erase/step/re-rasterize — is explicitly **not** the subject here. It
is well documented by the Noita talk and by two open-source reimplementations,
the tricky rasterization case is already identified in `PLAN.md`, and the first
two stages are already built in `rigid.rs`. It needs engineering, not research.

What needs research is everything the pipeline doesn't answer: what makes a
component become a body in the first place, what force the sand exerts back, what
buoyancy means on a cell grid, and — the item nobody has raised — whether a
third-party physics engine can satisfy this project's determinism requirement at
all.

---

## 0. Summary, stated first

1. **Sand pushing back on a body has a validated, cheap, directly applicable
   answer, and it is not a stress field.** Granular Resistive Force Theory
   partitions an intruder into surface elements, gives each a lift and drag that
   scale with depth and depend on orientation and direction of motion, and sums
   them linearly. It works with *no fitting parameters*, and the vertical-plane
   formulation — the one validated for legged locomotion — is exactly this
   engine's 2D case. `rigid::trace_contours` already produces the oriented
   boundary edges RFT needs as its elements. This is the report's main finding
   and it removes the strongest argument for adopting MPM or a granular stress
   field.

2. **Determinism is the sleeper risk in M8, and it is larger than the coupling
   problem.** `PLAN.md` reversed determinism to *required*. Rapier is
   deterministic only if bodies and colliders are "added/removed in the exact
   same order," and this engine's component discovery would naturally iterate a
   `HashMap` of chunks — the identical bug class already tracked in issue #7 for
   `scheduler::step`. Separately, rapier's `enhanced-determinism` feature
   **cannot be combined with its `parallel` feature**. This needs deciding before
   any rapier code is written, not after.

3. **Noita's answer to "when does a component become rigid" is mostly: it
   doesn't.** The trigger is explosion-driven and the conversion target is
   usually *sand*, not a rigid body. That is a much cheaper design than the one
   `rigid.rs`'s doc comment implies, and it is probably the right first cut here
   too.

4. **Buoyancy has a clean answer that composes with Report B.** The
   column-displacement method — project the body onto the ground plane, compute
   displaced volume per column, redistribute, then integrate the resulting
   forces back onto the body — was designed for heightfield water. Report B §5
   established that a 2D side-view heightfield is a 1D column array. The two fit
   together directly.

5. **MPM is the rigorous answer and should stay rejected**, for a reason worth
   recording rather than assuming.

---

## 1. What exists, and what the milestone actually requires

Built in `rigid.rs`: connected-component labelling over the CA grid, and
boundary extraction from a labelled component. The module's own doc comment is
candid that Douglas–Peucker, `earcutr` and the rapier loop are not built and
that their design decisions were deliberately not rushed.

Both existing pieces are pure queries wired to nothing. Nothing calls
`label_component`. That is the state to build from.

The pipeline stages are documented well enough elsewhere to treat as known work:
<cite index="15-1">take a 2D array of elements and use marching squares to get
the contour, use Douglas–Peucker line simplification to reduce the vertex count,
pass the simplified vertices to a triangulation algorithm, create a rigid body
from the resulting triangles, and if any elements in the rigid body are
destroyed, repeat the process.</cite> `PLAN.md` already records the rotation/leak
pitfall and the inverse-mapping fix.

Everything below is what that list doesn't cover.

---

## 2. The trigger: when does a component become a body?

`rigid.rs` has a labeller and a contour tracer and no answer to when to invoke
them. Running component discovery every frame over every solid region is
obviously unaffordable; running it never means the feature doesn't exist.

Noita's answer is narrower than the pipeline's prominence suggests. The trigger
is destruction, not stability: <cite index="8-1">when there's an explosion large
enough we look at the pixels that the explosion collided with, and if they happen
to be "good candidates", we will turn those into collapsing sand
materials.</cite> The conversion target for most debris is **sand**, not a rigid
body. Rigid bodies are reserved for things that need to tumble as a unit.

Their rigid-body representation is also simpler than a re-rasterization loop
implies: <cite index="8-1">each pixel in a rigid body knows that it belongs to
that rigid body and its location inside that rigid body, so when a rigid body is
updated the pixel gets its position that way; if a pixel belonging to a rigid
body is destroyed, the simulation recalculates the shape for the rigid body — or
bodies, if the shape is now cut into two or more pieces.</cite>

Two things this engine already has that map onto it:

- **`explosion::trigger` is the natural trigger site.** It already enumerates
  cleared cells and already calls `schedule_structural_check_around` on each.
  A "good candidate" test on the same enumeration costs nothing extra.
- **`structural.rs` already computes the more principled trigger.** A component
  that exceeds `max_unsupported_span` already breaks free — that is a detached
  component, identified by machinery that exists and runs. It is a better signal
  than an explosion proximity heuristic, and it is already wired.

**Recommendation.** Trigger on `structural.rs`'s break-free event, not on a
periodic scan. Convert to *powder* by default and to a rigid body only above a
size threshold and below a complexity threshold. This keeps the expensive path
rare, keeps the debris path on machinery that already works, and means the
milestone can produce visible results long before the rapier loop exists.

Report A §5's packing scalar matters here: a body converting to powder should
land as *loose* packing, since freshly disturbed material is loose. That is
free if A lands first.

---

## 3. Granular → rigid: resistive force theory

This is the question that has no answer in the pipeline: a rigid body resting in
or moving through sand needs a force back, and "which cell swapped with which"
does not supply one.

The general-purpose answer is a continuum stress model (Report A §7's μ(I), or
MPM). The cheap answer used in falling-sand games is to treat sand cells as
static collision geometry and let the physics engine resolve penetration, which
produces a body that sits *on* sand as though it were stone and never sinks,
settles, or gets buried.

There is a third option, and it is well validated.

**Granular Resistive Force Theory.** <cite index="19-1">In RFT a body is
partitioned into infinitesimal segments, each of which generates thrust and
experiences drag; linear superposition of forces from elements over the body
allows prediction of swimming velocities and efficiencies.</cite> Applied to
granular media, <cite index="18-1">both lift and drag forces on a plate element
are proportional to its depth, but depend sensitively on its orientation and
movement direction; without any fitting parameters, summation of element forces
over the intruder's leading surface accurately predicts the net lift and drag
forces on legs of complex geometry and kinematics.</cite>

Why this fits *this* engine specifically:

- **The vertical-plane formulation is the validated one, and this engine is a
  vertical plane.** The original work <cite index="20-1">develops a resistive
  force model in the vertical plane for legged locomotion on granular media,
  dividing an intruder of complex morphology and kinematics into small segments
  and measuring stresses as a function of depth, orientation, and direction of
  motion.</cite> Almost every other granular model in the literature is 3D and
  has to be reduced; this one is native.
- **The elements already exist.** `trace_contours` produces directed boundary
  edges "oriented so the filled interior is always on the edge's right." That is
  an oriented surface element with a known normal — precisely RFT's input. Depth
  below the granular surface and velocity direction come from the grid and from
  rapier respectively.
- **It is a sum, not a solve.** Per-element force, accumulated into a net force
  and torque, applied to the body. No iteration, no field, no global coupling. It
  respects `parallel.rs`'s bounded-local-write model because it *reads* the grid
  and writes only to the body.
- **The generality is unusually good.** <cite index="18-1">The complex dependence
  of lift and drag on intruder depth, orientation, and movement direction is
  generic to a large variety of granular media.</cite> One relation, scaled per
  material.

### 3a. The validity limits, which must be respected

RFT is not universally applicable and the boundaries are documented.

- **Frictional regime, low speed.** <cite index="19-1">RFT works well when the
  granular medium is slightly polydisperse, and in the "frictional fluid" regime
  such that frictional forces dominate material inertial forces, and when
  locomotion can be approximated as confined to a plane.</cite> The original
  validation held <cite index="23-1">for intruders moving in granular media at
  low enough speeds — for example ≤ 0.5 m/s for 0.3-mm glass particles — where
  intrusion forces are dominated by particle friction and are
  non-inertial.</cite>
- **Rate effects appear on impact.** <cite index="16-1">Resistive forces in
  granular intrusion can be explained with rate-independent drag laws when
  intrusion is slow, but rate effects can occur in situations like dynamic
  impact.</cite> A boulder dropped from height is exactly that case.
- **Pristine media only.** RFT's validations are for media described as
  <cite index="21-1">dry, homogeneous (no variation in volume fraction in space),
  flat, pristine (not previously disturbed)</cite>. A body that has already
  churned the sand around it is outside the validated regime.

The third limit is the interesting one, because Report A §5's packing scalar is
exactly the missing variable — spatial variation in volume fraction is what
"non-pristine" means. **RFT gives the force law; A's packing scalar gives the
state it should be scaled by.** This is a real dependency, not a courtesy
reference: implementing RFT without it means a body behaves identically in
freshly churned and undisturbed sand, which is the same class of complaint as
"trunks are one pixel wide."

**Recommendation.** Adopt RFT as the granular→rigid force law. Accept the
rate-independent form for the common case, and clamp or specially handle
high-speed impact rather than modelling it. Scale by packing fraction once A
lands.

---

## 4. Rigid → granular: displacement, and where mass goes

The reverse direction is simpler but has one trap.

The re-rasterize step erases the body's cells from the grid and rewrites them at
the new transform. **Any cell the body moves into must have its contents
displaced, not deleted.** The obvious implementation — write body cells, done —
silently destroys every grain the body passes through, and the engine's
conservation tests will catch it only if they are extended to cover rigid bodies,
which they currently are not.

Three cases, with different answers:

- **Into empty:** trivial.
- **Into powder or liquid:** displace. The material must be pushed to the nearest
  free cell along the body's motion direction. This is the *same primitive* Report
  A §3(ii) and the plant-growth discussion both wanted: push a bounded column of
  cells by one step. Cost is bounded by column length, which `MAX_REACH` already
  caps.
- **Into solid:** the body should not have got there. This is a collision the
  physics engine was supposed to prevent, and it means the collider and the grid
  have diverged. Worth a `debug_assert` — with the standing caveat that CI runs
  release only, so it would never fire there.

Note the ordering hazard with the parallel sweep: a body occupying cells in two
chunks that are in the *same* checkerboard pass violates the write-disjointness
proof, since the body writes to both. Either bodies are stepped in a separate
serial phase (simplest, and matches how particles and the active-site scheduler
already sit outside the sweep), or body cells must be treated as pinned during
the sweep. **The serial phase is almost certainly right** and should be decided
explicitly rather than discovered.

---

## 5. Rigid ↔ liquid: buoyancy by column displacement

Report B §5 established that a heightfield in a 2D side-view world reduces to a
1D array of water-column heights. That makes the standard heightfield coupling
method directly applicable.

The method: <cite index="14-1">coupling from a rigid body to a fluid is computed
using fluid displacement of the body in each grid cell — the body is projected
onto the simulation plane to determine which grid cells are covered by it, fluid
displacement from the body is computed for each grid cell based on displacement
within a corresponding vertical column of fluid, and that displacement is
distributed to neighbouring grid cells prior to the height field computation.
Coupling from the fluid to the rigid body is computed by integrating forces
imparted on the body by the fluid at each grid cell, and the integrated forces
are used to compute a new position for the body in a subsequent timestep.</cite>

That is a complete two-way scheme, it is per-column, and it is cheap. It also
has the same shape as §3's RFT — accumulate per-element forces, apply the sum —
which means both couplings can share one accumulation pass over the body's
boundary cells.

Two notes:

- Displacement redistribution to neighbouring columns is what produces the
  visible bow wave and the level rise when something heavy is dropped in. Skipping
  it gives a body that floats correctly but displaces no water, which reads as
  wrong immediately.
- This depends on Report B §5 landing. Until then, buoyancy has to be done by
  counting submerged body cells against liquid fill — workable as a stopgap, but
  it gives no displacement and no wave.

---

## 6. What is rejected, and why it's worth recording

**Material Point Method.** MPM with a Drucker–Prager or μ(I)-family plasticity
model is the rigorous unified answer: it represents sand, water and solids in one
solver and gets coupling for free rather than by construction. Kamrin's group has
the relevant continuum work, including <cite index="16-1">continuum modelling and
simulation of granular flows through their many phases</cite> and
<cite index="16-1">continuum modeling of projectile impact and penetration in dry
granular media</cite> — the latter being precisely the case §3a flags as outside
RFT's validity.

Rejected because adopting it means replacing the CA, not augmenting it. This
engine's entire economy — dirty rectangles, chunk sleeping, the active-site
scheduler, `Cell` as a 12-byte tagged union — is built on discrete cells and
activity-proportional cost. MPM is a particle-and-grid solver with a global
transfer step every frame. It is the right answer to a different project.

Worth recording because the reason is architectural rather than "too slow," and
because §3a's impact-regime gap is a real known limitation rather than an
oversight — if high-speed impact behaviour ever becomes important enough,
this is the direction, and it is a rewrite.

**salva2d.** Dimforge maintains an SPH fluid crate with built-in rapier coupling,
and it is genuinely tempting because the integration work is already done.
Rejected on Report B's grounds, which are unchanged: SPH is a particle method,
the particle budgets don't fit a 163,840-cell world, and there is no compute path.
Worth naming so that "why aren't we just using salva" gets answered once.

---

## 7. Determinism: the risk nobody has raised

`PLAN.md` reversed determinism from "not required" to **required**, scoped to
same-build, because off-camera catch-up is only sound if outcome is a pure
function of (state at unload, elapsed time, seed). It notes the engine is ~90%
there, with `scheduler::step`'s `HashMap` drain order as the one known violation.

M8 introduces two more, and they are worse because they are in a dependency.

**7a. Insertion order.** Rapier's determinism guarantee is conditional:
<cite index="32-1">two simulations run with the same initial conditions if all the
simulation parameters are initialized with the same values, rigid-bodies,
colliders and joints are constructed the same way, and they are added/removed in
the exact same order. All the values used to initialize the physics simulation
must result from cross-platform deterministic operations in order to preserve
determinism.</cite>

Component discovery naturally iterates chunks. `World::chunks` is a `HashMap`.
Iterating it to find detached components and creating rapier bodies in that order
is **the identical bug already tracked as issue #7**, relocated into a third-party
solver where it will be far harder to diagnose — divergence will show up as a
boulder landing somewhere different on replay, not as an obviously wrong drain
order.

`chunks_to_sweep()` already sorts, and it exists precisely because this problem
was foreseen for the sweep. Body creation needs the same treatment: a sorted,
position-derived ordering, established before the first body is inserted.

**7b. The feature-flag conflict.** <cite index="30-1">`enhanced-determinism`
enables cross-platform determinism (assuming the rest of your code is also
deterministic) across all 32-bit and 64-bit platforms that implement IEEE 754.
Currently, the `enhanced-determinism` feature cannot be enabled at the same time
as the `parallel` [feature].</cite>

The good news is that this project's requirement is *same-build only*, and the
default build is <cite index="26-1">still locally deterministic, on the same
machine</cite> without the feature. So `enhanced-determinism` is not needed.

The caution: **do not enable rapier's `parallel` feature.** It is separate from
this engine's own rayon usage in `parallel.rs`, so there is no conflict there,
but a parallel constraint solver is where local determinism is most likely to be
lost, and the documented incompatibility with `enhanced-determinism` is a signal
about exactly that. The engine's own parallel sweep already delivers its speedup;
rapier does not need to be parallel too.

**Recommendation.** Before any rapier code: fix the ordering (7a) at the same time
as issue #7, since it is the same fix; pin the feature set explicitly in
`Cargo.toml` with a comment pointing here; and add a determinism regression test
that runs a body-heavy scene twice in one process and compares final transforms
bitwise. That test costs almost nothing and is the only thing that will catch a
regression here.

---

## 8. Sleeping, streaming, and where bodies live

Two architectural questions the pipeline doesn't address.

**Sleeping.** Rapier sleeps inactive bodies natively, which matches this engine's
economy well. But a sleeping body still occupies cells, and those cells must not
be re-rasterized every frame — a sleeping body should be *written into the grid
as ordinary cells* and removed from the rapier world entirely, waking back into a
body only on disturbance. That is the same promotion/demotion pattern Report B §5
proposes for settled water bodies, and the two should share a design.

**Streaming.** A body straddling a chunk boundary when that chunk unloads is an
unsolved case, and per Report B §4c the resident set under M10 is large. Options
are to pin chunks containing bodies (simple, leaks memory if a body parks at the
frontier), or to demote bodies to cells on unload and re-promote on load (matches
the sleeping design, loses angular velocity and mid-air state). The second is
probably right and is another reason to build demotion first.

Neither of these is researched literature; both are design decisions this report
flags rather than answers.

---

## 9. Build order

1. **Fix body-creation ordering as part of issue #7 (§7a).** Before any rapier
   dependency is added. Cheapest item here and the hardest to retrofit.
2. **Trigger from `structural.rs` break-free, converting to powder (§2).**
   Produces visible results with no rapier at all, and exercises `label_component`
   for the first time.
3. **The displacement primitive (§4).** Shared with Report A §3(ii). Needed before
   any body can move through material without destroying it.
4. **Bodies as a serial phase, outside the parallel sweep (§4).** Decide
   explicitly.
5. **The rapier loop**, with the feature set pinned per §7b, plus the
   double-run determinism test.
6. **RFT for granular→rigid (§3)**, rate-independent form, scaled by Report A's
   packing scalar once available.
7. **Column-displacement buoyancy (§5)**, after Report B §5's 1D array exists.
8. **Sleeping/demotion (§8)**, shared design with Report B's settled-body
   promotion.

Steps 1–3 deliver visible falling debris with zero third-party dependency and
zero coupling work. That ordering is deliberate: it front-loads the parts that
work, and it means the milestone `PLAN.md` calls "most likely to consume months
without a playable result" produces a playable result early.

---

## 10. Acceptance criteria

- **10a. Conservation across rigid motion.** Total non-empty cell count must be
  conserved to within **0.1%** while a body traverses a powder field. The current
  conservation tests do not cover rigid bodies at all, so this is new coverage,
  not a tightening.
- **10b. No leaking at any rotation.** `PLAN.md`'s own verify criterion, stated
  numerically: with a body rotating through a full turn under a sand column,
  **zero** grains may appear inside the body's interior. Test at 16 evenly spaced
  angles including the off-axis ones, since axis-aligned rotations hide the bug.
- **10c. Sinking depth tracks packing.** A body of fixed mass released onto loose
  sand must come to rest **measurably deeper** than the same body on settled sand
  of the same depth. This is the RFT-plus-packing acceptance test and it fails
  today for want of both mechanisms.
- **10d. Static equilibrium.** A body resting on sand must not creep. Net RFT
  force at zero velocity must balance gravity to within the solver's tolerance,
  with no net drift over 2,000 frames. Numerical creep is the most likely failure
  mode of a summed-element force law and the least likely to be noticed by eye.
- **10e. Flotation.** A body of density below the liquid's must reach a stable
  waterline where displaced liquid mass equals body mass, within **one cell**.
  A body denser than the liquid must reach the floor.
- **10f. Displacement.** Dropping a body into a pool must raise the pool's level
  by the submerged volume, within **5%**. This is what distinguishes §5's method
  from cell-counting buoyancy.
- **10g. Determinism.** A scene with ≥ 20 bodies, run twice in one process from
  the same seed, must produce **bitwise identical** final transforms. Extend to a
  save/reload cycle once M10 exists.
- **10h. Cost ceiling.** A scene with 20 active bodies in a sand field must stay
  within the frame budget, with the per-body cost of RFT reported separately from
  rapier's own step so a regression can be attributed.

---

## 11. Deletion tests

| Mechanism | Test that must fail without it |
|---|---|
| Inverse-mapped rasterization | 10b — forward mapping leaks at non-axis-aligned angles |
| Displacement on re-rasterize | 10a — cell count drops as the body sweeps through powder |
| RFT depth term | 10c — sinking depth becomes independent of depth and packing |
| RFT orientation term | A flat plate dragged edge-on must experience measurably less drag than face-on; without it, identical |
| Column displacement (vs cell-count buoyancy) | 10f — the pool level doesn't move |
| Sorted body-creation order | 10g — two runs diverge |

---

## 12. What was not directly accessible

- **Purho's GDC 2019 talk** — the primary source for Noita's approach. Read
  through the 80.lv written interview and two secondary write-ups, which quote
  the rigid-body pixel-ownership scheme and the explosion-to-collapsing-sand
  trigger directly. **The talk itself was not watched**, and the 80.lv quote in §2
  is cut off mid-sentence at exactly the point where it turns to how chunks are
  made to fall. That is the sentence most relevant to §2's trigger question, so
  **watch the talk before finalising the trigger design** — it is freely
  available on GDC's YouTube channel.
- **Li, Zhang & Goldman (2013), Science 339:1408** — RFT's primary. Read the arXiv
  preprint's introduction and the abstract of the *Physics of Fluids* review
  directly, plus a conference abstract stating the depth-proportionality and
  no-fitting-parameters results. **The actual force relations — the functional
  form of lift and drag versus orientation β and intrusion angle γ — were not
  extracted**, and implementing §3 requires them. The Georgia Tech review PDF
  (Zhang & Goldman 2014) is openly hosted and contains the figures; it is the
  document to work from.
- **The column-displacement coupling method** is quoted from an NVIDIA patent
  filing, which is a legitimate technical description but is written as a patent
  claim rather than as an algorithm, and **patent scope is a real consideration if
  this ships commercially**. The underlying technique is standard in the graphics
  literature and almost certainly has non-patent prior art; that should be checked
  rather than assumed.
- **Rapier's determinism guarantees in §7** are quoted from the JavaScript
  binding's documentation page and the Rust getting-started page. The
  `enhanced-determinism`/`parallel` incompatibility is stated in the Rust docs
  and is current as of the pages read, but rapier is pre-1.0 and its own docs warn
  about breaking changes — **verify against the version actually pinned** rather
  than trusting this report.
- **MPM's suitability was assessed from citation context, not from the papers.**
  The rejection in §6 rests on an architectural argument this report makes, which
  does not depend on the papers' contents; the papers are named so that a future
  reconsideration knows where to start.

---

## 13. Handoff

**To Report A's implementation:** §3a makes the packing scalar a hard dependency
rather than a nice-to-have. RFT's validated regime is undisturbed media, and a
body in sand is by definition disturbing it. If A ships without packing, RFT
ships without its state variable.

**To Report B's implementation:** §5 depends on B §5's 1D column array, and §8's
sleeping/demotion pattern is the same design as B's settled-body promotion. Build
one promotion/demotion mechanism, not two.

**To Report D (ecology), if it happens:** nothing. C and D are independent.

**Back to `PLAN.md`:** §7 is the finding that should change the milestone's shape.
M8 is currently framed as a rendering-and-geometry pipeline. Its largest
unrecognised risk is that adding a third-party physics solver to a project with a
hard determinism requirement is a determinism problem first and a physics problem
second.
