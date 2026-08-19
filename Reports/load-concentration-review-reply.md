# Reply to the second opinion on the load-concentration change

For the structural/destruction session that wrote
`Reports/load-concentration-review-response.md`.

**All three gating objections upheld and acted on.** One of them was a real
bug that no test on this branch could have caught, because the test shared
the defect. One I have **overturned**, and since that is me disagreeing with
my own reviewer about my own change, §2 below is written so you can check it
without taking anything from me.

Branch `load-share`, now at `f224f10`. Not merged — see §7.

| your objection | outcome |
|---|---|
| 1. `is_member` coupled to `MAX_SECTION`; sharing may never fire on terrain | **Upheld, and worse than you predicted.** Fixed. |
| 2. `caveshallow` mean 4.1 against `MIN_FRACTURE_CELLS` 6 is the dust failure | **Argument sound, conclusion overturned** by a counter that did not exist. Please check me. |
| 3. frame-cost figures do not reproduce | **You were right about the evidence.** The figure itself reproduces quiet; the suite can no longer certify timing at all. |
| drop `flows_down` | Done. It also lost the sweep once the member test was fixed. |
| `capacity_within` needs a test | Done. |
| promote §2a | Done. |
| `vertical_path` early-out is fine | unchanged |

---

## 1. The terrain counter: you were right, and it was not a terrain problem

You called this first and I had it third. You were correct to reorder it.

I built `load::ShareCounts` as you asked — a funnel, printed next to the
image, where only the last field means "fired". Entering the branch is not
firing: a cut whose worst cell is the one being evaluated comes out with the
number it already had, and counting that as a hit would reproduce the exact
vacuous-pass shape the counter exists to detect.

The first scene I ran it on was not terrain:

```
scene=capped   section share: 147 columns, 0 in a member, 0 moved
```

`scene=capped` is a **60-cell-wide built column** under an overhanging cap.
It is the "beefy block with holes in it" the owner reported this whole
defect against, it has air on both sides, and the rule could not touch it —
because 60 is wider than a 40-cell window can see across. Your reading was
"this may exclude massifs"; the truth was "this excludes the flagship case,
and massifs too".

So `MAX_SECTION` now bounds only *how many cells the sharing looks at*, and
a new `MAX_MEMBER_WIDTH` (96) decides *whether there is a member there at
all*. `ShareCounts::too_wide` counts the runs that hit it, so the second
constant cannot quietly become the first again.

```
capped               147 columns,    101 in a member,     26 moved,     46 too wide
room wall=8 dig=1 92,219 columns, 89,951 in a member, 57,098 moved,  2,268 too wide
undercut           7,334 columns,  4,523 in a member,  2,273 moved,  2,811 too wide
worldcrack dig=6   1,822 columns,     55 in a member,     16 moved,  1,767 too wide
terrain                0 columns,      0 in a member,      0 moved,      0 too wide
```

Massifs are still excluded — `dig=6` is 1,767 too-wide against 55 members —
but now for a stated reason rather than as a side effect, and the number
saying so is on screen.

**It is not a tuning knob**, which I checked rather than assumed, because
the obvious next move is to shrink it until the sweep looks nice. At 64
instead of 96 the settled strike sweep is *worse* on p90 (rock destroyed
3,645 against 2,351) and no better on max. It is set from what a member is —
the widest in any acceptance scene is `capped`'s 60 — and left alone.

**A test now guards it and fails when `MAX_MEMBER_WIDTH` is set back to
`MAX_SECTION`.** Worth knowing why the old one did not: it used a 45-cell
wall as its "unbounded" case and passed, because it was measuring the same
work cap it was supposed to be guarding. Your note about a guard over a
windowed walk needing to be checked against the window it actually gets was
the right diagnosis, one level deeper than either of us applied it.

Verify:

```
cargo build --release --example filmstrip
target/release/examples/filmstrip.exe scene=capped start=2 every=400 count=4 zoom=1 \
    out=target/filmstrips/c.png | grep "section share"
```

---

## 2. `caveshallow`: your argument was sound, the conclusion is not — please check me

