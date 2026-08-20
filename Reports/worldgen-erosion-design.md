# Plan-space erosion: formations as side effects of simulated history

Design record for the 2026-08 world review's roadmap step "plan-space
process erosion" (`Reports/world-review-2026-08.md` §5.2) — the owner's
preferred *method* for rock formations: like plants and creatures, the
landscape should get its shapes from a process, not from authored
geometry. This document is written to be implemented from cold, and it
unblocks the worldgen data track's round 2 (formations, boulders). Read
`Reports/worldgen-design.md` §5 and `design-philosophy.md` §2b first;
this design lives inside their rules.

## The idea in one paragraph

Between `column.rs::plan_all()` and the realise passes, iterate a small,
deterministic erosion/deposition simulation **on the 1D column-plan
arrays** — `h[x]` (surface elevation), plus new `talus[x]` and
`sediment[x]` deposit arrays — with per-cell **hardness sampled from the
same strata field the realise passes will draw**, for a number of
iterations proportional to a new `world_age` parameter. Hard bands
shed slowly and cap what is under them; soft bands under a broken cap
retreat; what comes off piles as talus below and washes into basins as
sediment. Hoodoos, undercut-then-softened profiles, valley fills, and
boulder sockets all fall out of the differential rates — the outcome is
never authored, only the rates are (`design-philosophy.md` §2b's test:
the shape is a side effect of a mechanism).

Why plan-space and not cell-space: the plan is 2048 floats, so even
10,000 iterations is tens of milliseconds at build time, against
~40–80 ms per full-grid cell pass (measured class, review §2 cost lens)
inside a regen budget of ≤ ~800 ms total (`R`/`F6` must stay
instant-feeling; build is 542 ms today, measured this session). And the
plan is *pure* — same purity tests `column.rs` already carries extend to
the eroded plan for free. Cell-space erosion (Noita-style per-pixel
weathering at runtime) is out of scope and stays out (review §5's
"deliberately not on the roadmap": full-grid runtime erosion).

## Inputs, and where each already lives

| Quantity | Source (already exists) |
|---|---|
| `h[x]` pre-erosion | `Terrain::elev(x)` / `plan_all()` (`column.rs`) |
| Strata band at `(x, elevation)` | `strata_offset(x)` + `strata_thickness` (`column.rs` — the same function `strata_shade` draws bands from, so eroded ledges automatically sit on drawn bedding) |
| Hardness per band | new: a per-band scalar hashed from `Purpose::` (new appended stream) × `Character.resistance` — the same band is hard for its whole length, which is what makes ledges *coherent* |
| Rain supply per column | `1 - Character.aridity` (region blend, `region.rs`) |
| Repose limits for deposits | the existing two-sweep `taper_cover` clamp (`column.rs`) — deposits route through it, which is what keeps the at-rest guarantee |
| Iteration count | new `world_age` param (`worldgen.ron`, per-preset with a global default) |

## The two processes

**Thermal (dry) erosion — makes talus and breaks the plumb faces.**
Per iteration, for each x: if the slope to a neighbour exceeds the
material's stable angle *scaled down by softness at the surface band*,
move a small `Δh` from the high column to the low one, splitting it
into `talus[low_x]` (a gravel share) and plain lowering. Hard bands have
a steeper stable angle, so a face retreats *by band*: soft bands notch
back, hard bands stand proud as ledges — the dead-plumb 40-cell terrace
risers the review flagged become stepped, ragged faces without any
riser-specific hack.

