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

## 1b. The live bug: one dig erodes far more than it should

**Start here.** Reported from play against the generated world: *"one crack
in the ground basically propagates throughout the whole world and slowly
breaks everything."* Reproduction is `scene=worldcrack` (commit `de8fedd`),
which digs once into a generated world.

Measured. It is **bounded** — every preset and seed settles and sleeps
within ~1,500 frames, `max_chain_reach` stays at 13–22 cells against
`ROOTWARD_CHECK_STEPS`' 128, and a 20,000-frame run is flat after 4,000. So
it is not unbounded erosion. But one radius-6 dig on the app's own default
world (`rolling`, seed `0x5EED`) takes **1,558 cells**: a scar down the
entire flank of the hill, mostly the soil mantle sliding off and exposing
bare rock. One click, one hillside.

```
rolling 0-201   terraced 0-201   wetland 0
arid   52-348   flat   338-637   canyon 439-2,102
```

### Look at it before doing anything else

`target/filmstrips/front.png` — `preset=flat seed=7`, zoom 8 on the rock
*beside* the dig, four frames from 400 to 1150. **The rock is being eaten
into a spongy filigree of voids.** Not a crater, not a collapse, not
chunks falling: a fretwork of holes opening all through the slab and
joining up, spreading sideways well past the dig. (Empty renders as sky
now, so the pale-blue and orange are holes, not material.)

Two things that picture settles, and neither is what the counters suggest:

- It is **not a front advancing from the dig**. Holes appear scattered
  through the whole region at once. A large area became marginal together
  and is failing cell-by-cell wherever the sweep reaches it.
- The pieces are **single cells**. This is `CLAUDE.md`'s "turns to dust"
  failure in a new place, and any fix judged only on the cell count will
  miss whether that changed.

### What has been ruled out, by measurement

- **Stale distances.** `relax=1` on `scene=worldcrack` runs a converged
  pass immediately after the dig. Identical to the cell on eight of nine
  preset/seed pairs. `canyon` seed 3 went 5,364 → 394, so staleness is real
  but is not the cause here.
- **Gating the granular divisor on `parent.is_none()`.** Not one cell moved
  anywhere. The cells taking the cut have no solid parent to begin with.
  **Superseded — this entry was wrong, and the reason is the bug.** The gate
  was not a dead lever, it was *vacuous*: `structural::tick` rooted a cell's
  distance at 0 the moment powder touched its underside, so every powder-backed
  cell was parentless by construction and the gate had nothing to discriminate.
  Demote ground to a **last-resort root** (relax from neighbours first; take 0
  only when that leaves no path at all) and the same gate becomes meaningful —
  measured, one radius-6 dig across six presets x three seeds went from a worst
  case of 27,409 cells to a worst case of 70. That is only half a fix: it
  overcorrects, `roomcut` drops to 2 overload failures against its bar of 5,
  and `a_sprinkle_of_sand_under_a_beam_does_not_hold_the_beam_up` fails. The
  other half is a bearing model (compression-only bed, kern rule, capacity
  proportional to pile depth) replacing the flat 64x divisor. Full measurements,
  patches and sequencing are in the session plan file.
- **Granular capacity as a cap rather than a divisor.** 899/215/0 →
  25,838/51,262/0. The cap sits far below what a deep intact section
  resting on soil used to get, and generated terrain is mostly that.

Two attempts on the granular term, both failed, which by this file's own
rule means **the term is not where the fix goes.** Do not try a third.

### The hypothesis that is left

`break_free` turns every failure into rubble. Rubble is not body material,
so it neither carries load nor counts toward a section — which means each
failure strictly weakens its surviving neighbours, in two multiplicative
ways at once. That is positive feedback with no damping anywhere in it, and
both failed attempts tried to damp it at the *capacity* end. The untried
lever is at the source, in `rigid.rs`: what a failed cell becomes, and
whether a single-cell failure should produce debris that undermines its
neighbours at all.

**Unverified.** Look at `front.png` first and form your own view.

### A caution about every number in this section

The worldgen session is tuning `assets/worldgen.ron` and its generation
passes live. Between two sweeps an hour apart, `flat` seed 7 went from 338
cells to 27,886 — same preset parameters, different strata/soil/water code.
**Re-baseline before trusting any figure here**, and quote the worldgen
commit you measured against.

**Work `flat` first, not the hillside.** It is the minimal case and it needs
nothing from the owner: a homogeneous slab of bare rock, dead level, no
soil, nothing standing on it, still loses ~600 cells to one hole. Whatever
does that is almost certainly what takes the hillside too, and on `flat`
there is no slope, no powder and no strata to argue about.

