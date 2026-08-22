# Documentation audit

**Status:** findings, no documentation changed by this pass. Written to be
read as a work order — every finding carries the file, the line, the evidence
that establishes it, and the proposed correction, so any one of them can be
vetoed on its own.

**Scope:** `README.md`, `PLAN.md`, `CLAUDE.md`, `wiki/*.md`, `Reports/*.md`,
`research/*.md`, `docs/`. Roughly 1.9 MB across six surfaces. Rust doc
comments in `src/` are deliberately out of scope — `CLAUDE.md` declares source
comments load-bearing, so auditing them is its own project and a careless edit
there deletes knowledge rather than correcting it.

**Audited against:** `565e4b4`. Note that the tree was being committed to
*during* this audit — `565e4b4` ("Answer population-dynamics §7b: ecological
LOD") landed mid-pass and added `Reports/ecological-lod-design.md`, and
`wiki/the-gnome.md` is currently modified in the working tree by another
session. Nothing in this report stages or touches either.

## The headline

The documentation is not uniformly rotten, and it is worth saying which parts
are actually in good order before listing what is wrong.

**`CLAUDE.md` is clean.** Every symbol it names was verified to exist:
`field::noon_equivalent_light`, `update::step_monolithic`,
`render.rs`'s `FieldOverlay`/`OrganismOverlay`, `Material::sweep_reach`,
`material::SOIL_SATURATED`, `diffuse_resource`,
`schedule_structural_check_around`, `plant.rs`'s `thicken`, `pipe_ratio`.
Its load-bearing arithmetic holds — `MAX_REACH == CHUNK_SIZE / 2` is 32 and 64
at `chunk.rs:11,42`. Even the trap it warns about is real and still there:
`assets/species/tree.ron` does hold two `crowding_weight` lines, at 120 (30.0)
and 264 (0.0). Every file in its collision table exists. It has one stale
sentence, §F1 below, and nothing else.

**Cross-file link integrity is effectively perfect.** Extracting every
markdown link target and every backticked repo path across all markdown and testing each
for existence produced no true breaks. Every apparent one is a glob
(`Reports/*.md`), a `file.rs:123` line reference, a relative wiki link, or a
throwaway example the text itself annotates as deleted after use
(`examples/debug_explosion.rs` and friends).

**The wiki is well written and mostly accurate**, but it is not as fresh as it
claims, and two pages are physically damaged — §W1 and §W2 below.

The damage is concentrated where `CLAUDE.md` already predicts it would be: in
the contested files. `README.md` has drifted furthest, and `PLAN.md`'s front
matter has aged.

---

## §R — `README.md`

### R1. The architecture map covers about half the simulation *(highest value)*

`README.md:111-140` lists 14 modules under `src/sim/`. There are 27, and a
whole `src/worldgen/` module the map never mentions.

Unlisted, with current sizes:

| Missing | Size | What it is |
|---|---|---|
| `src/sim/organism.rs` | 147 KB | the shared cell-typed organism substrate |
| `src/sim/creature.rs` | 140 KB | creatures — worm, ants, beetle |
| `src/sim/structural.rs` | 134 KB | anchor-distance relaxation, M17 |
| `src/sim/rigid.rs` | 111 KB | chunk bodies, M8 |
| `src/sim/liquid.rs` | 101 KB | heightfield liquid bodies (test-only today) |
| `src/sim/load.rs` | 97 KB | the load/torque failure criterion |
| `src/sim/player.rs` | 87 KB | the gnome, M9 |
| `src/sim/weather.rs` | 48 KB | rain, snow, wind, lightning |
| `src/sim/pheromone.rs` | 38 KB | the two ant trail channels |
| `src/sim/brain.rs` | 25 KB | the caged creature brain |
| `src/sim/decay.rs` | 16 KB | ash to soil weathering |
| `src/sim/rng.rs` | 14 KB | the position-keyed jitter helpers |
| `src/worldgen/*` | ~120 KB | 6 files — the entire M10 worldgen redesign |

Also absent from the `src/` top level: `sky.rs` (38 KB), `tunables.rs`
(39 KB), `hud.rs` (13 KB), `lib.rs`.

That is roughly 1 MB of source invisible to the one document whose job is
orientation. A reader who trusts this map does not learn that `load.rs`,
`structural.rs` or `worldgen/` exist at all.

**Fix:** rewrite the map against the real tree, one line per module in the
existing voice. Verifiable mechanically afterwards — see the verification
section.

### R2. The Controls table is wrong in three places and missing six keys

Checked against every `KeyCode::` arm in `src/main.rs:308-517`.

**Wrong:**

- `README.md:34` — *"`F2` `F3` `F4` Cycle the gnome's **jump feel**, **water
  feel** and **spoil mode**"*. The actual mapping is scrambled:
  `main.rs:413-415` binds **F3 = movement feel, F4 = water feel, F2 = spoil
  mode**. Read in the table's own order, F2 is spoil mode, not jump feel.
- `README.md:49` — *"`Enter` Save the selected tunable back to its `.ron`
  file"*. `main.rs:436` binds `Enter` to `pin_selected()`; **saving moved to
  `S`** (`main.rs:409`, `KeyCode::KeyS if show_tunables => save_tunable()`).
  Both halves of the row are wrong, and `S` appears nowhere in the table.
- `README.md:48` — *"`←` / `→` Adjust the selected tunable's live value
  (**only while the panel is open**)"*. `main.rs:429-430` adds an unguarded
  fallthrough: with the panel closed the arrows call `adjust_pinned()`. The
  parenthetical is now false, and the pinned-tunable feature it implies is
  undocumented.

**Missing entirely:**

| Key | Binds to | Documented in |
|---|---|---|
| `Y` | `found_colony` — places an ant colony | `wiki/ants.md`, not README |
| `F6` | `next_seed` — roll a new world | `wiki/the-world.md`, not README |
| `F7` | `cycle_preset` | `CLAUDE.md` and `wiki/the-world.md`, not README |
| `F8` | `previous_seed` | `wiki/the-world.md`, not README |
| `F9` | `cycle_chain_mode` — how far damage travels | `wiki/structural-collapse.md`, not README |
| `L` | `cycle_organism_overlay` | nowhere |

`Y` is the entry point to the entire ant colony feature. Every one of these
except `L` is documented somewhere else in the repo, so the Controls table is
now the *least* current key reference the project has.

### R3. `README.md` contradicts itself on M14

`README.md:97` (Materials): *"**The M14 schema** (combustion, phase change,
reactions) is defined and loadable — see `oil.ron` ... — but **nothing reads
it yet**; that is the update logic M14 adds."*