**Hydraulic erosion — makes drainage structure and valley fills.**
1D flow accumulation: walk columns by descending `h`, each column passes
its accumulated rain (own supply + upstream) to its lower neighbour;
columns that are local minima bank their inflow as basin volume. Carve
`Δh ∝ flow^a × slope × softness` (stream power, a ≈ 0.5 — sublinear so
big flows don't knife); deposit `sediment[x]` where slope flattens
(capacity drops). Two visible products the review asked for by name:
valley floors with real sediment fill (drawn as soil/sand by the
soil_blanket pass reading `sediment[x]`), and **drainage structure** —
the terrain acquires the local minima, connected falls, and outlet
shapes the rivers track needs (`worldgen-design.md` §5a: "generate
terrain with drainage structure and let real water find it").

**What emerges, and the guard for each:**
- *Hoodoos/spires*: a hard cap band over soft rock, cap breached between
  spires. Guard: hardness is sampled with a **lateral coherence floor**
  (smoothed over ≥ 4 columns) so residual features are 4+ columns wide —
  a 1-column residual is the keyhole artifact wearing a costume, and the
  review's open finding 1b makes thin verticals a known failure smell.
- *Talus aprons*: from thermal shed. Drawn by the existing talus
  machinery reading `talus[x]` instead of (or in addition to) its own
  cliff detection — which also rescues the 34-148-cell invisibility the
  data track's task 6 attacks from the other side. The two changes must
  land coordinated (task 6 first; erosion then feeds the same arrays).
- *Boulder sockets*: where thermal shed from a hard band exceeds a
  threshold in one place, record a `boulder[x]` marker; a later realise
  pass seats 2–5-cell rounded attached-stone clusters there. Markers are
  data on the plan (the state-the-difference-as-data lesson) — the
  boulder pass never infers "boulder-worthy" from shape at realise time.

## What this must preserve (the non-negotiables)

1. **At rest, asleep in 45 frames** (`tests/worldgen.rs`): erosion edits
   the *plan*; every deposit realises through the repose-clamped taper;
   nothing is placed steeper than its material's angle. The suite is the
   gate, all presets × 5 seeds, plus the 16-seed sweep's p90s.
2. **Purity/determinism**: fixed iteration count, integer/f32 math on a
   Vec, all randomness through `Purpose` streams keyed on (seed, band,
   x). The eroded plan is a pure function of (seed, params) — extend
   `column.rs`'s existing purity test to the eroded arrays.
3. **Structural survival on frame 1**: residual spires and seated
   boulders are attached stone (the massif pass realises them
   attached), so they are held the ordinary way; but the load model must
   agree — cap residual aspect ratio (height ≤ 3× width at the base)
   until measured otherwise, and **defer all breach-tolerance tuning**
   past the live dig-over-removal bug (`next-session-handoff.md`, other
   session's territory).
4. **Finite margins**: erosion is a whole-*plan* computation, which is
   plan-level and cheap — but it must not become a fourth GLOBAL
   *cell* pass. The realise side reads only per-column arrays, margin 0.
   (Streaming later re-derives the plan per region window; the coarse
   (x,z) map step owns that.)
5. **Regen budget**: erosion adds ≤ ~50 ms to the 542 ms build at
   `world_age`'s default. Print the erosion time in
   `terrain_generation_cost` alongside the existing figures.

## `world_age`, and what it is not

One scalar, default mid-range, per-preset overridable: iterations =
`age × ITERS_PER_AGE`. Young worlds are sharp (terraces plumb-ish, thin
soil, little talus); old worlds are subdued (rounded, deep valley fill,
wide aprons). This is deliberately the same parameter
`worldgen-design.md` §6 wants for `worldgen(seed, coord, age)` — the
erosion pass is the first real consumer of world age, and ecological age
(pre-grown vegetation, the review's top beauty lever) joins it there
later. It is **not** a live process: nothing erodes at runtime.

## Acceptance (before/after, per the method)

- Strips: canyon + arid + rolling, seeds 1 and 7, at `world_age` 0 /
  default / 2× — nine images, judged by eye for: stepped (not plumb)
  faces, visible talus aprons, sediment-floored valleys, at least one
  coherent multi-column residual formation in arid/canyon, and **no
  1-column verticals** (the keyhole check, task 1b's finding feeding in).
- Counters beside the images: total plan volume moved, talus/sediment
  array sums, boulder-marker count, erosion wall-time — printed by
  `filmstrip scene=worldgen`'s per-pass table so "did it fire" is never
  read off a picture.
- The 16-seed sweep re-baselined (this changes what every pass counter
  means — re-deriving the baseline is part of the change, §7.14).
- `a_generated_world_survives_a_replay` and the determinism hashes green.

## Delegation

Frontier: the erosion core (both processes + hardness field + coherence
floor) and the first tuning session against the strips — the failure
mode is "looks wrong", and rate constants that fight each other are the
counterweight trap. Cheap model after that: the `world_age` preset
plumbing, the boulder realise pass from markers (spec'd pattern: clone
`pockets()`'s collect-verify-write shape), sweep re-baselining, and
per-preset rate tuning *once the frontier session has established the
tuning is monotone* (each knob moves its own thing in one direction).
