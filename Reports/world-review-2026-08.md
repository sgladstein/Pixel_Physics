# World review, August 2026: what the generated world looks like, and where it goes next

A planning-session review of the world itself — worldgen, water, underground,
graphics — run as a multi-agent review over *rendered images* of generated
worlds, per the method rule that has never yet been wrong here: look first.
This report carries the findings, the owner's decisions folded in as
directives, the rivers/waterfalls position, a sequenced roadmap with
per-item delegation notes for a cheaper implementation model, and the
landmines appendix that must travel with every delegated task.

Companion artifacts, same change: the review workflow itself
(`.claude/workflows/world-review.js`, re-runnable) and four rendered strips
under `Reports/img/world-review/`. **These are the first images of a
generated world ever committed to this repo** — sixty-two screenshots exist
under `docs/screenshots/` and not one shows what `worldgen::generate`
builds. Every other render referenced below regenerates exactly (worlds are
deterministic per seed; `tests/worldgen.rs` pins it) with the commands
given inline.

Baseline measured this session (`cargo run --release --example ascii`,
release build, this machine, 2026-08-19 — re-measure before comparing):
- Settled scenes: **0 chunks awake**, worst frames 0.4–3.1 ms; the
  dirty-rect render skip reads 0.002 ms.
- Full-screen stress + field, parallel driver: **worst 19.9 ms** (serial
  106.8 ms).
- The shipped 2048×640 world builds in **542 ms** (place 299 ms,
  structural pass 243 ms) — paid on start and on every F6/F7/F8/R.
- Field cost at 2048×1280: settled 0.008 ms / stepping 7.2 ms.

## §1 What the images show

Render matrix: all six presets × seed 1, rolling and canyon × seeds
{2, 7, 13}, plus rain, night, and mined variants — fifteen strips, each a
four-shot viewport traverse of a full 2048×640 world.
Regenerate: `cargo run --release --example viewshot -- seed=N preset=P shots=4`
(plus `rain=wet`, `frame=1800`, `mine=1 settle=240` for the variants).

**The good, first.** The strongest visual asset the game has today is the
sky: the dusk shot of `rolling-night.png` — orange horizon glow thinning
into a starfield over a silhouetted ridge, moon with a halo in the last
shot — is genuinely beautiful, and it is *earned*, because the same clock
drives plant light economics. Rock strata read well: tilted, gently folded
bands that a cut face exposes, followable across the world. Seed variety is
real: rolling seed 7 is a coastal world (a lake with a sand beach, a bright
sand bowl, water on the far horizon), seed 2 is a lake district with
islands, seed 1 is dry uplands — four seeds genuinely feel like four
places. And canyon seed 7 contains the best landscape yet generated:
proper flat-topped buttes with sheer stratified faces and brow overhangs,
straight out of the American Southwest (committed as
`img/world-review/canyon-s7.png`).

**The problems, in descending order of what they cost the picture:**

1. **Worlds arrive visually barren.** `life_scatter` places *seeds* — one
   pixel each — and sparse moss. At the default settle there is no visible
   vegetation anywhere in any strip; `wetland`, whose whole identity is
   lushness, reads as a mud flat with a pond. The mined strip (240-frame
   settle) shows the tell: tiny green sprouts appearing along the soil
   line. The world is *populated* but not *grown* — first impressions are
   of a dead planet, and the fix (generating a world already aged, or
   pre-running growth at build time) is the single largest beauty lever
   available. This is `worldgen(seed, coord, world_age)` from
   `worldgen-design.md` §6 arriving earlier than streaming needs it.
2. **Presets don't differentiate at play scale.** `rolling-s1`,
   `terraced-s1` and `wetland-s1` are near-indistinguishable at strip
   scale: same gray massif, same near-black soil crust, same pale pocket
   spots. Only `canyon` (sand, mesas) and `arid` (dune teeth) separate.
   Within a single world it is worse: regions vary the *heightfield* but
   not the *identity* — walking 2048 cells crosses escarpments but never a
   visibly different country. The variability the owner wants does not yet
   exist on screen, and the region `Character` vector that should drive it
   already exists in `region.rs` and drives almost nothing visible but
   elevation and sand-vs-soil.
3. **The palette is a monochrome.** Gray rock, near-black soil, near-black
   water, blue sky. The soil profile the wiki promises (dark rich top,
   paler below) is invisible at play scale; standing water renders so dark
   it reads as holes in the world. The world needs hue, and materials are
   data (`assets/materials/*.ron`) — this is cheap territory.