M14 shipped. `README.md:484` is its status section, `src/sim/fire.rs` is 45 KB
of the logic in question, and `README.md`'s own `## Status` describes fire
working. The Materials section comes 380 lines *earlier*, so this is what a
reader hits first.

**Fix:** rewrite the paragraph in the past tense and point at the M14 status
section.

### R4. `README.md` contradicts itself on whether M17 covers `Plant`

`README.md:1044-1046` (M17 status): *"Known simplification: `MaterialKind::Plant`
(trees, moss) is explicitly out of scope for this milestone ... and
`structural::tick` only ever activates for `MaterialKind::Solid`."*

`README.md:396` (field grid), same file: *"structural integrity (M17) now
covers `Plant` as well as `Solid`: `wood.ron` finally has the
span/`breaks_into` numbers ... a burnt-away trunk base brings the rest of the
tree down."*

The code settles it — `structural.rs:1513` is
`matches!(kind, MaterialKind::Solid | MaterialKind::Plant)`. The M17 section's
"known simplification" is the stale one.

### R5. "seven materials loaded from data"

`README.md:1523`. There are 21, compiled via `include_str!` at
`src/sim/material.rs:975-1023`: stone, sand, gravel, ash, water, oil, smoke,
wood, moss, worm, corpse, soil, deadwood, rubble, leaf, rootwood, seed, snow,
ant, nest, beetle.

### R6. The material schema example is missing two `kind`s

`README.md:72` documents `kind: Powder,  // Solid | Powder | Liquid | Gas`.

`Plant` and `Creature` both exist and both carry real semantics that the `.ron`
files themselves explain: `wood.ron` says `kind: Plant`, not `Solid`, and
`worm.ron` says `kind: Creature` — *"never moves via the CA sweep, exactly
like `Plant`"*. Confirmed across `assets/materials/`: 4 `Powder`, 2 `Liquid`,
1 `Gas`, 2 `Solid`, 4 `Plant`, 3 `Creature`.