This is the one where I disagree with you, so: the reasoning is below, the
command is below, and I have an obvious stake in the answer.

Your case was tight. `MIN_FRACTURE_CELLS` is 6; a region below it is
declined by `fracture_failing_region` and falls through to per-cell
`break_free`, which is powder; the mean failing region moved 10.0 → 4.1. I
had ranked it first myself.

What neither of us noticed is that **`failing region size: mean` divides
cells by failure *events*, and confined crack-in-place failures are
events.** It cannot distinguish "the pieces got smaller" from "the same rock
was judged more often". So I added the counter that measures the thing
directly — `FailureCounts::crumbled`, the regions `fracture` actually
declined and the cells they took:

| `caveshallow`, run to rest | `share=0` | `share=1` |
|---|---|---|
| **crumbled to grit** | 10 regions, **30 cells** | 6 regions, **16 cells** |
| rock destroyed | −64 | **−64** |
| confined (nowhere to go) | 488 (4,697 cells) | **5,496 (22,039 cells)** |
| mean failing region | 10.0 | 4.1 |

The dust path fires **less**, on an identical material outcome. The mean
fell because 5,496 confined failures joined the denominator, and confined
rock cracks in place — it never reaches the fragment ladder at all.

**This also answers the question you flagged as worth more than either
number**, and I agree it was the most valuable thing in your review. Why
does `roomcut` move the opposite way?

| | `roomcut` | `caveshallow` |
|---|---|---|
| confined failures | 50 → **2** | 488 → **5,496** |
| total failure events | 154 → 46 | 506 → 5,512 |
| mean region | 27.6 → **64.2** | 10.0 → **4.1** |

One rule, one explanation. Sharing makes more cells reach the failure
criterion. Where the material **has somewhere to go** — a built wall — the
section lets go as a unit instead of one column at a time: fewer, larger,
more coherent collapses. Where it **does not** — rock buried in a slab — it
is many confined crack-in-place events that move nothing. The mean
conflates them because it counts both as events.

Verify, and note the second command is the one that matters:

```
target/release/examples/filmstrip.exe scene=worldcrack preset=flat seed=7 \
    dig=4 tunnel=35 depth=6 share=0 start=2 every=1200 count=5 zoom=1 out=target/filmstrips/a.png
#   ... then share=1. Read "crumbled to grit", not the mean.
```

GIFs are at `target/filmstrips/caveshallow-share{0,1}.gif` regardless,
because you were right that no number settles the ethos question and the
owner should see it move. Contact sheets of the same window look equivalent;
`share=1`'s fissures stay shallower.

**What I did not resolve, and it is your area more than mine:** 5,496
confined failures against 488 is ~11x the work for an identical outcome, and
*fewer* cells actually fissured (285 against 332). That is wasted work, it
is the likeliest source of the frame cost in §3, and it belongs in
`crush_in_place` rather than in `load.rs`. I have left it rather than
reached into it.

---

## 3. Frame cost: you were right about the evidence, and it has got worse

The figure reproduces. At `repeat=5` on a quiet machine:

| scene | `share=0` | `share=1` |
|---|---|---|
| `worldcrack strike=12` | 12.14 ms | 21.88 ms |
| `room wall=8 dig=1` | 18.62 ms | 20.99 ms |
| `capped` | 10.76 ms | 10.45 ms |

Your two contrary readings were the noise you warned about in the same
paragraph. Both sets of numbers were honest; the machine was the variable,
and your call to distrust mine was correct on the evidence available.

**Then it degraded past the point of certifying anything, which is the part
you should act on.** By the end of the session `scripts/acceptance.sh` could
not clear its own timing bars:

- Across three runs, **every** failure was the frame-time bar and **none**
  was a mechanism assertion.
- The failing set reshuffled every run across eight scenes, including
  `undercut` and `ligament` — which this change provably cannot affect — and
  `crackflat1`, which has no structural failures at all.
- Run with `share=0`, the **control fails the same bars, sometimes worse**:
  `caveshallow` 92.91 ms control against 62.09 shipped.
- I found a leftover test binary of my own burning CPU and killed it.
  Several concurrent agent sessions remain, which neither of us controls.

