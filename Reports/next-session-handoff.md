# Next session: what the unzip actually was, and what is left of it

**Written to be picked up cold.** State, what was measured, what is still
wrong, and what has already been tried and must not be retried.

**Read first:** `CLAUDE.md`, then `Reports/building-rethink.md` §3a and §6,
then this. `Reports/destruction-plan.md` holds the wider backlog and the
"Pending owner verification" list.

---

## 0. Where things stand

`master`, 407 tests, clippy clean, **eight** acceptance cases gating in CI
via `scripts/acceptance.sh`.

The destruction model is right and working: torque vs capacity, section
failure, load flow over parallel supports, crack-driven detachment, a
stress view (`N`), a rectangle/room/line build tool (`Z`), a precise dig
verb (`D`).

The owner's verdict after the intact reframe was **"big improvement"** —
building stands. Then one dig unzipped a room into grit, which was the
live bug, and this session found out why.

---

## 1. The unzip: the previous diagnosis was wrong

The version of this file that stood here blamed a **self-propagating
front**: `is_structurally_interesting` treating an intact cell as evaluable
when adjacent to empty, so each removal made its neighbours evaluable, one
cell at a time, handing `rigid::fracture` regions below
`MIN_FRACTURE_CELLS` that fell through to per-cell conversion — which *is*
powder.

**Disproved, by measurement, on `scene=room`:**

- Material below the cut reads `not evaluated` at every captured frame. The
  front never runs. It cannot, because those cells are intact and not
  adjacent to anything empty.
- Failing regions measured **80–1,573 cells**, not 1–2.
- 45 chunk bodies formed, mean 27 cells each. The fragment ladder was
  receiving perfectly reasonable regions all along.

Nothing about region sizes was wrong. Do not go back to the propagation
front; the reproduction is three commands away and says no.

### What it actually was

`scene=room wall=8 dig=0`, probing straight down the inner face of a wall:

```
load (148,170): mass 1627 torque 76028 capacity 443904 stress 0.17
load (148,200): mass 1657 torque 76028 capacity 443904 stress 0.17
load (148,250): mass 1707 torque 76028 capacity 443904 stress 0.17
load (148,300): mass 1757 torque 76028 capacity 443904 stress 0.17
```