Omitting them from the schema comment hides the single most important fact
about how the sweep dispatches.

### R7. Four shipped features have no status section, and the sections are in scrambled order

The milestone sections run **M12/M13, M14, M7, M15, M6, M5, M16, M17, M18**,
then three overnight-run sections, then **M8**. No reader can navigate that.

Absent entirely:

- **M9, the gnome** — `src/sim/player.rs` (87 KB), a `TunableGroup::Player`, a
  whole `wiki/the-gnome.md` page, and four keys in the Controls table.
- **M10's worldgen redesign** — `src/worldgen/` (6 files),
  `assets/worldgen.ron`, `tests/worldgen.rs` (28 KB), `wiki/the-world.md`, and
  the F6/F7/F8 keys.
- **Weather** — `src/sim/weather.rs` (48 KB), `wiki/weather.md`.
- **The ant colony** — `src/sim/pheromone.rs`, `src/sim/brain.rs`,
  `assets/species/ant.ron` (16 KB), `wiki/ants.md`, the `Y` key.
- **M19 visual polish** — planned at `PLAN.md:3234`; `src/sim/sky.rs` (38 KB)
  and the day/night/stars/moon work described in `wiki/world-cycles.md` are
  clearly it, landed and undocumented here.

`## Status` (`README.md:1520`) stops at M18 for the same reason.

**Fix:** reorder into numeric order, write the five missing sections, bring
`## Status` forward, and add a table of contents. This is the bulk of the
Stage 2 work.

### R8. `README.md` never links to `wiki/`

The only file in the repo that references `wiki/` is `CLAUDE.md`. The
player-facing documentation set — 11 pages describing what every mechanic
actually does — is unreachable from the project's front door.

**Fix:** one line in `README.md` pointing at `wiki/README.md`.

---

## §W — `wiki/`

### W1. Two pages carry a duplicated freshness note and lost a heading

`wiki/world-cycles.md` has `*Current as of: this build.*` at **line 3** and
again at **line 64**. Between them sits `## The sky` — the only heading in the
file. After the second one sits a *second* day/night introduction and the decay
material, with no heading of its own. The page is two versions of itself
stacked.

`wiki/structural-collapse.md` has the same defect in its simplest form: a
stray trailing `*Current as of: this build.*` as the last line of the file.

This is a known failure mode here — `bb20167` is literally *"Repair a heading
my own edit doubled."* These two look like the rest of that incident.

**Fix:** delete the stray notes; give the orphaned day/night and decay content
its own headings under `wiki/world-cycles.md`'s existing title.

### W2. Two pages state that nothing produces smoke. Explosions do.

`wiki/liquids-and-gases.md:21-24`: *"nothing in the simulation currently
produces it on its own; an explosion's crater, for instance, is left clean and
empty once the dust settles rather than smoke-filled."*

`wiki/explosions.md:29-30`: *"Once the dust and glow settle, the crater is left
open and empty — nothing currently fills it back in."*

Both are wrong, and the code is emphatic about it:

- `explosion.rs:175` — `smoke_fraction: 0.18`, a live tunable.
- `explosion.rs:572` — `backfill_smoke` writes `material::SMOKE` into the
  crater.
- `explosion.rs:797` — a guard test,
  `an_explosion_leaves_smoke_behind_in_its_crater`.
- `README.md:593` says the same: *"the crater is backfilled with smoke, giving
  `SMOKE` its first producer anywhere in the simulation."*
- `app.rs:289` even carries a comment about a change that *"silently removed
  SMOKE's only producer"* — the project has already been burned by this once.

So `README.md` and the wiki state opposite facts about the same mechanic, and
the wiki is the one that is wrong.

### W3. `wiki/the-world.md` contradicts itself two sections apart

`## What is not here yet` says: *"Rivers, springs, **rain and evaporation** —
water currently sits where it was generated and does not cycle. Caves, and
**plant cover that arrives with the world** rather than being planted by
hand."*

The same page's own `## Life arrives with the world` section, forty lines
earlier, says *"A new world already has moss and tree seeds in it."*

And rain and evaporation both shipped: `src/sim/weather.rs`,
`src/sim/evaporation.rs`, and an entire `wiki/weather.md` whose sections are
`## Rain` and `## Standing water dries up`.

