# Review: load concentration (handoff §2d), and the defect it exposed

**Written to be reviewed cold.** One change, on branch `load-share` at
`43a57b7`. It closes `Reports/next-session-handoff.md` §2d — the one-pixel
stress line the owner reported three separate times — and it uncovers a
larger defect underneath which is **deliberately left open** and written up
in §7 below.

Read `CLAUDE.md` first, then handoff §2d and §3. This file assumes both.

---

## 1. Verify it in ten minutes, before reading any argument

```
cargo build --release --example filmstrip

./target/release/examples/filmstrip.exe scene=room wall=8 dig=0 \
    start=40 every=1 count=1 zoom=1 share=0 load=132,200 load=148,200 out=target/filmstrips/a.png
./target/release/examples/filmstrip.exe scene=room wall=8 dig=0 \
    start=40 every=1 count=1 zoom=1 share=1 load=132,200 load=148,200 out=target/filmstrips/a.png
```

`share=0` is `World::section_share` off — the shipped-before behaviour, in
the same binary, so this is a measurement rather than a memory of an older
build. Expect:

```
share=0   (132,200) mass  157      (148,200) mass 2956
share=1   (132,200) mass 2956      (148,200) mass 2956
```

Then look at the wall, which is what made the defect legible:

```
./target/release/examples/filmstrip.exe scene=room wall=8 dig=0 start=40 every=1 count=1 \
    crop=120,140,120,180 zoom=3 channel=stress out=target/filmstrips/wall.png
```

`channel=stress` is new. It draws the load model's own verdict on the app's
`N` ramp at full opacity, and it paints a **third** colour — dark blue —
for material the model *declines to evaluate*. That third state is the
point: a 17-cell wall renders as a one-cell green skin over thirteen cells
the model has never had an opinion about. Green and "never asked" must not
look the same, and before this channel existed they did.

---

## 2. The defect, restated in the terms the model uses

`capacity` is a **section** quantity: `base × D²` over the whole horizontal
run, times the attachment and crack terms. The demand it was compared
against was a **cell** quantity: what one cell's subtree sum said it
carried.

In a wall those describe different objects, and the gap is total.
`support_cost_below` and `support_cost_beside` are both `1`, so every cell
of a wall row sits at the same distance from an anchor. No cell of the row
is "closer to an anchor" than its neighbour, so `dependants` /
`support_count` — which already share load correctly over a DAG, and are
*not* the problem — find nothing to share. A 17-cell wall is seventeen
independent one-cell columns **by construction**, and the roof's whole load
enters whichever one the geometry funnels it into.

Consequence, quoting the handoff: *damage on that path is catastrophic,
damage anywhere else in the same wall is free.*

---

## 3. The change

On a **vertical** load path, the horizontal run through a cell is a *cut*
across the member, and every cell of that cut is charged the **worst** load
crossing it. About 25 lines in `load::evaluate_within`, two helpers
(`flows_down`, `is_member`) and a per-cut memo on `load::Cache`.

Three gates, each load-bearing, each measured into place rather than
reasoned into place:

| gate | says | if you delete it |
|---|---|---|
| `vertical_path` | only columns; a shelf's rows already chain independently | nothing measurable — it is an **early-out**, not the correctness gate (see §6e) |
| `is_member` | the run must end in air at both hands | §1c's lateral unzip: settled sweep p90 rock 2,227 → **4,465** |
| `flows_down` | only cells of the run whose own load goes downward | settled sweep max rock 2,433 → **4,357** |

---

## 4. The evidence

**The bar the task set.** `scene=room wall=8 dig=0`, one row of one wall:
outer face `157`, inner face `2956` → both `2956`. 19:1 to 1:1.

**The behavioural payoff, which matters more than the ratio.** Notch the
outer face and probe the notched row (`cut=132,250,4,6,20`, `load=136,252`):

```
share=0   (136,252) mass  109  torque     0  stress 0.00
share=1   (136,252) mass 2940  torque 17640  stress 0.07
both      (148,252) mass 2940  torque 17640  stress 0.07
```