4. **Arid dunes read as a mechanical sawtooth.** One wavelength repeated
   ~30 times across the world, and the teeth are *rock* — the dune
   function displaces the stone surface, with only a dusting of sand — so
   the preset that should read as a sand sea reads as a zigzag rock
   ridge. (`arid-s1.png`, x≈250–1700.)
5. **A recurring "keyhole" artifact.** One-to-two-column vertical slots
   cut the full height of cliffs and mesas: canyon-s7 x≈590–680 (two
   parallel slots through the mesa), canyon-s13 x≈205, rolling-s1 x≈1300,
   rolling-s2 x≈500. It looks like a terrace/brow interaction bug, is
   visible at every zoom, and any formation work will inherit it if it is
   not traced first.
6. **Every pond draws a pale dashed line at its surface** — the one-cell
   monolayer draw problem (`open-bugs-handoff.md` §1) is not just a
   spreading-front artifact; it is visible on *standing* water in every
   strip that has a pond. Rivers would exhibit it continuously.
7. **The underground is confirmed empty.** Three shafts of different
   widths (`canyon-mined.png`) find: uniform stone, strata shading, a
   sand pocket. The wiki's "a quarry rather than a destination" is exactly
   right, and the shafts themselves render as featureless black voids
   (correct under the stored-sky rule — and a preview of the cave-lighting
   question below).

Per-pass counters for three of these worlds (counter next to picture, per
method): every pass fires; canyon seed 1 generates **zero** standing water
(ponds 0, soil_moisture 0); `brows` places 34 cells and `talus` 148 in a
512×320 world — features currently too rare to register visually.

*(§2 lens findings and §3 prosecutor verdicts are filled in from the
workflow run — see below.)*

## §4 Rivers and waterfalls: yes — and the repo already did the hard thinking

