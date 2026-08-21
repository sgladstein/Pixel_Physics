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

## Status, 2026-08: core landed (stage 1), tuning session record

The core is implemented (`src/worldgen/erosion.rs`, hardness in
`column::Terrain::surface_hardness` / `HardnessField`) and wired into
`plan_all` behind `world_age`, **default 0.0 everywhere** — a guaranteed
bit-exact no-op (`plan_all_at_age_zero_matches_plan` pins it), so nothing
changes for any shipped preset until per-preset ages are deliberately
flipped on with the sweep re-baseline. That flip is stage 2.

What the first tuning session established, in strips (canyon s1/s7, arid
s1, rolling s1 at ages 0/1/2, `target/filmstrips/erosion3_*`):

- **The stable-angle contrast is the picture.** First rates (soft 0.9,
  hard bonus 2.6) converged every face to ~2.7 cells/cell — 70°, still
  reads plumb; the probe showed 1,848 cells of height moved while only
  1.5% of strip pixels changed. A picture cannot distinguish "weak" from
  "dead"; the probe (`erosion_probe`, `--ignored --nocapture`) can.
  Landed: soft 0.55, hard bonus 4.5.
- **Threshold rules never round a crest** — anything shallower than the
  stable angle is invariant, so "subdued old world" needed the textbook
  second process: hillslope creep (`SOFT_CREEP` Laplacian, softness-
  scaled). Rounding at ~4 columns per age unit.
- **Canyon seed 7 is the acceptance picture**: both mesas survive as
  coherent multi-column formations with stepped, skirted faces while the
  badlands beside them subdue and mantle. **Rolling seed 1's 1-column
  keyhole chimneys (review finding 1b) are erased outright** — the
  process removes that artifact class as a side effect. Arid weathers
  gently by construction (rain scales with 1 − aridity).
- **Deposits realise through the soil blanket** (added to `soil_depth`
  after the patchiness rule, before `taper_cover`): the deposit's volume
  is already banked in the surface h, so deepening cover *converts* those
  rows to loose material rather than minting height. At-rest is inherited
  from the same gates, and `an_aged_world_arrives_at_rest` holds the
  suite's own positional bar at age 1 (life off; the full-cell compare
  version flagged 2,185 pond-surface cells whose fill drifted under
  evaporation — position-and-material is the honest claim).
