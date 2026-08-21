# Worldgen round 6, Track A — caves worth walking into

**STATUS: APPROVED, execute in order.** You are the implementation session
for the cave track. The planning session that wrote this remains the
reviewer: **you land small, image-backed commits; you do not judge your own
visuals.** When a spec here does not survive contact with the code, **stop
and write a finding** into the Findings section rather than improvising —
rounds 1–5 wrote sixteen findings and every one is load-bearing.

Read first, in order: `CLAUDE.md`; `Reports/cave-beauty-review-2026-08.md`
(the whole thing, including the round-5 verdict at the bottom);
`Reports/worldgen-implementation-tasks-round5-2026-08.md` Findings R5-1…R5-5;
`Reports/world-review-2026-08.md` §7.

## Why this round exists

Round 5 met almost every bar it was set and made the cave **worse**:

| | before r5 | after r5 | |
|---|---|---|---|
| **reachable by the player** | 64–76% | **0–8%** | the round's real outcome |
| median open column | 30 | 4–5 | player is 14 tall |
| contrast p95/med | 2.0x | 5.2–5.8x | bar was 3.0 — far exceeded |
| worlds with a cave | 3–10/16 | 12/16 | genuinely better |

**`PLAYER_WIDTH x PLAYER_HEIGHT` is 7 x 14 and crouch is unimplemented**
(`Reserved … (phase 3)`). The reviewer's own task-2 bar ("median open column
3–8") was chosen to maximise a contrast ratio and never checked against the
character, so the round produced a beautiful plan the gnome cannot enter.
The owner's words on the render: *"it doesn't look like I could even enter
it"*, *"totally full of stuff"*, *"looks like a single room instead of a cave
system"*, and formations *"are all 1 pixel thick. They should have a taper
and be thicker but fewer of them."*

**The rule this round is built on: a bar met is not a round passed.** Bars on
per-column statistics cannot see composition, and cannot see the player.

## Ground rules

- **Branch** `claude/worldgen-caves-r6`, worktree
  `.claude/worktrees/caves-r6`, cut from
  `claude/game-world-gen-planning-h12713`. One task, one commit. Commit
  messages carry the numbers.
- **Files you own**: the *cave* functions in `src/worldgen/passes.rs`
  (`vaults`, `cave_system`, `carve_cave_void`, `settle_cave_void`,
  `erode_breaches`, `grow_monumental_chamber`, the speleothem block),
  `assets/worldgen.ron`, cave tests in `tests/worldgen.rs`.
  **Do not touch**: `boulders` / `brows` / `talus` / `cliff_edges` /
  `soil_blanket` (Track B is live in those), `src/worldgen/erosion.rs`,
  `src/worldgen/residual.rs`, `src/render.rs`, `src/sim/*`, `examples/*`
  (they are the measuring instruments — changing them changes the ruler),
  and the contested files (`src/app.rs`, `PLAN.md`, `README.md`,
  `CLAUDE.md`, `wiki/*`).
- **Reserved `noise::Purpose`**: **`CaveSize = 28`**, **`CaveVariety = 29`**.
  1–26 are taken and **27 is reserved for `CeilingGrain`** in two existing
  doc comments — do not take it. Append only, never renumber.
- **Before every commit**: `cargo test`; `cargo clippy --all-targets --
  -D warnings`; `cargo test --test worldgen`; `cargo run --release --example
  ascii` with no worst-frame regression; `scripts/worldgen_sweep.sh`
  re-baselined.
- **Run `cargo run --release --example pass_ablation` after any pass
  change.** It found `ponds` eating 49% of wetland's vegetation and
  `pockets` deleting every cave in arid. A pass that suppresses another is
  this generator's recurring defect.
- **Every visual change ships a render and a counter.** `viewshot`
  (`vault=`, `zoom=`, `crop=`), `cave_probe`, `pixel_stat`.
