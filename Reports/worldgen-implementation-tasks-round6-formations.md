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

### B1 — it is not "formed then destroyed"; nothing ever forms at all

Instrumented a copy of `erode`'s loop body (canyon and rolling, seed 7,
2048 columns, age 1.0 — measurement only, reverted before this commit) to
print max prominence at reach 15 every 20 iterations, and to record, per
column, its peak value across the whole run versus its value at the final
iteration. The question was literally *does a column ever cross into
residual territory mid-run and then get knocked back down* — a lifecycle,
not a snapshot.

**Answer: no column ever peaks above its iteration-0 value. Ever.**

| | it 0 (pre-erosion) | it 100 | it 300 | it 599 (age 1.0) | peak-ever (any iteration) |
|---|---|---|---|---|---|
| canyon s7 | 8.34 | 7.45 | ~4.3 | 4.24 | **8.34** |
| rolling s7 | 5.00 | 4.55 | ~4.3 | 4.24 | **5.00** |

Both presets: max prominence at reach 15 **decreases monotonically across
every printed sample**, and the "columns that peaked > 15 and later fell
below half their peak" counter — the direct test for the hypothesized
lifecycle — is **0 of 2048, in both presets**. Nothing ever peaks above 15
in the first place, so nothing can be "knocked back down from" it.

**This changes the diagnosis from what `worldgen-erosion-design.md`
hypothesized.** The design doc's guess was that a residual *does* form
transiently — presents a near-vertical face — and the stable-angle rule
shaves it down on the next iteration before anyone sees it. That would
still be "the mechanism is present but too eager." What is actually
happening is upstream of erosion entirely: **the raw pre-erosion
heightfield** (`Terrain::elev`, before `erode` touches a single column)
never contains a column that stands 15+ cells proud of both flanks 15
columns out. Its own ceiling is 8.34 (canyon) / 5.00 (rolling) — already
below the reach-15 p90≥20 bar B2 has to hit, before any erosion runs at
all. Erosion then makes this strictly worse (creep+stable-angle pull the
already-small bumps down further, converging to ~4.2-4.4), but it did not
create the deficit; it inherited one that was already there in the
multi-octave hill/terrace/dune stack that builds `elev`.

**Decision this settles**: B2 cannot be reached by retuning
`THERMAL_STABLE_HARD_BONUS` or any other erosion rate (per the ground
rules, not attempted) — there is no transient spike for a gentler rate to
spare, at any rate. Nor can it be reached by "protect what erosion finds
promising", because erosion is never offered a residual-scale candidate to
protect. **B2 has to construct residual geometry directly** — a
purpose-built pass that decides where a residual stands and writes it,
using `HardnessField` for shape, rather than a rule that hopes one
survives the relaxation. This is what B2 below does.

### B2 — a per-column ground row is the site centre's, not the column's own

First implementation of `residual.rs` hoisted `ground_y` (and the elevation
`ground_e` derived from it) once from the site's *centre* column and reused
it for every column across the whole footprint. On sloped ground — which a
footprint up to ~150 columns wide will cross often in `canyon` — this seats
every column relative to a row that is not its own local surface: on the
downhill side the residual's base floats clear of the real hillside, on the
uphill side it buries into it. The collect-verify-write seal usually still
rejects the worst mismatches (a wrong seat cell reads as empty or as
existing massif and the whole site aborts), but not always, and
`a_forced_residual_world_arrives_at_rest` caught the remainder directly:
6-10 cells adrift after 120 frames, on the very first seed tried. Fixed by
reading `ctx.plans[lx].surface_y` per column inside the paint loop (the same
convention `passes::boulders` already uses), keeping the centre-column
elevation only for the shape/height decision, which is legitimately about
the site as a whole rather than any one column. **Which object a rule
evaluates** — this time "whose ground row" rather than "whose span" —
is the same question `CLAUDE.md`'s method section keeps asking, over a new
mechanism this time instead of an old one.

### B2 — the 3x aspect rule, measured against undermining: one in five ends up floating, and that is not this pass's bug

