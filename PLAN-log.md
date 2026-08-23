# Pixel Physics Engine — Progress log

Split out of `PLAN.md` (2026-08-21): 2,127 lines of append-only session
record were the bulk of a 5,400-line file that only 2 of its ~120 inbound
references actually cited, and the roadmap deserved to be readable whole.
Kept so the plan and the actual build stay honest against each other —
updated at each milestone commit, not just when something is added.
**Append new entries here.** Entries are point-in-time records: they
describe the engine as it stood when written, and correcting them would
falsify the history. The current state of anything lives in `README.md`'s
status sections.

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


### Dark bands under overhangs, objects and digs — one wrong question, asked per column

Reported from play as *"dark bands under any overhangs or objects or when I'm
mining"*, with the guess that it was either the frozen background baseline or
a lighting shadow. It was the baseline, and it could not have been a shadow:
`sky::apply_light` takes one scalar for the whole screen, and the fake-AO
experiment was measured at ~10 ms/frame and cut years ago.

`World::sky_surface` asks *"is there anything `Solid` or `Powder` above me in
this **column**, as of frame one"*. That single question cannot tell a cave
roof from a cliff brow, a hillside from a rock standing in the sky at genesis,
or rock you removed from rock that was never there — which is why one bug wore
three costumes. Measured with a new `examples/underground_probe.rs`: 156–408
cells of open air per 2048x640 world drawn as cave, and 1,363 once a 64-wide
pit is dug.

Fixed in four steps, each judged by the owner before the next:

1. **Per-cell genesis map** (`World::freeze_underground_map`) — one bit,
   seeded by a flood fill at generation. `dead-ends.md` §977's *store more
   history, never infer*, so no width threshold anywhere and a dug shaft stays
   a tunnel at any width. Rescues 149/156, 406/408, 192/197 on seeds 1–3.
2. **Sky light** (`SkyLight`, `F12`) — the dig case needed propagation, not a
   better boolean. Seeded only where a cell was outdoors at genesis, then
   spread at Terraria's 0.91 per air cell / 0.56 per solid over four
   directional sweeps. /4 blocks, ~2.3 ms on a frame where something changed
   and **zero on a settled one**.
3. **`World::ground_datum`** — the top of the lowest run of cells the sky
   cannot reach, replacing the skyline as the *shading* datum. A brow set the
   skyline and so shaded its whole column to bedrock: a vertical tone seam,
   2,990 cells on seed 5, a single 494-cell column on seed 7.
4. **The terrain depth grade went off by default**, on a playtest — *"no
   question grade off is better"* — after a blind A/B went the same way. That
   overturns the 2026-08 world review's single most consistent graphics
   finding. Cost was ~0.44 ms of 15.3 and is *not* the reason. It also makes
   step 3 inactive in the default build, which is stated in the report rather
   than left implied.

Prior art in `Reports/prior-art-underground-lighting.md`: Terraria gates sky
light on a per-tile **wall** (mining does not remove it, so a tunnel stays
dark) and then floods light with distance decay; Noita does not classify at
all and blurs a coarse 32x32 fog. The landed per-cell bit is the wall layer's
first bit, and 0.91 reaching a tenth in 24 cells matches `CAVE_FADE_DEPTH`,
set by eye here independently.

#### The method failures, which are the part worth carrying

Six, and every one of them was a case of *the measurement or the picture not
containing what the words said it did*:

- **A metric that became a tautology and read as a triumph.** After the
  `ground_datum` fix the probe reported **0 over-darkened cells on every
  seed** — because it compared the grade's depth against the walk-up depth,
  and the new datum is *defined* as the top of the walk-up run. Same
  arithmetic twice. It reports the size of the *correction* now.
- **A guard that sampled where the ramp is flat and passed with the bug in.**
  Depths plainly wrong (115 against 55), brightnesses 241 against 248 — under
  3%, green. The grade is a smoothstep flat at both ends, so a sixty-row error
  costs ~1.6% near the floor and **~36% ten rows down**. Moved to ten rows
  down it reads 237 against 378.
- **A before/after posted as a contact sheet, on a card asking about a
  vertical seam** — whose tile join is a hard vertical edge at the focused
  region's border. The reply, "there is still a clear seam", was a correct
  reading of what was on screen.
