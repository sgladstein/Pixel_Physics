# Worldgen round 6, Track B — rock formations at the player's scale

**STATUS: APPROVED, execute in order.** You are the implementation session
for the surface rock-formation track. The planning session that wrote this
remains the reviewer: **you land small, image-backed commits; you do not
judge your own visuals.** When a spec here does not survive contact with the
code, **stop and write a finding** rather than improvising.

Read first: `CLAUDE.md`; `Reports/worldgen-erosion-design.md` — **the whole
Status section and the two 2026-08-20/21 addenda at the end**, which contain
every measurement this track is built on; `Reports/world-review-2026-08.md`
§7.

## Why this round exists

**The scale band between texture and landform is empty.** Prominence measured
at four reaches over the whole world (`viewshot boulder=1` prints this):

| preset/seed | reach 5 | reach 15 | reach 30 | reach 60 | relief | sky above high ground |
|---|---|---|---|---|---|---|
| canyon s7 | 3 | 8 | 8 | **39** | 136 | 86 |
| canyon s6 | 2 | 10 | 10 | 19 | 97 | 104 |
| rolling s1 | 2 | 4 | 8 | 18 | 65 | 87 |
| terraced s1 | 1 | 4 | 6 | 19 | 73 | 88 |

The world has **landforms** (reach 60 — canyon s7's mesa) and **texture**
(reach 5, 1–3 cells). At reaches 15 and 30 — exactly where a rock formation
lives — the tallest thing in the entire world is 4–10 cells. Not rare:
**absent**. No tor, no stack, no pinnacle, no standing residual anywhere.

`PLAYER_HEIGHT` is **14**, so a cell is roughly 4–5 inches and the owner's
stated range (5/10/20/40 ft) is **12 / 25–30 / 50–60 / 100–120 cells**. With
86–104 rows of sky above the highest ground, the world can host all of it.

**Owner's directive on the distribution**, verbatim: *"full spread weighted
small, but this is also something that should vary between biomes. some
should have large, some small, some mixed, some round boulders, some
angular, the real world variability."* So: **one continuous heavy-tailed
draw ~12→120 cells weighted small, and its parameters are a regional
property, not a global constant.** A spec that ships one global size draw has
missed the directive even if the sizes are right.

## Ground rules

- **Branch** `claude/worldgen-formations-r6`, worktree
  `.claude/worktrees/form-r6`, cut from
  `claude/game-world-gen-planning-h12713`. One task, one commit.
- **Files you own**: a **new module `src/worldgen/residual.rs`**, the
  `boulders` function in `src/worldgen/passes.rs`, `src/worldgen/erosion.rs`,
  `assets/worldgen.ron`, and boulder/talus tests in `tests/worldgen.rs`.
  **Do not touch**: any *cave* function in `passes.rs` (`vaults`,
  `cave_system`, `carve_cave_void`, `settle_cave_void`, `erode_breaches`,
  `grow_monumental_chamber`, the speleothem block) — **Track A is live in
  those right now** — nor `src/render.rs`, `src/sim/*`, `examples/*`, or the
  contested files (`src/app.rs`, `PLAN.md`, `README.md`, `CLAUDE.md`,
  `wiki/*`).
  Isolation in a new module is the point: two agents editing `passes.rs`
  overnight is the collision CLAUDE.md says has cost real hours.
- **Reserved `noise::Purpose`**: **`Residual = 30`**, **`ResidualShape = 31`**.
  27 is reserved for `CeilingGrain`; 28–29 belong to Track A. Append only.
- **Do NOT retune the erosion rate constants** (`SOFT_CREEP`, the soft/hard
  stable angles, `HardnessField`'s shape). They were set by eye across a
  whole tuning session. If your work appears to need them moved, **that is a
  finding**, not an edit.
- **Before every commit**: `cargo test`; `cargo clippy --all-targets --
  -D warnings`; `cargo test --test worldgen`; `cargo run --release --example
  ascii` with no worst-frame regression; `scripts/worldgen_sweep.sh`
  re-baselined.
- **Run `pass_ablation` after any pass change.**
- **Post review cards** for anything judged by eye:
  `python3 "$(git rev-parse --path-format=absolute --git-common-dir)/pixel-physics-review/bin/review.py"`,
  protocol at
  `git show origin/claude/agent-testing-platform-6efu1e:.claude/skills/review/SKILL.md`.
  Fire and forget; do not block on an answer.

**Landmines**: `.ron` edits do nothing until rebuild · registries are
append-only · determinism, no `HashMap` iteration influencing behaviour ·
**a size cap must bound work, never gate whether something happens** · sweep
an order statistic, never one seed · never `git add -A` · don't strip
load-bearing comments · `cargo fmt` is all-or-nothing, do not run it ·
generated terrain must arrive at rest and sleep · **check every bar against
`PLAYER_HEIGHT` before adopting it.**

---

## B1 — prove the diagnosis before building. **Do this first.**

Plan-space erosion does not merely fail to create formation-scale relief —
**it removes it.** Max prominence at reach 15, by age:

| | age 0 | age 0.8 (shipped) | age 2–3 |
|---|---|---|---|
| canyon s7 | **10** | 5 | 4 |
| canyon s1 | **10** | 3 | 3 |
| rolling s1 | **8** | 4 | 5 |
| terraced s7 | **8** | 4 | 3 |

Age 0 — no erosion at all — has 2–3x more than every shipped age, in every
preset and seed tried. (`viewshot age=N` overrides `world_age` per render.)

Meanwhile `worldgen-erosion-design.md`'s own *What emerges* section promises
"hoodoos/spires: a hard cap band over soft rock" with a lateral-coherence
floor. **It was specified and it is not there.**

**Your job**: instrument `erosion.rs` (measurement only, reverted before you
commit) to answer one question — *does any column ever reach residual height
mid-run and then get removed by the stable-angle rule?* Print a per-iteration
max-prominence trace, or a count of columns that exceeded N and later fell
below it.

**Deliverable**: a finding, either way. It decides whether B2 is a **new
mechanism** or a **retune**, and building B2 before answering it is guessing.

---

## B2 — residual landforms in the empty band

Tors, stacks and pinnacles as **residuals** — what was left standing while
its neighbours retreated — shaped by which strata band was hard
(`HardnessField` already knows). New module `src/worldgen/residual.rs`, plus
one entry in the `PASSES` table with an honest finite margin.

- **Size**: one continuous heavy-tailed draw, ~12→120 cells, weighted small.
  Not a two-tier "common plus a rare landmark" scheme — the owner asked for
  the continuous spread real talus-and-tor country has.
- **Regional**: the draw's *parameters* come from the region `Character`
  (`src/worldgen/region.rs`), not a global constant. One country
  boulder-strewn and coarse, the next a few monuments with bare ground
  between, the next smooth.
- **Shape from process, not authorship**: `HardnessField` separates a
  flat-capped stepped residual (hard cap over soft) from a rounded dome
  (uniform rock, long weathering) from an angular blocky pile (frost shatter
  along bedding). Round vs angular is the owner's ask and it should be a side
  effect of which band survived — `design-philosophy.md` §2b's test.

**Bar**: prominence at reach 15 **and** reach 30 rises from 3–10 to
**p90 ≥ 20, max ≥ 60** over 16 seeds, **and regions visibly differ** — the
acceptance artifact is a **strip across regions**, not a histogram from one
world. Post it as a card.

**A representation limit, stated so you do not promise past it**: the erosion
plan holds one `h[x]` per column, so it **can** express a tor, a pinnacle or
a stack and **cannot** express an undercut — the mushroom cap that makes a
hoodoo read as a hoodoo, or a balanced rock. Those are realise-pass work, the
way `brows` already hangs an overhang the plan cannot hold. Do not promise
them from plan-space erosion.

**At-rest**: residuals are attached `Solid`, so they have no movement rule and
hold by construction at genesis. But a 50-cell residual is the first object a
player can plausibly undermine, and the design's `height ≤ 3x base width`
rule was written *"until measured otherwise"* and has never been measured.
A test that digs the base out from under one is part of this task.

---

## B3 — boulders at a believable size

Today a boulder is 2–5 cells wide and **1–2 cells standing proud** — the dome
is a full-height ellipse but only rows above ground are written, so visible
height is `round(height/2)` — against a 14-cell player, seating in **3 of 24
worlds**.

Three independent shrinks compound, and **none is structural**: the erosion
design's non-negotiable #3 says only *height ≤ 3x base width*; round 4's task
file read that as "2–5 wide, 2–4 tall"; and the implementation clamped
tighter still with `height.min(width)`, a 1x ratio where 3x was allowed. A
12x8 boulder is 0.67x.

- Re-derive the size from the **real 3x rule**.
- Seat in a **socket** rather than displacing two cells of cover.
- Contrast is **already handled** — the pass writes `FAMILY_RESISTANT`
  cap-rock deliberately. Do not "fix" it.
- The surface is **smooth** at this scale (prominence p99 = 1), so a 6–12
  cell dome is unmissable. It is not lost in terrain noise; it is simply tiny.

**Bar**: visible height **p50 ≥ 6, max ≥ 20** (`viewshot boulder=1` reports
it, and finds the boulder by asking the generator for its marker array rather
than guessing).

**Frequency is the LAST thing to touch**, and only after size. Round-4
finding R4-1 established that `brows` gets to the dome's air first and
refusing to punch through it is correct behaviour; `pass_ablation` measures
`brows` deleting 100% of boulders in four of six presets. **Making a two-cell
pimple eight times more common produces eight pimples.**

---

## Findings

*(Write here when a spec above does not survive contact with the code. One
entry per surprise, with the numbers.)*
