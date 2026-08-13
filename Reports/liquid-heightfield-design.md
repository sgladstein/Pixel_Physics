# Liquid heightfield bodies: the ownership substrate, the promotion/demotion protocol, and the seam

**Audience:** the coding agent implementing this.
**Status:** design only, written just-in-time before implementation, per
`design-philosophy.md` §3's standing instruction and the precedent
`organism-substrate-design.md`, `tree-rewrite-design.md` and
`plant-substrate-v2-design.md` all set. No code in this pass.
**Direct brief:** `Reports/liquid-simulation-research-r2.md` §5 and §5a
(Report B). §5a names the two questions it could not answer from the
literature — how a body is identified and tracked cheaply, and what the
seam looks like when a body is only *partly* settled — and instructs that
the seam is what gets prototyped first, "because it is cheap to try and
its risk is concentrated entirely in the seam rather than in the physics."
This document answers both.
**Companion to:** `Reports/coupling-research.md` (Report C), whose §5
(buoyancy by column displacement) and §8 (rigid-body sleeping/demotion)
both depend on the 1D column array landing, and whose §13 instructs:
*"§8's sleeping/demotion pattern is the same design as B's settled-body
promotion. Build one promotion/demotion mechanism, not two."* §9 below is
that one mechanism.
**Does not contradict:** `Reports/granular-mechanics-research.md` §5's
claim on `Powder`'s `aux` slot for a packing scalar. This design touches
no `Powder` state and adds no `Cell` bytes — see §2d.

---

## 0. The decisions, stated first

| # | Question | Decision |
|---|---|---|
| 1 | Who is authoritative — grid or heightfield? | **The grid never lies.** A promoted body owns *when* its cells change; it never owns *what* they are. Every other subsystem (render, fire, `Absorb`, structural, the inspector, every existing test) keeps reading the grid unchanged. §2 |
| 2 | Height representation | **Integer fill units on the `LIQUID_FULL` scale**, not `f32`. Conservation becomes exact by construction rather than exact-to-a-tolerance, which is what the existing tests already assert every frame. §2b |
| 3 | Why this fixes ballooning at all | A promoted column is *by construction* `floor(h)` full cells plus one partial top cell. "Same mass, five times the cells" is **not a representable state**. The failure mode is removed by the representation, not guarded against by a rule. §2c, §10 |
| 4 | Is `rigid::label_component` reusable? | **Its shape yes, its predicate no.** Bounded 4-connected flood fill with a `max_cells` cap is exactly right; `is_body_material` (`Solid`, not `BEDROCK`) is wrong for liquid, and liquid needs a *validating* fill that also computes per-column extents. A second function in `rigid.rs`'s style, not a generalization of it. §3a |
| 5 | Incremental maintenance of a body | **There is none, deliberately.** A body is built by one bounded flood fill and destroyed wholesale. No merge/split/incremental-relabel machinery exists to get wrong. §3d |
| 6 | Promotion gate | **Structural, not dynamic.** Single vertical span per column, a free surface above every column, one material, ≥ `MIN_BODY_COLUMNS` wide. **Quiescence is explicitly *not* a promotion gate** — this corrects Report B §5's "large, settled, connected bodies" framing. §4a |
| 7 | Demotion trigger | Any write into a managed cell *or its managed container* by anything other than the body's own rasterizer, detected at the `World::set` / `CellSurface::set` seam — the one funnel every write in the engine already goes through. Whole-body demotion in v1. §5 |
| 8 | The partial-settlement seam | **Column-granular, never cell-granular.** Inflow is *absorption* (a falling CA cell is deleted and its fill added to the column it lands on); outflow is *edge demotion* (the body gives a column back to the CA rather than inventing free-surface dynamics). No column is ever both. §6 |
| 9 | The solver | Mei–Decaudin–Hu virtual pipes reduced to 1D and to integers, with **persistent signed per-interface flux** — the piece without which you have merely rebuilt O(width²) diffusion at a coarser granularity — plus a **terminal exact-equilibrium snap** so the body settles genuinely flat and then sleeps. §7 |
| 10 | Parallel sweep interaction | **A separate serial body phase**, after `parallel::step`, before `step_active_sites`. Same conclusion Report C §4 reached independently for rigid bodies, for the identical reason, and one phase hosts both. Absorption from inside the sweep is *queued*, exactly like `remote_writes` already is. §8 |
| 11 | Rasterize or render directly? | **Rasterize back to cells**, lazily — only when a column's whole-cell count changes. Rendering directly is rejected: it would make the grid lie, and the grid is what every other subsystem trusts. §7e |
| 12 | Shared with Report C's rigid bodies | One **ownership substrate** (`FLAG_MANAGED`, `World::bodies` generational slots), one **protocol** (promote → claim → step → disturb → demote), one **frame phase**. Not one physics and not one eligibility predicate — forcing those would be dishonest. §9 |

---

## 1. What this is answering, and what has already been ruled out

### 1a. Three empirical facts this design inherits and does not re-derive

These come from the investigation session that produced this task, plus
`PLAN.md`'s own record of it and the doc comments the session left in
`update.rs` and `parallel.rs`. Every one was measured, not reasoned.

1. **The stall is not a chunk/parallel/sleep bug.** A control run forcing
   `World::wake_all()` before every `parallel::step` — bypassing dirty
   rectangles and chunk sleeping entirely — reproduced the *identical*
   stall pattern. The fault is in `update_liquid`'s own transfer
   priority, specifically the interaction of `HORIZONTAL_TRANSFER_REACH`
   (8), `flow_rate` (1000 for water, i.e. uncapped) and
   `MIN_LIQUID_TRANSFER` (150, a 15%-of-full dead band). **The fix has to
   be architectural.**

2. **Two per-cell reordering fixes were tried and both reverted.** The
   unconditional horizontal-before-vertical swap fixed the stall and
   caused a landing column to balloon to ~5× its own cell count with mass
   *exactly* conserved every frame. Scoping the reorder to "the cell
   above me is not more of the same liquid at full fill" — a literal
   local free-surface test — did **not** fix the ballooning (~4.8× peak).

3. **The conclusion, which is why this document exists.** A per-cell
   local heuristic cannot distinguish *the free surface of one connected
   body* from *any cell that happens to have room nearby*, once that
   body's shape is irregular — and it is irregular the moment anything
   real happens to it. Once diagonal cascading off a column's edges
   roughens the top profile, many interior cells legitimately read as
   "at the surface" under a purely local check and collectively
   over-dilute.

### 1b. Why the heightfield answers exactly that, in one sentence

Being at a body's free surface is a **whole-body property**, and a
column's height is, by construction, a whole-body-of-that-column
property: there is exactly one free surface per column and the
representation knows where it is without asking any cell. A per-cell rule
cannot see the body; the column array *is* the body.

### 1c. "Better than before" now means beating two baselines, not one

The reverted reorder (`eeefceb`, reverted by `dcb761c`) was not merely a
narrow stall fix. The owner's report is that water behaved **noticeably
better overall** with it — faster, more responsive leveling generally.
So the bar for this design is not "the original vertical-first behaviour
minus the stall." It is:

- **Match or beat `eeefceb`'s leveling responsiveness** (its own recorded
  measurement: full flatness by frame 900 on the three-tall-columns
  repro, against a residual 3-cell step still present at frame 1800
  without it), **and**
- **never exhibit the incompressibility-violating cell-count
  ballooning**, and