- **A verification crop that missed the artifact entirely.** Aiming at world
  x 335 clamps the camera to 0, so viewport 180..300 is world 180..300 while
  the patch is at 332..337. Both renders looked identical because the changed
  region was in neither, and the fix was nearly reported as doing nothing.
- **A cost prediction contradicted by its own report.** `sky-light-design.md`
  said the block scan "must not be charged to this approach" because the
  engine already has occupancy — then the implementation built it with a
  `World::get` per cell and measured **+7.5 ms on a 13.2 ms redraw**, fifty
  times the prediction. `CHUNK_SIZE` is 64 and blocks never straddle one, so
  one lookup covers a block: +2.3 ms.
- **An accuracy claim measured only at sample points.** The four-sweep
  approximation agreed with an exact solve to three decimals *at the cells
  sampled*, and put a comb of vertical stripes down every pit between them.
  Fixed by running the sweeps twice. A handful of cells cannot see a comb.

Two instruments came out of it and both are reusable: `viewshot aim=N` (one
tile centred on a world x, no contact-sheet joins in it) and `pixel_stat
diff=1` (mean/max luma difference plus a per-column profile — a seam is a step
there and nearly nothing in a whole-image mean).

Also rejected, with numbers rather than argument: a **stored, incrementally
maintained** per-pixel field. It works, including the hard direction — light
*falls* when a cell is filled, handled by a bounded local re-solve, checked
against a full recompute after plugging a lit shaft — and its influence radius
was swept (30 → visibly wrong, 59 → just under 1/255, 90 → comfortable). It
loses on cost anyway: 0.7 ms *per edit site* against a flat 2.3 ms, so it wins
only below ~3 sites a frame, and a busy scene runs 16 of 40 chunks awake.

---

### Four playtest defaults, and the one that turned out not to be a default

Asked for four changes, framed as "maybe outside your scope but a simple
fix": spoil to CLEAN, jump to FLOATY, water to DIVER, chaining to TIGHT.

**Two were already done.** `MOVEMENT_FEELS[0]` has been FLOATY and
`WATER_FEELS[0]` has been DIVER since the playtest that picked them, both
mirrored into `Tuning::default` and guarded by
`the_defaults_are_the_first_feel_of_each_list`. Worth saying out loud
rather than silently doing nothing, because "I asked for it and nothing
changed" and "it was already like that" look identical from the outside.

**Spoil was a one-line default and a wrong comment.** `SPOIL_MODES`
reordered so CLEAN is index 0, `Tuning::default().dig_yield` 0.35 -> 0.0.
The field's doc had argued *against* this value on the grounds that
vanishing rock is the no-debris failure `CLAUDE.md` records — a misreading,
since `dig_yield` is consumed only by `rigid::mine` and no destruction path
consults it. Rewritten to say so. Measured on `scene=tunnel`, 42 identical
bites: CLEAN covers 120 cells of ground with the bore clear; DUST covers
46, wades from bite 19, and buries him in his own spoil for 7 ticks. Two
tests moved: `a_bite_opens_a_bore_and_removes_only_what_it_broke` pinned to
DUST (both its assertions need spoil to exist), and a new
`at_the_default_yield_a_bite_leaves_no_spoil_in_its_bore` covers the end
that had none.

#### Chaining was not a default. It was a gate nobody had switched on

`World::chain_reach` defaulted to `i32::MAX`, and `within_disturbance`
opens with `if self.chain_reach == i32::MAX { return true; }`. So the
disturbance ring had never been read in a shipped run. Setting the default
to `TIGHT` turned that early return off for the first time and the engine
promptly stopped working in three places, because only `rigid::strike`,
`rigid::mine` and `explosion` had ever called `record_disturbance`:

- **The brush.** `World::paint_capsule` erasing a support scheduled a
  structural check and licensed nothing, so undermining with the cursor
  did precisely nothing.
- **Fire.** A burnout removing a trunk's base: check scheduled, failure
  found, failure declined, tree hangs there.
- **Structural phase change.** Lava quenching into crust over open water:
  the crust minted and never came apart.

All three now record. Fire and phase change run inside the sweep with no
`&mut World`, so this needed a `CellSurface::record_disturbance` that
`ChunkView` queues and `run_pass` replays — the same shape
`schedule_active_site` already had.

