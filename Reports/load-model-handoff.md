# Load/torque failure — implementation handoff

**Status: SUPERSEDED BY LANDING — do not execute this document.** The step
it hands off shipped in `7e13e42`; `src/sim/load.rs` is the result. Read
`Reports/load-model-fit-review.md` instead, which reviews what actually
landed. Parts of this are now on the do-not-retry register: its §3 asks for
the support-parent side table on `World` that `load.rs`'s module doc records
as rejected. Kept for the rationale written before the work.

*(This header read "not started" until 2026-08-26, long after the step
landed. The report index had it right throughout; the header is what an
agent opening the file directly sees first.)*

What it was: the single remaining step of
`Reports/fracture-mechanics-design.md`, and the one that document said must
not be rushed, because it changes the failure criterion for every material
at once.

**Read first, in order:** `CLAUDE.md` (method and conventions),
`Reports/design-philosophy.md` §0a (satisfying is the requirement, and this
subsystem is its worked example), then
`Reports/fracture-mechanics-design.md` (the model and why the current one is
wrong). This document is the *how*, plus everything learned building the
four models that came before and is written nowhere else.

---

## 1. Where things actually are

`master`, green, 386 tests, clippy clean. Everything below is built and
working unless marked otherwise.

| Piece | Where | State |
|---|---|---|
| Distance-to-anchor relaxation | `structural::tick` | Live, reactive |
| Whole-world distance pass | `structural::compute_world_distances` | Live, runs in `build_terrain`; 9.1 ms |
| Background attachment | `Cell::attached`, `FLAG_ATTACHED` | Live |
| Attachment lost on damage | `structural::detach_exposed_neighbours` | Live; wired to eraser + explosions |
| Crack edges | `Cell::crack_right`/`crack_down` | Live; block support in both relaxations |
| Crack seeding | `rigid::score_cracks`, from `rigid::strike` | Live, `C` key |
| Cracks reduce span | `structural::weakened_by_cracks` | Live |
| Section depth raises span | `structural::depth_factor` | Live |
| Failing-region flood | `rigid::label_failing_region` | Live — **to be deleted**, see §4 |
| Fracture into size distribution | `rigid::fracture` | Live |
| Chunk bodies (translate + tumble) | `rigid::ChunkBody`, `step_chunk_bodies` | Live |

**The defect this step fixes.** Failure is evaluated per cell as *its own
reach* against *its own span*. A crack at a beam's root weakens a cell whose
distance is ~10, which would never have failed at any span; the far end that
*is* near its limit is not the part that was hit. So a worked root never
gives way, and an overhang hanging by a two-cell ligament does not notice.
Reproduction: `cargo run --release --example filmstrip -- scene=worked` —
six blows at the join of a 160-cell shelf, still standing.

---

## 2. The model, concretely

Fail when **torque > capacity**. Both sides are per cell.

### 2a. Torque, from two additive scalars

Bending moment about cell `c` from everything it supports:

```
torque(c) = | Sx(c) − x_c · M(c) |
```

where over the set of cells `c` supports:

```
M(c)  = Σ mass_i             (u32)
Sx(c) = Σ mass_i · x_i       (i64 — see pitfall 5.6)
```

Both are plain sums, so they accumulate up the support tree. `mass_i` is 1
per cell, or the material's `density` if you want heavy rock to load more —
start with 1 and only add density once the rest is calibrated.

Why torque and not mass: fifty cells stacked against a wall is fine, the
same fifty reaching fifty cells out is not. Mass alone cannot tell those
apart and will let any thick tower collapse or any long shelf stand,
depending on how you tune it.

### 2b. Capacity

```
capacity(c) = base_strength
            × depth_factor(c)          (already exists)
            × attached_span_bonus      (if attached; already exists)
            × uncracked_fraction(c)    (already exists as weakened_by_cracks)
```

Every term is already written. `structural::weakened_by_cracks` and
`structural::depth_factor` can be reused unchanged — only what they
multiply changes, from *span* to *capacity*.

**Calibration.** Root torque of a beam length `L`, depth `D`, unit mass is
about `D·L²/2`. Pick `base_strength` so a **1-cell-deep** beam still fails
near its current reach of ~16 cells, then tune by eye. Do not tune by test:
this is a feel quantity and `CLAUDE.md`'s method section is emphatic that
metrics written before looking measure the wrong thing.