```
target/release/examples/filmstrip.exe scene=worldcrack preset=flat seed=1 \
    dig=6 start=2 every=60 count=6 crop=180,190,160,110 zoom=4 \
    loadmap=1 out=target/filmstrips/flatcrack.png
```

Prime suspect, and it is §2d wearing a different hat: `mine` detaches a band
(`DETACH_DEPTH` 3 per removed cell, plus the crack rays at
`CRACK_DETACH_DEPTH` 2), every cell in that band loses the 12x attachment
bonus at once, and the ones still carrying load fail. Narrowing the band was
tried on `scene=room` and did nothing there — but `room` was dominated by
the column-moment defect, which is now fixed, so **that experiment is worth
re-running here**, where there is no column at all.

Still open with the owner: whether the hillside scar *is* what they saw, or
whether there is a case `worldcrack` does not cover. Needed to tell — the
preset and seed from the title bar, where they clicked, and `D` or `C`.

---

## 1c. OPEN: a big strike unzips the surface sideways

The dig cascade and the small-strike cascade are fixed (see `df78bc7`,
`fcc9873`, both on master). **Large strikes are not.** This is what is left
of the owner's "they chain too far and too much" and it has its own
mechanism.

### Reproduction

```
target/release/examples/filmstrip.exe scene=worldcrack preset=flat seed=24301     strike=12 start=2 every=250 count=6 crop=180,190,200,120 zoom=4     out=target/filmstrips/bigstrike.png
```

`strike=` was added for this (`0aa354d`); nothing in the repo had ever struck
generated terrain before, which is why a dig-shaped fix measured clean while
the hammer was still eating the world.

| blow | rolling | flat | canyon |
|---|---|---|---|
| strike=6 (the minimum) | 0 | 0 | 48 |
| strike=12 | 40 | 12,283 | 12,662 |
| strike=20 | 12,752 | 18,502 | 8,282 |

### Look at it first: it is a *lateral* unzip, not a crater

`bigstrike.png`. Frame 2 looks right -- a clean star of fissures round a
small crater, which is the verb doing its job. Then over the next thousand
frames the damage **travels sideways along the surface layer**, well past
the blow, while the deep interior never moves at all. It is not the crater
growing and it is not a front from the impact: it is the top ten or fifteen
rows peeling away horizontally.

Read the region sizes next to it: mean 87 falling to 49, largest 835. Those
are not dust -- the pieces are reasonable. There are just hundreds of
events, 153 by frame 752 and climbing, which is why it "lasts a while".

### Ruled out by measurement

- **Crack rays.** `CRACK_RAYS` 5 -> 0, i.e. a strike that scores no cracks at
  all: `strike=12` on flat is still **10,852** cells against 12,283 with
  them. Cracks are not what scales a big blow. Do not spend time on
  `CRACK_REACH` or ray counts.
- **Detaching only near the impact** (`DETACH_REACH`, letting the crack run
  its full length but unbracing only within one bite of the blow): flat
  16,634 -> 12,283, but `strike=20` on flat went 10,541 -> 18,502. Moves the
  number in both directions, so by this file's own rule it is reading the
  wrong quantity.

### The hypothesis that fits the picture

`strike` loosens a **chip disc of radius 2r/3** -- for a radius-20 blow that
is ~400 cells that lose the 12x attachment bonus at once, and the area grows
as the *square* of the brush while the bite grows linearly. In a thin
surface layer that produces a wide, shallow, wholly-unbraced sheet.

Then the lateral unzip is `failing_region` taking a whole section: capacity
goes as section squared, so when a section fails the cell beside the new
hole has its own section cut short, drops in capacity, fails in turn, and
the process walks along the layer. Deep rock is immune because its sections
are long in every direction; a surface sheet has nowhere to go.

That is the same *shape* as the room unzip that `0b5b175` fixed with the
column-moment clamp, in a different place, and it should probably be
attacked the same way: find the quantity that is being charged to a cell
that does not own it.

**Do not start by tuning the chip radius.** Check first whether the sideways
walk is even legitimate -- a detached surface sheet genuinely has little
holding it -- and if it is, the question becomes pacing and granularity
rather than prevention: hundreds of small events over a thousand frames is
what looks bad, not the total. The owner has said explicitly that some
chaining would be *good* if it looked better.

---

## 1d. OWNERSHIP CHANGE: the gnome and creatures are now structural work

**Stated by the owner, and it reframes the milestone:** *"you can take over
the gnome mechanism. that and creatures are what really dig in the game they
need to work with your structural mechanics."*

