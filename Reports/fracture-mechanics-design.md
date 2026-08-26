# Fracture: load, capacity and cracks

**Status:** design, not built. Written after playtesting rejected three
successive support models, and after the owner identified the defect the
third one still had.

**Implementation handoff: `Reports/load-model-handoff.md` — SUPERSEDED BY
LANDING, do not execute it.** The step it hands off shipped (`7e13e42`); it is
kept as the rationale written before the work, and parts of it are now on the
do-not-retry register (its §3 asks for the support-parent side table on
`World` that `load.rs`'s module doc records as rejected). **Read
`Reports/load-model-fit-review.md` instead** — the review of what actually
landed. What remains useful here is the four support models already tried and
rejected, the repo gotchas, and the acceptance cases; read on to understand
*why*.

**Read `Reports/design-philosophy.md` §0a first.** Satisfying is the
requirement here, and this subsystem is the worked example of failing it.

---

## 0. The defect, stated precisely

Everything built so far measures **reach**: a cell's distance to its anchor,
failing past a span. That answers *"how far out are you"*. It cannot answer
*"how much is hanging off you"*, and those come apart exactly where it
matters.

The owner's case: an overhang joined to the cliff by a two-cell ligament.
The ligament sits at *low* distance — it is right next to solid rock — so
the reach model says it is fine, while the tip, being far out, fails. That
is backwards. Rock does not fail where it is furthest from support; it fails
where the stress is highest, which is at the neck.

The "when anything fails, take the whole appendage" flood in
`rigid::label_failing_region` exists *because* the root never fails on its
own. It produces roughly the right outcome by the wrong mechanism, and it
degrades precisely where the owner is pointing: a partially cracked overhang
has no single distance-defined appendage to take.

**So the model needs load. Not as an addition — as the thing that replaces
span as the failure criterion.**

---

## 1. The model

Two quantities per structural cell:

- **Load** — what it carries.
- **Capacity** — what it can carry.

Fail when `load > capacity`. That is net-section failure, and it is the
whole rule.

### 1a. Load is a moment, not a weight

The first mistake to avoid, and it is not obvious: accumulating *mass* is
not enough. A cantilever's root does not care only how much hangs off it,
but how far out it hangs. Fifty cells stacked against the wall is fine;
the same fifty cells reaching fifty cells out is not. The quantity is
bending moment, `M = Σ mᵢ · dᵢ`, with `dᵢ` the horizontal lever arm.

That looks like it needs a per-cell scan of everything it supports. It does
not, because the sum separates:

```
torque about c  =  Σ mᵢ (xᵢ − x_c)  =  (Σ mᵢ xᵢ) − x_c · (Σ mᵢ)
```

So each cell accumulates **two additive scalars** — total supported mass
`M`, and mass-weighted position `Sx = Σ mᵢ xᵢ` — and its torque falls out
arithmetically. Both are plain sums up a tree, which is as cheap as this
gets.

This distinguishes the cases that matter, with no special-casing:

| Case | `Sx − x·M` | Result |
|---|---|---|
| Vertical tower | ≈ 0 — mass sits directly above | Stands at any height |
| Cantilever | grows with length | Root fails, whole span drops |
| Big overhang on a thin ligament | large | **Neck snaps** — the owner's case |
| Bulk attached rock | large, but capacity larger still | Nothing happens |

### 1b. Where the tree comes from — free

The distance relaxation already computes, for every cell, the neighbour it
took its minimum from. That parent pointer makes the support graph a
**forest rooted at anchors**, and `M`/`Sx` accumulate up it: a cell's totals
are its own mass plus the totals of every child. Processing in decreasing
distance order visits children before parents.

No new graph, no new traversal shape. The distance field stops being a
failure criterion and becomes purely a support-ordering potential.

### 1c. Capacity, and what cracks are for

`capacity ∝ material strength × uncracked cross-section × attachment`.

This is the point at which cracks stop being decorative. A cell with three
of its four edges cracked is hanging by one and should carry almost nothing.
Each new crack cuts capacity, so a piece **sags, strains, then goes** — the
graded outcome §0a demands, arriving as a consequence of the model rather
than as a tuned curve.

It also gives cracks somewhere principled to *propagate*. Real cracks extend
from their tips, where stress concentrates — and `load / capacity` **is**
stress. A crack should grow toward wherever that ratio is highest, so
fissures run along the line the rock is actually straining rather than
wandering randomly.

---

## 2. What this lets us delete

Worth stating up front, because it is the argument that this is a
simplification rather than another layer.

- **`support_cost_below: 0` goes.** Free compression existed only to stop
  towers snapping under the reach model. Under a load model a tower stands
  because it *carries little*, which is the real reason. Deleting it also
  removes two bugs it caused: the self-consistent-zero fixed point (a
  floating blob where every cell claims support from the one below and the
  whole thing is stable at distance 0), and the tie degeneracy in §3.1.
- **`max_unsupported_span` stops being the failure criterion.** Distance may
  then grow without bound harmlessly.
- **The "whole appendage" flood goes.** Once the root genuinely fails, the
  detached region is a *consequence* — ordinary connectivity — not a
  mechanism. **This must be removed in the same change**, or both fire and
  a single failure detaches twice.
- **`attached_span_bonus` becomes a capacity multiplier**, which is what it
  was always reaching for.

---

## 3. Pitfalls

Numbered because each is a specific way this goes wrong, several of which
have already bitten this subsystem in another form.

**3.1 — Zero-cost ties break the accumulation.** With any zero-cost step,
whole regions share a distance, "decreasing distance order" no longer orders
parent before child, and the parent relation can *cycle*. The load walk then
never terminates. Fixed by §2's deletion of the zero cost; do not
reintroduce a free step anywhere without re-deriving this.

**3.2 — The parent forest goes stale mid-convergence.** Distance is a
label-correcting relaxation that converges over several ticks. A load walk
over a half-converged forest can follow a parent pointer that is no longer
valid. Guard: walk only while distance *strictly decreases*, and cap the
walk length. Both are cheap; neither is optional.

**3.3 — Load is non-local, unlike everything else here.** Changing one cell
at a beam's tip changes the load of every cell between it and the anchor.
Walking up the parent chain is `O(depth)` and fine. *Removing* a cell is
worse — it can re-parent an entire subtree. Do not maintain load
continuously; recompute lazily, on the path from a disturbed cell to its
anchor, when a structural check fires.

**3.4 — Cascades want to happen all at once.** Each break changes loads,
which triggers more breaks, in the same frame. That is both a frame spike
and worse-looking than a progressive collapse. Needs the existing
`MAX_SITES_PER_FRAME` pacing *plus* a per-frame cap on fractures. Pacing
here is a feature, not a concession — a collapse that resolves over a second
reads better than one that resolves instantly.

**3.5 — Determinism.** Parent choice must tie-break deterministically
(`NEIGHBOURS_4` order already does). If load lives in a side table (§4.1),
it must not be a `HashMap` iterated for the walk — that is issue #7's trap,
which this codebase has already hit twice.

**3.6 — Cracks and load must land together.** Capacity depends on cracks, so
shipping load first makes cracks look decorative, and shipping crack seeding
first makes them *actually* decorative. Nothing currently seeds a crack at
all.

**3.7 — Vestigial mechanisms pass vacuous tests.** When `confinement_radius`
was superseded, its tests kept passing while testing nothing, because an
undisturbed slab sits at a self-consistent distance and stops rescheduling.
`max_unsupported_span` is about to be superseded the same way. Delete it, or
reinterpret it explicitly — do not leave both half-live.

**3.8 — Migration.** `max_unsupported_span` is shipped `.ron` data,
referenced by tests, tunables and docs. Decide its fate deliberately in the
same change.

---

## 4. Performance

### 4.1 Keep it proportional to surface, not volume

The vast majority of cells are attached bulk that cannot fail. Give them a
two-test early-out **before any computation**: *attached, no cracked edge,
no empty neighbour → cannot fail, skip.* That covers nearly the whole world.

Consequently load/capacity should live in a **sparse side table for
structurally interesting cells only** (unattached, or adjacent to a crack or
free face), not as new `Cell` fields. `Cell` has already grown once (32 MB →
48 MB) and two more scalars per cell is a real cost at M10 streaming scale
for data that is meaningless for most of them. Cost then scales with
*surface area*, which is the same argument that makes the distance pass
affordable.

### 4.2 Cache capacity rather than recomputing

Capacity changes only when a crack is added or a neighbour is removed — both
of which already schedule a structural check. So it can be computed once at
that moment rather than on every evaluation.

### 4.3 Existing costs worth fixing while here

- **The gen-time pass scans the whole world through hashed `World::get`** —
  measured 9.1 ms, dominated by the scan rather than the relaxation. This is
  issue #5's pattern (`~164k hashed lookups; index the chunk directly`).
  Iterating chunks directly would cut most of it, and it becomes per-chunk
  under M10 anyway.
- **Chunk bodies currently force a full-screen redraw.** `render.rs` treats
  any live body the way it treats particles, so a fall with several
  fragments defeats the dirty-rect skip for its whole duration. This is a
  regression I introduced and it gets worse as fracture produces *more*
  fragments. Should dirty only each body's bounding box.
- **`is_confined`'s removal left `MAX_THICKNESS`-style ring scans nowhere**,
  but the strike path still scans a full disc per blow. Bounded by brush
  radius, so acceptable, but worth an annulus where only the shell matters.

### 4.4 What to measure, and against what

`examples/ascii.rs` reports worst-frame timings and CI runs it; that is the
number to quote. Two specific scenes are needed and do not exist:

- **A large collapse**, for §3.4's cascade spike.
- **A settled world containing landed debris**, for §4.3's redraw
  regression — a settled world is exactly where the dirty-rect skip earns
  its keep, and per `CLAUDE.md` a cost must be measured against the state
  the optimisation exists for.

Note that the machine has been too contended to trust timings for several
sessions; take a baseline on a quiet machine *before* starting.

---

## 5. Build order

1. **Crack seeding** — the strike scores rock instead of pulverizing it.
   Visible immediately, changes no structural behaviour, and makes every
   later step observable.
2. **Capacity from cracks**, still with the reach criterion. Cracks begin to
   weaken rock without yet changing what failure means.
3. **Load/torque accumulation** and the switch to `load > capacity`,
   deleting §2's list in the same change.
4. **Crack propagation** along the stress ratio, once there is a stress
   ratio to follow.

Step 1 is independently shippable and judgeable by eye.

**Step 2 is not, and that was an error in this plan — recorded rather than
quietly corrected.** Capacity was expected to be visible on its own: score a
shelf near its root, watch it give way. Built and measured, it does almost
nothing for that case, and the reason is this document's own §0 argument
turned back on it. Failure is evaluated per cell as *its own reach* against
*its own span*. A crack at the root weakens a cell whose distance is ~10,
which was never going to fail at any span; the far end that *is* near its
limit is not the part that got hit. Six blows at the join of a 160-cell
shelf left it standing (`filmstrip scene=worked`).

Capacity only bites where a cell is both heavily cracked *and* already near
its reach limit — real, but a narrow case, and not the one anybody means by
"work a crack until the piece drops."

**So capacity is not a step, it is half of step 3.** `load > capacity` needs
both sides; shipping the capacity side alone buys a small effect and no
demonstration. The corrected order is:

1. **Crack seeding** — done.
2. **Load/torque accumulation, switching to `load > capacity`**, with
   capacity from cracks as its other half, deleting §2's list in the same
   change. This is the step that makes a worked root give way.
3. **Crack propagation** along the stress ratio, once there is one.

Step 2 is now the largest single change in this plan and still must not be
rushed: it changes the failure criterion for every material at once. The
capacity arithmetic itself is already in `structural::weakened_by_cracks`
and can be reused as-is.