Two sizing consequences fell out of that. The ring was 16 slots, sized by
a comment reading *"a player cannot disturb dozens of places in the same
second"* — true of a player, false of a fire front, which now writes a
disturbance per burned-out cell. So `record_disturbance` coalesces
spatially, and `MAX_DISTURBANCES` is 64. The merge radius is
`chain_reach / 2` rather than `chain_reach`: a coalesced record keeps the
older point, so merging at the full reach would let the licensed box sit a
full reach off-centre and TIGHT would quietly behave like 32. Keeping the
older point rather than moving it to the newer one is the deliberate
direction of that error — over-licensing shows as a collapse reaching
slightly far, under-licensing shows as "I hit it and nothing happened",
which is the failure the whole leash must not be.

#### What the leash actually buys, measured paired

`scripts/seedsweep.sh dig=6`, 6 presets x 4 seeds, TIGHT against SPREAD on
the same binary:

| | TIGHT | SPREAD |
|---|---|---|
| cells lost, max / p90 | 297 / 177 | 297 / 193 |
| rock destroyed, max / p90 | 40 / 5 | 135 / 8 |

Material removed is essentially unchanged. What moves is **granularity**.
On `rolling` seed 7, the seed where they diverge most: SPREAD fires 221
overload failures of mean region 14.4 cells, 217 of them in the 8-15
bucket; TIGHT fires 41 of mean region 41.6, 27 of them in 16-63. Same
largest region (1096) either way.

**And then the owner looked at it and said "there is nothing happening in
either of these images", which was right, and chasing that is the most
useful thing in this entry.** Measured after the fact:

- final frame, that same seed: **0.3%** of pixels differ;
- mid-collapse (frame 120), cropped to the cut: **0.8%**;
- awake chunks track each other 7-9/40 across 1,300 frames, so there is no
  "it keeps rotting for another thousand frames" signature either;
- `strike=12 seed=24301`: **bit-identical**;
- `dig=6 tunnel=8`, the compounding case: **bit-identical**, same 20
  overload failures and the same 425 cells, both settings.

The arithmetic behind that: the harness prints `furthest a failure landed
from its trigger` at **7-8 cells** on these scenes, and TIGHT's radius is
16. The leash is not binding. It binds on the minority of cases where a
failure would have landed past 16 — `next-session-handoff.md` measured
`max_chain_reach` at 13-22 — and rolling/7 is one of them.

So the claim that survives is "fewer, chunkier failure events where a
failure would otherwise have landed past 16 cells", not "visibly less
rotting", and the honest report to the owner is that the harness cannot
reproduce a visible difference and his playtest is the authority on
whether one exists. This is `CLAUDE.md`'s *ask which pixels a lever moves*
in its purest form: the counters moved 5x, three separate levers all
demonstrably fired, and the silhouette did not move at all. It also means
the substantive work in switching TIGHT on was the three unwired verbs,
not the leash.

