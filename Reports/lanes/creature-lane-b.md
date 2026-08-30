# Lane B — sizing the sense before anyone builds it (E15)

**Branch `claude/creature-lane-b-vision-sizing`, cut from `origin/main` at
`e7b72e7`. Complete, pushed, PR open.**

| | |
|---|---|
| **the tree to build against** | **`790af73`** — the last commit touching anything but this note. Everything after it on this branch edits only this file, so `origin/claude/creature-lane-b-vision-sizing` and `790af73` describe the same code |
| the work | `18f79c6` — probe, report, index, instruments row |
| this note | `cf80c08` |
| `main` merged in | `00856c8` — the branch is **0 behind** |
| re-measurement after that merge | `790af73` |
| PRs | **#146 MERGED** (the probe and report) · **#151 MERGED** (corrections) · **#161 OPEN** — https://github.com/sgladstein/Pixel_Physics/pull/161 |
| the pattern to know about | **both merges took the head *before* the latest re-measurement.** #146 merged nine minutes after a correction was pushed; #151 the same. Each time the branch was restarted from `origin/main` and the work re-applied as a new PR, because a merged PR cannot carry it. If you merge #161, check whether this branch has moved since |

I had GitHub tools (`mcp__github__get_me` resolved), so I opened the PR
myself. The coordinator merged it.

**One commit missed that merge and is the follow-up.** #146 was merged at
`79a9daa` at 04:42 UTC; the third-tree re-measurement was pushed at 04:51,
nine minutes later. So `main` carries the report quoting a frame cost from a
tree that no longer exists (2.98 ms, 0.14% of a frame, ~358 predators). The
branch was restarted from `origin/main` and the commit re-applied there — a
merged PR is finished and cannot track new work. **The simulation code in
current `main` is byte-identical to the tree the numbers were taken on**
(`git diff bfb4ced origin/main -- src assets` is empty), so nothing needed
re-measuring for the follow-up. Cost fork: **built the probe and answered the
question** — the geometry needed no engine work, so nothing was blocked.

## What landed

| file | what |
|---|---|
| `examples/vision_probe.rs` | new, 1,286 lines. Line-of-sight geometry over `World::get`: five modes, three positive-control arms, an occlusion axis, a paired cost measurement, a rendered overlay |
| `Reports/creature-vision-sizing-2026-08-30.md` | the report — the recommendation, the numbers, and what they cannot answer |
| `Reports/README.md` | its index line (house rule, same commit) |
| `Reports/instruments.md` | the harness's row (see **Scope deviation** below) |

**No source file was touched.** Nothing in `src/`, nothing in
`assets/species/`, nothing in `examples/common/`, no `Cargo.toml` edit
(examples are auto-discovered — verified). Lane A's and Lane C's files are
untouched.

## The answer, in one table

**Build the sight sense at radius 64, all-round, seeing over the floor
litter.**

| decision | build it as | the number |
|---|---|---|
| reach | **64 cells** | the **p10** seed's beetle sees prey **0.08–0.28** of the time at r32 and **0.26–0.39** at r64, over three presets × 18 seeds. 32 → 64 is the largest single step **in that p10** at every preset — no longer true of the median, see below |
| shape | **all-round** | a ±60° cone costs a third of every sighting (r64 median 0.622 → 0.394) and saves nothing measurable |
| occlusion | rock and soil, **never floor litter** | 24% of beetle-ant pairs blocked at head height, **6.4%** one cell up — which recovers the whole transparent-world ceiling, on **every** tree measured. The blockers are seed, corpse, soil, litter — clutter, not landscape |
| foliage | **not a binary blocker** | `dense` costs half the sense (0.667 → 0.340) and no eye height buys it back |
| cost | **free at this scale** | 521 cells read per beetle per cast; **0.005–0.007 ms/frame**, 0.15–0.24% of `ascii`'s 3.06 ms mean, below the wall clock's floor. Under 10% of a frame only past a few hundred predators |

## Three things worth carrying past this lane

1. **The `range`/`los` pair is the whole instrument.** `range` says there was
   something to see, `los` says it could be seen; a probe printing only the
   second cannot tell a *reach* failure from an *occlusion* failure, and
   those want opposite fixes. Everything in the report falls out of that gap.
2. **`vs blind` overstates the cost thirtyfold, and only a third arm shows
   it.** `cast_fan` scans the world to find beetles — which the engine never
   does, since the scheduler dispatches a creature at its own position. The
   `locate` arm prices that scan out. It also prices one `World::get` at
   **15.6 ns**, which any other harness on this box can borrow.
