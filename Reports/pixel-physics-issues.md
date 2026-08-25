# Pixel Physics — issue backlog

Eleven issues. #1-#10 come from a read-only review of `master` @ `4053cbe`; #11 was added later from the architecture work and has a deadline attached (it must land before the save format does). Each block below is a
complete issue: title, suggested labels, body. Paste directly into GitHub.

Line references are against `4053cbe` ("M19 tier 1: per-cell grain and continuous
heat glow").

**Suggested order of work:** #2 → #3 (one shared root cause, largest perf win),
then #5 → #6 → #4 (field grid), then #9 → #8 → #7 (M16 correctness before the
forest gets bigger), with #1 and #10 as housekeeping whenever.

---

## Issue 1

**Title:** Commit `Cargo.lock` — it's gitignored on a binary crate

**Labels:** `chore`, `build`

**Body:**

`.gitignore:3` ignores `Cargo.lock`. This crate produces a binary (`src/main.rs`)
as well as a library, and Cargo's guidance is to commit the lockfile for anything
with a binary target.

Consequences today:

- Builds are not reproducible. A fresh clone resolves whatever is newest within
  each `Cargo.toml` semver range — `pixels 0.17`, `winit 0.30`, `rayon 1.11`,
  `image 0.25.10` all allow drift.