- **Post review cards** for anything judged by eye:
  `python3 "$(git rev-parse --path-format=absolute --git-common-dir)/pixel-physics-review/bin/review.py"`,
  protocol at
  `git show origin/claude/agent-testing-platform-6efu1e:.claude/skills/review/SKILL.md`.
  Blind A/B where you have a stake; counter in `meta`; **fire and forget, do
  not block**.

**Landmines** (each has cost this project real time): `aux == 0` is FULL on
a Liquid and DRY on a Powder · `.ron` edits do nothing until rebuild ·
registries are append-only · determinism, no `HashMap` iteration influencing
behaviour · test both drivers (`update::step` serial, `parallel::step` is
what the app runs) · **a size cap must bound work, never gate whether
something happens** · sweep an order statistic (p50/p90 over seeds), never
one seed · never `git add -A` · don't strip load-bearing comments ·
`cargo fmt` is all-or-nothing, do not run it · generated terrain must arrive
at rest and sleep · **and the new one: check every bar against
`PLAYER_HEIGHT` before adopting it.**

---

## A0 — build the instrument and pay the cost down. **Do this first.**

Non-negotiable prerequisite for A2, and valuable alone.

The cave path is **O(N²) in envelope area**, measured: `settle_cave_void`
drops exactly **one** ceiling tooth per outer iteration, and
`erode_breaches` does **25 `World::get` per void cell per inner fixpoint
pass**. At 5x the area that is ~25x the settle cost — against a total regen
headroom of about **258 ms** (≤800 ms budget, 542 ms build today). And
**`vaults detail` prints no timing at all**, unlike `erosion detail`, so
there is currently no number to size anything against.

1. Add a wall-time field to `VaultReport` and print it in the `vaults detail`
   line. **Instrument before optimising** — otherwise you cannot tell a win
   from a wash.
2. Make `first_long_ceiling_run` return **all** long runs so every tooth
   drops in one pass, collapsing the outer loop.
3. Fill a `Vec<bool>` stone mask over the dilated envelope **once** instead
   of re-querying `world.get` on every inner fixpoint pass.

**Bar**: vaults wall-time reported, and after the optimisation **no worse
than before it** at unchanged envelope size. Report both numbers. Only then
may A2 grow anything.

---

## A1 — passages the player fits through

Retune `CAVE_THRESHOLD` / `CAVE_CELL` / `CAVE_SQUASH` against
**reachability**, not contrast.

`cave_probe` reports `reachable by player %` — a morphological opening: keep
every position where the whole 7x14 box is void, then measure the void
within one box of a kept position. Use `cave_probe field=1 t=.. cell=..
squash=..` to sweep the *rule* with no world build before building anything.

**Bar**: **reachable ≥ 50%, p50 over 16 seeds**, every caved preset. Contrast
≥ 3.0 held *subject to* that, not driving it. Median open column should land
near or above 16 rather than 4–5.

**Watch**: contrast and reachability pull against each other, and round 5
shows which wins if you let a ratio drive. If you cannot hit both, **hit
reachability and report the contrast you got** — that is a finding, not a
failure.

---

## A2 — bigger caves, heavy-tailed sizes

Owner's ruling: up to **~400 x 160**, heavy-tailed, most much smaller.
Today every system is exactly 181 x 71 because `CAVE_HALF_W/H` are `const`.

Convert them to **runtime per-system values** threaded through every cave
function (`cave_idx`, `planned_solid`, `keep_seed_component`,
`first_long_ceiling_run`, `carve_cave_void`, `settle_cave_void`,
`grow_monumental_chamber`, `erode_breaches`, `cave_system`), drawn
heavy-tailed (e.g. `MIN + (MAX-MIN) * u.powi(3)`) on `Purpose::CaveSize` in
`vaults`, **before** the `lo`/`hi` and world-edge tests that read them.
Free `Purpose::Vault` sub-coordinates: 3, 6, 7, or ≥98 (0/1/2/4/5/8/97 taken;
prefer a large N, since `Vault` is also used with world cell coordinates).

**Bar**: span across **p10 ≤ 120 and max ≥ 350** over 16 seeds — varied, not
uniformly bigger. Vaults wall-time inside budget. At-rest suite green.