**Identical torque all the way to the floor**, 140 cells below the roof
that produced it. `torque = |Sx − x·M|` is the moment of everything a cell
carries *about that cell*, which is right for a beam reaching sideways and
charges a column the eccentricity of its roof's centroid — fifty cells
away. So every cell of every wall in a building sat at exactly the stress
of the worst point of the roof it carried. Nothing was ever safely deep
inside a structure, and any one cell that lost its attachment bonus
(capacity ÷ 12, from `mine`'s detach band) failed wherever it happened to
stand. One failure, one subtree, the whole upper building.

The fix shipped in `0b5b175`: on a **vertical** support step the arm is
clamped to the column's own half-thickness. The load enters the column at
the joint; below the joint the column carries force, not the beam's
bending. Deliberately a clamp and not an exemption — a column still carries
`M × half-width`, so a thin wall under a heavy eccentric cap still fails.

---

## 2. What is still wrong

### 2a. `wall=3 span=200` collapses untouched while 2 and 5 stand

```
./target/release/examples/filmstrip.exe scene=room span=200 wall=3 dig=0 \
    start=150 every=1 count=1 out=target/filmstrips/w3.png
```

1,064 cells, against 0 for `wall=2` and `wall=5`. **Non-monotonic, so
probably a real defect rather than a threshold** — a thicker wall carrying
a proportionally thicker roof should be monotonically safer, and
`CLAUDE.md`'s own advice is that when the same knob moves a number in both
directions the rule is reading the wrong quantity. Worth one `loadmap=1`
run to see which cell tops out and what its section is; the suspicion is
the section walk, which is where the last non-monotonicity lived.

### 2b. Rooms wider than about 200 fail at every thickness

`span=260` loses 890–3,978 cells at wall 2 through 8. That may simply be
correct — a flat stone roof spanning 260 cells at 17 thick is a 15:1
span-to-depth ratio and real masonry does not do it either — but nobody has
decided whether it is the *envelope we want*. It is a design question for
the owner, not a bug to fix silently. The honest current envelope:

| span | wall 2 | 3 | 5 | 8 |
|---|---|---|---|---|
| 100 | ✓ | ✓ | ✓ | ✓ |
| 140 | ✓ | ✓ | ✓ | ✓ |
| 200 | ✓ | ✗ | ✓ | ✓ |
| 260 | ✗ | ✗ | ✗ | ✗ |

### 2c. The dig always cuts clean through a room wall

`Tool::Room` sets wall thickness from `brush_radius` and `App::mine` passes
the **same** `brush_radius` as the cut radius. A capsule of radius r is
`2r+1` thick and a dig of radius r is `2r+1` across, so a cut severs the
wall completely, at any height, at any brush size, and no ligament can
remain. Two verbs sharing one number where the whole point is that one must
be smaller than the other.

Not fixed here because the right answer is a design call: a smaller dig, a
thicker room wall, or a doorway that is dug from the ground up over several
clicks (which is the *satisfying* answer — it makes cutting a doorway a
verb rather than a click). Ask.

### 2d. Load still concentrates on one path

The one-pixel stress line the owner reported three times is **not fixed**,
only made less lethal. Measured on an intact wall: the inner face carries
mass 1707 and the outer face 307, because the whole roof's shortest path to
the ground runs down the single innermost column. Capacity happens to
compensate (it is computed from the full 17-cell section), which is why the
room stands — but it means damage *on that path* is catastrophic while
damage anywhere else in the same wall is free. The clamp removed the worst
consequence; the concentration is still there and is still the largest
open defect in `load.rs`. Its comment at `evaluate_within` already says so.

---

## 3. What has been tried and must not be retried

Newly added this session, both recorded in `capacity`'s comment:

- **Grading the attachment bonus over the section** — attachment as the
  mean over the section's cells, so three loosened cells of seventeen cost
  three seventeenths rather than the whole 12×. The obvious
  graded-beats-binary fix. It **took `scene=undercut` to zero failures**:
  undercut spalls precisely because the rows a dig loosened are weak while
  the rows above them are not, and at a section of 6 with 3 rows loosened
  the mean reads 6.5× where the old rule read 1×. Weakness being *per cell*
  is the spalling mechanism, not an artifact of it. It also did not help
  the case it was written for — at a cut, the entire cross-section is
  loosened, so the mean equals the minimum and nothing moves.
- **Narrowing the detach footprint** (`DETACH_DEPTH` 3→1,
  `CRACK_DETACH_DEPTH` 2→1). Acceptance stayed green and the room was
  unchanged (2,595 → 2,540 cells lost), because one loosened cell in a load
  path carrying the roof's whole moment is already fatal. The footprint was
  never the driver. Both constants are back at 3 and 2.

Carried over, still true:

- **Dividing torque by the section.** Fixed a beefy block, broke
  `scene=undercut`. Peak bending stress in a section of depth D is `M/D²`,
  which the model already has right — capacity carries the `D²`, torque the
  `M`. Dividing again gives `M/D³`, and it double-counts, because a shelf's
  rows already chain independently.
- **Intact as an *exemption*.** Broke `scene=ligament`, which fails from
  geometry alone. A structure standing only by exemption has no answer the
  moment anything asks, so one chip levels a castle. It must be a
  multiplier.
- **Raising `max_unsupported_span` to hold player spans.** 16→40 with
  `attached_span_bonus` 12→2 holds terrain capacity constant and does make
  built spans stand — and stops `undercut` spalling entirely.
- **Scheduling the parent on settle.** 26 pending sites climbing to 4,064,
  frame cost 2.5 ms to 3,160 ms. The bounded in-tick chain walk replaced it.
- **Four support models** (confinement, thickness, attachment-as-anchor,
  reach) — `Reports/load-model-handoff.md` §6.

---

## 4. The measurement loop

```
cargo build --release --example filmstrip
bash scripts/acceptance.sh                     # eight cases, mechanism-asserting
target/release/examples/filmstrip.exe scene=room wall=8 dig=1 \
    start=2 every=8 count=6 crop=100,120,280,200 zoom=2 loadmap=1 \
    out=target/filmstrips/room.png
```

`scene=room` is the reproduction: a hollow room built through
`paint_capsule_as` and cut with `rigid::mine`, exactly as `Tool::Room` and
`App::mine` do it. `wall=`, `dig=` and `span=` are separate knobs *because
the app gives two of them the same number* (§2c). **`dig=0` is the
control** — it makes no cut at all, and it is what established that the
room was collapsing untouched, which nobody had checked before assuming the
dig caused it.

Read `failures: overloaded N (M cells)` and `failing region size: mean X,
largest Y` next to the image every time. The mean alone lies: one 200-cell
break averaged with fifty 1-cell ones reads as a respectable 5, and 1-cell
failures are the shape that produces dust. `loadmap=1` prints the single
most-stressed cell with its mass, torque and capacity, and is the fastest
way to find *where* something is giving way.

**Timings:** always `repeat=2` or more, always read the minimum. This
machine has produced 60.65 ms and 52.72 ms as the slow half of pairs whose
fast half was 14.86 and 22.57 on the same scene, in the same run.

**Images:** write to `target/filmstrips/` (gitignored) and link them with
relative markdown paths — the owner's client does not render file-send
cards.

---

## 5. After this

In order, and **re-judge each rather than inheriting its justification**:

1. **§2a**, the non-monotonic `wall=3` case. Cheapest, and the one most
   likely to be a real bug rather than a design question.
2. **Ask the owner about §2b and §2c** — the build envelope, and whether a
   doorway should take several clicks. Both are design calls and neither
   should be decided quietly in a commit.
3. **Tumbling.** The owner wants "things tilted and fell over more as large
   pieces". Regions are now large and 45–62 bodies form per collapse, so
   there is finally something to tumble; check whether it already reads
   right before touching `SPIN_PER_SPEED`.
4. **§2d**, load concentration. The largest open defect, and the one the
   owner has reported most often.
5. **F3** (replay a playtest report from a world dump) — still the biggest
   gap in the loop. Every report has had to be reconstructed into a scene by
   hand, and at least two reconstructions have been wrong.
6. **C2** (mortar as a material) and doorway/window cuts on the room tool.

### Known defects not yet confirmed

- **`GRANULAR_CAPACITY_DIVISOR` may be dead code.** Flagged by a concurrent
  review: `evaluate_within` early-returns on `is_anchor`, which includes
  `rests_on_ground`. **Not verified.**
- **`filmstrip` never renders inside its timed loop**, so every worst-frame
  number in this repo's history excludes drawing. The owner found a render
  regression the harness structurally could not see.

---

## 6. Repo gotchas these sessions paid for

- **The app locks its exe.** `cargo build` fails with "Access is denied"
  while it runs; `cargo test` and building `--example filmstrip` still work.
- **The tree is worked concurrently.** Stage explicit paths, never
  `git add -A`, and check `git status` first — `Reports/worldgen-design.md`
  and `Reports/prior-art-worldgen-slicing.md` are someone else's work in
  progress.
- **Frame 0 is not a measurement.** Every scene spikes there; `filmstrip`
  excludes it deliberately.
- **A guard test must be seen to fail.** Both new room cases were verified
  in the inverted direction (demanding the standing room collapse reports
  "expected at least 1 overload failures, got 0"; demanding the cut room
  stand reports "expected at most 0 structural failures, got 30"), and the
  standing room was verified non-vacuous with `loadmap=1` at frame 300
  (stress 0.45) — because `scene=capped` once passed for two commits while
  its entire structure was frozen and had never been evaluated.