The face you have just damaged carried **nothing at all**. It does now.

**`scripts/acceptance.sh`: 16/16.** `undercut`, `ligament`, `capped`,
`terrain`, `roomstands`, `cavedeep`, `cavedeep1` and all five `crack` cases
are identical to the cell. What moved:

| case | overload failures | rock destroyed |
|---|---|---|
| `worked` | 7 → 8 | 21 → 25 |
| `roomcut` | 19 → 14 | 512 → 1,982 (**mid-fall; see §5**) |
| `caveshallow` | 214 → 528 | 64 → 64 |
| `strike` | 8 → 8 | +177 → +177 |

**`scripts/seedsweep.sh strike=12`, run to rest** (`FRAMES="start=2
every=900 count=5"`), 24 runs, paired on one binary:

| | `share=0` | `share=1` |
|---|---|---|
| cells lost, max | 1,155 | **847** |
| cells lost, p90 | 811 | **688** |
| rock destroyed, max | 3,307 | **2,433** |
| rock destroyed, p90 | 2,227 | **1,782** |

`dig=6` is bit-identical across the change.

**Frame cost**, min of 3, same binary: `room wall=8 dig=1` 18.60 → 21.26 ms;
`worldcrack strike=12` 11.97 → 21.67 ms; `capped` 10.48 → 10.55 ms. The
`worldcrack` figure is the honest cost and it is not small. Two things
already paid for most of it: the per-cut memo (the answer is the same for
every cell of a cut) and `capacity_within`, which reuses the caller's
section walk instead of repeating it. Without those the room was **55.89
ms**. What remains is largely irreducible here, because `structural::tick`
clears the load cache on every failure — which is exactly when a cascade is
walking.

**Determinism**: same build, same seed, twice — console output and PNG
byte-identical. The `NEIGHBOURS_4` tie-break is untouched; nothing here
reorders or re-weights it, so the `load.rs` / `structural.rs` agreement the
task warned about is not in play.