3. **Occlusion makes the sense cheaper as well as weaker.** 8 → 64 is an
   eightfold radius for a sixfold read count, because rays die on the first
   blocker. Anyone proposing to relax occlusion for performance has it
   backwards.

## What I got wrong on the way, since it is cheap to inherit

- **The first control assertion was wrong and the control caught it.** It
  asserted `los == 1.000` at every radius on a bare floor; the animals *walk*,
  so the ant placed 4 cells away is inside r8 in a tenth of samples. The
  claim the arm supports is `los == range`, which is tighter.
- **The `per beetle` column divided by beetles *placed*.** Nine are stood up
  and 3–6 are alive — a 1.5–3× error, and the same trap `CLAUDE.md` records
  live in `ascii` and `forage_probe`. It divides by beetles *located* now,
  counted on the far side of the casting call.
- **"One cell of eye height recovers the entire ceiling" was a one-preset
  claim.** True on `wetland`; on `rolling` it recovers ~70% of the gap,
  because real relief blocks some lines. Running the second preset is the
  only reason the report does not overclaim.
- **Clippy 1.94 passed what 1.98 rejects**, exactly as `CLAUDE.md` records —
  two `% == 0` sites flagged `manual_is_multiple_of`, plus `ptr_arg` and
  `needless_range_loop`. Fixed; `cargo +1.98.0 clippy --all-targets -- -D
  warnings` clean. The fixes were verified behaviour-neutral by re-running
  `mode=control` and diffing byte-for-byte against the pre-fix output.

## Scope deviation, declared

The lane brief scoped me to the probe, a report, this note and one line in
`Reports/README.md`, "nothing else". I also added **one row to
`Reports/instruments.md`**. The repo requires it — that file's own rule is
"if you do build one, add its row here in the same change", and
`scripts/docscheck.sh` check 5 fires on an example with no row. It is one
table row in a file nobody else in this program is editing. `docscheck` is
clean with it and would not have been without it.

## The sense was built, and the prediction landed

`creature-sight-sense-2026-08-30.md` shipped the same day, taking every
recommendation: `PreyNear` and `PreyBearing`, a 16-ray fan at radius 64 from
one cell above the head. **Predicted 0.572 of samples with prey in sight;
built sense reads 0.50** over 8 seeds. Pursuit moves two independent far-side
counters together — mean sighted range 15.2 → 12.5 cells, prey caught
302 → 323.

**One prediction was wrong by 2x and the report now says so.** The shipped
sense reads 1,020–1,100 cells per cast against the 525 this lane priced,
because a real implementation tests prey in the *un-lifted* frame and
blockers in the *lifted* one — two walks of the fan where `cast_fan` made
one. A modelling gap in the harness, not a measurement error. Still 0.3% of a
frame, so the conclusion holds; §0b and §5 both carry the correction, and
anyone sizing a different sense off §5's table should double it.

## Open, for whoever picks this up

- **Radius 128 is not measured.** The curve is still climbing at 64 and the
  median beetle is 55–65 cells from the nearest ant, so 128 would deliver
  more. Whether a sense that is on nearly all the time is still a *search*
  cue is a design question, not a measurement.
- **An attenuating occluder is not priced.** The report argues foliage should
  shorten the effective radius rather than block, on the ethos law that an
  outcome is a distribution and not a binary. Nobody has measured what that
  costs or delivers.
- **A `wetland`-only oddity, unexplained:** `eye=3` has lower pooled blocking
  than `eye=1` (4.8% vs 8.6%) and a *worse* median `los` at r64 (0.613 vs
  0.667). On `rolling` the two are identical. Do not read the pooled column
  as ranking eye heights.

## Review queue

Card `20260830T021057007Z-18900e`, board `creatures`, **verified on the
`review-queue` remote branch** (`cards/` and both images present) — a
labelled A/B of the sight lines at eye=0 against eye=1, asking which reads
right for an insect on a forest floor. **Answered 2026-08-30:** *"I don't think there is a clear good
answer. Just pick one that makes sense to you."* No preference between the
two eye heights, so that choice is delegated and rests on the measurement —
eye=1, because it recovers the transparent-world ceiling on `wetland` and
~70% of the gap on `rolling` at a third of the blocking. Recorded in §4 of
the report so a later reader knows the recommendation was put to the owner
and where the decision actually came from. The radius recommendation never
depended on this card.