- do it **without `eeefceb`'s cost**, which its own doc comment records
  as a measured ~12% worst-frame serial regression (37.9 ms → 42.6 ms on
  `examples/ascii.rs`'s sand-and-water stress scene) — most of Report B
  §10g's 15% ceiling spent on one reorder.

§12 states all three as numbers. See §15 for exactly which of those
figures this document verified directly and which it did not.

---

## 2. The representational commitment

### 2a. The grid never lies

A promoted body's cells stay in the grid, correct and readable, at all
times. What promotion changes is **who is allowed to move them**: the CA
sweep skips a managed cell, and the body's own solver decides when its
cells change and writes them through the ordinary `World::set`.

This is not a stylistic preference; three concrete things depend on it.

- **Rendering.** `render.rs` draws from `Chunk::cells()` and is
  incrementally driven by `World::take_touched_chunks`. A second render
  path for heightfield bodies would need its own dirty tracking and would
  break that contract at exactly the seam where a body meets ordinary
  water.
- **Every other subsystem reads cells.** `fire::update` (evaporation,
  oil), `plant.rs`'s `Absorb` draining adjacent `Liquid` cells,
  `field.rs`'s `apply_moisture_sources`, `creature.rs`, `structural.rs`,
  the hover inspector, `explosion.rs`. None of them know about bodies and
  none of them should have to.
- **Demotion becomes free and exactly conservative.** If the cells are
  already correct, demotion is "clear a flag" — no mass moves, so no mass
  can be lost. §10.

**The cost, named:** the body must rasterize back to cells, and doing
that every frame for every column would be O(area) and would keep every
chunk awake forever. §7e is the lazy-rasterization rule that makes it
proportional to visible change instead.

### 2b. Integer heights, not floats

Column height `h[i]` is a `u32` **in fill units on the `material::
LIQUID_FULL` = 1000 scale** — the same scale a `Liquid` cell's `aux`
already uses, so promotion and demotion are unit-free reinterpretations
rather than conversions. Flux is a signed `i32` in the same units.

Three reasons, in order of weight:

1. **Conservation becomes exact rather than tolerated.** Every transfer
   debits one column and credits another by the *same integer*, so
   `Σ h[i]` is invariant unconditionally. Report B §10a asks for 0.5%
   over 2,000 frames; the existing tests
   (`a_landing_column_does_not_balloon_in_cell_count`,
   `a_splash_settles_with_no_stray_droplets_and_no_mass_drift`) already
   assert **exact** equality every single frame and would fail against an
   `f32` accumulator. Matching what the suite already demands is the bar,
   not the report's looser one.
2. **Determinism.** `PLAN.md` reversed determinism to *required*.
   Integer state removes the whole question for this subsystem; Report C
   §7's rapier anxiety has no analogue here.
3. **It matches the grid it rasterizes to.** A column's cell contents are
   integer fill values; a float height would need a rounding policy at
   the boundary and would drift against the sum of the cells it produced.

### 2c. Why this makes the ballooning failure unrepresentable

A column of height `h` rasterizes to exactly `h / LIQUID_FULL` full cells
plus, if `h % LIQUID_FULL != 0`, one partial cell at the top. That is the
*only* state a managed column can be in.

The reverted reorder's failure was every cell in a body diluting to a
fraction of full — 23,400 cells becoming 102,915 at the same total mass.
Under this representation that configuration cannot be written down. A
managed body's cell count is pinned to `Σ ceil(h[i] / LIQUID_FULL)`,
which is within `columns` of `total / LIQUID_FULL`. §12's B-1 asserts
exactly that as an invariant, so a future regression is caught as a
broken invariant rather than as a screenshot.

**This is the strongest single argument for the design and it should be
the first thing verified in the prototype** (§11 step 1), before any
solver exists.

### 2d. What this design does *not* claim

- **No new `Cell` bytes.** One previously-unused bit of `Cell::flags`
  (three of eight are used today: `FLAG_MOVED`, `FLAG_BURNING`,
  `FLAG_FLOWING`).
- **No `Powder` `aux` bits.** `granular-mechanics-research.md` §5's
  packing scalar keeps that slot.
- **No `Cell::organism_id` reuse in v1.** See §9b — the rename to a
  generic `owner_id` is the right eventual move but it is M8's to make,
  not this design's, and doing it now would introduce a live bug at
  `update.rs:132` (`if cell.organism_id() != 0 { organism::
  diffuse_resource(...) }` would start firing on water).
- **No `Liquid` `aux` semantic change.** A managed cell's `aux` still
  means fill, and `liquid_fill`'s `aux == 0` ⇒ full sentinel
  (Report B §3d) is untouched.

---

## 3. Bodies: identification, ownership, tracking

### 3a. Is `rigid::label_component` reusable? Precisely: the shape, not the function

Report B §5a calls it "suggestive... currently wired to nothing." Checked
against the code rather than the description:

**What genuinely transfers.** Bounded 4-connected flood fill over the
grid with an explicit `max_cells` cap, returning cell positions, with the
cap justified exactly the way this case needs it ("a caller that actually
wants *the chunk the player just cut loose* passes a cap sized to a
plausible body, not `usize::MAX`"). The 4-connectivity choice is also
right here for the same reason it is right there — two puddles touching
only at a corner are not one body of water, and treating them as one
would put a single level on two things that are not hydraulically
connected.

**What does not transfer.** `is_body_material` is `Solid && != BEDROCK`.
Liquid needs same-`MaterialId` connectivity (water and oil must never
share a body — they have different densities, different `flow_rate`s and
they layer), and it needs the fill values, not just positions.

**And a liquid fill must do more work than a labeller.** Promotion needs,
per column, `min_y`, `max_y`, cell count, and total fill — which the fill
can accumulate as it goes, for free, but which `label_component`'s
`Vec<(i32, i32)>` return type throws away.

**Decision: a new `liquid::label_body` in `rigid.rs`'s style, not a
generalization of `label_component`.** Same file is fine (it is where
component labelling lives), same cap discipline, same tests-first
character. Trying to make one function serve both would mean generics
over a predicate plus an accumulator, which is more machinery than two
30-line functions.

It returns, or refuses:

```
struct BodyScan {
    material: MaterialId,
    x0: i32,
    // per column, indexed from x0:
    top_y:  Vec<i32>,     // topmost liquid cell
    bed_y:  Vec<i32>,     // first non-body cell below the bottom liquid cell
    fill:   Vec<u32>,     // Σ liquid_fill over the column's cells
}
```

with the validation of §3b applied *during* the fill so a disqualifying
column aborts early instead of after paying for the whole component.

### 3b. The eligibility predicate

A scanned component is promotable iff **all** of:

1. **One material.** Enforced by the fill's own connectivity predicate.
2. **Single vertical span per column.** For every column,
   `max_y - min_y + 1 == cell_count`. This is the direct answer to
   Report B §5a's "a column of x can contain several disconnected bodies
   of water at different levels": a component that needs two spans in one
   column (water on a cave floor and water on a ledge above it, connected
   around) is **refused**, and stays CA. One check, computed during the
   fill, no extra pass.
3. **A free surface above every column.** The cell directly above
   `top_y[i]` must be `Empty` or `Gas`-kind. This refuses sealed and
   ceiling-capped bodies, which are not a leveling problem at all — they
   are a *hydrostatic pressure* problem, and that is Report B §6's
   subject (the DF-style path trace), explicitly out of scope here (§14).
   Getting this wrong is how you build something that silently claims to
   solve U-tubes and does not.
4. **Width ≥ `MIN_BODY_COLUMNS`.** Below it, CA diffusion levels fast
   enough that promotion is pure overhead. The engine has a real
   empirical anchor for where that line sits, in
   `a_wide_deep_water_column_levels_out_instead_of_only_eroding_at_the_
   edges`'s own comment: at 40 columns wide, even
   `HORIZONTAL_TRANSFER_REACH` reduced to 1 levels within budget; at 100
   it does not. **Start at 32 and tune against §12's leveling bar.
   Flagged as untuned.** Per `design-philosophy.md` §2a this is
   gameplay-facing and belongs in data, not a Rust `const` — see §7f.
5. **Cell count ≤ `MAX_BODY_CELLS`.** The flood fill's cap. A refusal by
   cap is not an error; it is "not this frame."

Note what is *not* in this list: quiescence. §4a.

### 3c. Ownership: one flag bit, and a chunk→body index

**Per cell: `FLAG_MANAGED` on `Cell::flags`.** Meaning: *this cell
belongs to a promoted body; the CA sweep must not move it, and any write
into it that does not come from its body's own rasterizer demotes that
body.* One bit, one test, in the two hot paths that need it
(`update_cell`'s dispatch, and the write seam).

**Which body: not on the cell.** `World` holds
`body_index: HashMap<ChunkCoord, SmallVec<BodyId>>` — bodies are few
(tens, not thousands), a chunk overlaps at most a handful, and resolving
`(x, y) → BodyId` is one hash lookup plus a column-range test. That
lookup happens only on **disturbance** (rare) and on **absorption**
(bounded by how many cells are actually falling into a body), never in
the per-cell sweep.

**Why not a per-cell body id, given Report C wants one?** Because a
liquid body's cell has no body-local coordinate to remember — its
position *is* its column index, recoverable from `x` alone. A rigid
body's does, because it rotates (Report C §2 quotes Noita's "each pixel
knows... its location inside that rigid body"). That is a genuine
asymmetry, and it means v1 can stay out of `Cell` entirely. §9b names
exactly when to revisit.

**Managed container cells.** At promotion, the cells immediately below
and immediately beside the body's columns — its bed and its walls — are
*also* flagged `FLAG_MANAGED`, with no body id of their own. They are
never moved by the body and never counted in its mass; the flag means
only "a liquid body depends on you." This is what makes "someone dug the
floor out from under the lake" a demotion for free, through the exact
same single-bit test as any other disturbance, with no second mechanism.

### 3d. Incremental maintenance: there is none, and that is the answer

Report B §5a asks for "incremental maintenance rather than a full
re-label." **This design declines to build it**, and the reason is not
laziness:

- **Demotion is free** (§2a: the cells are already correct). So the
  fallback for *any* structural change is "demote, and let the next
  promotion attempt rebuild" — which costs one bounded flood fill,
  amortised over the several frames the disturbance takes to settle
  anyway.
- **The expensive disturbances are exactly the ones that shouldn't
  demote**, and they are handled by the seam instead of by relabelling:
  a waterfall feeding a lake is *absorption* (§6b), not a topology
  change. A body being fed continuously never re-labels at all.
- **Merge and split are the two operations most likely to have a
  mass-conservation bug**, and this design has neither. A merge is two
  demotions plus one promotion; a split is one demotion plus two
  promotions. Both are exactly conservative because demotion moves no
  mass.

The one operation worth having beyond that, because it *does* recur and
would otherwise thrash: **`try_extend`**, checked only at a body's two
edge columns and only every `EXTEND_INTERVAL` frames. If the column just
outside an edge holds unmanaged same-material liquid that satisfies §3b's
per-column tests, it is claimed into the body (its fill added to a new
`h` entry, its cells flagged). Two columns tested, no flood fill. This is
what lets a pool that has spilled sideways re-absorb its own spill
without a demote/promote cycle.

### 3e. The promotion trigger and its budget

**One promotion attempt per frame, maximum**, drawn from a candidate
queue. A bounded flood fill once per frame is microseconds against a
16.6 ms budget, and rate-limiting it means promotion can never be the
thing that blows a frame.

The queue is fed from three places, in priority order:

1. **The settle-transition scan, for free.** `World::end_step` already
   calls `Chunk::recompute_reach` on exactly the chunks that transitioned
   active→settled this step, and that is already a full scan of the
   chunk's cells (`world.rs:839`). It costs nothing to have that same
   scan also report "this chunk contains liquid, first at (x, y)". This
   is a genuinely free hook at exactly the right moment.
2. **Awake chunks containing liquid, round-robin.** Needed because a
   *pouring* body should be promoted long before it settles — waiting for
   the settle transition would only clean up after the CA had already
   stalled, fixing the residual unevenness but not the speed, which is
   the actual complaint. Requires a per-chunk `liquid_present` bit, set
   whenever a `Liquid`-kind cell is written. Both write paths already
   have the material in hand for other reasons (`World::set` calls
   `materials.get(cell.material).sweep_reach()`; `ChunkView::set` does
   the same), so the kind test is free there.
3. **Explicit events:** painting a large liquid region, a demotion's
   cooldown expiring.

> **Determinism hazard, named because it is issue #7's exact bug shape in
> a third location — and Report C §7a predicted precisely this for rigid
> bodies.** `World::end_step` iterates `self.chunks.values_mut()`, a
> `HashMap`. Collecting promotion candidates in that order makes *which
> body gets promoted first* depend on Rust's per-process-randomized
> hasher, and body ids are allocated in promotion order. **The candidate
> queue must be a `BinaryHeap`/sorted `Vec` keyed on `(ChunkCoord, x, y)`,
> never a `HashMap` drain** — the same fix `chunks_to_sweep()` and
> `ActiveSite`'s `Ord` impl already apply for the same reason.

---

## 4. Promotion criteria

### 4a. Quiescence is the wrong gate — correcting Report B §5

Report B §5a scopes the heightfield to *"large, **settled**, connected
bodies with a clear free surface"* and lists promotion as happening "on
connectivity **and quiescence**."

**Quiescence should not be a promotion gate, and requiring it would
undercut the whole point.** Three reasons:

1. **The symptom is about a body that is still moving.** The reported
   failure is a poured column taking forever to level. If promotion waits
   for the body to stop changing, the heightfield never gets to
   accelerate the part that is slow. It would only tidy up the residual
   step after the fact.
2. **The seam makes quiescence unnecessary.** §6's absorption rule means
   a body that is still being fed is fine: the inflow is CA, the bulk is
   heightfield, and the boundary is well-defined. A lake with a waterfall
   is not an edge case to be excluded, it is the design's central case.
3. **Measuring per-cell quiescence is exactly the trap this session just
   fell into twice.** Any "is this cell settled" test is the same class
   of local heuristic §1a(3) proved insufficient.

**Quiescence keeps one job, and it is a different one:** it gates the
body's own **sleep**, not its promotion (§7d). A body that has converged
stops running its solver and lets its chunks sleep. That is a whole-body
measurement (`max |Δlevel|` and `max |flux|` across the array) computed
in O(width) by machinery that already has the whole body in hand — never
a per-cell test.

### 4b. The gate, restated

Promotion fires when §3b's four structural conditions hold and the body
is not in cooldown. Nothing about motion.

### 4c. Thrash control

A violently splashing mass will fail §3b repeatedly (multi-span columns,
no free surface) and would otherwise cost one wasted flood fill per
frame; a body that demotes and immediately re-promotes would churn cell
flags across several chunks.

- **One attempt per frame globally** already bounds the wasted work.
- **Per-region cooldown after a demotion**, exponentially backed off from
  `DEMOTE_COOLDOWN_FRAMES` up to a cap, reset on a successful promotion
  that survives N frames. Same shape as `ActiveKind::Organism`'s
  `stale_ticks`, so it introduces no new pattern.
- **§12's B-4 is a hard bound on churn**, so thrash is a test failure
  rather than a thing someone notices in a profile later.

---

## 5. Demotion

### 5a. Detection at the write seam

**Decision: `FLAG_MANAGED` is tested on the *existing* cell inside
`World::set` and `ChunkView::set`.** If the old cell is managed and the
write is not from the body's own rasterizer, the owning body is looked up
via `body_index` and queued for demotion (applied in the serial body
phase, §8 — never mid-sweep).

The rasterizer's own writes go through a distinct entry point,
`World::set_owned`, so the two paths are lexically distinct rather than
separated by a mode flag someone can forget to clear.

**What this catches without enumerating anything:** brush painting,
erasing, `explosion::trigger`, `fire::transform` (water→steam, a burning
oil body), `structural::break_free` dropping debris in, a particle
landing, `plant.rs`'s `Absorb` draining a cell, `creature.rs` swimming
through, and — later — Report C's rigid bodies.

**The alternative, and why it is rejected.** Hooking each of those call
sites individually is cheaper per frame (no test in the hottest function
in the engine) but requires a list to be kept correct forever. This
codebase has been bitten by exactly that pattern three times, and Report
B §6a says so in as many words: the light channel had two readers and no
writer; canopy density had a writer and an always-zero reader; the
pressure field has producers and no liquid consumer. **A correctness
property that depends on an enumeration staying complete is the failure
mode this project keeps rediscovering.** Take the branch, and measure it
against §12's cost bar; if it does not fit, that is a real finding and
the fallback is the enumeration plus a `debug_assert` at the seam.

**Cost note, honest:** `World::set` does not currently read the old cell.
Adding the test costs one array index in a function called several times
per moving cell per frame. That is a real, measurable cost and it is why
§12 keeps Report B §10g's 15% ceiling as a hard bar rather than a
courtesy.

### 5b. What a demotion actually does

1. Clear `FLAG_MANAGED` on every cell of the body and on its managed
   container cells.
2. Remove its entries from `body_index`; free its slot in `World::bodies`
   (generation bumped, per the `World::organisms` precedent).
3. Start the region's cooldown.

**No mass moves.** The cells were already exactly the body's state (§2a,
§7e). This is the property that makes demotion trivially safe and is why
§11 builds it before anything else.

### 5c. Whole-body demotion in v1, and the interlock that makes it affordable

v1 demotes the **whole body** on any disturbance. A pebble dropped in one
end of a wide lake reverts the whole lake to CA for a few frames.

That is only affordable because of an interlock worth stating explicitly:
**the common disturbances are not disturbances.** A waterfall feeding the
lake goes through absorption (§6b), which is the body's own sanctioned
inflow and does not demote. Rain, a spill re-entering at the edge, a pour
landing on the surface — all absorption. What is left to demote on is
genuinely structural: the floor breaking, an explosion, a fire, a brush.
Those are rare and they *should* invalidate the representation.

**The v2 pattern, named now so M8 does not have to discover it:** for any
disturbance that turns out to recur, the fix is a *sanctioned
mass-exchange API on the body* rather than a demotion —
`LiquidBody::absorb(column, fill)` already exists as §6b;
`LiquidBody::withdraw(column, fill)` is what `plant::Absorb` should use
if roots beside a lake turn out to thrash it; `LiquidBody::displace_
column(column, volume)` is what Report C §5's buoyancy needs so that a
floating body does not demote the lake it is floating in (§9c). Same
shape each time: an integer transfer that debits one side and credits the
other in one operation.

---

## 6. The seam — a body that is part CA and part heightfield

This is the section Report B §5a says to prototype first. Its own words:
"Getting a visible discontinuity at the boundary between the two
representations is the obvious failure mode."

### 6a. Column granularity, never cell granularity

**Invariant: every liquid cell in the world is owned by exactly one
representation — the CA or one body — and mass crosses between them only
by an integer transfer that debits one and credits the other in the same
operation.**

The boundary runs along **column edges**. A column is managed or it is
not; there is no column that is half-and-half. The reason is direct: the
heightfield's atom is a column, so a boundary any finer means the two
representations hold different opinions about the contents of one column,
and *that disagreement is what a visible discontinuity is made of*.

### 6b. Inflow: absorption

**Rule: when the CA's `transfer_liquid_vertical` finds that the cell
below is `FLAG_MANAGED` and the same material, the entire source cell is
absorbed** — the source becomes `Cell::EMPTY` and its full `liquid_fill`
is added to that column's `h[]`.

- One new branch in an existing function that already reads its vertical
  neighbour.
- The whole cell, not `flow_rate`-worth: the body knows how to spread it
  in O(width), so throttling the handoff would only make the waterfall
  pile up.
- Exactly conservative: the removed cell's fill is the added integer.

**Horizontal transfer treats a managed cell as a wall.** No sideways
absorption. The reason is that a managed cell at *depth* is interior to
the body, and pushing fill into a column's interior has no meaning in a
free-surface model — inflow physically belongs at the surface. A CA
puddle sitting beside a body at the same level is instead picked up by
`try_extend` (§3d) within `EXTEND_INTERVAL` frames. **Named cost:** that
puddle sits still for up to `EXTEND_INTERVAL` frames looking mildly
stuck. Keep the interval small (single-digit frames) and check it in the
prototype's screenshots.

**Free fall is untouched.** `try_move`'s downward and diagonal cases
already require an empty destination and are unaffected; falling water
above a lake behaves exactly as it does today until it reaches the
surface. That matters: free fall is the part the CA is genuinely good at,
and §1a's investigation confirmed that the diagonal cascade path
"behaves correctly" and is unrelated to the fill-transfer rules.

### 6c. Outflow: edge demotion, not a special rule

The body must be able to spill over a ledge. Rather than teaching the
heightfield free-surface dynamics it is bad at:

**Rule: if the solver computes net outflow across the body's outer
boundary at an edge column — i.e. that column's level exceeds what the
body can contain and there is somewhere outside for it to go — that one
column is *demoted*.** Its cells lose `FLAG_MANAGED`, the body's array
shrinks by one entry, and the CA owns the spill from the next frame. The
column may be re-absorbed later by `try_extend`.

Consequences worth stating:

- Every genuinely dynamic edge is CA. The heightfield stays strictly
  interior, which is where its assumptions (single span, free surface,
  hydrostatic equilibrium) are actually true.
- It gives the eye a one-column CA "collar" at every free edge, so the
  transition is never a hard line at a place the player is watching
  something move.
- It is the same operation as §5b, scoped to one column, so it needs no
  separate mass accounting.

### 6d. The waterfall-into-a-lake walkthrough

The case Report B §5a names as unanswered, traced end to end:

1. A pour lands. Falling cells free-fall via `try_move` — CA, unchanged.
2. The pool at the bottom reaches `MIN_BODY_COLUMNS` and satisfies §3b.
   The promotion queue picks it up within a frame or two; the *lake*
   becomes a body. The falling stream does not — its columns are
   multi-span or lack a settled bed, so they fail §3b, which is correct.
3. Each frame, the lowest cell of the falling stream sits directly on a
   managed cell. `transfer_liquid_vertical` absorbs it whole. The stream
   above it falls one cell to fill the gap, on the next sweep. The stream
   is continuously consumed at its foot at one cell per frame per stream
   column — the same rate gravity delivers it.
4. Absorption raises `h[]` at the impact column only. The pipe solver
   spreads that over the body in O(width) with a visible travelling
   surface disturbance (§7a's persistent flux is what makes it *travel*
   rather than diffuse).
5. If the level reaches the lake's containing ledge, the edge column
   demotes (§6c), the CA spills it, and it either falls away or is
   re-absorbed by `try_extend`.
6. When the pour stops, absorption stops, the flux damps out, the
   terminal snap (§7d) lands the surface exactly flat, and the body
   sleeps — and so do its chunks.

At no point is a body both CA and heightfield in the same column, and at
no point does mass exist in two places.

### 6e. What can still look wrong, named rather than discovered

- **The impact has no splash.** Absorption is silent. `particle.rs`
  already exists for exactly this; emitting a few particles on absorption
  above a threshold is a one-line polish item, deliberately not required.
- **A stream landing on a *very* wide body** raises a bump that the
  solver takes O(width) frames to spread. That is correct physics
  (a real surface wave) but it is also the first thing to check against
  §12's seam-continuity bar, because the tuning that makes it look like a
  wave and the tuning that makes it look like a glitch are close
  together.
- **`try_extend`'s latency** (§6b).
- **A body under an overhang is refused** (§3b condition 3) and will
  level at CA speed. Correct and deliberate; it is §14's out-of-scope
  boundary showing through.

---

## 7. The pipe solver

Report B §5 calls this the easy part, and it is. One thing in its summary
is nonetheless wrong in a way that would sink an implementation, so that
comes first.

### 7a. The correction: flux must be persistent state, or you have rebuilt diffusion

Report B §5 claims the pipe model "converges geometrically rather than
diffusively." A plain relaxation on a height array does **not**: if each
step recomputes a transfer from the current height difference and applies
it, the update is `h' = h + c·(h[i-1] − 2h[i] + h[i+1])`, which is
explicit diffusion, is stable only for `c ≤ 1/2`, and levels a body of
width W in **O(W²) iterations** — the exact complexity being escaped,
merely with a smaller constant and a cheaper per-iteration cost.

What makes the virtual-pipes model actually O(W) is that **the flux is
itself state, carried from step to step.** The paper's flux update
accumulates:

> `f^L_{t+∆t}(x, y) = max(0, f^L_t(x, y) + ∆t · A · g · ∆h^L(x, y) / l)`

The `f_t` term is momentum. With it, the system is hyperbolic — a
shallow-water wave — and a level change propagates across the body at a
finite wave speed, i.e. in O(W) steps, then rings and must be damped.
Without it, the system is parabolic and you are back where you started.

**Therefore: `flux: Vec<i32>`, one entry per interface, persistent across
frames, is not an optimisation. It is the mechanism.** §13's deletion
test is written against exactly this.

### 7b. The 1D integer reduction

State, all integers, all in fill units on the `LIQUID_FULL` scale:

```
level[i]  = (REF - bed[i]) * LIQUID_FULL + h[i]   // surface elevation, up-positive
flux[i]   : i32                                   // signed, across interface i|i+1
```

`REF` is any fixed y below the body; only differences are used. Signed
flux replaces the paper's two opposed one-way pipes per interface —
identical in 1D, half the state, and it makes conservation textually
obvious.

Per frame, for a body of `n` columns:

```
// 1. flux update -- the persistent term is the whole point
for i in 0..n-1:
    d = level[i] - level[i+1]
    flux[i] = damp(flux[i]) + gain * d          // integer; see 7f

// 2. K clamp: a column cannot pay out more than it holds
for i in 0..n:
    out = max(0, flux[i]) + max(0, -flux[i-1])   // absent interfaces = 0
    if out > h[i]:
        scale flux[i] and flux[i-1] by h[i]/out  // exact rational, i64 intermediate

// 3. apply -- exactly conservative by construction
for i in 0..n-1:
    h[i]   -= flux[i]
    h[i+1] += flux[i]
```

Step 3 is where the conservation guarantee lives: every interface debits
one column and credits its neighbour by the *same* integer, so `Σ h` is
invariant regardless of what steps 1 and 2 computed, including if they
computed nonsense. That is a strong property and it is worth building on
purpose.

### 7c. Stability: the clamp makes the scheme safe even where it is inaccurate

The literature's pipe model is an explicit shallow-water scheme and has a
time-step restriction; the sources consulted for this document state that
a restriction exists but the specific condition was not obtained (§15).

That matters less than it would elsewhere, because of the integer + clamp
formulation:

- **`h` can never go negative** — step 2 guarantees it.
- **Mass is exact regardless** — step 3 guarantees it.
- So an over-large `gain` or an under-damped `damp` does not blow up or
  leak. It shows up as **visible sloshing that takes too long to settle**,
  which is a tuning symptom, observed in one screenshot, not a corruption
  hunted through a conservation test.

Recommended starting point: `gain` ≈ 0.4 (well inside the `c ≤ 1/2`
diffusive bound, which is the conservative reading), `damp` ≈ 0.9. **Both
untuned and flagged as such**; tune against §12's leveling bar and the
sleep bar together, since they pull in opposite directions (more gain
levels faster and rings longer).

### 7d. The terminal snap, and why it is not optional

Damped waves approach flat asymptotically. Two problems: the body never
quite stops (so it never sleeps, so its chunks never sleep, which
violates the engine's entire cost model), and any threshold you stop at
leaves a residual unevenness — which is `MIN_LIQUID_TRANSFER`'s failure
recreated at body scale.

**Decision: when `max|level[i] − level[i+1]| < SNAP_EPSILON` and
`max|flux[i]| < SNAP_EPSILON`, solve the equilibrium exactly and snap to
it.**

The equilibrium is analytically available in 1D. Find the integer surface
elevation `L` with

```
Σ_i clamp(L - bedlevel[i], 0, cap[i]) == total
```

which is monotone in `L`, so a binary search over the body's elevation
range finds it in O(n log range). Distribute the exact integer remainder
deterministically (lowest column index first — *not* by any
hash/iteration order; issue #7's standard). Then rasterize once, zero the
flux array, and mark the body asleep.

Consequences:

- **Surface flatness becomes exact, not bounded.** Report B §10b asks for
  under 2% of `LIQUID_FULL` between adjacent columns; the snap delivers
  ≤ 1 fill unit (0.1%) by construction.
- **`MIN_LIQUID_TRANSFER` can go back toward 8.** Report B §3c's
  diagnostic — "if a change to the leveling mechanism doesn't let this
  number go down, it didn't fix the underlying problem" — becomes a
  passing test rather than an aspiration.
- **The body sleeps**, and with nothing writing its cells, so do its
  chunks.

Because the snap only fires once the surface is already within
`SNAP_EPSILON`, the visual change is sub-cell — it is not water
teleporting. **`SNAP_EPSILON` must stay small enough that this is true**
and that is a screenshot check, not a unit test.

A woken body (absorption, `try_extend`, a neighbour demotion) clears
`asleep` and resumes at step 1 with zero flux.

### 7e. Rasterization: lazy, back into cells

Per column, the target cell contents are `h[i] / LIQUID_FULL` full cells
stacked on `bed[i] - 1` upward, plus a partial cell of
`h[i] % LIQUID_FULL` above them (omitted if zero).

**Rasterize a column only when its whole-cell count changes** — that is,
when `h[i] / LIQUID_FULL` differs from what was last written — and write
the partial top cell's `aux` in the same operation. Between those events,
the column's pixels are byte-identical to what is already there, so
nothing is written, so nothing dirties, so quiet regions of a moving body
still let their chunks sleep.

**Named cost:** the top partial cell's `aux` can lag reality by up to one
whole cell's worth of fill while the body is moving. That is sub-pixel on
one cell per column, and the terminal snap writes it exactly. Accepted.

**Rejected: rendering heightfield bodies directly and leaving the cells
stale.** It would be cheaper, and it would make the grid lie — breaking
`take_touched_chunks`'s contract, every other subsystem's reads (§2a),
and every existing liquid test, all at once. Not a close call.

### 7f. Where the constants live

Per `design-philosophy.md` §2a — a constant graduates to hot-reloadable
data if a non-programmer might plausibly want to tune it.

- **`gain` reuses the existing per-material `flow_rate`.** It is already
  in `water.ron`, already documented as "how much of a cell's fill may
  transfer per tick," already the knob meaning *runniness*, and is
  currently 1000 (uncapped) precisely because the CA's halving rule made
  it redundant. Rescaling it as the pipe gain gives it a real job back
  instead of adding a second, parallel viscosity knob. **Its water.ron
  doc comment must be rewritten if this lands** — the existing comment's
  reasoning is specific to `transfer_liquid_horizontal`'s halving and
  would become misleading.
- **`damp`** — new, per-material `.ron`.
- **`MIN_BODY_COLUMNS`, `EXTEND_INTERVAL`, `DEMOTE_COOLDOWN_FRAMES`,
  `SNAP_EPSILON`** — global, and gameplay-adjacent enough to belong in
  data rather than Rust `const`s under the same rule. `MAX_BODY_CELLS` is
  a capacity bound and stays a Rust `const`.

---

## 8. Cross-chunk and the parallel sweep

### 8a. Decision: a separate serial body phase

**Heightfield bodies step in their own serial frame phase, not inside the
checkerboard sweep.** Stated explicitly, as Report C §4 asked its own
version of this question to be decided rather than discovered.

Four reasons:

1. **The write-disjointness proof forbids the alternative.** `parallel.
   rs`'s module doc proves that two same-pass chunks never write the same
   cell, resting on every rule's reach being bounded by `MAX_REACH` and
   on a passive chunk receiving writes only from one geometrically
   opposite pair. A body spanning chunks `(0,0)` and `(2,0)` — both group
   `(0,0)`, both active in the same pass — writing its own columns from
   both workers violates that directly. This is the identical hazard
   Report C §4 flags for a rigid body straddling two same-pass chunks.
2. **There is no speedup to give up.** The phase is O(columns of awake
   bodies): hundreds of integer operations. Microseconds.
3. **The precedent already exists, twice.** `scheduler::step` and
   `ParticleSystem::step` both run outside the sweep, both write via the
   ordinary `World::set`, and `scheduler.rs`'s own doc gives the reason
   verbatim: *"nothing else is concurrently touching the world at this
   point in the frame."*
4. **One phase hosts both body kinds**, which is half of what §9 means by
   "one mechanism."

**Placement in the frame: after `parallel::step`, before
`world.step_active_sites()`** (`app.rs:228-232`). After the sweep because
the sweep is what produces this frame's absorptions; before active sites
because `plant::Absorb` reading an adjacent liquid cell should see this
frame's settled surface, which is the same reasoning `app.rs`'s existing
comment already gives for putting active sites after the sweep.

### 8b. Absorption is queued — and it is a genuine exception to `run_pass`'s stated proof

Absorption happens *inside* `update_liquid`, i.e. inside a parallel
worker, which holds only `&World` for everything outside its own chunk
and certainly cannot mutate a body's `h[]`.

**Solution: a fourth queue on `ChunkView`**, alongside `remote_writes`,
`dirty_touches`, `field_writes`, `light_writes` and
`pending_active_sites`:

```
absorptions: Vec<(BodyId, u32 /*column index*/, u32 /*fill*/)>
```

replayed serially in `run_pass`'s existing per-outcome loop, before the
body phase runs. The source cell is set to `Cell::EMPTY` in the same
sweep via the ordinary `set` path, so the debit and the credit are
emitted together and cannot be separated by a failure in between.

> **This must be documented as an explicit exception**, because
> `run_pass`'s own comment currently asserts: *"Replay order between
> different outcomes never matters here — by the module doc's proof, no
> two outcomes from the same pass ever queue a write to the same
> position."* Two workers absorbing into the same body **do** queue
> operations against the same target. It is nonetheless safe, and for a
> reason different from the existing proof: an absorption is a
> **commutative integer accumulate**, not a last-writer-wins `set`. Order
> genuinely does not matter because addition is associative and
> commutative, not because collisions are impossible. Write that
> distinction into the code, or the next reader will believe the proof
> covers a case it does not.

Demotion requests raised from inside the sweep (a `set` into a managed
cell by a CA rule — e.g. a falling grain of sand displacing a managed
water cell) are queued the same way and applied at the start of the body
phase.

### 8c. Waking and sleeping

- An **awake body** is stepped every frame by the body phase regardless
  of whether its chunks are asleep — the body list is its own schedule,
  exactly as the active-site heap is for growth. This is what lets a
  settled-looking lake still be converging internally without every
  chunk it touches staying dirty.
- A **sleeping body** (§7d) writes nothing, so its chunks settle
  normally. It wakes on absorption, `try_extend`, or a neighbour column's
  demotion.
- **Under M10 streaming:** a body straddling a chunk that unloads is
  demoted on unload and re-promoted on load. Same answer Report C §8
  reaches for rigid bodies, and it is free here because demotion moves no
  mass and the body's entire state is recoverable from the cells.

---

## 9. One mechanism, shared with Report C's rigid bodies

Report C §8: *"a sleeping body should be written into the grid as
ordinary cells and removed from the rapier world entirely, waking back
into a body only on disturbance. That is the same promotion/demotion
pattern Report B §5 proposes for settled water bodies, and the two should
share a design."*

### 9a. What is genuinely shared — the interface both kinds use

**One ownership substrate:**

- `Cell::flags`' `FLAG_MANAGED` bit: *the CA sweep must not move this
  cell; it belongs to a promoted body.* Kind-agnostic.
- `World::bodies: Vec<BodySlot>` — a generational slot allocator built
  from the `World::organisms` / `OrganismSlot` pattern that already
  exists and is already tested (`organism_ids_round_trip_and_encode_a_
  nonzero_generation`, `organism_id_zero_is_always_none`). Not a new
  pattern; the third user of an existing one.
- `World::body_index: HashMap<ChunkCoord, SmallVec<BodyId>>` — "which
  bodies touch this chunk," which is what disturbance lookup and
  streaming both need, for either kind.

**One protocol**, which is the actual deliverable Report C §8 asked for:

```
trait Body {
    /// Cheap structural test. Liquid: §3b. Rigid: size/complexity
    /// thresholds per Report C §2.
    fn eligible(scan) -> bool;

    /// Build the body's own representation FROM the cells, and claim
    /// them (FLAG_MANAGED + body_index). Must not move mass.
    fn promote(scan) -> Self;

    /// Advance one frame. Liquid: §7's pipe step. Rigid: the rapier
    /// step. Runs in the serial body phase (§8a).
    fn step(&mut self, world: &mut World);

    /// Write the representation back into cells and release them.
    /// MUST be exactly mass-conservative. For liquid this is nearly a
    /// no-op because rasterization already keeps the cells correct;
    /// for rigid it is the re-rasterize-at-current-transform step.
    fn demote(self, world: &mut World);
}
```

**One frame phase** (§8a), hosting both, serial, after the sweep.

**One disturbance rule**: a foreign write to a `FLAG_MANAGED` cell queues
a demotion of its owner, whatever kind that owner is.

### 9b. What is not shared, stated rather than forced

- **The physics.** Virtual pipes and a rapier constraint solver have
  nothing in common and pretending otherwise would produce an abstraction
  that fits neither.
- **The eligibility predicate.** §3b is about vertical spans and free
  surfaces; Report C §2's is about size and contour complexity.
- **Per-cell body identity.** Report C §2 quotes Noita's scheme where
  each pixel knows its body *and its position inside that body*, which a
  rotating rigid body needs and a liquid body does not (a liquid cell's
  position *is* its column index). **So v1 stores no body id on the cell
  at all** (§3c), and the eventual `Cell::organism_id` → `owner_id`
  rename is M8's to make when rigid bodies need it.

  > **When M8 does make that rename, it must fix `update.rs:132` in the
  > same commit.** `if cell.organism_id() != 0 { organism::diffuse_
  > resource(surface, x, y); }` currently uses "has an organism id" as a
  > proxy for "is organism tissue." Generalise the field without
  > kind-gating that call and resource diffusion starts running on water
  > and on rigid-body stone. Named here because this is the document that
  > proposes the generalisation.

**Honest scope of "one mechanism":** one *substrate*, one *protocol*, one
*phase*, two implementations. That is what §8 of Report C actually needs
(so that a rigid body's sleeping is not a second, divergent invention),
and it is as far as the unification honestly goes.

### 9c. The one place the two kinds interact, flagged for M8

Report C §5's buoyancy computes displacement per column against the 1D
water array. Under v1's whole-body demotion rule, **a rigid body entering
a lake would demote the lake** — destroying exactly the structure
buoyancy needs.

The fix is §5c's v2 pattern, and the interface is nameable now:

```
LiquidBody::displace_column(column, volume) -> ()   // raises neighbours, exactly conservative
LiquidBody::depth_at(column) -> u32                 // what buoyancy integrates against
```

with `FLAG_MANAGED` cells overlapped by a rigid body treated as
*displaced*, not *disturbed*. **This is the one item in this design that
M8 cannot simply inherit and must extend**, and Report C §5's
acceptance criteria 10e/10f are the tests for it. Not built here.

---

## 10. Mass conservation, and the incompressibility lesson

Report B §10a asks for 0.5% over 2,000 frames. This design targets
**exact**, and the reason is that the existing suite already does:
`a_landing_column_does_not_balloon_in_cell_count` asserts
`liquid_volume == start_fill` on **every one of 300 frames**, and
`a_splash_settles_with_no_stray_droplets_and_no_mass_drift` asserts
`fill_before == fill_after`.

Every mass-moving path in this design is an integer transfer that debits
one holder and credits another in the same operation:

| Path | Debit | Credit | Exact because |
|---|---|---|---|
| Promotion | — | — | No mass moves; `h[i]` is *computed from* the cells (§9a's `promote` contract) |
| Demotion | — | — | No mass moves; the cells are already the body's state (§7e) |
| Pipe step | `h[i] -= flux[i]` | `h[i+1] += flux[i]` | Same integer, same statement (§7b step 3) |
| Absorption | source cell → `Cell::EMPTY` | `h[col] += fill` | Same integer, emitted in one queued operation (§8b) |
| Edge demotion | — | — | Column's cells already hold its `h`; only flags change |
| Terminal snap | redistributes | redistributes | Solves for `L` such that the sum equals `total` exactly, integer remainder assigned deterministically (§7d) |

**And the second lesson, which is the one that actually cost this session
two reverts: exact conservation is necessary and not sufficient.** The
reverted reorder conserved mass perfectly and was still physically
nonsensical, because it produced *the same mass occupying five times the
volume*. Water is incompressible; a buried cell has no free surface and
no reason to redistribute sideways.

This design's answer is not a rule but a representation (§2c): a managed
column is `floor(h)` full cells plus one partial. **The over-diluted
state has no encoding.** §12's B-1 asserts that as an invariant on the
body, so the property is checked directly rather than inferred from a
cell count staying under a threshold.

**And the CA side keeps its existing protection unchanged.** Nothing here
touches `update_liquid`'s vertical-before-horizontal order. Cells outside
a body still get the accidental throttle that `update.rs`'s doc describes
as load-bearing. This design does not need to remove it — it routes
around it by taking the wide bodies, which are the only place it hurt,
out of the CA's hands entirely. That is the whole architectural claim, in
one sentence.

---

## 11. Build order — seam first, per Report B §5a

Each step is independently committable and independently testable, and
each leaves the suite green.

**Step 1 — the ownership substrate and the promote/demote round trip. No
physics at all.**
`FLAG_MANAGED`; `World::bodies` generational slots; `body_index`;
`liquid::label_body` with §3b's validation; the disturbance test at the
`set` seam; `World::set_owned`; the serial body phase, doing nothing;
`update_cell` skipping managed cells. The only body kind is
**`Body::Frozen`** — it holds `h[]`, rasterizes nothing, and never
changes.

*Independently testable, and this is the step that proves the design's
central claim:*
- A promoted body does not move, at all, for 2,000 frames.
- Total fill across promote → 2,000 frames → forced demote is **bit
  identical** to before promotion.
- Every enumerated disturbance (paint, erase, explosion, fire, a falling
  grain of sand, digging out the bed) demotes, and mass is exact across
  it.
- `cell_count == Σ ceil(h[i] / LIQUID_FULL)` holds while promoted.

*This is the seam, isolated from the physics entirely, which is exactly
what Report B §5a asks to be prototyped first.* If the ownership rules
cannot be made clean here, stop, and fall back to Report B §7's
multigrid, which stays inside the CA representation.

**Step 2 — absorption. Still no solver.** Gated on 1.
`transfer_liquid_vertical`'s managed branch; `ChunkView::absorptions` and
its serial replay; lazy rasterization (§7e).

*Independently testable:* pour onto a frozen body; total fill exact every
frame; the body's mass rises by exactly what the CA lost. **Expect, and
screenshot, the informative failure:** without a solver the absorbed mass
piles into one column and makes a spike. That spike *is* the seam
working; step 3 is what spreads it.

**Step 3 — the pipe solver.** Gated on 2.
Persistent flux, the K clamp, the apply. Tune `gain`/`damp` against §12's
leveling bars. The spike from step 2 becomes a travelling wave.

**Step 4 — quiescence, the terminal snap, and body sleep.** Gated on 3.
Now `active_chunk_count()` can reach zero over a promoted lake.

**Step 5 — `try_extend`, edge demotion, cooldowns.** Gated on 4. This is
what turns a working solver into something that survives real play.

**Step 6 — drop `MIN_LIQUID_TRANSFER` toward 8** and confirm settling
time stays in budget. Report B §3c's own instruction: *"Treat
`MIN_LIQUID_TRANSFER` as the diagnostic, not the setting. If a change to
the leveling mechanism doesn't let this number go down, it didn't fix the
underlying problem."* **This step is the verdict on steps 3–4.**

Not gating anything, and buildable in parallel by anyone: Report B §10h's
variable-resident-area stress scene in `examples/ascii.rs`. It is needed
before M10 regardless and it is the only thing that turns §12's scaling
bar into a measurement.

---

## 12. Acceptance criteria

Report B §10's bars are inherited where they apply, tightened where this
design can do better, and dropped where they belong to a different
mechanism. New ones are lettered `B-n`.

**Inherited and tightened**

- **10a → exact.** Total liquid fill must be **bit identical** across a
  2,000-frame dam-break, checked every frame, not within 0.5%. Justified
  by §10's table and by the two existing tests that already assert
  exactness.
- **10b → 0.1%.** A settled 100-column pool's maximum adjacent-column
  height difference must be **≤ 1 fill unit** (0.1% of `LIQUID_FULL`),
  not 2%. The terminal snap (§7d) makes this achievable by construction;
  if it is not met, the snap is broken.
- **10c, 300 frames.** The same 100-column pool must reach 10b's bar
  within **300 frames**.
- **10d, no detached droplets.** Unchanged, and unaffected by this design
  (it belongs to Report B §3b's VOF height function). Must not regress.
- **10f, pour slope ≤ 10° after 60 frames.** Unchanged. Note this is
  Report B §8/§9's `FLAG_FLOWING` item, not this one; listed so a future
  reader does not attribute a failure here to the heightfield.
- **10g, cost ceiling ≤ 15%.** Retained as a *hard* bar and reinterpreted
  per §1c: measured against the **pre-reorder** baseline
  (~37.9 ms serial / ~8.7 ms parallel on `examples/ascii.rs`'s
  sand-and-water stress scene), **not** against `eeefceb`'s ~42.6 ms.
  This design must not spend the reorder's ~12%; it should spend far
  less, because its per-frame cost is O(awake columns) plus one flag test
  in `set`.
- **10h, sub-linear scaling** in resident chunk count. Unchanged, and
  this design should do well on it — a sleeping body costs literally
  nothing.
- **10e (U-tube) is explicitly NOT inherited.** §3b condition 3 refuses
  to promote a ceiling-capped body, so a U-tube stays CA by design.
  10e belongs to Report B §6's pressure path trace. Claiming it here
  would be dishonest.

**New**

- **B-1 (hard constraint, must never regress). Incompressibility.**
  `parallel.rs`'s existing `a_landing_column_does_not_balloon_in_cell_
  count` must continue to pass **unchanged** — exact fill every one of
  300 frames, peak water cell count < 1.5× the starting 23,400. Plus a
  representation-level invariant asserted directly while any body is
  promoted:
  `water_cell_count == Σ_bodies Σ_i ceil(h[i]/LIQUID_FULL) + (unmanaged CA cells)`.
  The first catches a regression; the second explains it.
- **B-2. Leveling responsiveness, against two baselines on one scene.**
  Restore `eeefceb`'s deleted test
  `three_tall_columns_spanning_chunk_boundaries_flatten_within_900_frames`
  verbatim (its scene is *identical* to B-1's — same 512-wide world, same
  `floor_y = 300`, same three 30-column, 260-tall columns at
  `(80,110)`, `(220,250)`, `(400,430)` — and its bar is that no adjacent
  column pair differs by ≥ 3 cells at frame 900).
  - **Bar 1, the thing to beat:** flat by **frame 900**, matching
    `eeefceb`.
  - **Bar 2, the target:** flat by **frame 400**. The heightfield's
    O(width) claim should be visibly better than a reorder, not level
    with it. Failure at bar 2 with a pass at bar 1 is a tuning finding,
    not a design failure; failure at bar 1 means the design did not
    deliver what a four-line reorder already did.
  - **The point of using one scene for B-1 and B-2 is that these two
    criteria were in direct tension under every per-cell approach tried.
    Satisfying both simultaneously on the same scene is the falsifiable
    statement that the architectural fix worked.**
- **B-3. Demotion is exactly conservative.** promote → arbitrary
  disturbance → demote leaves total fill bit identical, across 1,000
  randomised disturbance sequences.
- **B-4. No promote/demote thrash.** Over a 2,000-frame
  pour-then-settle, total promotions ≤ 50 and total demotions ≤ 50 for
  a scene containing one lake and one waterfall.
- **B-5. Seam continuity under live inflow.** With a waterfall
  continuously feeding a promoted lake, no two adjacent columns
  *outside a 4-column radius of the impact point* may differ by more
  than 1 cell. (Inside that radius a real surface disturbance is
  correct and must not be suppressed.)
- **B-6. A promoted body sleeps.** A pour-and-settle scene must reach
  `active_chunk_count() == 0` **and** every body `asleep`, within 600
  frames. Without this, the design has traded a leveling problem for a
  sleeping problem.
- **B-7. Determinism.** The same scene run twice in one process must
  produce bit-identical `h[]` arrays and identical body ids. Integer
  state makes this achievable; the candidate-queue ordering hazard in
  §3e is the thing it actually tests.
- **B-8. Body-phase cost.** The serial body phase's worst frame, with
  ≥ 4 bodies totalling ≥ 1,000 columns, must stay under **0.5 ms**,
  reported separately from the sweep so a regression can be attributed
  (same discipline Report C §10h asks for RFT).
- **B-9. `MIN_LIQUID_TRANSFER` drops to ≤ 16** with 10b and 10c still
  passing. Report B §3c's diagnostic, as a test.

---

## 13. Deletion tests

| Mechanism | Test that must fail without it |
|---|---|
| Persistent per-interface flux (§7a) | B-2 bar 2 — with flux recomputed from scratch each step the solver is explicit diffusion again, and a 512-column body reverts to O(W²) leveling |
| The K clamp (§7b step 2) | Column heights go negative on a steep initial profile; the rasterizer produces garbage or panics |
| Terminal equilibrium snap (§7d) | 10b (residual unevenness persists asymptotically) **and** B-6 (the body rings forever and never sleeps) |
| Single-span promotion check (§3b.2) | A cave with stacked water bodies in one column promotes, and the two levels merge into one — mass conserved, geometry nonsense |
| Free-surface promotion check (§3b.3) | A ceiling-capped body promotes and its columns level *through* the ceiling, or the rasterizer clips them and loses mass |
| Absorption (§6b) | B-5 — a waterfall piles into a wall of CA water above the lake instead of feeding it; the lake's level never rises |
| Edge demotion (§6c) | Water in an over-filled basin refuses to spill; the body holds a level above its own ledge |
| `FLAG_MANAGED` on container cells (§3c) | Digging the floor out from under a lake leaves the lake floating |
| Disturbance test at the `set` seam (§5a) | B-3 — an explosion inside a body leaves the heightfield and the cells disagreeing, and the next rasterization manufactures or destroys mass |
| Serial body phase (§8a) | A body spanning two same-parity chunks corrupts under `parallel::step` while passing under `update::step` — the classic signature |
| Sorted promotion queue (§3e) | B-7 — two runs assign different body ids and diverge |

---

## 14. Explicitly out of scope

Named rather than silently omitted.

- **Hydrostatic pressure, U-tubes, and Report B §10e.** §3b refuses
  ceiling-capped bodies precisely so this design does not have to have an
  opinion. Report B §6's DF-style path trace is the mechanism, and it is
  a separate piece of work with its own cap-length decision.
- **Multi-span columns.** Caves with two water bodies at different levels
  in one `x` are refused promotion and stay CA. This is the most likely
  first extension and the data structure (`h`/`bed` per column) would
  need to become a list of spans to support it.
- **Two materials in one body.** Water and oil layering. Refused by
  §3b.1.
- **VOF's local height function and flotsam/jetsam** (Report B §3b).
  Orthogonal, lives on the CA path, and 10d must simply not regress.
- **`FLAG_FLOWING` gating on the liquid horizontal search** (Report B
  §8/§9 item 1). Independent; the bit already exists.
- **Rigid-body buoyancy** (Report C §5). The interface is named in §9c;
  nothing is built.
- **Worldgen hydraulic erosion.** The solver is deliberately shaped so
  that Report B §5's "do not choose a second water representation for
  worldgen" is satisfiable, but sediment transport, deposition and the
  erosion terms of Mei–Decaudin–Hu are entirely absent.
- **LBM** (Report B §4) and **hierarchical multigrid** (Report B §7). The
  latter is the named fallback if §11 step 1's seam turns out not to be
  clean.
- **Splash particles on absorption** (§6e). Polish.
- **Partial (column-range) demotion.** v1 demotes whole bodies (§5c).
- **Anything touching `Powder` state.** `granular-mechanics-research.md`
  §5's packing scalar keeps its `aux` slot untouched.

---

## 15. What was read, what was verified directly, and what was not

Following the convention every other report in `Reports/` uses.

**Read in full, in this repository, this session**

- `Reports/liquid-simulation-research-r2.md` — the whole report, with §5
  and §5a as the direct brief.
- `Reports/coupling-research.md` — the whole report; §4, §5, §7a, §8, §9
  and §13 are load-bearing above.
- `Reports/granular-mechanics-research.md` §5 — skimmed to the extent
  §14's non-contradiction claim requires.
- `Reports/design-philosophy.md` §2a–§2d.
- `Reports/plant-substrate-v2-design.md` §0–§6 (partial; ~1,000 of 2,081
  lines) and `Reports/tree-rewrite-design.md`'s section structure — read
  for tone and structure, not for content.
- `PLAN.md`'s liquid section (lines ~3,506–3,850), which is the primary
  record of the two reverts and the `wake_all()` control experiment.
- Source, read directly and not taken from documentation: `src/sim/
  update.rs` (whole file), `src/sim/parallel.rs` (whole file), `src/sim/
  rigid.rs` (whole file), `src/sim/scheduler.rs` (whole file),
  `src/sim/mod.rs`, `src/app.rs`'s `update()`, and the relevant parts of
  `src/sim/chunk.rs` (`Chunk`, `sweep_region`, `recompute_reach`),
  `src/sim/world.rs` (`World` fields, `get`/`set`/`touch_neighbours`/
  `end_step`), `src/sim/cell.rs` (flags, `aux`, `organism_id`),
  `src/sim/material.rs` (the liquid constants), and
  `assets/materials/water.ron`.

**Verified directly against git history this session, not taken on the
brief's word**

- `eeefceb` ("Fix liquid pour stalling at chunk boundaries: horizontal
  before vertical") and `dcb761c` ("Revert liquid transfer reordering:
  fixed a stall, caused worse ballooning") both exist. The reordered
  version is exactly a **four-line swap** in `update_liquid` — the
  `transfer_liquid_vertical` early-return and the two
  `transfer_liquid_horizontal` calls exchanged — plus doc comments. That
  makes it trivially reconstructible from the *current* tree, which is
  why §12's B-2 recipe is a one-line instruction rather than a checkout.
- The **frame-900-vs-1800 numbers are recorded in `eeefceb`'s own doc
  comment and in the test it added**, written at the time of measurement:
  *"unfixed, a 3-cell step is still present at frame 900 and needs ~1800
  to fully resolve; with horizontal tried first, frame 900 is already
  fully flat."* That is an in-repository primary record, not a narrated
  summary, and it is re-runnable.
- The **deleted test's full source** was recovered from
  `git show eeefceb:src/sim/parallel.rs` and its scene is quoted exactly
  in §12's B-2, including the ≥ 3-cell adjacent-step metric. It is
  **identical in scene geometry** to the surviving
  `a_landing_column_does_not_balloon_in_cell_count`, which is the
  observation §12's B-2 is built on.
- A cost figure the brief did **not** mention and this document found in
  `eeefceb`'s own doc comment: the reorder carried a measured **~12%
  worst-frame serial regression (37.9 ms → 42.6 ms)** on
  `examples/ascii.rs`'s sand-and-water stress scene, "consuming most of"
  Report B §10g's 15% ceiling. §12's 10g bar is written against the
  pre-reorder baseline because of this.
- `water.ron`'s `flow_rate` is **1000** — i.e. uncapped, not a
  throttle — which is why §7f proposes rescaling it as the pipe gain
  rather than adding a parallel constant.

**Not verified — taken on the brief's and `PLAN.md`'s word**

- The `wake_all()`-every-frame control experiment and its conclusion that
  the stall is intrinsic to `update_liquid`. Recorded in `PLAN.md` and in
  the deleted test's own comment; **not re-run here.**
- The ~5× and ~4.8× ballooning peaks, and the "102,915 cells against a
  start of 23,400" figure. Recorded in `PLAN.md` and in
  `a_landing_column_does_not_balloon_in_cell_count`'s comment; not
  re-measured.
- **The timing reproduction itself was not re-run.** Reconstructing
  `eeefceb`'s behaviour requires only the four-line swap described above,
  but running it (and a current-baseline comparison) needs a full
  dependency build in an isolated target directory, which was judged not
  worth the cost against numbers already recorded in-repo at measurement
  time. **The implementer should re-run it as step 0 of §11**, since
  B-2's bars are stated relative to it: apply the four-line swap in a
  scratch copy, restore the deleted test, and record both frame numbers
  fresh.
- The owner's qualitative report that the reordered version felt
  *generally* better, not only on the stall repro. That is a live-play
  judgement, is not reducible to a number, and is why §1c states the bar
  as "match or beat" rather than as a single threshold.

**External sources**

- **Mei, Decaudin & Hu (2007), *Fast Hydraulic Erosion Simulation and
  Visualization on GPU*** — **the primary PDF was not accessible this
  session.** The INRIA-hosted copy at `www-evasion.imag.fr` failed TLS
  hostname verification; a University of Nebraska Omaha mirror returned
  403; the HAL record (`hal.science/inria-00402079`) returned an
  access-denied page. Report B §12 states its own author read the INRIA
  PDF directly for the model description; **this document did not.**
- The flux, scaling-factor and water-height equations quoted in §7a/§7b
  were transcribed from a **secondary implementation write-up** — the
  [Interactive Hydraulic Erosion Simulator](https://huw-man.github.io/Interactive-Erosion-Simulator-on-GPU/)
  project page, which reproduces them as
  `f^L_{t+∆t} = max(0, f^L_t + ∆t·A·g·∆h^L/l)`,
  `K = min(1, d₁·l_X·l_Y / ((f^L+f^R+f^T+f^B)·∆t))` and
  `∆V = ∆t·(Σf_in − Σf_out)`, `d₂ = d₁ + ∆V/(l_X·l_Y)`. They are
  consistent with Report B's own summary of the model and with several
  independent reimplementations, but the original text was not read and
  the symbol definitions were not cross-checked against it.
- **The time-step/CFL condition was not obtained.** Search results
  indicate the paper discusses numerical stability and a maximum time
  step; the specific condition was not read, and §7c deliberately does
  not depend on one — the integer + clamp formulation is safe (no
  negative heights, exact mass) irrespective of it, and instability
  degrades to visible sloshing rather than to corruption. **If a future
  pass wants a principled `gain` rather than a tuned one, read the paper
  first.**
- Sources consulted: [Mei, Decaudin & Hu — HAL record](https://hal.science/inria-00402079)
  (inaccessible), [the INRIA PDF](http://www-evasion.imag.fr/Publications/2007/MDH07/FastErosion_PG07.pdf)
  (TLS failure), [Interactive Hydraulic Erosion Simulator](https://huw-man.github.io/Interactive-Erosion-Simulator-on-GPU/)
  (read, and the source of the quoted equations),
  [Jákó, *Fast Hydraulic and Thermal Erosion on the GPU*](https://old.cescg.org/CESCG-2011/papers/TUBudapest-Jako-Balazs.pdf)
  (fetched but not machine-readable).

**This document's own contributions, claimed as such and not attributable
to any source**

The 1D reduction to a single signed flux per interface; the integer
reformulation and the exactness argument that follows from it; the
terminal exact-equilibrium snap; the observation in §7a that the naive
relaxation is still O(W²) and that persistent flux is the mechanism, not
an optimisation; the column-granular seam with absorption as inflow and
edge demotion as outflow; the argument in §4a that quiescence is the
wrong promotion gate; the ownership substrate and the four-method
protocol in §9a; and the observation that B-1's and B-2's scenes are the
same scene. None of these are in Report B, Report C, or the external
literature consulted.

---

## 16. Handoff

**To Report C's implementation (M8):** §9a is the interface. Build
against it rather than inventing a second sleeping/demotion path. Two
things there are yours and are named but not built: the
`Cell::organism_id` → `owner_id` generalisation (with the
`update.rs:132` fix that must accompany it, §9b), and
`LiquidBody::displace_column` so buoyancy does not demote the lake it is
floating in (§9c).

**To Report B's remaining items:** §14 is the list of what this design
deliberately did not take. The two that most change the player's
experience and are now unblocked by nothing here are the VOF local height
function (§3b there) and the hydrostatic path trace (§6 there) — and the
latter is now *scoped*, because §3b condition 3 above draws the line
between "a leveling problem" and "a pressure problem" explicitly rather
than leaving it to be discovered.

**Back to `PLAN.md`:** the item that should change shape is Report B §9's
build order step 4, "1D virtual-pipes prototype for large settled bodies
— prototype the *seam* first, not the pipe physics, which is trivial."
That instruction is right and §11 follows it, but two of its assumptions
do not survive: the pipe physics is trivial *only if the flux is
persistent state* (§7a), and the bodies must not be required to be
*settled* (§4a).