- **Budget**: 37–41 ms at age 1.0, 2048 columns, in-lib release probe —
  inside the ≤50 ms line — after precomputing the hardness invariants
  (`HardnessField`; the fBm strata offset and region blend per column per
  iteration were most of the pass's cost).
- Boulder sockets fire rarely at the landed rates (rolling: 1–5; canyon/
  arid: 0). The marker plumbing is in and tested pure; whether the
  threshold is right is a stage-2 question to answer *with the boulder
  realise pass on screen*, not before.

### Boulders on screen, 2026-08-20 — the deferred question, answered

The Status note above deferred "whether the threshold is right" to a
judgement made *with the boulder realise pass on screen*. This is that
judgement, at the shipped 2048x640. **The threshold is not the problem.
A seated boulder is 2-5 cells wide and 1-2 cells tall, on a player who
is 14 cells tall, and it is less prominent than the median piece of
hillside.**

**How the object was found matters, because two earlier attempts found
the wrong one.** `viewshot boulder=1` now asks the generator directly —
`Terrain::plan_all_with_deposits` is public and `Deposits::boulder` is
the same marker array the pass reads, so the answer is exact. Before
that: a "most prominent stone bump" finder returned an ordinary
sandstone outcrop, and a "2-6 column run of cap-rock at the surface"
finder returned 6-16 candidates per world, because cap-rock *beds*
outcrop in the same palette family. A first draft of this section was
written from a render of one of those. CLAUDE.md's rule — *when a
mechanism appears inert, check the scene still contains the situation
you think it does* — applies to the harness as much as to the world.

**Size, measured exactly** (`boulder_true_canyon_s6.png`, seed 6,
x=809, 8x):

| | |
|---|---|
| drawn width | 2-5 cells |
| drawn height | 2-4 cells, clamped never taller than wide |
| **height actually standing above ground** | **1-2 cells** |
| prominence vs its flanks | canyon s6 **0**, s12 **-1**, s23 **-8**; rolling s17 **0** |
| that prominence as a percentile of ordinary hillside | **2nd to 55th** |
| player height (`sim::player::PLAYER_HEIGHT`) | **14** |

The visible height is half the drawn height and that is arithmetic, not
a bug: the dome is an ellipse of full height `height`, `b = height / 2`,
and only the rows *above* `ground_y` are written — so the centre column
rises `round(height / 2)` = 1-2 rows. A "4 tall" boulder is two cells of
stone on a hillside.

The prominence numbers are the ones that settle it. A seated boulder
stands zero or negative cells proud of the ground five columns either
side, which puts it below the median hillside column (the median is -1
because most columns sit on a slope). It is not a landform; it is
texture.

**Two things that are *not* the defect**, both checked because both were
the obvious guess:

- **Contrast is already applied.** The pass writes `FAMILY_RESISTANT`
  (the pale cap-rock family) unconditionally, exactly as round-4 task 3
  specified, "distinct from the banded wall behind them". A first draft
  of this section claimed there was no contrast treatment; that was
  wrong. The treatment is there and cannot rescue a two-cell object.
- **It is not lost in the terrain's own texture.** Surface prominence
  over the whole world is **median -1, p90 0, p99 1, max 2-3** — the
  hillside is smooth at this scale. That is *good* news and it changes
  the fix: a 6-12 cell dome against a p99-of-1 surface would be
  unmissable, so raising the size works, and no amount of calming the
  surrounding terrain would have helped.

**Why the size is capped, since it is the first question anyone asks.**
Nothing structural requires it. The erosion design's non-negotiable #3
says only *height ≤ 3x width at the base*, and the round-4 task file
authored "2-5 cells wide, 2-4 tall" as a concrete reading of that; the
implementation then clamped tighter still, `height.min(width)` — a
ratio of 1x where 3x was allowed. A 12-wide, 8-tall boulder is 0.67x
and satisfies the stated rule with room to spare. **The cap is an
authored number that was never re-examined against the player's scale,
not a limit the load model imposes.**

**Frequency, measured at the shipped size** (24 seeds each; round-4's
R4-1 measured at the 512-column harness):

| preset | marker columns | boulders seated | worlds with one |
|---|---|---|---|
| canyon | 49 | 4 | 3 of 24 |
| rolling | 8 | 1 | 1 of 24 |
| terraced | 3 | 0 | 0 of 24 |

R4-1's reading stands — the dome's air is usually already spoken for by
a `brows` lip, and refusing to punch through it is correct — but note
the ordering of the fix: **frequency is the third problem, not the
first.** Making a two-cell pimple eight times more common produces
eight pimples.

### The scale band between texture and landform is empty

Owner's push-back, 2026-08-20: *"maybe boulder was just not the right
term — rock formations can easily be 5, 10, 20, 40 ft tall or larger,
and they have many shapes, smooth and round, sharp and jagged."* Right,
and a first answer to it (6-12 cells) was still anchored on the existing
mechanism rather than on what rock does. The measurement below is the
honest version.

**Prominence at four reaches**, because a reach is a scale and one reach
cannot see past it. A 40-cell hoodoo twelve columns wide scores **zero**
at reach 5 — both sample points land on top of the formation — so the
"max prominence is 2" figure above means "nothing narrower than ten
columns", not "nothing at all". Measured over the whole world:

| preset / seed | reach 5 max | reach 15 max | reach 30 max | reach 60 max | relief | sky above the highest ground |
|---|---|---|---|---|---|---|
| canyon s7 | 3 | 8 | 8 | **39** | 136 | 86 |
| canyon s6 | 2 | 10 | 10 | 19 | 97 | 104 |
| rolling s1 | 2 | 4 | 8 | 18 | 65 | 87 |
| terraced s1 | 1 | 4 | 6 | 19 | 73 | 88 |

Read across a row: the world has **landforms** (reach 60 — canyon s7's
mesa at 39 cells, which is the one landscape the review called the best
yet generated) and it has **texture** (reach 5, 1-3 cells). Between them,
**at reaches 15 and 30 — exactly the scale a rock formation occupies —
the tallest thing in the entire world is 4 to 10 cells.** Not rare:
absent. There is no tor, no stack, no pinnacle, no standing residual
anywhere in any of these worlds.

**In the player's units.** `PLAYER_HEIGHT` is 14 cells, so a cell is
roughly four to five inches and the owner's range converts as:

| | cells |
|---|---|
| 5 ft | ~12-15 |
| 10 ft | ~25-30 |
| 20 ft | ~50-60 |
| 40 ft | ~100-120 |

Against 86-104 rows of sky above the highest ground and 65-136 cells of
relief, a 20 ft formation is a landmark that fits comfortably and a
40 ft one is comparable to a rolling world's entire relief. The world
can host the whole range the owner named; it currently hosts none of it.

**Why it is empty, and it is not one reason.**

1. **The mechanism was scoped as debris, not as landform.** A
   `Deposits::boulder` marker records where a hard band shed enough to
   leave a socket, and the realise pass drops a small dome there. That
   is a rock that *came off* something. A tor or a hoodoo is the
   opposite object: what was left standing when everything around it
   retreated. Nothing in the pipeline produces residuals.
2. **This design says residuals should fall out of the rates, and they
   do not.** The opening paragraph promises "hoodoos,
   undercut-then-softened profiles, valley fills, and boulder sockets
   all fall out of the differential rates". Valley fills and sockets
   arrived; hoodoos did not, and the reach 15/30 column is the
   measurement that says so. Suspect the **stable-angle rule**: a tall
   narrow residual presents a near-vertical face to its neighbour, which
   is steeper than any stable angle, so thermal shed knocks it down on
   the iteration after it forms unless the hardness contrast there is
   large enough to protect it.

   **Tested, and the answer is "missing mechanism", not "tuning".**
   `viewshot age=N` overrides `world_age` for one render, so the
   question is answerable without touching `erosion.rs`. Maximum
   prominence at reach 15 (the formation band), by age:

   | | age 0 | age 0.8 (shipped) | age 2-3 | age 4 | age 8 |
   |---|---|---|---|---|---|
   | canyon s7 | **10** | 5 | 4 | 3 | 5 |
   | canyon s1 | **10** | 3 | 3 | — | — |
   | rolling s1 | **8** | 4 | 5 | — | — |
   | terraced s7 | **8** | 4 | 3 | — | — |

   **Erosion does not create formation-scale relief; it removes it.**
   Age 0 — no erosion at all — has two to three times more prominence in
   the formation band than the shipped age does, in every preset and
   seed tried, and raising the age drives it down further. The one
   recovery is canyon s7 at reach *30* climbing to 21 by age 8, which is
   hydraulic incision cutting channels (the same non-monotonicity
   round-4's finding R4-2 measured in roughness) — canyons, not residual
   spires, and at an age no preset ships.

   This is consistent with the stable-angle hypothesis: whatever the raw
   heightfield offered at this scale, thermal shed flattens, and nothing
   in the pass ever builds it back. The "hoodoos/spires" bullet in *What
   emerges* above, with its lateral-coherence floor, describes a
   mechanism the shipped rates do not produce at any age. It was
   specified and it is not there — which is a different and more
   actionable finding than "the rates need tuning".
3. **A heightfield cannot represent half of the shapes asked for.** The
   erosion plan is one `h[x]` per column, so it can express a tall
   narrow column (a tor, a pinnacle, a stack) and **cannot** express an
   undercut — the mushroom cap on a thin stem that makes a hoodoo read
   as a hoodoo, or a balanced rock. Those have to be realise-pass work,
   the way `brows` already hangs an overhang the plan cannot hold. Any
   spec that promises hoodoos purely from plan-space erosion is
   promising something the representation forbids.
4. **The authored numbers, compounding.** Round-4's task file read
   "height ≤ 3x width" as "2-5 wide, 2-4 tall"; the implementation
   clamped to `height.min(width)`, a 1x ratio; and only the top half of
   the ellipse is drawn. Three independent shrinks, none of them
   required.

**On shape variety**, the other half of the push-back: there is one
shape today, an ellipse. The variety should come from where the sizes
come from — the process and the rock. The `HardnessField` already knows
which band is hard, which is exactly the input separating a flat-capped
stepped residual (hard cap over soft) from a rounded dome (long
weathering, uniform rock) from an angular blocky pile (frost shatter
along bedding). Shape as a side effect of which band survived, per this
design's own test that the shape is never authored.

**So the fix, in order**: (1) run the stable-angle probe in 2 — it
decides tuning vs. new mechanism and it is the cheapest thing here;
(2) residual landforms in the 12-60 cell band, shaped by which band
survived, with undercuts as realise-pass work per 3; (3) size and shape
variety at the small end, re-derived from the real 3x rule rather than
from round 4's reading of it; (4) frequency last — R4-1's
footprint/ordering against `brows`. Contrast is already handled. **Do
not reach for the erosion rates as a first move**: they were set by eye
across a whole session against the *landform* scale, which the table
above says is the one scale that already works.

At these sizes the structural claim stops being free and has to be
tested rather than argued. A 50-cell residual is attached `Solid`, so it
has no movement rule and holds by construction at genesis — but it is
also the first object in the world a player can plausibly undermine, and
the 3x aspect rule was written "until measured otherwise" and has never
been measured. A test that digs the base out from under one is part of
the work, not a follow-up.

Scheduled for round 6 with the palette-dither fix (open bug 0b), both
being `passes.rs` work, held off while round 5 is mid-flight in that
file.

Stage 2 (per §Delegation, cheap-model after the erosion core): talus
drawn as gravel (`Deposits::talus` is already split out), the boulder
realise pass from markers, per-preset `world_age` defaults + the 16-seed
sweep re-baseline, and retargeting `soil_blanket`'s valley-floor check
(passes.rs:335) from pre-erosion `elev`/`slope` to the plans — deferred
here because the round-3 branch owns `passes.rs` at time of writing.