`a_residual_survives_its_base_being_dug_out` digs a residual's base out with
the real mining primitive (`World::paint_capsule`, the same call the
player's own dig makes) and measures the outcome rather than asserting one,
because whether the result *should* be a collapse is `load.rs`'s claim, not
`residual.rs`'s, and `Reports/load-model-handoff.md` §1 states plainly that
load/torque failure is **not started** — what exists today evaluates
failure per cell against its own span, the exact defect that document
exists to replace.

Measured over 18 canyon seeds (`residual_density` forced to 3.0 so enough
seeds seat one to dig under), each dug and settled for 480 frames (the
ordinary 120-frame at-rest bar undersells a genuine collapse: one case had
1,985 cells still adrift at 180 frames and needed until ~400 to fully
settle — a debris pile takes longer to finish sliding than an undisturbed
generated world does):

| outcome | count | of 15 checked |
|---|---|---|
| collapsed (no longer reads as solid stone) | 6 | 40% |
| still reachably anchored (the dig missed its real footing) | 6 | 40% |
| reads as solid stone with **no path to any anchor** | 3 | 20% |

So roughly one undermined residual in five ends up in exactly the state
CLAUDE.md's own gotchas warn is easy to manufacture by accident and hard to
notice: `Solid` (so a player can stand on it, walk into it, never sees it
fall) while `structural::compute_world_distances` has already given it
`u16::MAX` — the pass's own honest "cannot reach an anchor" value. It is a
stable state, not a transient one (re-checked to 2000 frames on the seed
that produced it, unchanged from frame ~400 on). **This is not a defect
`residual.rs` introduced.** The same is true of any `Solid`/`attached`
terrain in this engine today — an ordinary massif overhang undermined the
same way would read identically — because nothing between here and
`load.rs`'s still-unbuilt failure step converts "cannot reach an anchor"
into "comes down" on its own; `Reports/load-model-handoff.md` is the
document already tracking that gap. What round 6 adds is simply the first
measurement of how often a *residual specifically* lands in it (~20% at
this dig geometry), which the load-model work should have on hand once it
picks the step back up — a residual is exactly the "first object a player
can plausibly undermine" the task file asked this test to check for, and
this is what checking it actually found.

### B2 — the p90 bar is not met, and it is not a density problem

The bar is prominence at reach 15 *and* 30, **p90 >= 20, max >= 60**, over
16 seeds. Max is comfortably met at every density tried: **73-95** across
three settings. **p90 is not**, and does not respond to density the way a
coverage shortfall should:

| `residual_density` | reach 15 p90 | reach 15 max | reach 30 p90 | reach 30 max |
|---|---|---|---|---|
| 0.8 (first shipped) | 0 | 73 | 1 | 90 |
| 1.6 | 1 | 76 | 2 | 95 |
| 3.5 (4.4x) | 1 | 91 | 2 | 95 |
| **1.4 (shipped)** | **1** | **76** | **2** | **91** |

Quadrupling density moved p90 from 0 to 1. If this were a frequency
problem — too few residuals — density would move it close to linearly;
instead it saturates almost immediately. Three things are actually
happening, found by instrumenting placement directly (`RESIDUAL_PROBE=1`,
reverted before this commit, per the same measurement-only discipline as
B1):

1. **A residual's painted footprint is usually narrower than its nominal
   width.** `FlatCapped`/`AngularBlocky`'s base ring is `a * (0.55 + 0.45 *
   hard)` or `a * (0.35 + 0.65 * jitter)` — both can and often do draw well
   under `a` at the very first ring, so the visible base is already
   narrower than the width the aspect draw implied before any shrinkage
   toward the top even starts.
2. **A wide residual reads as a plateau, not a spike, to this specific
   probe.** Prominence at reach *r* compares a column to points *r* away on
   both sides; for a residual wider than `2r`, both flank samples land on
   the residual too, so the interior scores as flat as open ground and only
   the two true edges register. Canyon seed 1 at density 3.5 placed 18
   residuals summing ~350 columns of nominal width, and still barely moved
   p90 — most of that width was interior, not edge.
3. **Density increases pile more attempts into the same already-coarse
   regions rather than spreading coverage.** `residual_density *
   Character::formation` is evaluated once per 256-column placement window;
   a region that already drew several residuals mostly produces overlap
   rejections on further attempts (the collect-verify-write seal correctly
   declines a site sitting on another residual's own attached stone), while
   a smooth region (`formation` near 0) gets no more attempts at any global
   density — by design, since a smooth region is supposed to stay smooth.

None of these three is a bug — 1 and 2 are inherent to reading a
per-column heightfield prominence off a real, mixed-aspect distribution of
shapes, and 3 is the direct, working consequence of `Character::formation`
being regional rather than global, which is what B2 was asked to build.
Reaching p90 >= 20 by density alone would mean either abandoning "some
regions stay smooth" or packing coarse regions solid enough to read as
wallpaper rather than landmark country — neither of which is what the
owner asked for, and `CLAUDE.md`'s own conventions say a bar the engine
cannot yet hit should be recorded with the gap visible, not quietly met by
inflating a knob until a number moves. Shipped at `residual_density: 1.4`
(a modest lift from the first-tried 0.8, chosen for visible presence
without the diminishing-returns region-saturation the 3.5 trial showed),
with `residuals_lift_prominence_at_reach_15_and_30` left failing on p90 and
its doc comment carrying this table, rather than the test relaxed to pass.

**Open question for the owner or a later session**: is p90 the right
statistic here at all? A world that is honestly "some coarse regions, most
smooth" will never clear a percentile computed over the *whole* world,
almost by construction — the aggregate is diluted by every column of every
deliberately-smooth region. A per-region-character statistic (p90 measured
only within columns whose `Character::formation` exceeds some coarse
threshold) would answer the question B2 actually cares about — "does a
coarse region read as tor country" — without being sunk by the smooth
regions the owner explicitly wants to keep. That is a different metric
than the one written into this task file, so re-deriving it is a decision
for whoever reads this next, not something to have swapped in unasked.

### B2/B3 — a seated feature on soil can be structurally solid and still float

`tests/worldgen.rs::every_solid_is_anchored_and_no_liquid_carries_a_stale_
fill` failed on the default preset (rolling, seed 3) once B2 shipped: one
stone cell at (90, 113) had `aux == u16::MAX` -- present, attached, and
reading as ordinary massif, but with no path to any anchor at all.
Flood-filling the connected stone at that point found a 611-cell island,
bbox 13 columns by 46 rows, touching neither bedrock nor the world edge:
one entire residual, floating.

**The mechanism**: converting the single seat row (the column's own
topmost soil/sand/gravel cell) to attached stone is not the same as
*connecting* to the massif. `structural::compute_world_distances`'s
relaxation only walks *relaxable* (body) material -- ordinary soil is not
one -- so a residual's new stone layer sitting on top of an unconverted
soil blanket has no route down through that soil to the bedrock-connected
rock underneath it, however solid it looks. A residual wide enough that
every column of its footprint happens to sit over deep cover, with no
column's edge close enough to outcropping bare rock, floats entirely.

**This is not unique to residuals.** `boulders`' own socket had the
identical shape of bug, just harder to trigger at a boulder's smaller
footprint: it converted a *fixed fraction* of the visible height's worth
of rows below grade (`~30%`), which on a soil blanket deeper than that
fraction leaves exactly the same gap. Neither the boulder nor the
residual acceptance tests (`a_forced_boulder_world_seats_stone_and_
arrives_at_rest`, `a_forced_residual_world_arrives_at_rest`) catch this
class of bug at all -- both only check that nothing *moves*, and a
floating `Solid` never does, by construction. Only checking `aux !=
u16::MAX` (equivalently, that every solid reached an anchor) catches it,
which is exactly what `every_solid_is_anchored_and_no_liquid_carries_a_
stale_fill` already existed to do -- it is a pre-existing, general-purpose
gate that a new pass has to run under, not something either new pass came
with its own copy of.

**The fix, same shape in both files**: seat by walking down from grade
through consecutive soil/sand/gravel, converting each cell, until hitting
real (`ctx.stone`) rock -- not a fixed row count. A shape is contiguous by
construction, so any *one* column of the footprint threading all the way
to bare rock is enough to anchor the whole feature; the walk is bounded
(`MAX_SOCKET_DEPTH = 80`, generous headroom over every shipped
`soil_depth`) so a column whose cover is pathologically deep or that has
wandered into something unexpected still rejects the site rather than
looping. Landed in both `residual.rs` and `passes.rs::boulders` -- the
second before it could ever ship, since nothing had yet measured it there.

**Method note**: this was caught by re-running the *existing* test suite
gate after a change that looked, by every metric this track had built for
itself, complete -- prominence bar met on max, at-rest held, pass_ablation
clean. `cargo test --lib`'s full run is not optional scaffolding around the
task-specific tests; it is where a cross-cutting invariant like "every
solid reaches an anchor" actually lives, and it found a bug none of B2's
or B3's own measurements were shaped to see.

### B3 — size re-derived; the instrument named in this task file cannot show the fix

`boulders`' three shrinks are fixed: width redrawn 3-13 (from 2-5), height
drawn independently up to a real `3x` ceiling (from `height.min(width)`,
1x), and `b` used directly as the visible semi-axis instead of halved by
the dome-writes-only-the-top-half arithmetic. Both draws are skewed toward
their own top half (`sqrt` of the unit draw) after measuring that a
uniform draw put p50 at 4 against the bar of 6: a marker is a steep-drop
site by construction, so the tallest attempts are also the ones least
likely to find enough open air to seat in, and a uniform draw's
successfully-*seated* population skews small for exactly that reason
(confirmed directly, `BOULDER_PROBE=1`, reverted before this commit --
e.g. a width-7/height-19 draw at one site and a width-6/height-15 draw at
another both came back `sealed=false` the same run a width-5/height-14
one seated).

**Measured** (canyon, age 1.0, seeds 1..=600, 13 boulders seated):
**visible height p50 12, max 30** -- both bars (6 / 20) cleared with
headroom.

**The instrument this task named cannot demonstrate the second bar.**
`viewshot boulder=1`'s height print is `(1..=6).take_while(...)` --
capped at exactly 6 rows, the p50 floor this task set, so it can report
"at least 6" but never "20". `examples/*` is off limits to this track, so
`tests/worldgen.rs::a_seated_boulder_stands_at_a_believable_height` is the
uncapped measurement instead, and it needed one more fix on the way: a
naive version credited each raw shed-marker run's own centre column with
whatever resistant-family stone stood there, which double-counted --
a *rejected* run's centre can sit inside a wider *accepted* neighbour's
own footprint (up to its own half-width away, and width now reaches 13),
reading back that neighbour's tapered edge as if it were this run's own
short boulder. Confirmed by cross-checking every suspiciously short
reading against `BOULDER_PROBE=1`: the short readings' own draws were all
`sealed=false`, never written at all. Fixed by requiring every column of
a run's own raw marker range to show seated stone before trusting its
peak, and by measuring with `residual_density: 0.0` for this test
specifically -- `residual.rs` shares `strata_shade` with the ordinary
massif, so a residual's own family-3 cells are naturally speckled, not a
guaranteed run, and without isolating it a residual sitting near a boulder
site could contaminate the same scan.

**Frequency untouched**, per the task's own ordering: `boulders`' marker
rejection rate (still dominated by `brows`) was not touched, and
`pass_ablation`/`scripts/worldgen_sweep.sh compare` both show boulder
counts moving only within ordinary seed-to-seed noise, no counter past
+/-30%.


---

## Reviewer's verdict (2026-08-21) — merged, and it exposed two of my errors

**All three tasks landed and merge.** Gates re-run independently: `cargo
test` 648 + 31 passed, 0 failed; clippy clean. The one failing acceptance
test is `#[ignore]`d with an honest reason and its doc says why it was left
failing rather than relaxed — which is the repo's own convention, not a
dodge.

### B1 is the most valuable thing in the round, and it reverses my hypothesis

I wrote that erosion *removes* formation-scale relief and told the track to
find out whether the stable-angle rule was shaving residuals down after they
formed. Instrumented: max prominence at reach 15 is **monotonically
decreasing from iteration 0**, and **0 of 2048 columns** ever peaked above 15
and later fell.

So erosion never offers a residual to protect. The deficit is in the *raw
heightfield*, before a single erosion iteration runs — which means no rate
retune could ever have produced one, and the "hoodoos/spires fall out of the
differential rates" promise in `worldgen-erosion-design.md` was never
reachable from those rates. Exactly the finding B1 existed to get, and it
saved the round from tuning constants that could not have worked.

### The p90 bar was mis-specified, and that is my error

The bar was *prominence p90 ≥ 20 and max ≥ 60 at reach 15 and 30*. Measured
after: **max 73–95 (met), p90 1–2 (missed)**, and the session reported the
miss rather than quietly hitting one of the two.

**p90 was never achievable and should not have been set.** It is the 90th
percentile over *every column in the world*, so reaching 20 requires more
than a tenth of every world to be standing residual — which is wallpaper, and
the opposite of the heavy-tailed weighted-small distribution the owner asked
for. The right statistic is **p99**, and by that measure the pass works:
reach-15 p99 moved from ~4 to **18–26**, reach-30 max from 8–10 to **69–71**
(canyon s1/s7, verified independently). The session diagnosed the metric
correctly and the acceptance test is left failing against the wrong number;
**re-derive it to p99 before anyone reads that failure as a defect.**

### The floating-island bug is a real find, in two places

A residual seated entirely over soil converted only the *top* soil cell,
leaving a 611-cell island that renders as solid rock and is structurally
disconnected — caught by `every_solid_is_anchored_and_no_liquid_carries_a_
stale_fill`, which is the landmine test CLAUDE.md §6b keeps for exactly this.
The same bug was then found and fixed in `boulders`' socket before it could
ship independently. Two defects that would have reached the player as rock
that vanishes the moment anything disturbs it.

### B3 met both bars, and caught my instrument lying

Boulders: **visible height p50 16, max 30** over 600 seeds, against bars of 6
and 20, and against **1–2 cells** before. The three compounding shrinks are
gone (`height.min(width)`'s 1x clamp where 3x was allowed, and the dome's
semi-axis being implicitly halved).

`viewshot boulder=1` still prints "2 cells tall" for a seated boulder — **my
instrument, now stale**, because it walks up from `plans[cx].surface_y` and a
socketed boulder no longer starts there, and its height counter is capped at
6 rows regardless. The session was right not to touch `examples/` and right
to write `a_seated_boulder_stands_at_a_believable_height` as the uncapped
replacement. **Reviewer to fix the print.** That is twice in one night an
instrument of mine went stale under a change it was supposed to measure —
`cave_probe` looking for `stone`/`crystal` after formations became their own
materials was the first. Worth stating as a pattern: *a ruler that names a
material or a coordinate convention breaks silently when either changes, and
reads as "the mechanism is dead" rather than as "the ruler moved".*

### What is still wrong, and it is the same thing as the caves

Judged by eye on a four-shot traverse of canyon seed 7
(`target/filmstrips/r6b_residual_canyon_s7.png`): the scale is right and the
**shape is not**. The spires read as flat vertical slabs — straight sides,
uniform width, flat top — rising abruptly from flat ground with no talus at
the foot and no broken profile. The pair at the far left reads as two fence
posts.

That is the *same* failure the owner named in the caves ("all 1 pixel thick
... should have a taper"), in a different pass, which makes it worth naming
as a pattern rather than a bug: **this generator authors shape as extents —
a width and a height — and an extent-shaped object is a rectangle at every
scale.** A profile (width as a function of height, with a foot and a
weathered crown) is what both passes are missing. Posted to the owner as a
blind A/B; their verdict decides whether that becomes round 7's spine.