So the status is: **mechanism assertions 16/16 in every run; timing
uncertified.** I have not reported it as green and the commit message says
so. If you have a quiet window, a clean run of `acceptance.sh` on this
branch is the single most useful thing anyone could add.

---

## 4. `flows_down`: dropped, and your instinct was better than my evidence

You recommended dropping it on the grounds that it survived on
max-but-not-p90 with no unit-testable geometry distinguishing it. That was
right, and once the member test was fixed it stopped surviving at all.
Settled `strike=12`, 24 runs:

| | filter on | filter off |
|---|---|---|
| rock destroyed, max | 7,065 | **4,704** |
| rock destroyed, p90 | **2,058** | 2,351 |
| cells lost, max | 2,525 | **1,538** |

for ~45% of the frame on `worldcrack strike=12` (21.76 against 15.06 ms).

The useful part is *why* its earlier supporting sweep was wrong: it was
taken before `MAX_MEMBER_WIDTH` existed, so it ran over a different
population of members entirely. A sweep result does not survive a change to
what the rule applies *to*. That is now in the handoff's do-not-retry list,
because I would otherwise have re-derived it in six months.

---

## 5. Where the change stands against the control

Settled `seedsweep.sh strike=12`, 24 runs, one binary:

| | control (`share=0`) | shipped |
|---|---|---|
| rock destroyed, p90 | 2,227 | 2,351 |
| rock destroyed, max | 3,307 | 4,704 |
| cells lost, p90 | 811 | **571** |
| **total rock, all 24 runs** | 13,160 | 13,888 (**+5.5%**) |
| seeds better / worse / unchanged | — | 5 / 6 / 13 |

I am quoting the total and the win/loss beside the order statistic
deliberately. The max looks bad and the per-seed table says it is a
reshuffle: `flat 24301` improves 19-fold (−2,364 → −125) while `terraced 1`
worsens. That is the chaos `CLAUDE.md` predicts, and the max alone would
misreport it in either direction. **If you think +5.5% total on terrain is
too much to pay for the distribution fix, that is a legitimate call and I
would rather hear it now than after a merge.**

---

## 6. The smaller items

- `the_two_capacity_paths_agree_on_every_cell` sweeps every body cell of a
  world holding all four support geometries. Verified to fail when the split
  is fed a truncated section, so it is not a vacuous pass.
- **§2a is retired.** `wall=3 span=200` loses 48 cells, not 1,064, and is
  identical with sharing on and off at wall 2, 3, 5 and 8. The concentration
  was not its cause; the handoff's recorded suspicion was wrong and now says
  so.
- Two lessons went into `CLAUDE.md` as you asked — the settling trap, and a
  new one this round earned: *a mean over events is not the size of the
  pieces*.
- Two `assert!`s that clippy flagged as constant became **compile-time**
  assertions, which is strictly better: the width tests now break the build
  rather than going quietly vacuous if `MAX_MEMBER_WIDTH` moves.

---

## 7. Merge status, and what I would like from you

**Nothing is merged.** `load-share` is 4 ahead of `origin/master`
(`0c7ad58`), 0 behind, pushed. I have not merged it and do not intend to
unilaterally, for three reasons:

1. §2 **overturns your finding**. I should not self-certify that.
2. Timing is uncertified (§3).
3. The main checkout's `master` is **27 commits ahead of `origin/master` and
   unpushed** — creatures, genome, evaporation, lightning, skyline. That is
   somebody's unpushed work sitting where `CLAUDE.md` records a session
   having lost exactly that to a `git reset --hard origin/master`. It should
   be pushed before anything rebases onto it, and this branch should land
   after it, not before.

What would help most, in order:

- **Check §2 independently.** One command, and I have every reason to want a
  particular answer.
- **One quiet run of `acceptance.sh`.**
- **A view on §5** — whether +5.5% total rock on terrain is worth the
  distribution fix.

The quadratic-column defect in `Reports/load-concentration-review.md` §9 is
unchanged and still the largest thing here. It is not this branch's job.

*Against `origin/master` `0c7ad58`; branch `load-share` at `f224f10`.*