### 2c. Where the tree comes from

Every cell's support parent is the neighbour its distance relaxation took
its minimum from. That makes the support graph a **forest rooted at
anchors**, and `M`/`Sx` accumulate up it.

**Record the parent explicitly.** Do not re-derive the subtree by flooding
"neighbours with greater distance" — that is what `label_failing_region`
does and it is wrong for *accumulation* because equal-distance siblings get
counted twice (see pitfall 5.1).

---

## 3. Storage — do not add fields to `Cell`

`Cell::flags` is **full**: all 8 bits used (`MOVED`, `BURNING`, `FLOWING`,
`MANAGED`, `UNDERCUT`, `ATTACHED`, `CRACK_RIGHT`, `CRACK_DOWN`). There is no
room for a parent direction there.

`Cell` is currently `material, shade, flags, temperature(i16),
burn_timer(u16), aux(u16), organism_id(u16)`. It has already grown once
(32 MB → 48 MB, recorded in `README.md`) and adding `M`, `Sx` and a parent
per cell would grow it again — for data that is meaningless in the vast
majority of cells.

**Use a sparse side table on `World`**, keyed by position, holding
`{ parent: Dir, mass: u32, moment: i64 }`. Populate it only for
**structurally interesting** cells:

> unattached, **or** adjacent to a crack, **or** adjacent to an empty cell.

Everything else is bulk rock that cannot fail. This keeps cost proportional
to **surface area**, which is the same argument that makes the existing
distance pass affordable.

**It must not be a `HashMap` you iterate.** Iteration order is randomized
per process and determinism is required (`PLAN.md`). Use a `BTreeMap`, or a
`HashMap` for lookup plus a separately-sorted `Vec` for any ordered walk.
This is issue #7's trap and this codebase has hit it twice already.

---

## 4. What to delete in the same change

Leaving any of these alongside the new model produces double-firing or
silently dead code. Both failure modes have already happened here.

- **`rigid::label_failing_region` and the "whole appendage" flood.** It
  exists *only* because the root never fails on its own. Once it does, the
  detached region is ordinary connectivity — a consequence, not a mechanism.
  Leave it in and one failure detaches twice.
- **`support_cost_below: 0`** in `stone.ron`. Free compression existed only
  to stop towers snapping under the reach model; under load a tower stands
  because it *carries little*, which is the real reason. Removing it also
  kills the zero-cost tie degeneracy (pitfall 5.1) and the
  self-consistent-zero fixed point (§6, model 3).
- **`max_unsupported_span` as a failure criterion.** Either delete it or
  reinterpret it explicitly as the capacity base. **Do not leave it
  half-live** — see pitfall 5.7.

---

## 5. Pitfalls

Each is a specific way this goes wrong. Several already have.

**5.1 — Equal distances double-count.** With any zero-cost step, whole
regions share a distance, "decreasing distance order" no longer orders
parent before child, and the parent relation can *cycle* — the accumulation
never terminates. Even without zero costs, two equal-cost paths make
"flood to greater-distance neighbours" count a subtree twice. Fixed by
recording parents explicitly (§2c) **and** deleting the zero cost (§4).
Note the existing `label_failing_region` deliberately uses `>=` for a
*contour*; that is right for finding a detached piece and wrong for
accumulation. Do not copy it.

**5.2 — The parent forest goes stale mid-convergence.** The distance
relaxation is label-correcting and converges over several ticks. A walk over
a half-converged forest can follow a parent that is no longer valid. Guard:
walk only while distance **strictly decreases**, and cap walk length. Both
cheap, neither optional.

**5.3 — Load is non-local, unlike everything else here.** Changing one cell
at a beam's tip changes load for every cell between it and the anchor.
Walking up the parent chain is `O(depth)` and fine. *Removing* a cell can
re-parent an entire subtree, which is not. **Do not maintain load
continuously.** Recompute lazily, on the path from a disturbed cell to its
anchor, when a structural check fires.

**5.4 — Cascades want to resolve in one frame.** Each break changes loads,
triggering more breaks, same frame. That is both a frame spike and worse
looking than progressive collapse. Needs the existing
`MAX_SITES_PER_FRAME` (2000, `scheduler.rs`) **plus** a per-frame cap on
fractures. Pacing is a feature here — `STRUCTURAL_TICK_INTERVAL` (5 frames)
exists for exactly this reason and a collapse that resolves over a second
reads better than one that resolves instantly.

