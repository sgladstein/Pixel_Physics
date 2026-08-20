# Worldgen data track, round 4 — erosion made visible

The erosion core landed (`src/worldgen/erosion.rs`,
`Reports/worldgen-erosion-design.md` — read its **Status, 2026-08**
section first; it is the handoff) and it is dark: `world_age` defaults
to 0.0 everywhere, so no shipped world is weathered yet, and its
deposits realise only as generic cover. This round turns the mechanism
into pictures: talus that reads as gravel, boulders seated where hard
caps shed, aged presets on by default, and the counters that prove any
of it fired.

Same contract as rounds 1–3, inherited whole from
`Reports/worldgen-implementation-tasks-2026-08.md`'s Ground rules —
branch `claude/worldgen-data-track-r4` from
`claude/game-world-gen-planning-h12713` (at or after the round-3 merge
`8424032`), image-backed commits, findings over improvisation, gates
green at every land (`cargo test`, `clippy -D warnings`, at-rest suite,
`worldgen_sweep.sh compare`). Append findings to the round-1 file.
Owned files as ever (`src/worldgen/*`, `assets/worldgen.ron`,
`tests/worldgen.rs`, `scripts/`); `examples/viewshot.rs` belongs to the
reviewing session — flag any needed change rather than making it.

**What must not change**: the erosion *rates* (`erosion.rs`'s constants
and `HardnessField`'s shape were set in a by-eye tuning session against
strips — retuning them is the reviewing session's call; if a task below
seems to need it, stop and write a finding); the age-0 no-op guarantee
(`plan_all_at_age_zero_matches_plan` must stay green **unchanged** in
what it asserts); the at-rest suite; determinism.

## Task 1 — Plumb `Deposits` to the realise side

`column::Terrain::plan_all` computes `erosion::Deposits` and folds it
into `soil_depth` internally; nothing downstream can see which cover is
deposit and which is native blanket, and the boulder markers go nowhere.
Restructure minimally: a method (e.g. `plan_all_with_deposits`) returning
both, `plan_all` delegating to it, and `Ctx` (worldgen/mod.rs) holding
the `Deposits` beside `plans`. Pure plumbing — bit-identical worlds at
every age, which the existing purity/no-op tests should confirm without
edits. Everything below reads `ctx.deposits`.

## Task 2 — Talus reads as gravel

Where `deposits.talus[x]` is at least 1, the **topmost**
`min(round(talus[x]), soil_depth)` cells of that column's cover draw as
gravel (buried-gravel family, same as a lens — see `soil_blanket` and
the vug floor for the pattern) instead of soil/sand. Rockfall lands on
top of the blanket, so the gravel is the top of the profile, not the
bottom; the dithered soil/stone contact at the bottom stays as it is.
Zero new placement — the cells were already cover, only the material
changes, so at-rest is untouched by construction. Verify with a paired
strip (canyon seed 7, age 1: `erosion3_canyon_s7_age1.png` in the
reviewing session's filmstrips is the before) — the aprons under the
mesas should read rock-grey-brown against the soil, and the counter in
task 5 says how many cells changed material.

## Task 3 — Boulders seated at the sockets

A new realise pass (after `pockets`, before `vaults`), cloning
`pockets`' collect-verify-write shape: for each run of columns with
`deposits.boulder[x]` set (merge adjacent marks; one boulder per run),
seat one rounded attached-stone cluster on the surface at the run's
centre —

- 2–5 cells wide, 2–4 tall, wider than tall or square, **never taller
  than 3x its base width** (the erosion design's structural
  non-negotiable #3), roughly elliptical, drawn from a `Purpose` stream
  keyed on the run (append a new discriminant ≥ 24 — 20–23 are taken;
  say which you claimed in the commit).
- Attached stone (`with_attached(true)`), stone material, resting on the
  massif surface (displace cover; the boulder sits *on* rock or *in* the
  blanket, never floating on loose powder alone — write down through
  cover to seat at least its bottom row on stone or on cover backed by
  stone).
- Shade: the pale cap-rock family (`FAMILY_RESISTANT`) — these are hard-
  band survivors and should read as the resistant stone they came from,
  distinct from the banded wall behind them.
- Collect-verify-write: propose all cells, verify each write target is
  currently surface/cover (never carve into solid massif below the
  surface, never overwrite water or a vault), else skip that boulder.
- The at-rest suite and `an_aged_world_arrives_at_rest` (erosion.rs)
  must stay green; forced-density test at harness size so the pass is
  exercised (the r2/r3 lesson: a guard that cannot see the feature has
  no teeth). Boulders fire rarely at current rates (rolling 1–5 markers,
  canyon/arid 0 at age 1) — the forced test should raise `world_age`
  rather than touching erosion constants.

## Task 4 — Ages on by default, and the sweep re-baselined

- Proposed per-preset `world_age` (both in `assets/worldgen.ron` and
  compiled defaults where a preset has one — remember `rolling` is
  asserted equal to `WorldgenParams::default()`, so flipping rolling
  means flipping the compiled default *with* it): rolling 0.8,
  canyon 1.0, arid 0.7, wetland/other soil presets 0.8; `flat` and any
  structural test bed stay 0.0. Record what you shipped.
- Retarget `soil_blanket`'s valley-floor check (the
  `ctx.terrain.slope(x) / elev(x)` read near passes.rs:335) to the
  plans: slope from `surface_y` central difference, elevation from
  `datum - surface_y`. At age 0 this is the same quantity it read
  before (rounding aside); at age >0 it is the *true* surface. Tiny
  diff, state it in the commit.
- Re-baseline the 16-seed sweep at both sizes (this changes what every
  counter means — re-deriving the baseline is part of the change,
  CLAUDE.md §7.14) and re-run the dual-size cave guards (cave placement
  shifts with the surface; the density may need a nudge to keep
  32-of-40-ish — if it moves past ~±8 worlds, stop and write a finding
  rather than retuning blind).
- Deliverable for the owner: A/B strips (age 0 vs shipped age) for
  rolling/canyon/arid at seeds 1 and 7, committed file paths listed in
  the commit message.

## Task 5 — Counters

Erosion runs in the plan phase, so it has no pass-table row and a bare
`println!` in `plan_all` would spam every test build. Route it like the
vaults detail line: hold the numbers (volume moved, exported, talus and
sediment sums, boulder-marker count, boulders seated, talus cells
re-materialed, erosion wall-time ms) on `Ctx` / the new pass, and print
one gated `erosion detail: ...` line from `generate_reported` only —
the same deliberate-format argument as `vaults detail`. Zero lines for
an age-0 world.

## After task 5

Stop and push. Springs placement (springs where the water table
daylights on a slope — the rivers track's worldgen half) is **not** in
this round: it needs the reviewing session's flow-budget rulings first.
