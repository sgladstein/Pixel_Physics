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

### A6-2 — A1's own bars pull apart at this envelope size exactly the way the beauty review predicted; reachability was already above bar, contrast is not, and 3.0x is not reachable here without A2

Starting point (A6-1): reachable 64-76%, contrast p95/med ~2.0x, median
open column 30 -- the round-3/4 baseline, already clearing the 50%
reachability bar with 14-26 points of headroom, and well short of the 3.0x
contrast bar. So A1 here is the opposite problem from the one it was
written to guard against (round 5 traded all its reachability for
contrast): the task is to buy contrast without spending the reachability
already banked.

**Swept `cave_probe field=1` before building anything**, per the task's own
instruction, across `CAVE_CELL` in {52,46,44,40,38,36,35} and
`CAVE_THRESHOLD` in {0.34..0.15} at 5-8 field seeds each (`t3=0`, `squash`
held at 2.0 first). Two findings from the field alone:

1. **Squash is not a lever here.** Sweeping 2.0 -> 1.0 at fixed cell/t
   moves median column *up* slightly (less vertical compression -> taller
   passages) and leaves contrast flat (~2.0-2.7x either way). Left
   unchanged.
2. **Cell and threshold trade reachability for contrast along one curve,
   and the curve tops out well short of 3.0x.** Lowering threshold alone
   (cell=52) pushes median contrast from ~2.4x at t=0.34 to ~2.6-3.0x at
   t=0.15 -- but median open column falls from 18-31 to 8-17 over the same
   range, i.e. *below* `PLAYER_HEIGHT` in most seeds by t=0.18. Shrinking
   cell alone (t=0.34 fixed) moves contrast within noise (1.5-3.6x,
   overlapping the baseline's own spread) while leaving median roughly
   where it was. The two together (round 5's own move) is what actually
   crushes reachability -- shrinking cell narrows every passage's absolute
   width in cells regardless of threshold, and round 5's lower threshold
   on top of that narrowed cell compounded it, which is the field-level
   version of the 0-8% reachable result already on record.

**Validated candidates against real world builds** (the field is a proxy;
`reachable by player %` only exists in a built world) at `cell=40, t=0.28`
(reachable 48-52%, arid/wetland *miss* the 50% bar) and `cell=38, t=0.24`
(reachable 22-41%, a clear miss everywhere, and contrast did not improve
over the milder settings below it -- 2.0-2.17x, *worse* than the setting
finally shipped). Both rejected on measurement, not guessed away.

**Landed: `CAVE_CELL 52.0 -> 46.0`, `CAVE_THRESHOLD 0.34 -> 0.28`,
`CAVE_SQUASH` unchanged at 2.0.** Real-build sweep, 16 seeds x 5 caved
presets:

| preset | reachable (was) | contrast x100 (was) | median open column (was) |
|---|---|---|---|
| arid | 62% (70%) | 247 (200) | 21 (30) |
| canyon | 61% (70%) | 214 (203) | 21 (30) |
| rolling | 63% (76%) | 219 (203) | 21 (30) |
| terraced | 62% (75%) | 216 (203) | 21 (30) |
| wetland | 58% (64%) | 214 (196) | 20 (22) |

**Bar 1 (reachable >= 50% p50) met on every preset, with 8-13 points of
headroom.** **Bar 2 (contrast >= 3.0x) is not met anywhere** -- every
preset improved (14-24%, arid's 200 -> 247 is the largest single move) but
tops out at 2.1-2.6x, matching the field ceiling found above. Per the
task's own "watch" clause, this is the finding to report rather than a
failure: **at this envelope's ~12 Worley lattice cells (up from ~9 at the
old `CAVE_CELL`), 3.0x contrast is not achievable without giving up
reachability below 50%** -- confirming the beauty review's own diagnosis
("the envelope holds ~9 Worley lattice cells... there is no anatomy to
have") is still the binding constraint after this round's retune, just
less severely. Median open column (20-21, was 22-30) stayed comfortably
above `PLAYER_HEIGHT` (14) with real margin, which is the number this task
exists to protect and the one round 5's own retune broke.

**Worlds-with-a-system count held**: arid 13/16 -> 13/16 none, canyon
11/16 -> 10/16, rolling 9/16 -> 9/16, terraced 9/16 -> 10/16, wetland
7/16 -> 8/16 -- within a seed or two of baseline in either direction, not
a systematic loss.

A2's larger envelope is the actual next lever for contrast: more lattice
cells at the same `CAVE_CELL` (grow the envelope rather than shrink the
cell) is the one direction this sweep did not touch, because A1's scope is
the three field constants and A2 owns envelope size. If A2 does not close
the contrast gap either, that is worth a fresh field sweep at the larger
size before concluding 3.0x needs a different mechanism.

Gates: `cargo test --release` (646+8+2+30 = 686 passed, 0 failed, across
lib/main/determinism/worldgen), `cargo test --release --test worldgen` (30
passed, 8 ignored --
seal, at-rest, roof-span, determinism and speleothem-bridge tests all
still green at the new constants), `cargo clippy --all-targets -- -D
warnings` (clean), `scripts/worldgen_sweep.sh compare` (0 counters moved --
that harness builds at 512x320, where `vaults` never fires at all per
`tests/worldgen.rs`'s own comment, so it is a no-op gate for this change
specifically, not a validation of it; `cave_probe` at 2048x640 is the
instrument that actually exercises this). `cargo run --release --example
ascii`'s worst-frame number was contended by a concurrent session's own
`cargo test` on this shared machine during this run (33.7ms / 113ms /
72.7ms across three measurements in the same few minutes, all on
unrelated ants/organism scenes) -- noise from machine load, not this
change: `vaults` is genesis-only and cannot affect a per-frame organism
simulation cost regardless of its own tuning.