**Five traps, all already measured — do not rediscover them:**

1. **The margin contract is unenforced.** `vaults` declares `margin: 96`,
   derived as `CAVE_HALF_W + VAULT_RIND` rounded up. At half-width 200 the
   true reach is 202 and **nothing fails** — `pass_summary()`'s only consumer
   checks the GLOBAL list, not numbers. Raise the margin **and add a test**
   asserting `margin ≥ MAX_CAVE_HALF_W + VAULT_RIND`.
2. **World-edge rejection nearly doubles**, 9.0% → 19.7% of draws at
   half-width 200, which fights round 5's presence win directly. Measure the
   no-cave rate; if it regresses, **that is a finding** — do not quietly
   raise `vault_density` to hide it.
3. **`MIN_SYSTEM_CELLS = 80`** is 0.62% of today's grid and 0.12% at 64k
   cells; it stops meaning "is this a system at all". Scale it to the drawn
   envelope.
4. **`MAX_CEILING_SPAN = 36` is far below the load limit** — the load model
   clears ~200–334 for a 6-deep roof (`Reports/load-model-fit-review.md`).
   At 400 wide its teeth become a regular picket fence, which is the size-cap
   landmine again. Raising it means editing a hardcoded literal at
   `tests/worldgen.rs` (search `MAX_CEILING_SPAN`) **with the load-model
   derivation in the commit message**.
5. **The floor taper is a per-cavity sweep**, so a 400-column cavity makes
   floors systematically flatter and thinner. A picture regression to look
   for, not a correctness one.

The depth band is **fine**: 160 tall fits every preset (canyon's worst column
allows ~197), though `cy` freedom drops from ~206 rows to ~116.

**If A2 does not fit the budget, cap lower and record the measurement.** This
is the one task allowed to come back saying "this size does not fit".

---

## A3 — fewer, thicker, tapered

Owner, verbatim: the columns *"are all 1 pixel thick. They should have a
taper and be thicker but fewer of them"*, and the cave is *"totally full of
stuff"*.

Round 5's formation half is a **regression**: heavy-tailed heights (4a),
clustering (4b) and spacing 1–2 each passed their own bar and compose into a
picket fence. Replace that composition:

- **Fewer.** Count per system back to **12–20** (round 5: 29–36).
- **Thicker.** Median base width **≥ 3**, range **3–8 cells** (today 1–2).
- **Tapered.** Width must fall from base to tip — a real profile, not
  round 5's single secondary column at 3/5 height, which is a rectangle with
  one step.
- Keep 4a's height distribution (p90 ≥ 10, max ≥ 25).

Phase 0 already gave you `flowstone` and `spar` — formations are **scenery**
now, so they never block the player. Density costs nothing in walkability, so
**spend the budget on size, not count.**

**Also yours**: relax the *"a formation must never bridge floor to ceiling"*
rule and `SPELEO_PAIR`'s almost-meeting hack. True columns are legal now that
they do not split the passage. This was deliberately **not** done in Phase 0,
because relaxing it without the taper produces *more* one-cell columns.

**Bar**: the three numbers above, **and a composed render judged as a
whole** — post it as a card. Three separately-verified bars are exactly what
produced the picket fence.

---

## A4 — the round-5 leftovers

**Task 5 (waterline formations) is sitting uncommitted** in
`.claude/worktrees/r5` — about 117 lines across `src/worldgen/passes.rs` and
`tests/worldgen.rs`, it builds, it was never gated, and the agent that wrote
it died before committing. Rescue it (`git -C .claude/worktrees/r5 diff`),
port it onto your branch, gate it, and either land it against its bar (**≥8
formations at a waterline per flooded system**; last measured ~1.5) or write
a finding explaining what it actually achieves.

**Task 6 (ceiling grain) stays blocked** until A1 passes. Structural grain on
the ceiling of a cave nobody can walk into decorates the wrong problem.

---

## Findings

*(Write here when a spec above does not survive contact with the code. One
entry per surprise, with the numbers. A finding is a success, not a
failure.)*
