# Load concentration (handoff §2d): the change, and the defect it exposed

**Revised after independent review.** `Reports/load-concentration-review-response.md`
is that review; this document is the current state and supersedes its own
first version. Branch `load-share`.

Read `CLAUDE.md` first, then handoff §2d and §3.

---

## 0. What the review changed

Three findings, all upheld, and one of them was a real bug that no test here
could have caught because the test shared the defect.

| review's objection | outcome |
|---|---|
| `is_member` was coupled to `MAX_SECTION`, so sharing may never fire on wide things | **Confirmed and fixed.** It was worse than "terrain only": `scene=capped`'s 60-cell *built* column was excluded too. New `MAX_MEMBER_WIDTH`, new counters. §3 |
| `caveshallow`'s mean region 10.0 → 4.1 is the dust failure | **Investigated and overturned**, by a counter that did not exist. The dust path fires *less*. §4 |
| the frame-cost figures do not reproduce | **Both right.** The original figure reproduces on a quiet machine; the review's contradiction was the noise it warned about; and the machine has since degraded past the point of certifying anything. §5 |
| drop `flows_down` | **Done.** Under the corrected member test it is now worse on the sweep as well as costly. §6 |
| `capacity_within` needs a test | **Done.** §7 |
| promote the §2a finding | **Done.** §8 |
| `vertical_path` as a redundant early-out is fine | unchanged, no objection |

---

## 1. Verify it in ten minutes

```
cargo build --release --example filmstrip

./target/release/examples/filmstrip.exe scene=room wall=8 dig=0 \
    start=40 every=1 count=1 zoom=1 share=0 load=132,200 load=148,200 out=target/filmstrips/a.png
./target/release/examples/filmstrip.exe scene=room wall=8 dig=0 \
    start=40 every=1 count=1 zoom=1 share=1 load=132,200 load=148,200 out=target/filmstrips/a.png
```

`share=0` is `World::section_share` off — the shipped-before behaviour in the
same binary. Expect `157` / `2956` becoming `2956` / `2956`.

`channel=stress` renders the model's own verdict on the app's `N` ramp, with
a **third** colour for material it declines to evaluate — that is how a
17-cell wall turns out to be a one-cell green skin over thirteen cells
nobody ever asked about.

---

## 2. The change

`capacity` is a **section** quantity (`base × D²` over the horizontal run).
The demand compared against it was a **cell** quantity. In a wall those
describe different objects: `support_cost_below` and `support_cost_beside`
are both `1`, so every cell of a wall row sits at the same distance from an
anchor, none is "closer" than its neighbour, and a 17-cell wall is seventeen
independent one-cell columns *by construction*.

So on a **vertical** load path, the horizontal run through a cell is a *cut*
across the member, and every cell of that cut is charged the **worst** load
crossing it. Two gates:

| gate | says |
|---|---|
| `vertical_path` | only columns. An **early-out, not a correctness gate** — `section_cells` returns a vertical run for a sideways-supported cell, so a shelf would be asking an unrelated question |
| `is_member` | the run must end in air at both hands, within `MAX_MEMBER_WIDTH` |

**The behavioural payoff.** Notch the outer face of a wall
(`cut=132,250,4,6,20`) and probe the notched row:

```
share=0   (136,252) mass  109  torque     0  stress 0.00
share=1   (136,252) mass 2940  torque 17640  stress 0.07
both      (148,252) mass 2940  torque 17640  stress 0.07
```

The face you have just damaged carried **nothing at all**. It does now.

---

## 3. The bug the review found: a work cap was deciding behaviour

`is_member` used to read the ends of `section_cells`, which stops at
`MAX_SECTION` (40). So anything wider than 40 could never be a member. The
review predicted this excluded terrain. It was worse: it excluded
**`scene=capped`** — a 60-cell-wide built column under an overhanging cap,
which is the "beefy block with holes in it" shape the owner reported this
whole defect against.