**5.5 — Determinism.** Parent choice must tie-break deterministically;
`NEIGHBOURS_4` order already gives that if you take the first strict
minimum. Any ordered walk over the side table must be sorted (§3).

**5.6 — `Sx` overflows `i32`.** A 4000-cell region at x≈500 gives
`Σ m·x ≈ 2×10⁶` — fine — but a streamed world's coordinates are unbounded
and the square in the torque calibration is not. Use `i64` and
`saturating_*`. `aux` is `u16` and saturates at `u16::MAX`, which the
count-to-infinity behaviour relies on; do not change that.

**5.7 — Superseded mechanisms pass vacuous tests.** When
`confinement_radius` was superseded, its tests kept passing while testing
nothing, because an undisturbed slab sits at a self-consistent distance and
stops rescheduling. They were deleted, not ported, and the reason is
recorded in `structural.rs`'s test module. `max_unsupported_span` is about
to be superseded the same way. **After the switch, deliberately break the
new criterion and confirm the old tests fail.** If they still pass, they are
testing nothing.

**5.8 — "Bigger therefore skip" is backwards, and has been written twice.**
Both `rigid::strike` and `rigid::try_promote_failing_region` gated fracture
on `MAX_BODY_CELLS`, so a *large* failure declined and fell through to
per-cell conversion — the bigger the collapse, the more certain it dissolved
to dust. Both are fixed; do not reintroduce the shape. A size cap belongs on
a **fragment**, never on whether a region breaks at all.

**5.9 — Attached bulk must early-out before any computation.** *Attached,
no cracked edge, no empty neighbour → cannot fail, skip.* Two tests,
covering nearly the whole world. Without it this pass is O(volume) per
event.

---

## 6. Four support models have been tried. Do not retry them.

This is the most important section for a fresh session, because each of
these sounded right and each passed its own tests.

1. **Confinement as an anchor.** A cell buried deep enough counted as
   anchored outright. Made terrain stable — and made *thickness into
   immunity*, so nothing at or past the threshold could fail anywhere,
   mid-air included. Removed; its tests deleted as vacuous.
2. **Thickness scaling the span, inferred from burial.** Same failure from
   the other side: per-cell burial gives every free face a bare span, so
   thick rock eroded one skin at a time from every surface; taking the max
   over the neighbourhood fixed that and made structures too strong. Parked
   on branch `structural-thickness-wip`, never merged.
3. **Attachment as an anchor.** Correct that support cannot be inferred from
   *shape* — it is a property of what the material *is* — but anchoring on
   it made attached rock immune, so an undercut shelf could never fall
   however much was dug from beneath it. Now attachment buys *reach*
   (`attached_span_bonus`), not immunity.
4. **Reach with all of the above** — where it stands today. Cannot fail a
   root, which is what §1 describes.

**The through-line:** every model that inferred support from geometry was
either strong enough to hold a mountain *or* weak enough to let built
structures break, never both, because geometry cannot tell a mountain from a
stacked wall. Attachment fixed *that*. Load fixes the remaining half —
*where* in a structure the failure happens.

Note also that model 3's `support_cost_below: 0` created a
**self-consistent zero**: a floating blob where every cell claims support
from the one below is stable at distance 0 and never even schedules a
recheck. Instrumented and confirmed. §4 deletes the cause.

---

## 7. Verification

**Look before measuring, and again after** (`CLAUDE.md`). Scenes exist:

```
cargo run --release --example filmstrip -- scene=worked  start=2 every=50 count=6 crop=40,120,220,170 zoom=3
cargo run --release --example filmstrip -- scene=capped  start=2 every=90 count=4 crop=150,70,220,190 zoom=3
cargo run --release --example filmstrip -- scene=undercut start=1 every=45 count=6 crop=0,120,240,190 zoom=2
cargo run --release --example filmstrip -- scene=strike  start=2 every=1 count=1 crop=200,90,120,120 zoom=6
cargo run --release --example ascii
```

**The acceptance cases, in order of importance:**

1. `scene=worked` — the shelf **must now come down** after those six blows.
   This is the whole point of the change and currently fails.
2. `scene=capped` — the thick column **must still stand**. This is the
   regression the change most easily causes.