- A future CI run (see #10) re-resolves the graph on every job, so a dependency
  regression shows up as a mysterious failure on an unchanged commit.
- The performance numbers in `README.md` are stated against an unrecorded
  dependency set, so they can't be reproduced or compared later.

**Fix:** drop the line from `.gitignore`, `git add -f Cargo.lock`, commit.

**Acceptance:** `Cargo.lock` is tracked; a fresh clone plus `cargo build` resolves
identical versions.

---

## Issue 2

**Title:** `touch_neighbours` fast-path guard is unreachable — every `set` pays the full neighbour loop

**Labels:** `performance`, `simulation`

**Body:**

`src/sim/world.rs:319` and the mirrored copy at `src/sim/parallel.rs:284`:

```rust
if (MAX_REACH..CHUNK_SIZE - MAX_REACH).contains(&lx) && ly > 0 && ly < CHUNK_SIZE - 1 {
    return;
}
```

`MAX_REACH` is `32` and `CHUNK_SIZE` is `64` (`src/sim/chunk.rs:11,30`), so the
range is `32..32` — empty. `contains` is always `false` and the guard can never
fire.

The guard is not *wrong*. With reach at exactly half a chunk, every column really
is within reach of some neighbour, so there is no interior to skip. But the code
reads as though a fast path exists, and it doesn't.

**Cost.** Every `World::set` — and `move_cell` is two of them — falls through to
the full loop: two `ChunkCoord::containing` calls, then up to 6 iterations
(2 chunks wide × 3 tall) each doing a `HashMap` lookup and a `mark_dirty`. On the
full-screen stress scenario in the README (512×320 fully saturated, ~10⁵ moving
cells) that is on the order of 10⁶ hashed lookups per frame spent entirely on
neighbour bookkeeping. Plausibly a large share of the 23 ms serial worst frame.

**Options:**

1. Leave the semantics and delete the dead guard, with a comment pointing at
   `MAX_REACH == CHUNK_SIZE / 2` as the reason there's nothing to skip. Honest,
   no perf change.
2. Reduce the effective reach so the guard becomes live — see #3, which is the
   same root cause and where the real win is.

Whichever way this lands, both copies must stay in sync;
`ChunkView::queue_touch_neighbours` explicitly documents itself as replicating
`World::touch_neighbours` exactly.

**Acceptance:** the guard either fires for real interior writes, or is removed
with the reasoning recorded. No behavioural change to dirty-rect propagation
either way — the existing chunk-boundary tests should pass untouched.

---

## Issue 3

**Title:** Decouple `SURFACE_SEARCH` from `MAX_REACH` so sweep regions aren't widened by 32 for every material

**Labels:** `performance`, `simulation`

**Body:**

`Chunk::sweep_region` (`src/sim/chunk.rs:242`) widens the dirty rect by
`expanded_xy(MAX_REACH, 1)` — ±32 cells horizontally, unconditionally. A single
falling grain therefore wakes a 65×3 band rather than roughly 3×3.

`MAX_REACH` is 32 only because `const SURFACE_SEARCH: i32 = MAX_REACH`
(`src/sim/update.rs:256`) — how far a *free liquid surface* looks along its row
for somewhere to fall. Actual movement reach is far smaller:

| Rule | Reach | Source |
|---|---|---|
| liquid `dispersion` | 5 (water), 2 (oil) | `assets/materials/*.ron` |
| gas `dispersion` | 3 (smoke) | `assets/materials/smoke.ron` |
| powder `roll_reach_at` | 1–3 | derived from `friction_angle` |
| fire neighbour checks | 1 | `src/sim/fire.rs` |
| `SURFACE_SEARCH` (read only) | **32** | `src/sim/update.rs:256` |

So one read-only lookahead used by exactly one branch of `flow_sideways` is
setting the widening cost for every chunk in the world, including chunks holding
nothing but sand and stone.

**Proposal:** track the reach a chunk actually needs — the max over materials
resident in it, maintained on `set`, or recomputed on `end_sweep` — and widen by
that. Most chunks would drop from ±32 to ±3 or ±5. This also makes #2's guard
live again for the majority of writes.

**Constraint that must be preserved.** `parallel.rs`'s module doc leans on
`MAX_REACH == CHUNK_SIZE / 2` in two places: the cross-chunk write-disjointness
proof, and the interleaved reinsert-then-replay loop in `run_pass`. Both remain
valid if reach *decreases* (a shorter reach can only shrink the footprints the
proof calls disjoint), but the reasoning is written against the equality, so it
needs re-stating as an inequality (`reach <= CHUNK_SIZE / 2`) rather than left
implicit. The `same_group_chunks_are_never_within_reach_of_each_other` test should
be parameterised over the reach rather than hardcoding 32.

**Acceptance:** worst-frame numbers re-measured via `cargo run --release --example
ascii` for the saturated scene, both drivers; README performance section updated;
the parallel-safety proof restated in terms of an upper bound.

---

## Issue 4

**Title:** `field::step` runs over the whole world every frame with no sleeping

**Labels:** `performance`, `simulation`, `blocks:M10`

**Body:**

`field::step` (`src/sim/field.rs:335`) collects **every** resident chunk, allocates
a fresh `HashMap<ChunkCoord, FieldTile>` sized to all of them, and runs five full
passes (`rebuild_blocked`, `step_pressure`, `step_velocity`, `step_diffusion`,
`step_advection`) over the lot. Nothing checks whether anything in a tile changed.

`README.md` states "a quiet field costs almost nothing since nothing in it is
changing." That isn't what the code does — a quiet field costs a whole-world
allocation plus five whole-world passes, identical to a busy one. Either the code
or the claim should change.

At the sandbox's 40 chunks this is affordable. It is the wrong shape for M10
(streaming world), where resident chunks will greatly outnumber chunks near
anything happening, and the per-frame cost becomes proportional to how much world
is loaded rather than to how much is going on in it.

**Direction:** the CA sweep already has the concept this needs. A field tile could
carry a settled flag cleared by `add_pressure_impulse`/`add_heat`/`add_light`, by
`rebuild_blocked` noticing occupancy changed, or by a neighbour tile being awake
(so a shockwave still propagates outward). Reusing the `FieldTile` allocations
across frames — double-buffer rather than rebuild — is worth doing regardless.

Related: #5 and #6 reduce the constant factor of each pass; this issue is about
not running them at all.

**Acceptance:** a world with a quiet field reports a measurably lower per-frame
field cost than the same world with an active blast; no regression in the existing
`field.rs` wall/diffusion tests.

---

## Issue 5

**Title:** `rebuild_blocked` does ~164k hashed `World::get` calls per frame

**Labels:** `performance`, `simulation`

**Body:**

`rebuild_blocked` (`src/sim/field.rs:362`) determines occupancy per field cell by
scanning its full `FIELD_SCALE`×`FIELD_SCALE` block of CA cells through
`World::get` — each call a bounds check plus a `HashMap<ChunkCoord, Chunk>` lookup
plus local indexing.

The `break 'scan` exits early on the first solid cell, so walls are cheap. Open
air is the worst case and it's the common case: an empty field cell scans all 64
positions. For the 512×320 sandbox — 40 chunks × 64 field cells × 64 CA cells —
that's roughly **164,000 hashed lookups per frame**, every frame, and it runs
before any of the four physics passes.

This is almost certainly the bulk of the ~5 ms the field step adds to the worst
frame (README: ~23 ms CA-only vs ~28 ms with the field).

**Fix:** the loop already knows `coord`. Fetch `world.chunk(coord)` once per tile
and index into it directly, or better, ask the chunk to answer the whole 8×8
occupancy question itself. That's 1 lookup per tile instead of 4,096 — a ~4000×
reduction in lookups for the same work.

Further win available: a chunk could maintain a running count of resident
`Solid`/`Plant` cells and short-circuit `rebuild_blocked` entirely for tiles in a
chunk with zero of either.

**Acceptance:** field-step cost re-measured on the saturated ascii scene; occupancy
results bit-identical to the current implementation (the existing
`step_pressure`-reads-current-occupancy and `step_diffusion`-respects-walls
regression tests are the guard).

---

## Issue 6

**Title:** Hoist loop-invariant `next.get(&coord)` / `get_mut(&coord)` out of the field inner loops

**Labels:** `performance`, `good first issue`

**Body:**

Every field pass looks up its own tile in the `next` map from inside the innermost
`lx` loop, though `coord` is invariant across both `ly` and `lx`:

| Location | Call |
|---|---|
| `field.rs:389` | `next.get_mut(&coord).unwrap().set_blocked_local(..)` |
| `field.rs:414` | `next.get(&coord).unwrap().is_blocked_local(..)` |
| `field.rs:427` | `let tile = next.get_mut(&coord).unwrap();` |
| `field.rs:455` | `let tile = next.get_mut(&coord).unwrap();` |
| `field.rs:541` | `next.get(&coord).unwrap().is_blocked_local(..)` |
| `field.rs:572` | `let tile = next.get_mut(&coord).unwrap();` |
| `field.rs:616` | `next.get_mut(&coord).unwrap().set_local(..)` |

`step_pressure` and `step_diffusion` pay this **twice** per field cell (the blocked
check, then the write). Across five passes that's roughly 8–10 hash lookups per
field cell per frame for a pointer that never changes within a tile.

**Fix:** take `let tile = next.get_mut(&coord).unwrap();` once above the `ly` loop
in each pass. Where a pass needs to read `next` for the blocked map and also write
to the same tile, one `&mut` covers both.

Note the borrow shape: the passes read `old` (via `world.fields_ref()`) and write
`next`, which are distinct maps, so hoisting doesn't create an aliasing problem.
`step_advection` takes no `world` at all and is the easiest one to do first.

Smaller than #5 but nearly free, and it makes the passes read better.

**Acceptance:** no behavioural change; existing `field.rs` tests pass unmodified.

---

## Issue 7

**Title:** `scheduler::step` is O(all pending sites) and reallocates the whole schedule every frame

**Labels:** `performance`, `simulation`, `M16`

**Body:**

`scheduler::step` (`src/sim/scheduler.rs:71`) takes the entire active-site map,
drains it, tests `site.next_frame > due` for **every** site, and re-pushes the
not-yet-due ones into a freshly allocated `HashMap` — including a fresh `Vec` per
chunk.

The module doc says the pass checks "only sites that are actually due this frame."
It checks all of them, and rebuilds the whole structure whether or not anything
was due. The headline claim in `README.md` — cost proportional to how much is
growing rather than to world size — does still hold (it's O(growing things), not
O(world)), but the constant is a full map rebuild per frame rather than a lookup.

At today's scale — a handful of tips from the `T`/`M` debug keys — this is
invisible. It's the wrong shape for M16's own verify criterion ("a forest burns
and regrows"), where a mature forest is thousands of simultaneously-scheduled tips
with `TREE_TICK_INTERVAL` spacing, meaning the overwhelming majority are not due on
any given frame and get walked and re-hashed anyway.

**Direction:** a `BinaryHeap<Reverse<(next_frame, ActiveSite)>>`, or a small ring of
per-frame buckets given that intervals are bounded and known. Either gives
O(due · log n) with no per-frame reallocation. The chunk keying exists only so
growth writes near a boundary are cheap to find; check whether anything actually
depends on it before preserving it through the change.

**Acceptance:** a benchmark with ~1000 scheduled sites at a 30-frame interval shows
per-frame cost tracking the number *due*, not the number pending;
`a_site_scheduled_for_the_future_is_kept_untouched_until_due` still passes; the
module doc matches what the code does.

---

## Issue 8

**Title:** `World::trees` never shrinks — dead trees leak for the process lifetime

**Labels:** `bug`, `simulation`, `M16`

**Body:**

`trees: Vec<TreeState>` (`src/sim/world.rs:45`) only ever grows; `push_tree`
(`world.rs:121`) appends and returns the index as a stable id. Every tree ever
planted retains its `TreeState` — attractor list, all tips, all roots — forever,
including trees whose every tip and root has gone `alive: false` and whose wood
has since burned to ash.

The field comment documents this ("Never shrinks"), and the id-stability guarantee
it buys is real: `ActiveKind::TreeTip { tree, tip }` and `RootTip { tree, root }`
index into it directly, so naive removal would invalidate live sites.

Currently bounded by how many times someone presses `T`. It stops being bounded as
soon as the M16 "regrows" half lands (tracked separately) and trees start seeding
themselves — at which point a long-running world accumulates state for every tree
that ever existed.

**Direction:** generational indices (`{ index, generation }`) with a free list, so a
slot can be reused while a stale `ActiveSite` referring to the old generation
resolves to `None` and drops itself. Preserves exactly the property the current
comment relies on. Cheaper interim step: a tree with no living tips and no living
roots can at minimum drop its `attractors` vector, which is the largest part of
`TreeState`, even if the entry itself stays.

**Acceptance:** a test that plants a tree, exhausts it, and asserts the reclaimed
storage; no stale-index panic when a dormant site comes due after reclamation.

---

## Issue 9

**Title:** Tree and root tips don't validate their own cell still exists — a burned tree keeps growing

**Labels:** `bug`, `simulation`, `M16`

**Body:**

`moss_tick` (`src/sim/plant.rs:55`) opens with the right check:

```rust
// The tip cell may have burned, been erased, or already be something
// else by the time its schedule comes due — nothing to grow from.
if world.get(x, y).material != moss_id {
    return Vec::new();
}
```

`tree_tip_tick` (`plant.rs:304`) and `root_tip_tick` (`plant.rs:436`) check only
`tips[tip_id].alive` / `roots[root_id].alive`. Neither reads the world at its own
position. `alive` is set false by the plant's own logic — channel decayed past
`MIN_CHANNEL`, starvation ticks exceeded — never by anything happening *to* the
plant.

**Consequence:** burn a tree and every tip keeps its schedule. Each tick it still
accrues `AMBIENT_GROWTH_ENERGY`, still consults its attractors, and still calls
`world.set(wx, wy, wood)` — so a tree whose trunk is now ash keeps extending
branches from a position in open air, disconnected from anything. The same applies
to a trunk erased with the right-mouse brush. (Note the brush guard at
`world.rs` protects `Solid | Plant` from being *painted over*, but erasing with
`material::EMPTY` deliberately bypasses that check.)

This sits directly on the path to M16's stated verify criterion, "a forest burns
and regrows" — the burn half currently leaves live orphan tips behind.

**Fix:** mirror the moss check. On tick, confirm the cell at the site's position is
still this tree's wood; if not, mark the tip `alive = false` and drop the site. The
tip's float `pos` and the site's integer `(x, y)` are the last written cell, so the
check is a direct comparison.

Worth deciding at the same time: should losing a *tip* kill the tree, or only that
branch? Losing the base of the trunk arguably should propagate, but that needs
connectivity information the current `TreeState` doesn't carry — probably M17's
anchor-distance `aux` field rather than this issue.

**Acceptance:** a regression test that grows a tree, replaces its tip cell with
`EMPTY` (or ignites it), runs the scheduler past `TREE_TICK_INTERVAL`, and asserts
no new wood appears and the site is gone.

---

## Issue 10

**Title:** Repository housekeeping: default branch, LICENSE, CI, lint config

**Labels:** `chore`

**Body:**

Several small things, grouped since none warrants its own issue.

- **Default branch is `main`, which holds only a 15-byte stub README.** The entire
  project is on `master`. Anyone landing on the repo page sees an empty
  repository. Switch the default to `master`, or merge and delete.
- **No LICENSE.** Without one the code is all-rights-reserved by default, which
  matters given the stated intent that games be built on top of this crate as
  separate binaries against the same library.

  > **Superseded 2026-08-24, and the reversal matters more than the original
  > item.** This was closed with MIT on 2026-08-21 and MIT has since been
  > reversed to proprietary: the owner intends to sell the game, and MIT
  > explicitly grants everyone the right to sell the software. The premise
  > above — separate game binaries linking the engine as a library — no longer
  > holds either; engine and game ship as one product. All-rights-reserved,
  > called a defect here, is now the deliberate position. See
  > [dependency-license-audit.md](dependency-license-audit.md) before acting on
  > this bullet.
- **No CI.** `examples/ascii.rs` is written specifically to need no window or GPU
  so it "works over a remote shell and in CI" — and there's no workflow. A minimal
  one running `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and
  `cargo run --release --example ascii` would cover the 151 tests plus the visual
  scenes. Depends on #1 for a stable dependency graph.
- **No `rustfmt.toml` or clippy config**, and no `[lints]` table in `Cargo.toml`.
  Some lines run past 120 columns (the `reschedule(...)` calls in `plant.rs` are the
  worst offenders); pick a width and enforce it rather than leaving it to whoever
  is editing.
- **No `rust-version` in `Cargo.toml`.** `u64::is_multiple_of` (used in
  `update::step` and `parallel::step`) stabilised in 1.87, so the real MSRV is at
  least that. Worth stating.

**Acceptance:** default branch shows the project; LICENSE present; CI green on
`master`; `cargo fmt --check` and `cargo clippy -D warnings` both clean.

---

## Issue 11

**Title:** Reserve a slice-identifier field on `ChunkCoord` before it reaches the save format

**Labels:** `chore`, `architecture`, `blocks:M10`

**Body:**

`ChunkCoord` is currently `(x: i32, y: i32)` (`src/sim/chunk.rs`). It is constructed in **42 places**, used as the `HashMap` key for both `World::chunks` and `World::fields`, and it will appear in the save format once M10 lands.

`worldgen-design.md` §0 settles that the play world is a 2D vertical slice through a 3D coarse map, and leaves open whether the engine ever supports **more than one slice** — either straight cuts at different `z`, or curved routes following a drainage network. Both need a way to say *which slice a chunk belongs to*.

**Cost of adding it now:** one extra field, defaulted to zero, plus mechanical updates at the construction sites.
**Cost of adding it later:** the same 42 sites, plus a save-format break for every existing world.

**Make it a generic slice identifier, not specifically `z`.** Straight slices want a `z` coordinate; routes want a route id. A single `u32` covers either without committing to which — and committing to `z` specifically would foreclose the route option that §0 currently prefers.

**Acceptance:** `ChunkCoord` carries a third field, always zero, with a doc comment explaining what it is reserved for. No behavioural change; all existing tests pass unmodified.

---

## Issue 12

**Title:** Grass does not spread — a sown patch is the patch you keep

**Labels:** `feature`, `worldgen`, `plants`

**Body:**

Filed from an owner verdict on review card `20260824T030939054Z-088ae6`
("How long grass takes to fill an area: it doesn't"). His words:

> "patches of grass should spread over time and completely fill up an area
> without trees and the correct environment (temp, light, etc.). This is a
> fine density at the start of a game, but how long does it take to fill an
> ideal area" — and then, once measured: "Grass should spread."

**The measured answer to his question is that it never fills.** Worldgen sows
grass at a chosen density and that density is the end state; there is no
vegetative spread, so a patch neither colonises bare ground next to it nor
closes as the world runs. What ships today is an initial condition wearing the
appearance of a standing crop.

**Measured, by W3, on a treeless control** — 2,048-column worlds, ~500
plantable columns, `flora_census seeds=2 w=2047 h=639 treedensity=0
mossdensity=0 frames=45000`:

| seed | plants @5,000 frames | @45,000 |
|---|---|---|
| 1 | 63 | 76 |
| 2 | 61 | 63 |

Standing cells move under 10%. Grass reaches its sown footprint inside 5,000
frames and holds it. Full cover of ~500 columns needs ~250 plants, so at the
observed rate that is on the order of **700,000 frames** — which is the honest
answer to "how long does it take to fill an ideal area".

**The leading explanation, with two independent supports:** `plant::set_seed`
places a seed into an empty **8-neighbour of the parent cell**, so offspring
land inside or against the clump that made them and grass cannot cross a gap —
the sown positions are very nearly the final ones. The code says one cell, and
the measurement says the footprint does not grow.

**This is a candidate cause, not an established one, and the distinction is
load-bearing.** Three other mechanisms could each also cap the stand:
`crowding_weight: 30.0`, the seed bank's 18,000-frame half-life, and soil
moisture on marginal ground. Fixing dispersal against the wrong one of these
buys nothing.

**The run that settles it** — not yet built — is *one founder on uniformly
ideal ground, scored on how far its descendants get by 45,000 frames*. That is
a scene rather than a knob: `PlantScene` already takes `soil=` and
`soil_moisture`, so it is a small addition to `examples/plant_probe.rs` or a
sibling, not new machinery. **Do that before choosing a fix.**

**The mechanism that would change it is review item A5 (dispersal)** —
per-species seed mass, float and carry. This issue gives A5 a named consumer
and a measured motivation rather than leaving it a speculative nicety.

**Why this is a feature and not a density tune.** Raising the sown density was
tried first and is what the earlier cards were about; the owner accepted it as
a starting value and then asked the question that density cannot answer. A
denser sowing still does not spread, so every future complaint of the form
"this area should have filled in by now" survives it.

**What it has to respect** — the reason this is not a two-line change:

- **Environment must gate it.** The owner named temperature and light
  explicitly, and "an area without trees" — so shade from a canopy has to
  suppress it, which means reading the same light channel plants already use.
  That channel oscillates 20:1 over the day/night cycle by design, so any
  threshold must go through `field::noon_equivalent_light` rather than
  sampling raw light at an arbitrary phase (see `CLAUDE.md`, *a channel that
  oscillates by design must be divided out of decisions*).
- **It must not become a per-cell sweep cost.** Spread is a low-rate process
  over a large area, which is the shape that quietly keeps chunks awake and
  costs the dirty-rect render skip. Guard it at a call site that already holds
  the cell, per `CLAUDE.md`'s hot-path rule, and say what it costs in
  `examples/ascii` worst-frame terms when proposing it.
- **It interacts with fire.** Grassfire landed in W2/W3; a spreading grass and
  a spreading fire over the same substrate is a feedback loop, and the
  patchwork burn the owner approved ("the in-between, found") is the state
  that regrowth would erase. Whatever rate is chosen has to be judged
  *after* a burn, not only on virgin ground.

**Acceptance is judge-by-eye, not a count.** A bare, well-lit, tree-free area
adjacent to an existing patch closes over a plausible number of frames, and a
shaded or hostile one does not. Post a before/after or a GIF; the count of
newly-grown grass cells goes in the card's `meta`, because "did it fire at
all" needs a counter and a still cannot show it.

**Explicitly not urgent.** The owner's routing was: "Tell the integrator to
add it to the to do list, but it doesn't need to hold up you finishing your
current work."

---

## Appendix — a note for the milestone docs, not an issue

Two of the findings above (#4, #7) are cases where prose in `README.md` or a module
doc claims a property the code doesn't have. That's the specific exposure a
codebase this heavily documented carries: the docs are good enough to be trusted,
which makes drift more expensive than it would be somewhere nobody reads the
comments.

Worth noting which invariants held up. Everything with a *named regression test*
behind it — `same_group_chunks_are_never_within_reach_of_each_other`,
`a_connected_mass_of_cooling_cells_actually_settles`,
`a_settled_world_with_a_growing_tree_still_sleeps_between_growth_ticks` — is
accurate. The claims that live only in prose are the ones that slipped. A
reasonable standing rule for `PLAN.md`'s "standing invariants" section: a
performance or cost claim in a doc needs either a test or a measurement command
next to it, or it gets written as an intention rather than a fact.