`CLAUDE.md`, twice: *a size cap must bound work, never gate whether
something happens.* Third time in this file's history.

**Settled with a counter, per `CLAUDE.md`'s "did it fire at all".**
`load::ShareCounts` is a funnel, and only the third field answers the
question — entering the branch is not firing, because a cut whose worst cell
is the one being evaluated comes out with the number it already had:

| scene | columns | in a member | **moved** | too wide |
|---|---|---|---|---|
| `capped` — before the fix | 147 | **0** | **0** | — |
| `capped` — after | 147 | 101 | **26** | 46 |
| `room wall=8 dig=1` | 92,219 | 89,951 | 57,098 | 2,268 |
| `undercut` | 7,334 | 4,523 | 2,273 | 2,811 |
| `worldcrack flat dig=6` | 1,822 | 55 | 16 | 1,767 |
| `terrain` | 0 | 0 | 0 | 0 |

The fix separates the two concerns that had been sharing one number:
`MAX_SECTION` still bounds **how many cells** the sharing looks at;
`MAX_MEMBER_WIDTH` (96) decides **whether there is a member there at all**.
`too_wide` counts the runs that hit it, so if that constant ever starts
deciding outcomes rather than bounding a search, it is visible.

A massif is still excluded — `dig=6` on flat rock is 1,767 too-wide against
55 members — but now for a stated reason rather than as a side effect.