**The conservation worry is already answered by shipped code.** Water in
this engine is deliberately non-conserved in both directions: rain
*creates* full water cells out of the sky (`weather.rs`,
`WATER_CELL_CHANCE = 0.06`, work-capped at 24 columns/frame regardless of
world width), snow melts into full water cells, and evaporation *deletes*
fill unbanked (`weather-handoff.md`: "Evaporated water is gone, not
banked"). Every conservation test is scene-scoped to exclude weather and
soil. An edge source is the same class of mechanism as rain — a budgeted
boundary condition — not a new architectural transgression. Dwarf Fortress
has shipped exactly this for two decades (rivers enter at one map edge and
drain off the other); Terraria's liquids are non-conserved by declared
design. The precedent is respectable.

**The design fork is already written down.** `worldgen-design.md` §0
frames the world as a 2D slice of 3D worldgen (the `ChunkCoord.slice`
field is already reserved), and §5a poses the exact question the owner
asked, verbatim: off-plane flux can be **real** — a coarse `(x,z)`
drainage map genuinely computes upstream and downstream and the slice
honours it — or **plausible** — water appears at the boundary at a
believable rate — and the report demands the choice be made explicitly.

**Position: build *plausible* flux first, and say so in the code.** The
recommended shape:

- **Source**: a *perched spring* — a budgeted emitter at a cliff brow or
  high rock face, patterned exactly on rain's capped emission (N units per
  interval, a handful of columns), rather than a bare world-edge column.
  Springs read better (the brow machinery already generates the ledges),
  work anywhere in the world, and are what the aquifer design in
  `worldgen-design.md` §7 already calls for ("a perched aquifer high in a
  rock face **is** a spring").
- **Sink**: evaporation already is one (and is fill-rate-scaled); for a
  true through-river, one opened edge column low on the opposite side,
  deleting arriving fill at a capped rate — the mirror of the source.
  Opening it touches three deliberate seals (`Cell::OUT_OF_BOUNDS`
  bedrock sentinel, the field solver's edge wall, the `ponds` pass
  treating edges as the tallest barrier) plus rescoping
  `nothing_escapes_the_world` — engine work, one session, not this one.
- **Waterfalls are the cheap half and should lead.** Free fall is
  whole-cell and unthrottled; in-flight water is exempt from evaporation
  scheduling by design; a spring on an existing brow over a basin *is* a
  waterfall with zero new physics. A chain of pools linked by short falls
  — which is what gravity-fed water with no hydrostatic pressure will
  naturally produce on this terrain — should be judged by eye as a
  candidate *feature*, not a failure: step-pool cascades are among the
  most beautiful water forms in nature.

**Pre-registered kill criterion** (set before building, per the sweep
rule): run `ascii`'s stress scene and a spring scene at steady state in
the same session; if the flowing channel's *standing* worst-frame cost
lands in the class of the reverted global wind (≈3.5 ms permanent), the
approach dies, not the tuning. The chunk math says a modest spring-fed
cascade wakes a band of chunks permanently — that band must stay small
(bounded run length, pool-to-pool) for the cost model to survive. The
whisker/monolayer draw (§1.6) is a *prerequisite for rivers looking
right* and is render-side work.

**What survives the later "real flux" upgrade:** the spring/sink emitters
and their budgets become *outputs of the coarse (x,z) map* instead of
authored placements; everything else (emission machinery, rendering,
tests) carries over unchanged. Nothing about plausible-first is throwaway.

## §7 Landmines appendix — for the implementation model

Every one of these has already cost this project real time. They travel
with every delegated task; the delegating spec should quote the relevant
ones verbatim.

1. **`aux == 0` means FULL on a `Liquid` and DRY on a `Powder`.** Writing
   a literal 0 fill manufactures water. A drained liquid is
   `Cell::EMPTY`, never `with_aux(0)`.
2. **Editing an asset `.ron` does nothing until rebuild** — materials and
   species are `include_str!`-compiled. Identical output across sweep
   settings means the knob was never connected. Rebuild between sweep
   points.
3. **Generated terrain must arrive at rest and sleep fast.**
   `tests/worldgen.rs` enforces zero cells moving in 120 frames and
   `active_chunk_count() == 0` within 45 frames, every preset × 5 seeds.
   Any new pass output (formation, chamber, decoration) must hold both.
4. **Only three GLOBAL worldgen passes exist, and a test pins the list.**
   New passes declare a finite column margin
   (`only_the_water_passes_read_the_whole_world`).
5. **Registries are append-only**: material ids (`EMBEDDED` order) and
   noise `Purpose` discriminants must never be renumbered.
6. **Determinism is required** (same-build). No iteration over a
   `HashMap` may influence behaviour — `BTreeMap` or a sorted `Vec`. No
   wall-clock, no non-seeded randomness.
7. **Test both drivers.** `update::step` is serial; `parallel::step` is
   what the app runs. Behaviour only the player sees is behaviour only
   the parallel driver produces.
8. **A size cap must bound work, never gate whether something happens.**
   `if too_big { return }` claims the largest cases deserve the least
   behaviour — written twice here already.
9. **Sweep before changing anything that governs procedural content**, and
   gate an order statistic (p90/max over seeds), not a single seed —
   `scripts/seedsweep.sh` exists because eight green acceptance scenes
   twice rubber-stamped changes that ate 26–50× more world.
10. **Do not add structural checks to organism paths**
    (`schedule_structural_check_around` amputates crowns — measured 26×
    outcome difference from one line).
11. **Never `git add -A`; stage explicit paths.** Contested files
    (`src/app.rs`, `README.md`, `PLAN.md`, `CLAUDE.md`) get minimal
    diffs landed fast.
12. **Don't strip load-bearing comments** — they record reverted
    approaches that must not be retried. Add to them in the same voice.
13. **`cargo fmt` is all-or-nothing** (formats 28 files); the full pass
    is deliberately deferred — do not let it ride along.
14. **When a fix changes what a number means, re-derive the constants
    that read it** — that re-derivation is part of the fix, not scope
    creep.
15. **A green suite is not evidence the screen changed.** Every visual
    change ships with a rendered artifact (`viewshot`/`filmstrip`) and,
    for "did it fire at all", a printed counter next to the image.
16. **The world edge is sealed on purpose in three places** (bedrock
    sentinel, field wall, ponds rim) — opening any of them is a
    coordinated change with test rescoping, never a local edit.
17. **Do not build on `src/sim/liquid.rs` heightfield bodies** — never
    runs in production, measured slower than the CA it was meant to
    replace, purpose explicitly unsettled (`open-bugs-handoff.md` §6).
18. **`load.rs`, `structural.rs`, `rigid.rs` and the filmstrip structural
    scenes belong to the destruction workstream** — read-only from world
    work, and the "one dig eats too much world" bug lives there
    (`next-session-handoff.md`).