So `src/sim/player.rs` joins `load.rs` / `structural.rs` / `rigid.rs` as this
area's files. The player's `D` key is a *sandbox* verb; the gnome and the
creatures are what actually excavate in play, and they are the diggers the
structural model has to be correct for.

### Where the two systems already agree

`scene=tunnel` (the gnome held-digging into a cliff, written by the M9
session) runs **zero structural failures over 1,000+ frames** with the
bearing model in place. His bore opens and stays open. Nothing here is on
fire.

### What is wrong, measured

**Pending sites climb 233 -> 11,190 and keep climbing**, on a scene with no
failures at all. That is the structural scheduler re-examining a bore that
has already settled, forever, and held-digging is the case that generates it.
`load-model-fit-review.md` predicted this shape ("a settled, standing,
motionless foreground structure costs a 12,000-cell walk on a repeating
schedule"). It is a real cost and it belongs to whoever owns this area now.

**Two diggers, two spoil models.** `player::dig` calls `rigid::mine` and then
applies `thin_to_dust(.., tuning.dig_yield)`; `App::mine` calls `rigid::mine`
and takes whatever falls out. So the gnome honours `dig_yield` and the `D`
key ignores it. Same one-number-two-verbs shape as `brush_radius` (2c).

### The owner's requirements for tunnels, stated this session

1. **Small tunnels self-sustaining; big ones need built support, like a mine.**
2. **Collapse must depend on height, not only span.** *"a super short tunnel
   should be able to have a longer span. ant tunnel vs digging a mine."*
   Today height does not enter the criterion at all -- a roof is judged purely
   as a span, which is why the engine and the intuition disagree. The textbook
   form is Terzaghi's rock load, where the load a roof carries scales with
   *(width + height)* rather than the full overburden: an ant tunnel carries
   almost nothing and can run a long way, a gallery carries much more and
   cannot. It drops into the existing capacity arithmetic.
3. **Collapse must be obvious and delayed**, so the player can get supports in
   first. Same warning-band machinery the big-strike nibbling (1c) needs.
4. **Rock and compacted soil must dig differently.** Mostly a data question,
   except soil is a `Powder` and not load-bearing at all, so "compacted soil
   you can tunnel through" wants to be its own material or state.

### Spoil: decided, and the options that were weighed

The owner asked for **vanish now, with the alternatives recorded as future
plans**. Then the M9 session turned out to have already built the better
version, so *do not hardcode vanish*:

- `Tuning::dig_yield`, 0.0-1.0, live in the tunables panel, with named
  `SpoilMode`s cycled on **F2** -- `DUST` 0.35 ("a third stays as rubble, the
  rest blows away"), `SPOIL` 0.55 ("half stays - tunnels silt up behind you").
- **Vanish is `dig_yield = 0.0`.** It wants a named `VOID` mode, not a code
  change.
- A blanket vanish inside `rigid::mine` was written, measured and **reverted**:
  it breaks `player::at_full_yield_nothing_leaves_the_world`, whose contract is
  "at yield 1.0 a dig may move material but never delete it", and it drops
  `roomcut` to 2 overload failures against its bar of 5.

Future options, recorded rather than discarded:

- **Bulking inverse** -- yield less spoil than was cut. Real rock bulks ~1.5x,
  so inverting it is one tunable. *This is what `dig_yield` already is.*
- **Eject toward the mouth** -- spoil displaced back along the tunnel axis,
  heaping outside the hole. `displace()` and `nearest_free` already exist.
- **Low repose spoil** -- debris that runs flat along the floor instead of
  heaping to the roof. Stacks with the above.
- **Spoil as a carried resource** -- mining yields the material that pays for
  the supports a long tunnel needs. Closes the loop requirement 1 opens, and
  waits for supports to exist.

### Measured tunnel envelope (with spoil vanishing, `dig=4`)

| length | rolling | flat |
|---|---|---|
| ~36 | holds | holds |
| ~72 | holds | holds |
| ~108 | holds | 10,010 cells |
| ~180 | holds | 6,785 cells |

`rolling` holds throughout because the bore sits under far more cover.
Reproduce with `scene=worldcrack preset=flat dig=4 tunnel=N`.

### Suggested order

1. **One spoil model for every digger** -- move it into `rigid::mine` so the
   `D` key, the gnome and the creatures share it; add `VOID` at 0.0.
2. **Height in the criterion** (requirement 2). The single biggest gap between
   the model and what the owner expects.
3. **Warning band** (requirement 3) -- also fixes 1c's nibbling.
4. **The site backlog**, folded into whichever of those touches the scheduler.

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