**`MAX_MEMBER_WIDTH` is not a tuning knob**, and that was measured rather
than assumed. At 64 instead of 96 the settled strike sweep is *worse* on p90
(rock destroyed 3,645 against 2,351) and no better on max. A constant that
moves the number in both directions is reading something other than what it
claims; this one is set from what a member *is* (the widest in any
acceptance scene is `capped`'s 60) and left alone.

**A test now guards it**, and it fails when `MAX_MEMBER_WIDTH` is set back to
`MAX_SECTION`. The old test could not have: it used a 45-cell wall as its
"unbounded" case and passed, because it was measuring the same work cap.

---

## 4. `caveshallow`: the dust reading, overturned

The review's argument was tight: `MIN_FRACTURE_CELLS` is 6, a region below it
falls through to per-cell `break_free` which *is* powder, and the mean
failing region moves 10.0 → **4.1**. That reads as the typical break now
being dust.

It is not what is happening, and the mean cannot show it either way — so a
counter went in. `FailureCounts::crumbled` counts regions `fracture` actually
declined, and the cells they took:

| `caveshallow`, settled | `share=0` | `share=1` |
|---|---|---|
| crumbled to grit | 10 regions, **30 cells** | 6 regions, **16 cells** |
| of all failed cells | 1% | 0% |
| rock destroyed | −64 | **−64** |
| confined (nowhere to go) | 488 (4,697 cells) | **5,496 (22,039 cells)** |
| mean failing region | 10.0 | 4.1 |

The dust path fires **less**, the material outcome is identical to the cell,
and the mean fell because the *denominator* changed: 5,496 confined failures
joined the count. Confined rock cracks in place and never reaches the
fragment ladder at all.

**Why `roomcut` moves the opposite way**, which the review rightly said was
worth more than either number:

| | `roomcut` | `caveshallow` |
|---|---|---|
| confined failures | 50 → **2** | 488 → **5,496** |
| total failure events | 154 → 46 | 506 → 5,512 |
| mean region | 27.6 → **64.2** | 10.0 → **4.1** |

One rule, one explanation. Sharing makes more cells reach the failure
criterion. Where the material **has somewhere to go** — a built wall — that
means the section lets go as a unit instead of one column at a time: fewer,
larger, more coherent collapses. Where it **does not** — rock buried in a
slab — it means many confined crack-in-place events that move nothing. The
mean conflates the two because it divides by events of both kinds.

Both lessons are now in `CLAUDE.md`.

GIFs for the owner's own eye, since no number settles the ethos question:
`target/filmstrips/caveshallow-share0.gif` and `-share1.gif`. Contact sheets
of the same window look equivalent; `share=1`'s fissures stay shallower.

**Left open:** 5,496 confined failures against 488 is ~11x the work for an
identical outcome, and only 285 cells actually fissured against 332. That is
wasted work, not a visual defect, and it is the likeliest source of the frame
cost in §5. Worth a look, in `crush_in_place` rather than here.

---

## 5. Frame cost: both parties were right, and it is now unmeasurable here

Re-measured at `repeat=5` on a quiet machine, the original figure reproduces:

| scene | `share=0` | `share=1` |
|---|---|---|
| `worldcrack strike=12` | 12.14 ms | 21.88 ms |
| `room wall=8 dig=1` | 18.62 ms | 20.99 ms |
| `capped` | 10.76 ms | 10.45 ms |

The review measured `share=1` faster twice; that was the noise it warned
about in the same breath. Both readings were honest and the machine was the
variable.

**And then it got worse, which matters more than the numbers above.** By the
end of the session `scripts/acceptance.sh` could not certify timing at all:

- Across three runs, **every** failure was the frame-time bar and **none**
  was a mechanism assertion.
- The failing set reshuffled every run across eight different scenes,
  including `undercut` and `ligament`, which this change provably cannot
  affect, and `crackflat1`, which has no structural failures at all.
- Run with `share=0`, the *control* fails the same bars, sometimes worse:
  `caveshallow` 92.91 ms control against 62.09 ms shipped; `strike` 65.79
  against 41.00 in one pairing and the reverse in another.
- A leftover test binary of this session's was found burning CPU and killed;
  several concurrent agent sessions remain.

So: **mechanism assertions 16/16 in every run; the timing bars need
re-certifying on a quiet machine or in CI.** That is the honest status and it
should not be reported as green.

---

## 6. `flows_down`, withdrawn

An earlier version filtered the cut to cells whose own load travels
downward. On the **summing** version it was load-bearing (`mass 62,391` on a
room built from 14,000 cells). Under a **max** it cannot double-count, and
the tests say so — none could be made to fail for its removal.

Under the corrected member test it now loses on the sweep as well. Settled
`strike=12`, 24 runs:

| | filter on | filter off |
|---|---|---|
| rock destroyed, max | 7,065 | **4,704** |
| rock destroyed, p90 | **2,058** | 2,351 |
| cells lost, max | 2,525 | **1,538** |

for ~45% of the frame on `worldcrack strike=12` (21.76 ms against 15.06).
It survived one earlier sweep that said the opposite — taken before
`MAX_MEMBER_WIDTH` existed, so over a different population of members.
Reinstating it needs a sweep that separates it, not the argument from
physics, which is sound and still loses.

---

## 7. Where the shipped change stands against the control

`scripts/seedsweep.sh strike=12`, `FRAMES="start=2 every=900 count=5"`, 24
runs, one binary:

| | control (`share=0`) | shipped |
|---|---|---|
| rock destroyed, p90 | 2,227 | 2,351 |
| rock destroyed, max | 3,307 | 4,704 |
| cells lost, p90 | 811 | **571** |
| cells lost, max | 1,155 | 1,538 |
| **total rock destroyed, all 24 runs** | 13,160 | 13,888 (**+5.5%**) |
| seeds better / worse / unchanged | — | 5 / 6 / 13 |

The max looks alarming and the per-seed table says it is a reshuffle rather
than a systematic worsening: `flat 24301` improves 19-fold (−2,364 → −125)
and `flat 3` four-fold, while `terraced 1` and `canyon 7` get worse. p90 is
level and the total moves 5.5%. `CLAUDE.md` predicts exactly this — outcomes
are chaotic in the seed and which one is worst reshuffles on any legitimate
change — which is why the total and the win/loss count are quoted beside the
order statistic rather than the order statistic alone.

`dig=6` is unchanged from the control except at the crater rim, where 16
cells now share.

**Other gates.** 527 lib tests pass, 5 of them new (522 on master); the 19 `render` and
`sim::weather` failures are pre-existing on master. Clippy clean.
Determinism confirmed byte-identical on a repeat run. Both drivers agree.

`the_two_capacity_paths_agree_on_every_cell` closes the review's point about
the `capacity` → `capacity_within` split: it sweeps every body cell of a
world holding all four support geometries and fails if the two paths differ.
Verified to fail when the split is fed a truncated section.

---

## 8. §2a is retired

`wall=3 span=200` loses **48 cells, not 1,064**, and is identical with
sharing on and off at wall 2, 3, 5 and 8. The worst-stressed cell in every
one of those runs is a *roof* cell, which takes its support horizontally.

So the handoff's recorded suspicion — that the concentration was §2a's root
cause — is **wrong and retired**. What is left is small and still
non-monotonic (48 against 0 either side), and wants re-baselining before
anyone works it.

---

## 9. Still open: a column's strength is quadratic in its width

Unchanged from the first version of this document, and still the largest
thing here.

For a column of width `D` carrying axial load `N`: capacity `base × D²`,
demand `N × D/2` (the kern clamp), so allowable `N = 2 × base × D` — linear,
and correct, **if** `N` is the load crossing the whole section. It is one
cell's share, so a wall is really quadratic in thickness. A 17-thick wall is
about seventeen times stronger than the formula it is written in claims.

The honest fix — the cut's **sum** rather than its worst — was built,
measured and withdrawn:

| | before | with the sum |
|---|---|---|
| `worked` overload failures | 7 | **918** |
| `worked` rock destroyed | 21 | **1,351** |
| `caveshallow` overload failures | 214 | **2,260** |
| acceptance | 16/16 | `ligament`, `cavedeep1` **FAIL** |

Nothing is wrong with the arrangement; `subtree_sum` divides every hand-off
by `support_count`, so the shares across a cut add up without
double-counting. What breaks is that every constant in `load.rs` is
calibrated against the quadratic.

**What a fix must clear:**

1. **`base` has to come up**, and `max_unsupported_span` is on the
   do-not-retry list *because raising it stops `scene=undercut` spalling
   entirely*. So it cannot be one global multiplier: the shelf case and the
   column case must move independently, and today they share `base`.
2. The section that sets capacity and the cut that sets demand must be the
   same object. They are today only by accident.
3. `bearing_moment` reads the **piece's** footing width, so if demand becomes
   a section quantity its `mass` argument must too, or you rebuild the "rule
   correct for a piece, applied per cell" bug `CLAUDE.md` records. The
   current change already pairs these — see the `capacity.min(bearing_moment(...))`
   call — and that pairing must survive.
4. Build the seed sweep first, run it **to rest**, and gate max and p90.

**Recommendation: not next.** Its own session, with the sweep already in
hand, expecting to re-derive `base`, `attached_span_bonus` and the
`undercut` bar together.

---

## 10. What is still not covered

- **No live play.** Every number here is headless. The GIFs in §4 exist for
  this reason and nobody has watched them yet.
- **Timing is uncertified** — §5. Needs a quiet machine or CI.
- **Confined failures are ~11x more frequent for an identical outcome** on
  `caveshallow` — §4. Wasted work, probably the frame cost, not investigated.
- **`is_structurally_interesting` untouched**: a wall's interior is still
  never evaluated. Thirteen of seventeen cells stay dark blue in the stress
  channel. The two that *are* evaluated now agree with each other and with
  the capacity they are judged against, which is what §2d asked for.
- **Streaming / large worlds**: `MAX_MEMBER_WIDTH` is 96 and the cut memo is
  keyed on a coordinate triple. Neither was weighed against M10.
- **Organism cells excluded**, as before.

*Current against `origin/master` `0c7ad58`.*
