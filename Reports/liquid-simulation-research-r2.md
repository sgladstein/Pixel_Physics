# Liquid simulation research, revision 2: the three method families revision 1 didn't survey

**Report B of four.** This does **not** replace
[`Reports/liquid-simulation-research.md`](liquid-simulation-research.md). That
report's §1–§4 — the SPH → PBF → PIC/FLIP survey, the falling-sand engine
comparison, and the diagnosis that `update_liquid`'s powder-identical
diagonal-fall phase runs to exhaustion before any liquid-specific mechanism gets
a turn — are correct and stand unchanged. Its §5 recommendation is also correct
and still unimplemented.

What revision 1 surveyed was one axis: **particle methods**, plus full Eulerian
Navier–Stokes, plus what other falling-sand games do. That axis was correctly
rejected. Three other families were never examined, and two of them are shaped
*with* this engine's architecture rather than against it.

---

## 0. Summary, stated first

1. **This engine already implements Volume of Fluid**, without knowing it. A
   0–1000 fill fraction per cell on a fixed grid is Hirt & Nichols' VOF function
   F. That means 45 years of literature applies directly to the artifacts —
   including the residual-droplet symptom, which has a name (*flotsam and
   jetsam*), a documented cause (piecewise-constant interface reconstruction),
   and a published fix that fits in three cells.

2. **Free-surface LBM is the method actually shaped like this engine**, and the
   arithmetic is more favourable than revision 1's rejection of particle methods
   would suggest — roughly 0.6–1.2 ms for a full-grid D2Q9 step at this world
   size, against a 16.6 ms budget. It is not rejected here on cost. It is
   deferred on two objections revision 1 never had to make: **composability** —
   LBM would own the liquid layer completely, and density-driven displacement
   (sand sinking through water) is not something it expresses — and **scaling**,
   since that per-step figure holds only at today's fixed 512×320 world and rises
   to 6–12 ms at streaming-scale resident area (§4c). Standard LBM is the one
   candidate here whose cost tracks world size rather than activity, which is the
   opposite of how every other subsystem in this engine is built.

3. **In a 2D side-view world, a heightfield is one-dimensional.** This is the
   finding that reframes the leveling problem. Bridson's own real-time
   recommendation — which revision 1 quotes and then moves past — reduces here
   to an array of water-column heights indexed by x, a few hundred entries wide.
   The virtual-pipes model on that array levels a pool in **O(width)** rather
   than the current O(width²), costs microseconds, and is the same technique
   worldgen will need for erosion.

4. **Hydrostatic pressure is a fifth symptom nobody has named**, and the engine
   already owns the channel it belongs in. Dwarf Fortress's answer is a path
   trace with a deliberate, documented cheat; it is directly portable.

5. `MIN_LIQUID_TRANSFER = 150` is doing more damage than its doc admits. §3c.

---

## 1. Four symptoms, not one bug

The playtest finding and revision 1 both treat this as a single problem. It is
at least four, with different fixes and different owners:

| # | Symptom | Status | Fix lives in |
|---|---|---|---|
| 1 | Pours build an angle-of-repose slope | Diagnosed (rev 1 §3) | rev 1 §5, unimplemented |
| 2 | Wide bodies level in O(width²) frames | Mitigated by tuning, never solved | §5 below |
| 3 | Draining pools decay into residual droplets | Observed in the harness, never explained | §3 below |
| 4 | No hydrostatic pressure — no U-tube, no spurt | Never raised | §6 below |