Frame cost unchanged: `ascii` mean over 12,000 frames 3.770 ms against a
3.746 ms baseline re-measured in the same session on the same machine.
(The stress-scene worst-frame numbers swung *both* directions between the
two trees — 87 vs 101 ms on one scene, 21 vs 13 on the next — which is
this machine's documented noise, not a signal.)

#### The method failures, which are the part worth carrying

- **A default is not a tuning value when it gates a fast path.** Every
  behaviour downstream of `chain_reach` had been dead code. Reading the
  diff, this looked like four constants; running it, it was a feature
  being switched on for the first time. Before changing a default, check
  whether the code that reads it has an early return for the current one.
- **Two guards were vacuous in opposite directions and only measurement
  told them apart.** `ligament` and `rockdrop` broke outright (0 overload
  failures on the case that exists to show a neck snapping; 600 cells of
  slab left hanging). `capped` was *assumed* to have gone vacuous and had
  not — run at `chain_reach=spread` it still measures 0 failures, so the
  model was holding that column up all along. The comment written before
  that check said it "would pass on the leash rather than on the model";
  it was corrected to what was measured. Assuming a guard went vacuous is
  as wrong as assuming it did not.
- **A scene comment can go false without the scene changing.**
  `ligament` read *"one structural check at the neck, which is all a
  disturbance would do"* — accurate when written, because the ring was
  never consulted, and silently false afterwards. `rockdrop`'s comment had
  even predicted its own failure mode ("the slab hangs there and the
  harness reports zero of everything, which reads exactly like the splash
  being broken") and then hit it through a second door.
- **One disturbance does not cover an object wider than the reach.**
  `rockdrop`'s first fix recorded at the slab's centre and left 231 of 600
  cells hanging, because the slab is 60 wide and a record licenses 32.
  Fixed by recording per column and letting the coalescing do the work —
  which is also the honest statement of what the scene means.
- **Unleashing a test helper is right for the model's own tests and wrong
  for the feature's.** `World::without_chain_limit` went into three
  `test_world()` helpers, and deliberately *not* into
  `burning_a_trees_base_collapses_the_rest_of_the_trunk` or the quench
  crust case — those two now pass only because the fire and phase-change
  fixes are real, which is the whole point of keeping them leashed.

#### And then the merge measured it properly, and TIGHT came back out

`origin/main` had moved 58 commits while this branch ran, and two of them
matter here. One reshaped what a failing region is, so a room's ceiling now
comes down as **one** paced 1,903-cell failure instead of thirty-seven
separate ones. The other made `F9` reach work already in flight
(`relicense_staged_fractures`), which is the fix for "switching to NONE
does nothing". Together with TIGHT as the default, on the acceptance pair
that encodes *"cutting a wall brings the room down"*:

| `chain_reach` | failing cells | roofed void left |
|---|---|---|
| TIGHT (16) | 238 | **100%** |
| LOCAL (48) | 1,975 | 19% |
| SPREAD | 1,975 | 19% |

**At TIGHT the room does not come down at all.** Not a bug:
`licence_radius` is `chain_reach + extent`, a radius-3 chisel's extent is
5, and a 200-wide room's ceiling fails as one region reaching ~100 cells
from the cut, so `clip_region_to_licence` correctly keeps only the part
within reach.

**And the mechanism above is a correction.** This entry, the commit message
and the PR body all first named `relicense_staged_fractures` as what shrank
the collapse. It is not: that runs only from `App::cycle_chain_mode`, so a
harness scene built at a fixed reach never calls it. The clip is what a
scene measures, and the two were conflated because both filter a region by
`within_disturbance` and both are new in the same merge. **Naming a
mechanism is a claim, and it needed the same "did it fire at all" check as
any other** -- one grep for the call site would have settled it before three
documents carried it. Main's
own `wiki/structural-collapse.md` already named that trade as the open
question on the page — *"a long span can lose the part near the blast and
leave the far part standing on nothing"* — and kept SPREAD as the default
because of it.

So the default went back to SPREAD and the choice went to the owner with
the table. Everything else stayed: the three verbs, the coalescing, the
larger ring, CLEAN, TRACE. It is one line in `CHAIN_MODES` when he calls
it, and `LOCAL` is the answer that gives him the containment he asked for
without the cost.

The lesson is the merge, not the measurement: this branch measured TIGHT
against a tree where a room's collapse was thirty-seven separate failures,
each small enough to sit inside the licence. **A default measured against
the wrong base is not measured.** The seed sweep and the acceptance run
that said TIGHT was safe were both honest and both stale by 58 commits.

#### A concurrent branch caught a number whose provenance I had not stated

Relayed from the agent on `load-share-rescue`: they measured **41-47**
failing cells at TIGHT on `scene=room` where this branch reported **238**,
and guessed the cause without seeing the diff -- *"could be their D1 extents
changing what's `within_disturbance`"*. Correct, and reproduced exactly:
`scene=room` builds its walls with `paint_capsule_as`, and D1's brush fix
records a disturbance per structural cell written, so constructing the room
blankets its own walls with licences. Suppressing that one call measures
**41** cells. Their number and mine are the same measurement of two
different trees.

The roofed void is **100% in both**, so the conclusion does not move -- but
"238" was reported as a property of the scene when it is a property of this
branch. **A number carries the tree it was measured on**, and a
cross-branch comparison is where that surfaces, which is an argument for
making the comparison rather than assuming a shared baseline.

They also reported that the load-concentration fix does *not* rescue TIGHT
-- with sharing on, the rule fires hard (21,540 cell-evaluations moved) and
TIGHT still leaves the void unchanged. The two mechanisms are orthogonal:
sharing decides which cells reach the failure criterion, the reach leash
decides how far a failure may travel from a disturbance. So this is not a
load-model artifact a better model clears up.

One thing to carry: their port (`5e6e79b`) moves the SPREAD baseline from
1,975 to 2,733 failing cells. It is on `origin/load-share-rescue`, **not on
`main`**, so every figure here is correct against the base this branch
targets -- and every TIGHT-vs-SPREAD number in this entry will need
re-measuring the day that lands. Checked rather than assumed: `git
merge-base --is-ancestor` says no, and this branch is 0 commits behind
`origin/main`.

#### Main had already written this bug down, and deferred it

`Reports/open-bugs-handoff.md` D1, from the explosion branch: *"The brush
and fire license nothing, so a burnt trunk leaves its crown in the air"*,
with the fix shape *"give the brush and the fire burnout a
`record_disturbance` with an extent"*. Diagnosed, correct, and deferred —
and it could sit there indefinitely precisely because SPREAD's early return
made it unreachable. Building TIGHT is what forced it. The third verb
(structural phase change) is one neither the entry nor its fix shape named.

Also from that merge: main had independently given `Disturbance` an
`extent` field, so the licence scales with the tool's own wound. That is a
better answer than this branch's per-column `record_disturbance` loop for
`rockdrop` — a 60-wide slab is now one record with `extent: 30` rather than
sixty records leaning on the coalescing. Theirs was taken and the loop
deleted.

#### TRACE, and a card posted with placeholders in it

The owner's verdict on the spoil pair was *"most of the options produce
too much dust... if there was a 10% option that would be interesting, but
1/3 is even too much"*. `SPOIL_MODES` stepped 0 -> 0.35 -> 0.55 -> 1.0
with nothing in the gap, so `TRACE` (0.10) went in at index 1, one `F2`
press off the default. Measured on `scene=tunnel`: 90 cells of ground
covered against CLEAN's 120 and DUST's 46, 140 cells of spoil left
underfoot against 1,108, and never buried.

Two process notes from the same round, both mine:

- **The first spoil card cropped 40 rows above the gnome.** He works at
  y≈300 and the crop was y 180-260, so both panes showed identical
  untouched rock — which is exactly what the owner reported. Same class of
  error as the seam card earlier in this branch. `pixel_stat diff=1` was
  built during that earlier failure precisely to catch this, and was not
  reached for until after the second complaint. **Diff the two images
  before posting them**, every time; it is one command.
- **The replacement card went out with `"see counter below"` in two
  `meta` fields** because the TRACE run had not been measured yet. Posted,
  measured, reposted with real numbers and a line saying which card it
  supersedes. A card's `meta` is the half the owner is asked to trust.

`ascii` also panics on an ant moisture-gradient assertion. Verified
identical at the baseline commit in a separate worktree, with identical
counters — pre-existing, alongside
`plant::root_and_shoot_branching_read_different_slots` and acceptance's
`wood` case.

#### §L closed: the sessile colony was a worldgen argmax, not a movement rule (2026-08-23)

The bisect said scene, and the scene said it in one look: the creature
parent's foraging world has two residual stone towers standing inside the
nest patch (x≈42–68), and the merge's world does not. The rock-country
guarantee's fallback (`region.rs`) set its gate to the best country draw
exactly, which admits only the argmax region — on the 512-column scene
world that is a choice between 0.4141 and 0.4691, two samples of a field
whose period is 1700 columns. The towers went, the freed soil columns grew
worldgen trees, and the canopy edge moved from x≈88 to x≈64, inside the
nest patch. Ablations (one build, same seed): towers alone 35 trips,
doorstep food cleared alone 30 trips and 9 deliveries, both 245 trips and
0 deliveries — no single lever restores the parent's 92; the balance of
vertical home terrain plus food at the patch edge is what the bar
measured.

The fix widens the fallback from an argmax to a country: regions within
half a `ROCK_COUNTRY_SCALE` of the best draw's centre belong to it. The
scene reads 100 trips (bar 14, set from 98), nest-visits 3,792 against
the parent's 3,598, mean depth 10.3 against 10.3, and its 2,000-frame
counters are identical to the parent's run. On the gated path (best ≥
0.70) nothing changes; a shipped-size fallback world (1 in 16 seeds) gets
one country-sized band instead of one sub-screen cluster — the cluster
shape is the exact failure `FORMATION_BARREN`'s comment records the owner
rejecting. `known-red-ascii` and its `skip=foraging` exclusion are gone
per that job's own instruction. Not §M: springs place zero in this
scene's world (0 cliff candidates, measured), and §M's water-at-rest reds
stand — though their counts move with the terrain (wetland seed 3: 87
cells now fails first, was terraced 57 / rolling 47), recorded there.