## Gates, on the merged tree

`cargo test --lib` **1,086 passed / 0 failed / 54 ignored** ·
`cargo +1.98.0 clippy --all-targets -- -D warnings` clean ·
`cargo build --release --examples` clean (all 41) ·
`scripts/docscheck.sh` clean · `scripts/contextbudget.py --gate` 9,748 B
under the ceiling.

## Reproducing every number in the report

```
cargo build --release --examples                 # NOT --release alone
./target/release/examples/vision_probe mode=control
./target/release/examples/vision_probe mode=survey seeds=18 preset=wetland   # also rolling, arid
./target/release/examples/vision_probe mode=survey seeds=18 settle=3000      # the placement control
./target/release/examples/vision_probe mode=occlusion seeds=18               # ~17 min
./target/release/examples/vision_probe mode=cost frames=3000                 # quiet box only
cargo run --release --example ascii                                          # the whole-frame baseline
```

**Everything here was re-measured on every tree `main` landed underneath —
six in all.** Each landing could plausibly have moved the result, so each
got a full re-take rather than a wave-through: the worldgen revamp (716 lines
of `passes.rs`, five new rock materials), tree-breaking (355 lines of
`plant.rs`), the creature-economy rework (`ant.ron`, `beetle.ron`,
`creature.rs`, `organism.rs` — which changes where the animals stand), and a
further plant landing which changed what lies on the floor.

| tree | what it moved |
|---|---|
| 1 → 2 | nothing; byte-identical (only the base rock's *name*, `stone` → `basalt`) |
| 2 → 3 | nothing; byte-identical |
| 3 → 4 | third decimal only — `arid` r64 median 0.440 → 0.420, some p90s, blocking by a tenth of a point |
| **4 → 5** | **materially** — `wetland` r8 median 0.383 → 0.283, r64 0.572 → 0.622, blocking 28% → 24%, litter 21% → 11% of blockers, a new `gravel` appears |
| 5 → 6 | nothing that matters — a blocked-pair count by 7 in 17,190, one blocker percentage by a point. The beetle's own code changed 245 lines and moved no order statistic |
| 6 → 7 | **nothing at all** — byte-identical. A new species (`ant_block_shaded`) plus small `organism.rs`/`material.rs` additions; the new species is never planted in this scene |

**The early identity is what makes the later movement informative.** Three
byte-identical runs could equally have meant the instrument was insensitive
to the world. It is not — it moves when the population and the floor move,
which is exactly what it is measuring.

**One supporting claim was wrong on tree 5 and is corrected rather than
quietly dropped.** The report said *"32 → 64 is the largest single step at
every preset"*. On tree 5 that is false of the **median** on `wetland`, where
8 → 16 is larger. It remains true of the **p10** at all three presets
(+0.233, +0.177, +0.067) — and the p10 is the statistic the house rule says
to gate on, so the radius argument is unaffected and now states which
statistic it rests on.

**The most stable finding is the eye height.** The absolute blocking
percentages drifted on every tree; that `opaque eye=1` recovers the entire
transparent-world ceiling on `wetland` has not moved once.

**The per-read cost is the loosest number and is quoted as a range.** Five
readings: 15.6, 13.8, 14.9, 22.1 and 16.4 ns. The 22.1 came from a run whose
own control spread was twice any other's, on the tree where `ascii`'s worst
frame read 78 ms against a usual 27 — a loud box, not slow code. The
conclusion is identical at either end (0.15% vs 0.24% of a frame), which is
why the range is quoted rather than a pick.

**The report now carries its own staleness guidance**, which is the durable
fix for all of this: a table of which findings never moved across six trees
(the eye height, the radius ordering on the p10, the cost conclusion) against
which drift (every absolute percentage), the one command that re-takes them,
and the rule for when it is worth doing — `main` touching `creature.rs`,
`organism.rs`, `plant.rs`, `assets/species/*.ron` or `src/worldgen/`. Both
landings that moved the numbers materially were plant-side, because what
blocks a sight line here is what is lying on the ground.

**And the cost moved in a way that confirms the mechanism.** `cells read` at
r64 went 909,763 → 898,619 → 977,415. The last jump is the same landing that
dropped blocking from 28% to 24%: less litter means rays travel further
before dying, so **less occlusion makes the sense more expensive**. That is
the cost side of the occlusion finding, arriving independently.

`mode=cost`'s wall clock does not and cannot reproduce — which is the point
of the `cells read` column beside it.