Implementing rev 1 §5 fixes symptom 1 and nothing else. That is worth knowing
before doing it and being disappointed, because the visible complaint ("water
looks wrong") will only partly go away.

---

## 2. What revision 1 established, and what it left open

Standing conclusions, not revisited:

- SPH, Position Based Fluids and PIC/FLIP are all correctly rejected. No compute
  path exists, the particle budgets don't fit, and the technique tier is wrong
  for a 163,840-cell grid sharing a frame with six material kinds.
- Every comparable falling-sand engine uses a discrete per-cell CA rule.
- The diagnosis of symptom 1 — mechanism *ordering*, not mechanism tier — is
  right.

One argument from revision 1 §5 needs qualifying, because it is load-bearing and
it does not generalize:

> "every real particle/grid fluid technique surveyed needs either global coupling
> (an Eulerian pressure solve reaching across the whole connected fluid body) or
> a genuinely interacting particle set — both structurally different from the
> bounded-local-write model `parallel.rs`'s correctness proof depends on."

That is true of everything revision 1 surveyed. It is **not** true of LBM, whose
entire step is nearest-neighbour, and it is not true of a 1D heightfield, which
is a separate data structure that doesn't touch the CA grid's write model at
all. The architectural-mismatch argument was sound about particle methods and
should not be carried forward as a general one.

---

## 3. Finding 1: this engine implements VOF, and the artifacts have names

`Cell::aux` holding a fill amount on the `LIQUID_FULL = 1000` scale, on a fixed
grid, with cells at 0 < F < 1 being surface cells, **is** the Volume of Fluid
method (Hirt & Nichols 1981). Not an analogy — the same representation, arrived
at independently. <cite index="84-1">In the VOF method a function F is
introduced with values between zero and one, indicating the fractional volume of
a cell filled with a certain fluid.</cite>

That has an immediate payoff, because the failure modes are catalogued.

### 3a. Symptom 3 is flotsam and jetsam

<cite index="87-1">The original VOF method has two main drawbacks. The first is
that flotsam and jetsam can appear — small droplets disconnecting from the free
surface. The other is the gain or loss of water due to rounding the VOF function
when F > 1 or F < 0.</cite>

Both of those are exactly what the debug harness saw. The droplets are not an
RNG tie-breaking artifact; they are the standard consequence of a
piecewise-constant, stair-stepped interface. <cite index="84-1">In the original
VOF method, where a piecewise constant, stair-stepped reconstruction of the
interface is used, a lot of flotsam and jetsam can occur.</cite>

The engine's `write_liquid_transfer` handles the F < 0 half correctly — a cell
drained to zero converts to `Cell::EMPTY` outright rather than lingering. The
F > 1 half is handled by the `LIQUID_MAX_COMPRESS` clamp. Neither addresses the
droplets.

### 3b. The published fix is three cells wide

<cite index="87-1">By combining the VOF method with a local height function,
these problems do not appear any more. For every surface cell a function is
defined locally giving the height of the fluid in a column of three cells, in
the direction of the coordinate axis most normal to the interface.</cite> The
measured result: <cite index="87-1">the standard VOF method generates much
flotsam and jetsam, whereas in the adapted method only some small holes in the
fluid are present. Mass conservation is perfectly satisfied in the adapted
method, whereas the fluid level in the standard VOF method increased by
2%.</cite>

A three-cell column read, done only for surface cells (0 < F < FULL), inside a
rule that is already reading its vertical neighbour. This is small, it is
principled, and it fixes both the droplets and a mass-conservation error the
engine has never measured.

Related, and worth having on file for whoever implements the advection: the
transfer scheme itself has a name and a known accuracy bound. <cite index="82-1">Hirt and Nichols proposed the donor–acceptor method, the simplest
treatment for controlling numerical diffusion during advection of the VOF
function F; however it is only first-order accurate.</cite> The mechanism is
<cite index="91-1">controlled downwinding — including data from the downwind
(acceptor) cell as well as the upwind (donor) cell to avoid non-physical volume
fractions below zero or above unity</cite>, which is essentially what
`transfer_liquid_horizontal`'s "half the difference, capped by the destination's
remaining room" already does. The engine reinvented donor–acceptor too. PLIC
(piecewise-linear interface reconstruction) is the higher-order alternative and
is almost certainly not worth it here.

### 3c. `MIN_LIQUID_TRANSFER = 150` is a bigger concession than its doc says

The constant's own comment is honest that it is "a real trade of some precision
for settling speed, not a rounding-level tweak." It understates it. 150 is 15%
of `LIQUID_FULL`, and it is a **dead band**: two horizontally adjacent cells may
differ by up to 15% of a full cell and be treated as settled. Across a wide
pool, adjacent-pair errors of that size can accumulate into a permanently
non-flat surface that no amount of further simulation will remove — the
mechanism has switched itself off.

This is not a tuning error. It is the correct response to a real constraint: the
underlying process is O(width²) diffusion, and the only lever available was
where to stop it. Fix the convergence rate (§5) and this constant can drop back
toward its original 8 without the 12,000-frame settling time that forced it up.

**Treat `MIN_LIQUID_TRANSFER` as the diagnostic, not the setting.** If a change
to the leveling mechanism doesn't let this number go down, it didn't fix the
underlying problem.

### 3d. One hazard while in here

`liquid_fill` treats `aux == 0` as `LIQUID_FULL`, not empty. That sentinel is
documented and load-bearing (it is what lets every pre-existing liquid-creation
site keep working), but it means **any code path that writes `aux = 0` on a
`Liquid` cell silently creates a full cell of water from nothing**.
`write_liquid_transfer` guards this correctly by converting to `Cell::EMPTY`.
`Cell::set_aux` is public and unguarded.

This is the same tagged-union hazard that produced the 8→12 byte `Cell`
widening, in a new location. A `debug_assert` in `set_aux` — that a `Liquid`
cell is never given `aux == 0` — would catch it, though note it would only fire
in a debug build, which CI currently never runs.

---

## 4. Finding 2: free-surface LBM, with the arithmetic actually done

The Lattice Boltzmann Method is neither a particle method nor a pressure-projection
solver, so revision 1's rejection reasoning does not reach it. Each cell holds
distribution functions along a fixed set of velocity directions —
<cite index="80-1">the most common model for two dimensions is D2Q9 with nine
velocities</cite> — and a step is *collide locally, then stream to nearest
neighbours*. No global solve. No neighbour search. Pressure comes from density,
so symptom 4 is solved by construction.

Free surfaces are handled per-cell: <cite index="80-1">only distribution
functions from fluid cells or other interface cells stream normally, while those
that would come out of empty cells are reconstructed from the free-surface
boundary conditions — these can be handled on a per-cell basis and do not
require additional ghost layers around the interface.</cite> And on complexity:
<cite index="81-1">in comparison with a simple finite-difference Navier–Stokes
solver, the implementation is much simpler, however it requires more
memory.</cite>

Thürey, Rüde & Körner reported this at interactive rates on 2005 CPUs in 3D.
This engine is 2D.

### 4a. The cost, computed rather than asserted

LBM is memory-bandwidth bound, not compute bound — every performance paper in
the area says so. So the estimate is a bandwidth calculation.

- D2Q9 in f32: 9 × 4 = 36 bytes of distribution functions per cell.
- Classic two-lattice streaming: read 36 + write 36 = **72 bytes/cell/step** of
  traffic, and 2× storage.
- This world: 512 × 320 = **163,840 cells**.
- Full-grid traffic per step: 163,840 × 72 ≈ **11.8 MB**.

At a modest sustained 10–20 GB/s on a 4-logical-core desktop (the machine the
README's ~3.6× parallel speedup implies), that is **0.6–1.2 ms per full-grid
step**. Free-surface bookkeeping — mass tracking, interface cell promotion and
demotion, distribution reconstruction — realistically multiplies that by 2–3, so
call it **1.5–3.5 ms**, and less in practice since only fluid, interface and a
thin gas layer need updating rather than the whole grid.

Storage is the real pressure: 11.8 MB two-lattice, which on a modest desktop is
comparable to the entire L3 and would thrash against the CA grid's own ~2 MB of
`Cell` data. That is addressable — in-place streaming schemes exist precisely
for this. <cite index="159-1">Esoteric-Pull eliminates the redundant copy of
distribution functions in memory, almost cutting memory demand in half and
slightly increasing performance due to implicit bounce-back boundaries</cite>,
and <cite index="159-1">decoupling arithmetic precision from memory precision —
FP32 arithmetic with FP16 storage — almost cuts memory demand in half again and
almost doubles performance, without impacting overall accuracy for most
setups.</cite> That path lands at ~3 MB.

Measured CPU behaviour supports the bandwidth model: for D2Q9 on a desktop
part, <cite index="158-1">for geometries occupying more memory than cache
capacity, performance corresponds to about 2/3 of theoretical peak memory
bandwidth on an i9-12900K</cite>, with the caveat that this is strongly
machine-dependent — <cite index="158-1">in the bandwidth-limited regime the
performance of the 7945HX *decreases* with a growing number of threads</cite>,
which matters for a rayon-parallel design.

**Conclusion on cost: LBM is affordable here.** That is a genuine correction to
the impression revision 1's §5 leaves.

### 4b. Why it is still deferred

Not cost. Two other things:

- **Composability.** LBM would own the liquid layer entirely, and this engine's
  liquids are not a standalone fluid — they participate in density-driven
  displacement (sand sinks through water, gas rises through it), material
  reactions, phase change from `fire.rs`, and being eaten by roots. All of that
  lives in the CA sweep and is expressed as cell swaps. LBM has no notion of
  "swap with the denser material." Multiphase LBM exists and is substantially
  harder than the single-phase free-surface method above. This is the real
  objection and it is architectural, not numerical.
- **Time stepping.** LBM's step is tied to the lattice; getting believable
  gravity-driven flow means either many substeps per frame or a scaled gravity,
  and the free-surface work in this area needed adaptive time stepping to stay
  stable. That is a tuning surface the engine doesn't currently have anywhere
  else.

### 4c. §4a's numbers are for today's fixed world, and do not survive M10

**Added after review. This is the finding that changes §4's conclusion from
"affordable, deferred on composability" to "affordable only at the current world
size, and gated on a subsystem that does not yet exist."**

§4a's estimate is computed against 512×320 = 163,840 cells and 40 chunks. The
quantity that actually drives LBM cost is **resident simulated area**, and M10
multiplies it. Taking a plausible streaming resident set of ~400 chunks
(roughly one screen of margin in each direction, ≈1.6M cells) — a 10× factor:

| | Today (163,840 cells) | ~400 chunks resident (≈1.6M cells) |
|---|---|---|
| D2Q9 traffic per full step | 11.8 MB | ≈118 MB |
| Time at 10–20 GB/s | 0.6–1.2 ms | **6–12 ms** |
| Storage, two-lattice f32 | 11.8 MB | ≈118 MB |
| Storage, in-place + FP16 | ≈3 MB | ≈30 MB |

The 6–12 ms figure is optimistic rather than conservative: at 163,840 cells the
working set is near cache capacity, at 1.6M it is comfortably outside it, which
pushes sustained bandwidth toward the low end of that range. This is the fluid
solver alone, before the CA sweep, the field grid, or rendering.

**This engine's own worldgen design makes the sparsity escape unavailable.**
`Reports/worldgen-design.md` §2 makes the water table a first-class structure —
everything below it is saturated — and §7 adds local aquifers on top. "Only run
LBM where fluid exists" is a weak constraint in a world where a large fraction
of every resident chunk below a depth line is water.

**The structural objection, which matters more than the numbers.** Every cost in
this engine is proportional to *what is changing*, not to world size: dirty
rectangles, chunk sleeping, field sleeping (issue #4), the active-site
scheduler. Standard LBM is a whole-grid pass every step — the same shape
`field::step` had before issue #4, which this engine has already had to retrofit
sleeping onto once. LBM would be the only subsystem whose cost scales with
resident world size rather than activity, which makes it the thing that decides
how far the world can stream.

**So the gate changes.** It was "does it fit in the budget." It is now **"can it
sleep."**

Two reasons to think that is answerable, and one reason it is not cheap:

- LBM's step is strictly nearest-neighbour, so "is this region quiet" is a
  chunk-local decision — unlike a global pressure solve, where skipping a region
  breaks the solve. The equilibrium distribution at zero velocity is analytic, so
  a settled body could plausibly be frozen and reconstructed on wake rather than
  stored, which addresses the memory figure as well as the time one.
- The literature does go here. Thürey & Rüde's *Stable free surface flows with
  the lattice Boltzmann method on adaptively coarsened grids* (Comput. Vis. Sci.
  12(5), 2009) is the direct precedent, and the same group reports
  <cite index="75-1">an algorithm using adaptive grids to reduce required
  computational time by more than a factor of three for simulations with large
  volumes of fluid</cite>. Patch-based LBM parallelization with dynamic load
  balancing, which <cite index="75-1">divides the problem into separate
  simulation chunks distributed over multiple parallel processors</cite>, maps
  onto this engine's existing chunk model directly.
- **But coarsening is not a small addition.** Mass conservation at a coarse–fine
  boundary is a known hard sub-problem with its own literature:
  <cite index="173-1">lattice-Boltzmann methods require interpolation and
  mechanisms for ensuring conservation at each lattice velocity direction near
  the interface between coarse and fine grids</cite>, and the published fixes are
  substantial machinery — a volumetric formulation in which
  <cite index="176-1">mass conservation is imposed by allowing lattice-Boltzmann
  particles to move from coarse grid cells to fine grid cells and vice versa in
  the propagation step</cite>, constrained least-squares space–time
  interpolation with <cite index="173-1">an imposed mass conservation
  constraint, with distributions corresponding to edge lattice velocities
  requiring special handling</cite>, or refluxing at the interface. This lands on
  the one axis free-surface LBM is already fragile about, and it is a subsystem
  in its own right rather than a flag on an existing one.

**Recommendation: prototype-gated, not adopted or rejected — and the gate is now
stricter.** If a use case appears that genuinely needs pressure waves in water —
a breached dam, a pressurised pipe network — build a standalone D2Q9
free-surface prototype against the ascii harness before deciding. Do not adopt
it on paper; do not reject it on paper either, since the arithmetic says the
per-step cost fits and that is worth having on record. The prototype must:

1. Demonstrate a working sleeping or coarsening scheme, **not** defer it. If it
   cannot sleep, the answer is no regardless of how good the per-step number
   looks.
2. Be measured at **streaming-scale resident area**, not at 512×320.
3. Report mass conservation across any coarse–fine boundary it introduces,
   against §10a's bar.

---

## 5. Finding 3: in a side-view world the heightfield is 1D

This is the finding with the best cost-to-benefit ratio in the report, and it
turns on a geometric observation that is easy to miss.

Revision 1 §1.6 quotes Bridson's three real-time answers — procedural,
heightfield, small particle systems — and then moves on, treating heightfield as
a technique for large open bodies in 3D. But this world is a **2D side view**.
A heightfield over a 2D side-view world is not a 2D surface; it is a
**one-dimensional array of water-column heights indexed by x**. For a 512-wide
world that is 512 entries.

The virtual-pipes model on that array is Mei, Decaudin & Hu's technique reduced
to one dimension: <cite index="93-1">each cell carries a water column, connected
to its neighbours by virtual pipes, with water transported by the hydrostatic
pressure difference between neighbouring cells — the pipe model can be seen as
an explicit method for the shallow-water equations.</cite> Despite its usual
association with GPUs, <cite index="96-1">the algorithm works equally well on a
CPU and can easily be parallelized.</cite> And the property that matters most
here: <cite index="99-1">heightmap methods can simulate liquids that are
arbitrarily deep or shallow, with no impact on the resolution of the
simulation.</cite>

Consequences for this engine:

- **Leveling becomes O(width), not O(width²).** A pressure-difference-driven
  flux between adjacent columns propagates a level change across the whole body
  at one cell per step, and the flux is proportional to the height difference,
  so it converges geometrically rather than diffusively. This is the direct fix
  for symptom 2 — the one thing neither `flow_rate` nor
  `HORIZONTAL_TRANSFER_REACH` nor `MIN_LIQUID_TRANSFER` could address.
- **It is nearly free.** Hundreds of columns, a handful of arithmetic ops each.
  Microseconds, not milliseconds, against a 16.6 ms budget.
- **It doubles as worldgen erosion.** The virtual-pipes model is the standard
  hydraulic erosion technique — the same paper does both, and there is an
  extensive follow-up literature including <cite index="95-1">extensions to
  multi-layered heightmaps able to create overhangs, arches and to some extent
  caves, with an iterative bedrock support check that prevents floating
  terrain</cite>. That last clause is notable: it is independently solving the
  same problem `structural.rs` solves. Report D's worldgen work should build on
  whatever is decided here rather than choosing a second water representation.

### 5a. The seam, which is where the difficulty actually is

A side-view world is not a heightfield. It has caves, overhangs, pipes, and
sealed vessels — a column of x can contain several disconnected bodies of water
at different levels. A naive single-height-per-column model is simply wrong for
all of them.

The workable shape is a **hybrid, applied per connected body**:

- Small, dynamic, falling, or splashing water: the existing CA rule, unchanged.
  This is what it is good at.
- Large, settled, connected bodies with a clear free surface: represented by
  per-column heights within that body, leveled by virtual pipes, and rasterized
  back to cells.
- Promotion and demotion between the two on connectivity and quiescence.

Two hard questions this report cannot answer from the literature, because nobody
has published this exact hybrid:

1. How is a body identified and tracked cheaply? The engine has
   `rigid::label_component` — a bounded flood fill — already built and currently
   wired to nothing. That is suggestive, but it labels `Solid` and would need a
   liquid variant plus incremental maintenance rather than a full re-label.
2. What does the seam look like when a body is *partly* settled — a lake with a
   waterfall entering it? Getting a visible discontinuity at the boundary
   between the two representations is the obvious failure mode.

**This is the piece to prototype first**, because it is cheap to try and its
risk is concentrated entirely in the seam rather than in the physics.

---

## 6. Finding 4: hydrostatic pressure, and a channel with no reader

Symptom 4 has never been raised in any report or issue. Water in this engine
does not push. A sealed U-tube won't equalise; a breached dam won't spurt; water
under a heavy column exerts no more force than water in a puddle.

Revision 1 §2.2 established that The Powder Toy simply doesn't simulate pressure
for water. The interesting question is what the engines that *do* pay for it.

Dwarf Fortress's answer is a path trace, and it is candid about being an
approximation. <cite index="169-1">Fluids moving under pressure do not just move
to adjacent tiles; they trace a path through other full tiles of fluid, and can
effectively teleport through tiles already filled with fluid. When teleporting,
fluids generate no flow and do not push objects around. The path can go back up
— but never higher than the z-level of the first full tile on the path.</cite>
And the cheat is explicit: <cite index="165-1">DF water pressure does not exactly
match natural hydrostatic pressure — it fills to a z-level one level lower than
the source. This is for reasons of CPU time-saving, as stated by Toady; the game
stops not when all ends of the system are at the same level, but when the far
levels are one lower than the source.</cite>

That is directly portable, and the "never higher than the first full cell on the
path" rule is what keeps it bounded and stops it becoming a global solve. Cost
is a path trace along full cells, which is the same shape as
`transfer_liquid_horizontal`'s existing outward walk.

Note also the contrast case, which is what this engine currently is:
<cite index="167-1">the water in DwarfCorp does not currently support a pressure
model (as in Dwarf Fortress), so water never moves upward.</cite>

### 6a. The channel already exists

The coarse field grid has a **pressure** channel. `explosion::trigger` writes it.
`structural.rs`'s `break_free` writes it. Debris velocity reads it. **Liquid
neither reads nor writes it.**

That is the third instance of a pattern this codebase has now hit repeatedly: the
light channel had two readers and no writer until architecture §2; canopy density
has a writer and an always-zero reader today; pressure has producers and no
liquid consumer. It is worth a standing check rather than a fourth individual
fix.

Three cautions on using it:

- **The trace needs an explicit length cap, and this is not optional under M10.**
  DF's "never higher than the first full cell on the path" rule bounds the trace's
  *height*, not its *length*. In today's fixed world the longest possible path is
  a few hundred cells; in a streamed world with the water table
  `worldgen-design.md` §2 makes a first-class structure, a connected saturated
  body can span the entire resident set, and an uncapped trace along it is an
  unbounded per-frame cost. A hard cap is required, and it becomes a visible
  gameplay constant — pressure equalises over N cells and no further. Decide it
  deliberately rather than discovering it when a saturated zone stalls a frame.
- Field resolution is `FIELD_SCALE = 8`. A hydrostatic gradient over an 8-cell
  block is coarse, and `field_at` is block-nearest — the same degeneracy that
  broke worm thermotaxis and tree phototropism. Any liquid pressure reader must
  go through `field_at_bilinear`, not `field_at`. This is already a known trap
  with a known fix; it just needs applying here too.
- Report A §4 warns that granular stress is a *different* quantity from air
  pressure with different boundary conditions. Hydrostatic liquid pressure is a
  third. Whether all three can share one channel or need separating is an open
  design question, and the cheap answer — start by having liquid *read* the
  existing channel and write nothing — is probably right for a first cut.

---

## 7. Finding 5: hierarchical leveling on the grid you already have

No citation needed for this one, just multigrid's basic principle: diffusive
relaxation converges slowly on a fine grid and fast on a coarse one.

The engine already maintains `FIELD_SCALE = 8` field tiles, resident, stepped
every frame, with an existing settled/converged gate from issue #4. A
coarse-level water-volume equalization pass with fine-level correction would cut
the leveling exponent substantially, using infrastructure built for an unrelated
reason.

**This is listed for completeness and is probably redundant if §5 lands.**
Virtual pipes on a 1D height array is both cheaper and more direct than a
two-level multigrid over a 2D field. If §5's seam turns out to be intractable,
this is the fallback that stays entirely inside the existing CA representation.

---

## 8. Inheritance from Report A

Report A §2 established that granular material has two critical angles — a
steeper one at which motion *starts* and a shallower one at which it *stops* —
and recommended expressing it as one state bit plus two thresholds.

Revision 1 §5 already reaches for the same idea from the fluid side, citing Zhu
& Bridson's Mohr–Coulomb yield check as the frame for giving a liquid cell an
explicit per-step choice between "behaving as a settled mass" and "behaving as a
flowing surface."

**These are the same mechanism and should be one implementation.** A `Liquid`
cell that is already flowing should use a looser threshold than one at rest,
exactly as a grain does, and both should read `FLAG_FLOWING`. Report A's flag is
not powder-specific and should not be named or scoped as if it were.

This also improves rev 1 §5's proposed fix. A same-step horizontal search
evaluated unconditionally makes *all* liquid restless. Gating it on the flowing
state means a freshly poured cell searches sideways immediately (fixing symptom
1) while a settled pool doesn't pay for the search at all (protecting the frame
budget, and letting chunks sleep).

---

## 9. Recommendation and build order

1. **Rev 1 §5's mechanism-ordering fix, gated on `FLAG_FLOWING` per §8.** Fixes
   symptom 1. Small. Already designed; just needs the flag from Report A to land
   first.
2. **VOF local height function (§3b).** Fixes symptom 3 and an unmeasured mass
   error. Three-cell column read, surface cells only.
3. **Instrument mass conservation and surface flatness (§10a–c).** Before
   anything structural. The engine currently cannot tell whether a liquid change
   helped.
   **Add the variable-resident-area stress scene (§10h) in the same pass.** It is
   a small change to `examples/ascii.rs`, it is the only thing that turns §4c's
   arithmetic into a measurement, and it is needed before M10 regardless of what
   happens to liquids.
4. **1D virtual-pipes prototype for large settled bodies (§5).** Fixes symptom 2.
   Prototype the *seam* first — body identification and promotion/demotion — not
   the pipe physics, which is trivial.
5. **DF-style pressure path trace (§6).** Fixes symptom 4. Bounded by the
   "never higher than the first full cell" rule.
6. **Drop `MIN_LIQUID_TRANSFER` back toward 8** and confirm settling time stays
   inside budget. This is the check that step 4 actually worked.
7. **LBM: prototype-gated, not scheduled (§4, §4c).** Only if a use case demands
   real pressure-wave propagation in water, and only through a prototype that
   demonstrates sleeping/coarsening and is measured at streaming-scale resident
   area. Read Thürey & Rüde (2009) before scheduling even the prototype.
8. **Hierarchical multigrid (§7): fallback only**, if step 4's seam proves
   intractable.

---

## 10. Acceptance criteria

The engine has water tests, but they check qualitative properties ("levels out
instead of only eroding at the edges", "settles flatter than a powder would").
None of them would catch a 2% mass gain or a permanently uneven surface.

- **10a. Mass conservation.** Total liquid fill across a dam-break scenario must
  be conserved to within **0.5%** over 2,000 frames. The VOF literature's
  reference point for what a bad implementation looks like: <cite index="87-1">the fluid level in the standard VOF method increased by
  2%</cite> where the height-function version was exact. Currently unmeasured.
- **10b. Surface flatness.** A 100-cell-wide pool, settled, must have a maximum
  adjacent-column height difference under **2% of `LIQUID_FULL`**. The current
  `MIN_LIQUID_TRANSFER` dead band permits 15%, so this test fails today by
  construction — which is the point.
- **10c. Leveling time.** The same 100-cell pool must reach 10b's flatness bar
  within **300 frames**. Present behaviour is "hundreds to thousands" per the
  harness, and the constant was tuned to land under 1,000 by widening the dead
  band rather than by converging faster.
- **10d. No detached droplets.** After a pool drains through a 3-cell opening,
  the count of isolated single-cell liquid remnants on the floor must be **zero**
  once the body has settled. This is the flotsam-and-jetsam test.
- **10e. Communicating vessels.** A U-tube with arms of unequal height, filled
  from one arm, must equalise to within **one cell** on both sides. Fails today
  outright — this is the acceptance test for §6, and DF's own documented
  one-level cheat is the reason the bar is one cell rather than exact.
- **10f. Initial shape.** A column of water poured onto a flat floor must not
  hold a slope steeper than **10°** at any point after 60 frames. This is
  symptom 1 stated as a number; the current behaviour reproduces a sand-like
  repose slope of roughly 30–40°.
- **10g. Cost ceiling.** The full-screen sand-and-water stress scene's worst
  frame must not regress more than **15%** against the ~28 ms serial / ~9 ms
  parallel on record.
- **10h. Scaling.** Every performance figure on record for this engine — the
  ~23 ms CA-only, the ~28/~9 ms combined, §11's rendering numbers — is measured
  on a fixed 512×320 world with 40 chunks, and none of them predict anything
  about a streamed one. `examples/ascii.rs` needs a **variable-resident-area
  stress scene** reporting worst frame against chunk count, so that "how does
  this scale" is answered by measurement rather than by arithmetic like §4a's.
  Bar: any liquid mechanism adopted here must show worst-frame cost growing
  **sub-linearly** in resident chunk count on a scene where only a bounded region
  is actually active. This is the test that distinguishes a mechanism that sleeps
  from one that merely happens to be cheap at the current world size — and it
  should be built before M10, not during it.

---

## 11. Deletion tests

| Mechanism | Test that must fail without it |
|---|---|
| `FLAG_FLOWING` gating on liquids | 10f — the initial pour slope returns to powder-like |
| VOF local height function | 10d — detached droplets reappear after drainage |
| Virtual-pipes leveling | 10c — settling time returns to thousands of frames |
| Pressure path trace | 10e — the short arm of the U-tube stays empty |
| Bilinear sampling on any liquid field read | A pressure gradient inside one 8-cell field block must produce differential flow; block-nearest gives none |

Every row fails today, because none of these exist.

---

## 12. What was not directly accessible

- **Hirt & Nichols (1981), J. Comput. Phys. 39:201–225** — the VOF primary.
  Paywalled at Elsevier. Read through four independent secondary sources that
  quote the donor–acceptor scheme, the F formulation and the flotsam/jetsam
  drawback consistently, plus Hirt's own retrospective account of the method's
  history. The claims attributed here are well corroborated; the original text
  was not read.
- **Kleefsman et al. (2005), the local-height-function VOF variant** — read from
  a university repository preprint including the comparison figures and the 2%
  mass-gain measurement. This is the closest thing to a primary in the report
  and the numbers in §10a come from it directly.
- **Thürey, Rüde & Körner, *Interactive Free Surface Fluids with the LBM*** —
  read the FAU technical report directly, including the free-surface
  reconstruction and interface-cell handling. The "interactive rates on 2005
  CPUs" claim is from the paper's own framing rather than a benchmark table this
  report verified.
- **Mei, Decaudin & Hu (2007)** — read the INRIA-hosted PDF's model description
  directly. The CPU-viability claim in §5 is from a practitioner blog post
  reporting a working CPU implementation, not from the paper, which is
  GPU-framed throughout.
- **Dwarf Fortress's actual implementation** — sourced from the DF wiki, which
  is community documentation of observed behaviour, not source. DF is
  closed-source. The mechanism described (path trace, teleport through full
  cells, never above the first full cell's level, the deliberate one-level
  shortfall attributed to Toady) is consistent across multiple wiki versions,
  but it is behavioural reverse-engineering and should be treated as a design
  pattern to adapt rather than a specification to match.
- **The LBM bandwidth estimate in §4a is this report's own arithmetic**, not a
  measured figure for this engine or anything like it. The inputs (D2Q9 f32
  footprint, bandwidth-bound behaviour, the ~2/3-of-peak figure on a desktop
  part, the in-place and FP16 reductions) are cited; the multiplication is not.
  It is an order-of-magnitude sanity check that says "this is worth
  prototyping," and should not be quoted as a performance prediction.
- **§4c's 10× resident-set multiplier is an assumption, not a measurement.** No
  streaming resident-set size has been decided for this engine — `PLAN.md`'s M10
  is one line and `worldgen-design.md` does not fix a number. ~400 chunks is a
  plausible guess from "one screen of margin in each direction"; the real figure
  could be half or triple that, and every number in §4c's table scales linearly
  with it. **The conclusion is insensitive to the exact multiplier** — any factor
  above ~3 puts a whole-grid LBM step into the same order as the entire frame
  budget — but the specific milliseconds should not be quoted. This is precisely
  what §10h exists to replace with measurement.
- **Thürey & Rüde (2009), adaptively coarsened free-surface LBM** — identified
  via citation lists and an abstract summary only; the paper itself was not read.
  It is named in §4c as the direct precedent for the sleeping/coarsening question,
  and **it should be read in full before any LBM prototype is scheduled**, since
  it is the single source most likely to determine whether the gate can be
  cleared at all.
- **The LBM grid-refinement conservation literature** (volumetric formulation,
  constrained space–time interpolation, refluxing) was read at abstract and
  summary level across several sources that agree on the shape of the problem.
  The claim in §4c is only that coarse–fine mass conservation is a real,
  non-trivial sub-problem with dedicated published machinery — which is well
  supported at that level. Which specific technique would suit a free-surface,
  chunk-sleeping design was **not** determined and would need its own pass.

---

## 13. Handoff

**To Report C (solid–granular–fluid coupling):** §4b's composability objection is
C's problem restated. A rigid body in water needs buoyancy and drag *from* the
fluid, which requires the fluid to have a pressure field and a velocity field.
The engine has both, coarsely, and liquids currently populate neither. If C
concludes that rigid–fluid coupling needs a real fluid velocity field, that
materially changes §4's cost-benefit — LBM supplies both fields for free, which
would be the strongest argument for it, and stronger than anything in this
report.

**To Report D (worldgen, if promoted):** §5's virtual-pipes model is the
erosion mechanism. Do not choose a second water representation for worldgen.