3. `scene=undercut` — an undercut shelf still spalls.
4. A big overhang on a deliberately thin ligament snaps at the neck. **No
   scene exists; write one.** It is the owner's original case.
5. `ascii` M17 bridge still collapses when cut.

**Read `filmstrip`'s printed `bodies N (M cells)` line, not just the
image.** A coherent falling slab and a tight scatter of grains are not
distinguishable in a contact sheet — this exact confusion made a `mine`
scene read as "chunks are working" while the count said `bodies 0`
throughout and the feature had never once fired.

**Timings:** `examples/ascii.rs` reports worst-frame and CI runs it. Take a
baseline **before** starting — this machine has been too contended for
several sessions for the numbers to mean anything (two identical runs gave
10.7 ms and 100.7 ms on the same scene).

**Known perf regression to fix while here:** `render.rs` forces a
**full-screen redraw** whenever any chunk body exists, the way it does for
particles. Fracture now produces many fragments, so a collapse defeats the
dirty-rect skip for its whole duration. Should dirty only each body's
bounding box. Measure on a *settled* world containing landed debris — a
settled world is where that skip earns its keep.

---

## 8. Repo gotchas that cost time this session

- **Verify code matches its commit message.** A `git stash` cycle restored
  an older blob over `rigid.rs`, so a commit claiming the overhang predicate
  changed only the doc comment while the code kept the old behaviour. It
  went unnoticed for four commits. After any stash/rebase, re-read the
  function, not the diff.
- **Never `git add -A`** (`CLAUDE.md`): this tree is worked concurrently and
  doing so once swept ~1,200 lines of someone else's work into an unrelated
  commit. Scope it: `git add -A -- src examples`. Other worktrees exist
  (`plant-substrate-v2` and two agent worktrees) — check `git worktree list`.
- **The app locks its exe.** If the sandbox is running, `cargo build` fails
  with "failed to remove pixel-physics.exe". `cargo test` still works.
- **Stale incremental artifacts cause bogus linker errors** (`LNK2019
  unresolved external symbol anon.…`). `rm -rf target/debug/incremental`
  fixes it; it is not a code error.
- **`cargo fmt` is all-or-nothing** and reformats ~28 files. Deferred work
  (`PLAN.md` issue #10). Do not let it ride along.
- **`App::select_material` is 1-based** (number-key driven). A 0-based index
  silently leaves the brush on sand.
- **Painting rolls a per-cell density**, so asserting on the brush's exact
  centre cell is a coin flip. Sample the brush area.
- **Two drivers**: `update::step` (serial) and `parallel::step` (what the
  app runs). Test both. `update::step_monolithic` is the control for "is
  this the rules or the chunk decomposition?"
- **Chunk bodies must be stepped outside the CA sweep.** A body spanning two
  same-parity chunks would write to both and break `parallel.rs`'s
  write-disjointness proof (`Reports/coupling-research.md` §4). They run in
  their own serial phase beside `step_liquid_bodies`.
- **Structural tests' `run()` does not step the CA sweep.** Rubble produced
  mid-collapse then never falls away — and since powder underneath counts as
  ground (`is_resting_on_ground`), it props up whatever is above it and the
  collapse stalls. Add `update::step` to the loop when a test involves
  debris.
- **`Cell::is_empty()` is managed-aware.** Use `cell.material ==
  material::EMPTY` when the question is "is there material here".
- **`liquid_fill`: `aux == 0` on a `Liquid` means full, not empty.**

---

## 9. Suggested order within the change

1. Baseline timings on a quiet machine; capture `scene=worked` and
   `scene=capped` as they are now.
2. Side table + parent recording, with the §5.9 early-out. No behaviour
   change yet; assert the forest is acyclic and rooted at anchors.
3. `M`/`Sx` accumulation and `torque()`. Still no behaviour change — add a
   debug readout (the hover inspector, `I`, is the natural place) and
   **look at whether the numbers are sane on a beam** before wiring failure
   to them. `CLAUDE.md`: sanity-check a new metric against a case you know
   is fine.
4. Switch the criterion to `torque > capacity`, delete §4's list in the
   same commit, fix the fallout, and check §5.7.
5. Re-verify §7's five cases by eye, then re-measure.

Steps 2 and 3 are safe and independently committable. Step 4 is the one that
changes everything at once.
