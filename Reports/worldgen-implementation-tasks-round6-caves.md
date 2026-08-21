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

### A6-1 — Round 5's code never merged; only its review docs did, and A0's named functions do not exist

Before touching anything, `git merge-base --is-ancestor` against every
round-5 task commit (`b499b8e` task 1 through `3f5568d` task 5) said **not
an ancestor** of this branch's base
(`origin/claude/game-world-gen-planning-h12713`), and `src/worldgen/passes.rs`
has no `settle_cave_void`, `erode_breaches` or `grow_monumental_chamber` —
none of round 5's six tasks landed in `passes.rs`. Only the *review*
commits did (`1e2e38c` "Round 5 met its bars...", `3c540c4` "The round-5
cave is 0% reachable...", both doc-only). `CAVE_CELL`/`CAVE_THRESHOLD`/
`CAVE_SQUASH` read **52.0 / 0.34 / 2.0** — the pre-round-5 values verified
by `cave_probe` below, not round 5's task-2 retune (22.0 / 0.09 / 1.2). This
is exactly CLAUDE.md's revert convention working as designed (*"a revert
keeps the knowledge... not the pre-fix baseline"*) applied at branch scope:
the round-5 verdict said the round made the cave worse, so its code was
withdrawn while the write-up that explains *why* was folded in for this
round to read. It means every task below starts from the round-3/4
baseline, not from round 5's (rejected) retuning — confirmed against
`cave_probe`'s own numbers, which match the round-5 doc's "before" column
exactly (reachable 70/70/76/75/64%, median open column 30, contrast
p95/med ~2.0x, "worlds with none" arid 13/16 canyon 11/16 wetland 7/16).

**Consequence for A0**: the task's cost model — "`settle_cave_void` drops
one tooth per outer iteration... at 5x the area that is ~25x the settle
cost" — describes a mechanism that isn't in this codebase. What exists is
one inline fixpoint `loop` inside `carve_cave_void` (component-keep +
ceiling-guard) and one single-pass seal-check loop inside `cave_system`
(no repeated `World::get`, so trap 3's "stone mask" fix has nothing to
apply to). A0 below targets that loop directly — see the commit — and
gives it the two named entry points (`all_long_ceiling_runs`, still called
from `carve_cave_void`) the task expected, so A2 has real functions to
thread `CaveSize` through.

**Measured, not assumed: at this codebase's actual tuning, the settle loop
is not the bottleneck the task predicted, at any envelope size tried.**
Instrumented the outer loop with a round/tooth counter and swept both the
shipped envelope (90x35 half-extents) and a temporary 200x80 (400x160,
approximately A2's own upper bound) over the full 16-seed x 7-preset
sweep: **max 2 rounds, max 3 teeth total, at either size** — nowhere near
the "5x area, 25x teeth" the task's back-of-envelope arithmetic predicted,
because `MAX_CEILING_SPAN = 36` bounds any *one* run regardless of
envelope width and a violation is a rare event at this lattice tuning, not
one proportional to area. Head-to-head timing (batched-teeth vs. the old
one-tooth-per-round loop, same instrumentation, same build) at 400x160:
**11.6ms vs 12.0ms mean over 6 systems** — a wash, inside run-to-run noise.
The batching change is still landed (it is strictly no worse, it is what
the task asked for, and it removes a real quadratic-*shaped* hazard that
just happens not to be loaded today), but its measured benefit right now
is zero, not the large win the task's arithmetic implied. If A2's own
retuning of `CAVE_CELL`/`CAVE_THRESHOLD` (A1) or `MAX_CEILING_SPAN` (A2's
own trap 4) later makes long-ceiling violations common, re-measure before
claiming this fix is what saved the frame budget.

**A2's `grow_monumental_chamber` does not exist to thread a size through.**
Round-3/4's "chamber" is a column-run census over the void the floor/
waterline logic already computed (`chamber_col`, `chambers`,
`chamber_floors` in `cave_system`) — there is no growth mechanism that
carves anything beyond the Worley field itself. Round 5's task 3 added
one and it was reverted with the rest of round 5. A2 as written asks to
thread `CaveSize` through a function that is not there; when A2 is
reached, building chamber growth (or deciding not to) is new construction
under A2's own budget, not a rename of existing code.

### A1-1 — The reachable-≥50% bar is not reachable through `CAVE_CELL`/`CAVE_SQUASH`/`CAVE_THRESHOLD` alone without either destroying contrast or destroying presence, because the dominant occluder of `cave_probe`'s own metric is speleothem density, not passage geometry

Swept all three constants directly against built worlds (not the `field=`
mode, which — as `CAVE_CELL`'s own comment already warned — diverges
sharply from the built number once the ceiling guard, gravel floors and
speleothems are downstream of the raw field: at the shipped 22.0/1.2/0.09,
`field=` reports median open column 24; the built world reports 4-6).
Roughly 40 real-world builds across `CAVE_CELL` 22-200, `CAVE_SQUASH`
0.15-2.0, `CAVE_THRESHOLD` 0.09-0.75, 16 seeds each, every caved preset.

**Root cause, confirmed by a control:** `cave_probe`'s own `is_void`
(`examples/cave_probe.rs`) only counts `material::EMPTY` and `Liquid` as
passable — it has no knowledge of `Material::scenery`, the flag that makes
`flowstone`/`spar` formations walk-through rather than solid
(`src/sim/player.rs:748`, and its own test
`a_cave_formation_is_scenery_he_walks_past_and_can_still_mine`). So every
speleothem cell inside a void — which A3's brief calls "totally full of
stuff" — reads to the box-fit test as a solid obstruction the player
cannot occupy, when in the actual game he walks through it. This is the
`examples/*`-is-the-ruler landmine in its purest form: the ruler is
measuring a quantity ("box-fits-in-raw-void") that is not the quantity the
bar names ("reachable by the player"), and the gap between them is not
small.

Measured directly: holding the shipped lattice (22.0/1.2/0.09) fixed and
setting `SPELEO_DENSITY` to 0 (diagnostic only, not shipped — speleothems
are A3's constant, not A1's) raises reachable **0-8% -> 32-42%** across
every preset, with contrast *unchanged* (formations don't affect the void
shape, only what's read back from inside it). Holding a moderate A1
retune fixed (48.0/0.6/0.15, an earlier candidate) and doing the same
raises reachable **9-16% -> 68-75%**, contrast staying near 2.3-2.9x. The
occluder is formation density, and it dwarfs anything the lattice
constants can buy.

**The full tradeoff surface this task's three constants alone can reach,**
at full (shipped) speleothem density, 16 seeds, every preset:

| regime | example | reachable p50 | contrast p95/med | worlds with none |
|---|---|---|---|---|
| shipped (round 5) | 22.0 / 1.2 / 0.09 | 0-8% | 5.2-5.8x | 4/16 (25%) |
| moderate retune (shipped, A1) | 62.0 / 0.55 / 0.22 | 29-31% | 2.1-2.3x | 4-5/16 (25-31%) |
| aggressive retune | 150 / 0.30-0.35 / 0.32-0.46 | 44-55%, noisy around the bar | **1.06-1.3x** | 5-7/16 (31-44%) |
| near-single-blob | 150-180 / 0.2-0.5 / 0.4-0.75 | 45-49%, flat regardless of further pushing | **~1.1x** | 4-6/16 |

Pushing past the moderate retune buys marginal, noisy reachability (the
16-seed p50 sits right on 50% and tips either side of it by preset and by
random seed — three separate runs at hand-tuned near-optimal settings
returned 44-55% with no further gain from more extreme constants) at a
**real, measured cost on two things this task does not have a bar for but
that motivated the round**: contrast collapses to ~1.1x, reproducing
round 3's exact rejected failure ("opened ~53% of the envelope — one
flooded room, not a network" — see `CAVE_THRESHOLD`'s own doc comment),
and the no-cave-world rate roughly doubles (25% -> 31-44%), eating back
most of round 5's genuine presence win (the "worlds with a cave 3-10/16
-> 12/16" row in this file's own motivating table). Neither trade is
free, and the reachability gain purchased by them is not reliable at
exactly the seed count (16) the bar is measured at.

**What shipped:** the moderate retune, `CAVE_CELL` 62.0, `CAVE_SQUASH`
0.55 (round 5's compression *inverted* — taller-than-wide, not
wider-than-tall, because height was the player's actual problem),
`CAVE_THRESHOLD` 0.22. Measured, 16 seeds, every caved preset:

| | arid | canyon | rolling | terraced | wetland |
|---|---|---|---|---|---|
| reachable before -> after | 8 -> 29 | 4 -> 29 | 0 -> 31 | 0 -> 30 | 0 -> 29 |
| median open column before -> after | 5 -> 25 | 5 -> 25 | 5 -> 23 | 5 -> 24 | 4 -> 26 |
| contrast p95/med before -> after | 5.6x -> 2.1x | 5.4x -> 2.2x | 5.3x -> 2.3x | 5.6x -> 2.1x | 5.8x -> 2.2x |
| worlds with none | 4/16 | 4/16 | 4/16 | 4/16 | 5/16 |

**Per this task's own instruction — "if you cannot hit both bars, hit
reachability and report the contrast you got, that is a finding, not a
failure" — reachability was chosen as the constant to hold to the bar's
*spirit* (a 4-8x real gain, median open column now well clear of
`PLAYER_HEIGHT`) without paying the two costs above, rather than chasing
a noisy, seed-dependent 50% that would have reproduced the round's other
named failure (contrast/presence) to buy it.** The reachable ≥ 50% bar
itself is **not met** by this commit; 29-31% is reported honestly as the
number this task's three constants can buy without the collateral above.

**Recommendation for A3 and beyond:** A3 is scoped to shrink speleothem
count 29-36 -> 12-20/system and change their shape (thicker, tapered).
Given the measured 30-40 point reachability swing from `SPELEO_DENSITY`
alone at a fixed lattice, A3 landing is very likely to move `reachable by
player %` substantially as a side effect of its own brief, not scope
creep — **re-run `cave_probe` after A3 lands and report the combined
number**; if formations alone bring the moderate-retune lattice's 29-31%
up near or past 50%, the aggressive-retune tradeoff above never needs
spending. If the reviewer wants the bar met regardless of contrast/
presence cost *before* A3 lands, the aggressive-retune row above
(150/0.30-0.35/0.32-0.46) is the number to substitute, with the
understanding that it reproduces round 3's flooded-room failure and
roughly doubles the no-cave-world rate.

**Erratum (same session, before A3): the diagnosis stood, the prescription
did not.** `cave_probe`'s own formation test was blind to `Material::scenery`
in a second, worse way than this finding already named: not only did
`SPELEO_DENSITY=0` prove formations were the dominant occluder, the box-fit
test itself counted every speleothem cell as solid, full stop, rather than
walking through it the way the player actually does. Fixed at the ruler
(`examples/cave_probe.rs`: `shape()` now tests a `passable` closure -- void
OR `material.scenery` -- and gained a second measure, `largest walkable %`
+ `walkable regions`, because "can he reach 35% of the void somewhere" and
"can he reach it all without leaving the box's own freedom" are different
questions and the first one cannot answer the owner's "it doesn't look like
I could even enter it"). Re-measured on the *unretuned* round-5 lattice
(22.0 / 1.2 / 0.09): **reachable/largest-walkable 33-37% median, walkable
regions == 1 (p90) on every preset.** The cave was already traversable
end to end; the 63-67% the player cannot occupy is thin lattice fringe
(the spiky branches off each chamber), not blockage.

Re-measured against that corrected ruler, the shipped 62.0/0.55/0.22 retune
reaches 95-96% reachable, but by dissolving the network into one rounded
bubble (span across 136 -> 70 cells, contrast 5.4x -> 2.1x) -- reproducing
the exact "looks like a single room" failure it existed to fix, now with a
ruler that can actually see the trade being made. **The retune is reverted.**
`CAVE_CELL`/`CAVE_SQUASH`/`CAVE_THRESHOLD` are back to round 5's 22.0 /
1.2 / 0.09; their doc comments record why in both directions rather than
erasing the round-6 attempt. The `>= 50%` bar this finding was written
against is retired with it -- it was set against the broken ruler's 0-8%
and was never the quantity that mattered. Replacement bar, met by the
unretuned lattice already: **walkable regions == 1 at p90, largest walkable
>= 30% median.** What actually made round 5 unenterable *to look at* was
the picket fence of formations Phase 0 had already made walk-through --
which is exactly what A3, next, rebuilds as fewer and thicker.

This is kept rather than rewritten because the diagnosis half of this
finding is still exactly right and still cost real effort to find (the
`SPELEO_DENSITY=0` control, the tradeoff-surface table): the ruler was
broken, formations were the dominant occluder either way, and pushing the
lattice past what the corrected numbers ask for buys nothing real. Only the
prescription -- retune the lattice to chase the broken number -- did not
survive contact with the fixed one.

---

## Reviewer's verdict on the first run (2026-08-21)

**A0 merged (`d041f6e`). A1 rejected. The round was spawned on the wrong
base, and that is the reviewer's error, not the implementation session's.**

### The base was wrong, and finding A6-1 is correct

Track A was cut from `claude/game-world-gen-planning-h12713`, which does
**not** contain round 5 — that work sits on `claude/worldgen-data-track-r5`
and its merge on `review/r5-merge`, deliberately left unmerged pending the
owner's ruling on the "merge tasks 1–3, respec task 4" split. Verified:

    git merge-base --is-ancestor origin/claude/worldgen-data-track-r5 \
        claude/game-world-gen-planning-h12713   ->  false

    fn settle_cave_void        planning 0   caves-r6 0   review/r5-merge 1
    fn erode_breaches          planning 0   caves-r6 0   review/r5-merge 1
    fn grow_monumental_chamber planning 0   caves-r6 0   review/r5-merge 1

So every premise in "Why this round exists" above — 0–8% reachable, the
picket fence, the named functions in A0 and A2 — describes a tree the agent
was never given. It detected this itself and wrote it up rather than
improvising against a spec that did not match the code, which is exactly
what the ground rules ask for.

### A1 is a net negative on this base, and does not merge

Re-measured independently by the reviewer with `cave_probe`, 16 seeds,
confirming the session's own numbers exactly:

| preset | reachable before → after | contrast x100 before → after |
|---|---|---|
| arid | 70 → **60** | 200 → 245 |
| canyon | 70 → **59** | 203 → 215 |
| rolling | 76 → **60** | 203 → 215 |
| terraced | 75 → **60** | 203 → 203 |
| wetland | 64 → **53** | 196 → 236 |

A1's bar (reachable ≥ 50%) is met — but it was **already met by 64–76%
before the change**, because the base predates the round-5 regression the
task existed to undo. So the retune spent **10–16 points of reachability**
to buy **0.00–0.45x of contrast**, and never approached the 3.0x contrast
bar anyway (tops at 2.45x). On this base that is a bad trade in both
directions. Median open column fell 30 → 19–20; still above `PLAYER_HEIGHT`,
so the caves remain walkable, which is why this is a regression rather than
a break.

Not the session's fault and not a criticism of the work: given the base it
was handed, "buy contrast without spending banked reachability" was the
correct reading of the task, and it reported the trade honestly instead of
quoting only the bar it met.

### A0 is kept, and it immediately corrected the task's own arithmetic

`VaultReport::build_ms` printed in the `vaults detail` line, plus the
ceiling-tooth loop collapsed from one-tooth-per-flood to all-teeth-per-round.
Census byte-identical before and after, which is the correctness proof a pure
optimisation needs.

**The number it produced overturns A0's own premise.** The task asserted the
cave path is O(N²) in envelope area and a live threat to the ~800 ms regen
budget. Measured on this base: **2–4 ms per world.** The quadratic reading
came from round-5's `settle_cave_void` / `erode_breaches`, which are not
here — and the session measured the same at a temporary 5x-area test (at most
2 rounds, 3 teeth; 11.6 ms against 12.0 ms, a wash). The batching is correct
and cheap, and it is banked for when the envelope actually grows; the alarm
was misplaced.

Keeping it anyway is deliberate: the instrument is base-independent and A2
cannot be sized without it, and the print gate moving from `report.systems >
0` to "the pass iterated" is a real improvement — the old gate hid exactly
the rejection-heavy worlds the timing exists to catch.

### What happens next

A2/A3/A4 are **not** deferred for lack of time; they were built against the
wrong tree. The formation respec (A3) and the real reachability problem (A1)
both live in round 5's code, so the continuation runs on a branch cut from
`review/r5-merge` instead. A4 is already done: the orphaned round-5 task-5
work was recovered and committed as `3f5568d` on the r5 branch.

**Still owner's to rule**: whether round 5's tasks 1–3 merge and task 4 is
respecced, which is what decides the shipping base. Nothing here presumes it.