**Both drivers** (`driver=parallel`, the app's, and `driver=serial`) give
the same readings on the intact wall.

**Tests**: 525 pass, 3 of them new. The 19 failures in `render` and
`sim::weather` are pre-existing on master and untouched.

---

## 5. Two measurement traps this change walked into

Both are now in the handoff, and a reviewer will hit them if they
re-measure casually.

**A cascade censused before it settles reads a *delay* as damage.**
`roomcut` at frame 202 says cells lost 251 → 1,501 and looks like a
disaster. Given 1,500 frames it says 235 → 273, because the fixed version
stands a tile *longer* and the earlier census caught it mid-air. The
default sweep does the same: at frame 1,202 `strike=12` says p90 rock
destroyed rose 1,152 → 1,366; run to rest it says it *fell* 2,227 → 1,782.
Same binary, same seeds, opposite conclusions.

**A single cascade scene cannot compare two load models at all.** Two runs
that diverge on one frame are different worlds by the next. On
`scene=worldcrack strike=12` the `flows_down` filter measured *ten times
worse*; 24 seeded runs to rest said it nearly halves the worst case. Use
the sweep, gate the order statistic, and run it to rest.

---

## 6. Attack these, in this order

Ranked by how likely I think it is that a reviewer changes my mind.

### (a) `caveshallow` fails in more, smaller pieces

The one I would attack first, and the only result here pointing the wrong
way against `CLAUDE.md`'s ethos.

```
./target/release/examples/filmstrip.exe scene=worldcrack preset=flat seed=7 \
    dig=4 tunnel=35 depth=6 share=0 start=2 every=1200 count=5 zoom=1 out=target/filmstrips/cs.png
#   share=0: cells lost 11 (rock -64), failing region mean 10.0, largest 146
#   share=1: cells lost 13 (rock -64), failing region mean  4.1, largest 118
```

The material outcome is *identical* — `rock -64` either way — but a shallow
tunnel roof now comes apart in more than twice as many, less than half as
large, events. "A graded outcome beats a binary one" argues one way; "turns
to dust" argues the other, and **nobody has watched this in motion**.

Note that `roomcut` moves the opposite way — mean failing region 27.6 →
**64.2**, largest 1,903 → 1,839, i.e. bigger pieces. So this is not a
uniform trend, and understanding why the two differ is worth more than
averaging them.

### (b) `is_member` is the part closest to a rule `CLAUDE.md` forbids

It says a run is a member only if it ends in air at both hands, and a run is
capped at `MAX_SECTION` (40) — so *in practice* a 41-cell-thick wall is not
a member and a 39-cell one is. I argue in the source that this asks what the
object **is** rather than capping work, and that the fallback is the shipped
per-cell reading rather than something weaker. The argument is honest; it is
still an argument, and the cliff at 40 is real. A formulation that does not
lean on `MAX_SECTION` is probably better than mine.

### (c) `flows_down` survives on one statistic

Settled sweep, filter off against on: rock destroyed **max 4,357 vs 2,433**,
cells lost max 1,392 vs 847 — but **p90 is a tie inside the noise** (1,753
vs 1,782). It costs about 33% of the frame on `scene=worldcrack strike=12`
(22.72 vs 15.10 ms). It is also the physically right statement (a cut counts
what crosses it downward), and under a *max* rather than a *sum* it cannot
double-count, so no hand-built scene shows it doing anything at all. That is
thin evidence for a real cost. A second settled sweep at other seeds would
settle it either way.

### (d) The wall's interior is still never evaluated

Thirteen of seventeen cells stay dark blue in the stress channel.
`is_structurally_interesting` declines anything buried and attached, which
is what keeps the sweep proportional to surface area rather than volume, and
I did not touch it. The two cells that *are* evaluated now agree with each
other and with the capacity they are judged against, which is what §2d
asked for — but if you think the fix should have made the interior
participate, say so.

### (e) The `vertical_path` gate is redundant

Deleting it changes nothing measurable, because the `flows_down` filter
inside the loop already contributes nothing from a sideways-loaded cell. It
is kept as an early-out — one comparison against a walk of the run, per
evaluated cell, per frame — and the source says exactly that. It is not
doing correctness work and I have not pretended otherwise.

### (f) Nobody has played it

Every number here is headless. `CLAUDE.md` is explicit that playtest reports
have overturned three models that looked correct in tests, and the ethos
question — does a wall now *feel* like it notices being hit — is not one a
contact sheet answers.

### The guards, and what each was seen to fail for

Per `CLAUDE.md`, each was run inverted by **breaking the mechanism**, not by
flipping the assertion:

| break | `both_faces…` | `a_run_with_no_ends…` | `a_cantilevered_shelf…` |
|---|---|---|---|
| rule switched off entirely | **FAILS** | **FAILS** | passes (correctly) |
| `is_member` deleted | passes | **FAILS** | passes (correctly) |
| `flows_down` deleted | passes | passes | passes |

The last row is the honest gap, and it is stated in the test's own comment:
under a max, no unit-testable geometry distinguishes it. It earns its place
on the sweep's worst case or not at all — see (c).

`a_run_with_no_ends…` is also a cautionary tale worth reading before writing
any similar guard. Its **first** version probed the far face of a wide wall
and passed with `is_member` deleted, because `section_cells` fills leftward
first and that cell's 40-window never reached the load at all. It now probes
`x=40`, whose window does. A guard over a windowed walk has to be checked
against the window it actually gets, not the one you pictured.

---

## 7. The unfixed defect: a column's strength is quadratic in its width

**This is bigger than §2d was, and it is the reason §2d's fix is a
redistribution rather than a correction.** Written up to be picked up cold.

### The statement

For a column of width `D` carrying axial load `N`, this model gives

- capacity `base × D²` (a section modulus — right for bending)
- demand `N × D/2` (the kern clamp in `evaluate_within`)
- so allowable `N = 2 × base × D`

which is **linear in D**, and correct: a column in compression carries in
proportion to its area. That is what the arithmetic says *if* `N` is the
load crossing the whole section.

It is not. `N` is one cell's share. For a uniformly loaded wall that is
`N_total / D`, so the wall's real allowable total is `2 × base × D²` —
**quadratic**. A 17-thick wall is about seventeen times stronger than the
formula it is written in claims.

### Why it was not fixed here

Because the honest fix — make the demand the cut's **sum** rather than its
worst — is a global recalibration wearing a distribution fix's clothes. It
was built, measured and withdrawn:

| | before | with the sum |
|---|---|---|
| `worked` overload failures | 7 | **918** |
| `worked` rock destroyed | 21 | **1,351** |
| `caveshallow` overload failures | 214 | **2,260** |
| acceptance | 16/16 | `ligament` and `cavedeep1` **FAIL** |

Nothing was wrong with the *arrangement*: the flow across a cut really is
the sum of what crosses it, and `subtree_sum` divides every hand-off by
`support_count`, so the shares add up without double-counting. What broke is
that every constant in `load.rs` is calibrated against the quadratic, and
removing it made thick rock roughly seventeen times weaker overnight.

There is a separate, sharper trap in that version, kept because it is easy
to walk back into: summing a run that reaches out of a wall and into a roof
takes the cut **along** the flow rather than across it, and neighbouring
roof cells each carry most of the *same* cantilever. It reported
`mass 62,391` on a room built from about 14,000 cells. A number larger than
the world it came from is the tell. (Under a max this cannot happen, which
is why `flows_down` is hard to justify today — see §6c.)

### What a fix has to clear

1. **`base` has to come up**, and `max_unsupported_span` is on the
   do-not-retry list precisely because raising it *stops `scene=undercut`
   spalling entirely* (handoff §3). So the recalibration cannot be one
   global multiplier: the shelf case and the column case have to move
   independently, and today they share `base`.
2. **The section that sets capacity and the cut that sets demand must be
   the same object.** They are today only by accident.
3. **`bearing_moment` reads the *piece's* footing width**, so if the demand
   becomes a section quantity its `mass` argument has to become one too, or
   you rebuild the exact "rule correct for a piece, applied per cell" bug
   `CLAUDE.md` records. §2d's change already pairs these two — see the
   comment at the `capacity.min(bearing_moment(...))` call — and that
   pairing must survive.
4. **Build the seed sweep first.** Two load-model changes have already
   shipped green through acceptance while eating tens of thousands of cells.
   Run it to rest (§5) and gate the max and p90, never a single seed.

### Why it might be worth doing anyway

It is the last thing making thickness a magic number. The honest
player-facing statement today is "a thicker wall is *much* more than
proportionally stronger", which is probably why the build envelope in §2b is
shaped as oddly as it is — `span=260` fails at every thickness, `span=200`
fails at wall 3 but not at 2 or 5. A linear column would make thickness
legible and would likely reshape that whole table.

**Recommendation: do not do this next.** Do it as its own session, with the
seed sweep already in hand, and expect to re-derive `base`,
`attached_span_bonus` and the `undercut` bar together.

---

## 8. What is not covered

- **No live play.** See §6f. This is the gap that matters most.
- **`is_structurally_interesting` untouched** — a wall's interior is still
  never evaluated. §6d.
- **Streaming / large worlds untested.** `MAX_SECTION` is 40 and the cut
  memo is keyed on a coordinate triple; neither was weighed against M10.
- **Organism cells excluded**, as before — they route through
  `structural::organism_structural_tick`'s own BFS.
- **`capacity` was split** into a public wrapper and `capacity_within`,
  which takes a caller-supplied parent and section. Behaviour is intended to
  be unchanged; it exists because walking a 40-cell run twice per evaluated
  cell was most of the added frame cost. Worth a second pair of eyes that
  the two paths really are equivalent.
- **§2a was re-measured and is largely stale** in the handoff: `wall=3
  span=200` now loses 48 cells, not 1,064, and is identical with `share=0`
  and `share=1` at wall 2, 3, 5 and 8. So the concentration was *not* its
  root cause, and the suspicion recorded against it was wrong.

*Current as of `43a57b7` on branch `load-share`, against `origin/master`
`bef468a`.*