**Fix:** cut rain, evaporation and starting plant cover from the not-yet list;
keep rivers, springs and caves.

### W4. `wiki/fire-and-heat.md` omits the most flammable material in the game

The page names oil (0.5), deadwood (0.35) and corpse (0.15). Actual
`flammability` across `assets/materials/`:

```
leaf 0.75   ant 0.6     worm 0.6        oil 0.5    seed 0.5
beetle 0.5  moss 0.4    deadwood 0.35   wood 0.25  corpse 0.15   rootwood 0.02
```

**`leaf` at 0.75 is the most flammable material in the world** and the fire
page never mentions it — nor wood, moss, or that creatures burn at all. For a
page about what catches fire, in a world with forests, that is the central
omission.

Related: the page says the inert materials *"never catch, never melt"*, and
never mentions that anything melts. Exactly one material does — `snow.ron`
has `melting_point: 2.0` — which is documented on `wiki/weather.md` instead.
Not wrong, but the reader is told melting exists nowhere.

### W5. "Current as of: this build" cannot go stale

Nine of eleven pages carry it verbatim. `wiki/README.md:10-11` promises these
notes as the staleness signal — *"if something reads as wrong, the game has
moved past this page"* — but a note that says "this build" reads as current
forever, including on the pages that turned out to be wrong in W2, W3 and W4.
Only `ants.md` dates itself in any way (*"Fresh as of the first ant
milestone"*).

**Fix:** replace with a dated note, e.g. `*Current as of: 2026-08-19.*`. This
is what makes `CLAUDE.md`'s wiki-freshness rule enforceable instead of
aspirational.

### W6. Minor: `snow` and `seed` are `Powder` and appear on no powder page

`wiki/powders.md` covers sand, gravel, ash, soil, corpse, deadwood and rubble.
`snow.ron` and `seed.ron` are both `kind: Powder`. Snow's behaviour is fully
covered on `wiki/weather.md`, so this may be deliberate; a cross-reference
would close it either way.

---

## §P — `PLAN.md`

`PLAN.md` is a build plan with an append-only progress log, and most of it is
*supposed* to read as a point-in-time record. These findings are confined to
the front matter, which is not.

### P1. The Stack table promises five dependencies that were never added

`PLAN.md:96-108`, headed *"All versions verified current as of Aug 2026."*

`Cargo.toml` actually contains: `image`, `notify`, `pixels`, `rayon`, `ron`,
`serde`, `winit`.

Listed and absent: **`rapier2d` 0.35, `mlua` 0.12, `earcutr` 0.5, `glam`,
`puffin`** (the last marked *"Add at M4"* — M4 is `✅ done`).

Three of those were declined on purpose, and `README.md:1382` says so:
*"No new dependency (`rapier2d`, `earcutr`, `glam`) has been added to
`Cargo.toml`, deliberately."* The table has no way to show that, so it reads
as a list of what the project uses.

The same table's milestone labels are also off: it says **"Scripting (M9)"**
and **"Triangulation (M7)"**, but `PLAN.md`'s own milestone list has M9 =
Character physics, **M11** = Lua scripting, M7 = Free particles, **M8** =
Rigid bodies. The labels are from an older numbering.

**Fix:** add an "in use / planned / declined" column and correct the two
milestone labels. Do not delete the planned rows — the intent is the point of
a plan.

### P2. Status markers stop after M4

`M1 ✅ done`, `M2 ✅ done`, `M3 ⚠️ half done`, `M4 ✅ done` — and then nothing.
M5, M7, M9, M12–M18 all shipped and carry no marker at all, so the milestone
list reads as though the project stalled at M4 and the reader has to cross to
`README.md` to find out otherwise.

**Fix:** carry the markers through, sourced from `README.md`'s status sections.

### P3. Five trailing session-handoff sections, unstated state

`PLAN.md` ends with sections at 3376 (*"Code review findings — parked, not
started"*), 3504, 3645, 4220 and 4655, several of whose titles already embed a
state (*"done, not started"*, *"started, on branch `plant-substrate-v2`"*,
*"landed, plan updated"*). Branch-scoped titles age badly once the branch
merges.

**Fix:** a one-line current state at the head of each, and a table of contents
so they are reachable without scrolling 5,400 lines.

### P4. Recorded precedent, worth keeping visible

`PLAN.md:3489-3504` already documents a previous instance of exactly the
README defect in R3/R4: the file simultaneously describing issue #4 as
implemented and *"not yet fixed"*, a *"correction to a correction"* that stood
for some time. This audit's R3 and R4 are the same failure recurring, which is
an argument for the mechanical checks in the verification section rather than
another manual sweep.

---

## §F — `CLAUDE.md`

### F1. The determinism warning is itself now stale

`CLAUDE.md:359` — *"**Determinism is required** (same-build, per `PLAN.md`) —
it was reversed from 'not required' and **some older comments still say
otherwise**."*

Searched across `src/`, `examples/`, `tests/` and every markdown file. There
are exactly four hits, and all four correctly describe the reversal:
`tests/determinism.rs:3`, `PLAN.md:69`, `Reports/coupling-research.md:307`,
`Reports/emergent-world-architecture.md:14`. **No stale comment survives.**

**Fix:** drop the trailing clause. Keep the requirement.

### F2. Minor: the Commands block omits the two scripts CI gates

The block lists `cargo test`, `cargo clippy`, `ascii` and `filmstrip`.
`.github/workflows/ci.yml` also gates **`bash scripts/acceptance.sh`**, and
`CLAUDE.md`'s own conventions demand a seed sweep before changing a procedural
model — which is `scripts/seedsweep.sh`. Both are referenced elsewhere in the
file but neither is in the block a reader copies from.

Also minor: CI runs clippy as `--all-targets --release --locked`; the block
omits `--release`. Harmless, but the two differ.

---

## §D — `Reports/`, `research/`, `docs/`

### D1. `Reports/` has no index, and about half of it is unreachable

41 reports, ~1.4 MB. Referenced from `README.md`, `PLAN.md` or `CLAUDE.md`:
about 20. The rest are findable only by listing the directory — including all
eight `tree-*` reports (~200 KB), `building-rethink.md`, `destruction-plan.md`,
`m9-gnome-character-plan.md`, both `prior-art-*` surveys,
`weather-handoff.md`, `underground-definition.md`, `load-model-fit-review.md`,
`plant-species-authoring.md` and the brand-new `ecological-lod-design.md`.

The good news is that an index is nearly mechanical: **almost every report
already opens with a `**Status:**` line** — *"proposal, not built"*, *"design
only"*, *"research, no code changed"*, *"not started"*, *"direction agreed with
the owner"*. The metadata exists; nothing aggregates it.

**Fix:** `Reports/README.md`, one line per report — subject, and state taken
from the report's own header. This is the single highest-value item in Stage 3.

### D2. `Reports/load-concentration-review-response.md` answers a document not in the tree

Its line 4 cites `Reports/load-concentration-review.md` at `b4fb357`. No such
file exists. **Fix:** recover it from history if it was ever committed
(`git log --all --diff-filter=D -- 'Reports/load-concentration-review.md'`),
otherwise note in the response document that the review it answers is external.

### D3. `docs/future-directions.md` is unreferenced and reads as current

Nothing in `README.md`, `PLAN.md` or `CLAUDE.md` links to it. It opens
*"Written after M3"* but then states in the present tense: **"`Cell` is exactly
4 bytes and full."** `Cell` is 12 bytes (`cell.rs:498` asserts it), widened
twice since — a history `README.md:431-457` tells properly.

A reader who finds this file has no signal that its central premise expired two
milestones ago.

**Fix:** a `**Status:** historical — written at M3` header, plus a line in
`Reports/README.md` or `README.md` so it is reachable at all.

### D4. `research/` is in good order

Three files, all clearly scoped as raw source material for M16/M18/M19, all
referenced from `PLAN.md` and `README.md`. No action.

---

## Proposed order of work

Stage 2, content and light restructure, in descending value:

1. **R1** architecture map — the orientation fix, and mechanically verifiable.
2. **R7** missing milestone sections + numeric reorder + table of contents.
3. **R2** Controls table — three corrections, six additions.
4. **W2, W3, W1** wiki: the smoke contradiction, the self-contradiction, the
   two damaged pages.
5. **R3, R4, R5, R6, R8** the remaining README corrections.
6. **W4, W5, W6** fire coverage; dated freshness notes.
7. **P1, P2, P3** PLAN front matter; **F1, F2** CLAUDE.

Stage 3:

8. **D1** `Reports/README.md` index, then the archive move.
9. **D2, D3** the missing review; the `docs/` header.

## Verification

Three of these findings are recurrences of defects this project has already
fixed once (R3/R4 per P4; W1 per `bb20167`), which argues for checks that run
rather than another manual sweep. Two are cheap:

**Architecture-map completeness** — diff the module list in `README.md`
against the real tree:

```sh
comm -3 <(grep -oE '^  [a-z_]+\.rs' README.md | tr -d ' ' | sort -u) \
        <(ls src/sim/*.rs | xargs -n1 basename | sort -u)
```

**Link and path integrity** — extract every markdown link target and every backticked
repo path from all markdown and test each for existence. It reports no true
breaks today; it must report none after the `Reports/archive/` move, which is
the check that catches a bad `git mv`.

Both belong in `scripts/`, run by hand rather than in CI — a docs check that
fails a build on a link typo would be its own kind of tax.

## Working notes for whoever executes this

`README.md`, `PLAN.md` and `CLAUDE.md` are all on `CLAUDE.md`'s contested
list, and this tree is being worked in concurrently right now — `565e4b4`
landed mid-audit and `wiki/the-gnome.md` is currently dirty from another
session. So: one commit per contested file, landed immediately rather than
held across a session, explicit paths only, and never `git add -A`.

---

# Addendum — re-ranked for the actual consumer

**Added after the findings above, on an owner steer:** this project is
developed entirely by Claude, so the primary consumer of every document here
is a **fresh agent session with no context** — arriving repeatedly, paying
tokens to read, and acting literally on what it finds. That reframing does
not change any finding above; it changes their weights, and it surfaced six
new ones a human-reader audit missed.

### A1. There is no index of what has been tried and reverted *(now the top finding)*

A ten-agent sweep over source comments, `Reports/`, and `README.md`
extracted **501 distinct tried-and-reverted entries**, most with a recorded
reason and many with the condition under which the rejection held. By area:
plants 121, structural 76, destruction 73, creatures 53, liquids 45, field
32, rendering 28, weather 18, powders 13, worldgen 12, scheduler 9,
parallelism 8, other 13. Nothing aggregates them; `CLAUDE.md` is a
hand-curated sample held small by its always-loaded budget. Re-attempting a
known dead end costs a whole session, and this repo's own history shows it
happening. **Fix:** `Reports/dead-ends.md` — an index (1–3 lines per entry:
tried → happened → condition to re-test → where recorded, symbol refs), one
section per area, with a `CLAUDE.md` routing row. Boundary rule:
`open-bugs-handoff.md` owns "is this broken?", `dead-ends.md` owns "was this
tried?".

### A2. PLAN.md is two documents fused: a read surface and a write-only archive

Inbound-reference census: ~120 references from 44 files reach `PLAN.md`,
and exactly **2** target the 2,127-line progress log (`README.md`'s M5
anchor link, and a passing mention in `rigid.rs`'s module doc that survives
a split unchanged). Splitting the log into a sibling `PLAN-log.md` (stub
heading kept in place) shrinks `PLAN.md` ~40% and makes its live half cheap
to read whole. Owner call.

### A3. CLAUDE.md is accurate but under-organized for the thing that reads it every session

A dedicated review produced 13 recommendations: three true duplications
(~180 tokens), two narrations moveable behind pointers (~250 tokens), a
missing task-to-rule topic map, five Method rules invisible to a heading
skim, a 93-line unclustered Conventions list, the seed sweep mandated but
never named in the Commands block, and the routing rows the two new indexes
need. Voice-preserving throughout. Owner call on depth.

### A4. Read cost has measurable hot spots

No wiki page and no part of `CLAUDE.md` has a heading-less span over 120
lines; `README.md` has three (worst: the field-grid section, 224 lines) and
`PLAN.md` twelve (worst: 602 lines, in the progress log). Subheadings on the
README three are cheap; the PLAN.md ones dissolve with A2.

### A5–A6. Two owner requirements added to the executing plan

**Drift protocol** — this audit is pinned to `565e4b4` and the tree moves
daily: execution opens with a delta pass over `565e4b4..HEAD`, re-verifies
any finding touching a changed file before applying it, edits shared files
surgically rather than regenerating them, and generates both indexes from
the tree at execution time. **Maintenance regime** — `scripts/docscheck.sh`
(link/path integrity, map-vs-tree diff, doubled-freshness-note detector,
report-missing-from-index, undated notes) plus same-commit conventions in
`CLAUDE.md`: dated freshness notes, new report → index line, revert →
dead-ends entry, shipped feature → README status section.

### Re-ranked order of work

**Tier 1 (routing/recall):** `Reports/README.md` index → `dead-ends.md` →
`CLAUDE.md` routing rows → R1 architecture map → `docscheck.sh`.
**Tier 2 (literal truth):** R3, R4 → W2, W3, W1 → R5, R6, R8 → W5, W4, W6.
**Tier 3 (read cost / remaining):** R7 missing sections + TOC (**no**
wholesale reorder — grep navigation makes it churn without payoff on a
contested file) → README subheadings → R2 → P1–P3, F1–F2.
**Tier 4 (owner calls):** A2 split, A3 depth, and archive — where the
recommendation **reverses** to skip: with per-report status in the index,
archiving buys an agent nothing and the `git mv` breaks `Reports/` paths
held in other sessions' uncommitted worktrees.

Findings R2's key list and W-series demotions/promotions above supersede the
"Proposed order of work" section where they differ; the findings themselves
stand unchanged.

---

# Post-freeze delta (2026-08-21, execution start)

The audit sat for two days before execution; this records what moved, per
the drift protocol.

- **The canonical branch is now `main`** (`9430346`): CI gates `main`, the
  worktree procedure names `origin/main`, and `master` is a mirror slated
  for retirement. This overhaul executes from `origin/main` (`8a8bed6`).
- **`origin/main` gained the review queue** (`.claude/skills/review/`, a new
  CLAUDE.md section "Getting the owner's judgement", a knowledge-table row,
  and `scripts/review.py` in the Commands block). The §F findings were
  re-checked against that version and stand; the CLAUDE.md organization
  review's line references shift but its recommendations are unaffected.
- **D2 is resolved**: `Reports/load-concentration-review.md` exists on the
  unmerged `load-share` branch at exactly the cited `b4fb357`. It arrives
  when that branch merges; it is listed in the index as in flight rather
  than cherry-picked from someone else's branch.
- **In-flight documentation on unmerged local branches**, inventoried so it
  is not lost: `creatures-m18` (creature-evolution-plan.md; ants wiki and
  creature-direction updates), `load-share` (load-concentration-review.md
  and -reply.md; CLAUDE.md census-timing sections; structural-collapse wiki
  edit), `plant-branch-angle` (branch-angle-and-the-width-bound.md,
  genetic-variability-study.md, plant-appearance-design.md,
  plant-night-session-handoff.md; CLAUDE.md edit — **the four documents
  merged 2026-08-22 with the plant lines, which carried them; the branch
  itself has not**), `plant-ecology-design` (plant-evolution-design.md,
  plant-implementation-plan.md, plant-work-split.md, a new wiki/plants.md
  — **all merged 2026-08-22**), `plant-genome` / `plant-substrate-v2`
  (plant-genome-design.md and three companions — **all merged
  2026-08-22**), `perf-audit` (performance-audit.md, untracked), `perf-lock`
  (measurement-under-contention.md untracked, plus an uncommitted CLAUDE.md
  edit adding scripts/perf.sh). Twenty-plus `origin/claude/*` cloud branches
  also exist; the index's in-flight section and docscheck's
  missing-from-index check are the net that catches all of these at merge.
- **The CLAUDE.md pass is re-scoped for the in-flight forks**: three
  branches edit regions the approved reorganization would move (load-share
  inserts sections beside the oscillator passage; plant-branch-angle
  rewrites next to the git-reset passage; perf-lock edits the knowledge
  table). Additive and line-local items land now (routing rows, duplicate
  merges, heading promotions, topic map, F1/F2, the maintenance
  conventions); the Conventions re-clustering and the two narration
  move-outs are deferred until those branches merge, recorded as a queued
  follow-up so the approval is not silently dropped.
- **`wiki/the-gnome.md` is excluded from this pass** — it is dirty with
  another session's uncommitted work in the shared checkout.
