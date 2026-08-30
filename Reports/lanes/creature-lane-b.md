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
| PR | **#146**, https://github.com/sgladstein/Pixel_Physics/pull/146 |

I had GitHub tools (`mcp__github__get_me` resolved), so I opened the PR
myself. The coordinator owns the merge. Cost fork: **built the probe and answered the
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
| reach | **64 cells** | the **p10** seed's beetle sees prey 0.108–0.260 of the time at r32 and **0.240–0.389** at r64, over three presets × 18 seeds. 32 → 64 is the largest single step at every preset |
| shape | **all-round** | a ±60° cone costs a third of every sighting (r64 median 0.572 → 0.400) and saves nothing measurable |
| occlusion | rock and soil, **never floor litter** | 28.1% of beetle-ant pairs blocked at head height, **8.5%** one cell up. The blockers are seed, litter, corpse, soil — clutter, not landscape |
| foliage | **not a binary blocker** | `dense` costs half the sense (0.667 → 0.350) and no eye height buys it back |
| cost | **free at this scale** | 485 cells read per beetle per cast; **0.004 ms/frame**, 0.14% of `ascii`'s 2.98 ms mean, below the wall clock's floor. Under 10% of a frame only past ~358 predators |

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
right for an insect on a forest floor. Fire-and-forget; the verdict bears on
the occlusion recommendation, not on the radius one. Collect with
`python3 scripts/review.py inbox`.

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

**Everything here was measured twice, on two different trees.** The worldgen
revamp (716 lines of `passes.rs`, five new rock materials) landed on `main`
underneath this lane, so the whole study was re-taken after merging it in.
**Every order statistic came back identical**, as did `mode=cost`'s
`cells read` column bit for bit; the only changes anywhere are that the base
rock is now called `basalt` rather than `stone` in the blocker census, one
blocking percentage moving 8.6 → 8.5, and pair counts moving by a handful out
of ~20,000. That is the staleness check, and it is a stronger one than a
repeat on one tree.

`mode=cost`'s wall clock does not and cannot reproduce — which is the point
of the `cells read` column beside it.
