# Where one frame actually goes, at the shipped world size

**Status: measurement of record for whole-frame cost. Taken 2026-08-24 on
`main` @ `1882dc9`, 4 logical cores.** Supersedes nothing; it is the first
measurement of `App::update` as a whole. `Reports/field-settling-2026-08.md`
remains the record for the field's *internal* split and is not contradicted
here.

## Why this exists

Every frame-cost number in this repo before today measured *part* of a frame,
and the three that existed were taken at three different world sizes:

| what was measured | where | at what size |
|---|---|---|
| CA sweep worst frame | `README.md` Performance, `ascii` | 512x320 |
| field step, amortised and worst | `field_cost` | 8192x2560 |
| sweep + field + RSS + generation | `scale_probe` | any, by `size=` |
| **the whole of `App::update`** | **nothing** | **--** |

So "the field is the problem" was a reading off two numbers that had never been
put beside the other nine phases, and the load model's own 118 ms
(`open-bugs-handoff.md` §1j) had no frame to be a share *of*. `scale_probe
phases=1` closes that: it runs `App::update`'s exact phase order and times each
phase, bucketed by the two designed oscillators the way `field_cost` does.

## The headline: the shipped world does not fit in its budget, idle

`scale_probe size=8192x2560 phases=1 warm=1500 frames=7200` -- two full
day/night cycles, `rolling` preset, seed 1. **Nobody is playing in this world:
no player, no digging, no blast.** This is what it costs to stand still.

```
                   phase       mean        p90      worst     share
                   field   20.739ms   38.827ms   63.056ms     68.9%
            active sites    7.143ms   18.045ms   63.647ms     23.7%
  sweep (parallel::step)    2.194ms    2.945ms   38.841ms      7.3%
      (the other seven)     <0.02ms                            0.0%
             WHOLE FRAME   30.099ms   49.904ms  104.727ms    100.0%
            budget @60Hz                       16.600ms
```

**5,687 of 7,200 frames -- 79.0% -- exceeded the 16.6 ms budget.** Amortised
cost is 30.1 ms, which is 1.8x the budget; the worst frame is 104.7 ms, which
is 6.3x it.

This is the number the project did not have. The world-scale handoff's target
of "≤4 ms amortised at 4x" was set against the *field alone*; against the whole
frame the shipped world is running at roughly half its intended frame rate with
nothing happening in it.

## The split, and the one surprise

A second run (3,600 frames, one cycle) splits `step_active_sites` into its two
halves, which are bounded differently and want opposite fixes:

```
                   phase       mean        p90      worst     share
                   field   21.020ms   39.356ms   65.325ms     80.7%
 active sites: organisms    2.666ms    6.718ms   18.678ms     10.2%
  sweep (parallel::step)    1.972ms    2.486ms  108.562ms      7.6%
 active sites: scheduler    0.359ms    0.655ms    6.729ms      1.4%
             WHOLE FRAME   26.036ms   44.866ms  130.597ms    100.0%

live organisms: 277   chunks: 5120   awake chunks: 28
```

Three things to read off this:

1. **The field is the frame.** 69-81% of it depending on the window. Nothing
   else is close, and the seven small phases together are under 0.1%. A
   perfect fix to everything except the field would still leave the world over
   budget.
2. **`step_active_sites` is not one cost, it is two**, and the larger one is
   `plant::step_organisms` (10.2%) rather than `scheduler::step` (1.4%). That
   matters because the scheduler is capped -- `MAX_SITES_PER_FRAME` plus a
   load budget -- while `step_organisms` runs once per *live organism* with no
   cap at all. One scales with how much is happening; the other with how much
   world has been sown. Read as a single 23.7% row they are indistinguishable.
3. **The counter is the finding.** 28 awake chunks out of 5,120 -- 0.5% of the
   world has anything moving in it -- and the field still costs 21 ms. The
   field's cost is therefore not driven by activity. It is driven by the sun:
   `sky_drifted` puts every *lit* tile into the solve set whenever the
   amplitude moves, and at 8192 wide that is the whole surface of the world.
   `field-settling-2026-08.md` found the same thing one level down ("the sun
   wakes tiles over rock that has not moved in ten thousand frames").

## Inside `step_organisms`: it is per-cell work, not per-organism overhead

The split above raised a question a single total could not answer, and the two
answers wanted opposite fixes. `ORGANISM_PASS=<every N frames>` (new, in
`plant.rs`) settles it — and it is the **counters** that settle it, not the
timings:

```
[organism] frame  live ticked cells  total | transport frontier support anchor buds roottips upkeep
     1800   318      6     33   0.15 |  0.05  0.01  0.02  0.02  0.01  0.00  0.04
     2700   302      6    202   0.94 |  0.22  0.07  0.16  0.16  0.07  0.00  0.25
     3600   298      5    360   1.75 |  0.42  0.28  0.22  0.28  0.09  0.00  0.46
     4500   278      5    969   5.27 |  0.93  1.88  0.46  0.67  0.27  0.00  1.06
```

**Cost tracks `cells`, not `live`.** ~300 organisms are alive and 5-6 tick per
frame (`ORGANISM_TICK_INTERVAL` is 45), and the total is very nearly linear in
cells ticked — 33 → 0.15 ms, 969 → 5.27 ms, about 5 µs a cell throughout. So
the cost is the organisms that actually tick doing real work on their own
cells. **The obvious hoist is worth nothing**: `step_organisms` resolves
`is_creature` for all ~300 organisms before the cadence gate, and that
overhead is invisible against this. It was the leading hypothesis before the
counters and would have been a wasted change.

The consequence worth flagging: this scales with **plant biomass**, and plants
grow. It is 5.27 ms on the largest sample here and will keep climbing as a
forest matures — an M10 scaling problem in the making, not a fixed cost.

**One pass is superlinear, and only one.** Per-cell, `frontier`
(`allocate_to_frontier`) costs 0.35 µs at 202 cells, 0.78 at 360 and 1.94 at
969, while `transport`, `anchor`, `support` and `upkeep` all stay flat at
~1 µs. The cause is in its donor loop: for **every** frontier cell it clones
the whole donor list and re-sorts it with a comparator that calls
`world.carbon_at` twice per comparison — `O(frontier × donors log donors)`
world lookups. The re-sort is load-bearing (donors deplete as they are drawn),
but the lookups inside the comparator are not.

**Caching the sort key was tried and reverted**, and the reason is worth
knowing before anyone tries it again: it changes the element type, Rust's
`sort_unstable_by` specialises on element type, and the tie order among
equal-carbon donors changes with it. Donor carbon is equal constantly — mature
cells sit at `RESOURCE_SCALE`. Full entry in `Reports/dead-ends.md`, including
the standing risk it leaves behind: **a Rust upgrade that retunes the sort can
silently change how every plant grows**, and nothing in the suite would catch
it.

## What this reranks

The audit was run before any fix, precisely so the fix list could be wrong.
Two entries move:

- **Issue #2 (`touch_neighbours`'s dead fast-path guard) drops.** It targets
  `World::set`, which lives in the sweep -- 7.6% of the frame. The issue's own
  cost estimate ("plausibly a large share of the 23 ms serial worst frame") was
  written at 512x320 against the serial driver; at shipped size with the
  parallel driver and the dirty-rect work since, the whole phase it optimises
  is a twelfth of the frame. Still real, still worth doing eventually, but it
  cannot repay being done first.
- **`plant::step_organisms` enters the list** at #2. It was in no plan and no
  issue; it is 10.2% of the frame and nothing had ever timed it.

## Caveats on these numbers, all of which have burned this repo before

- **Two runs, two lengths, and they disagree on `active sites`** (7.14 ms over
  two cycles against 3.03 ms over one). Plants grow over a run, so a longer
  window has more organisms in it. Neither number is wrong; the two-cycle one
  is the one to quote, and any comparison must hold the frame count fixed.
- **The sweep's worst frame is an outlier, not a cost** (38.8 ms in one run,
  108.6 ms in the other, against a 2 ms mean). A single worst frame over 3,600
  is one event, and the two runs disagree by 2.8x on it. Do not tune against
  it without attributing the event first.
- **4 logical cores.** The sweep is threaded and the field is not, so the
  sweep's share here is smaller than it would be on fewer cores and larger
  than on more. Any threading result must be reported as a scaling curve
  across `RAYON_NUM_THREADS`, never as one figure.
- **Timing overhead is 12 `Instant::now()` calls per frame**, well under
  0.001 ms. Every row under ~0.005 ms should be read as "free", not measured.

## What landed, and what it came to

Five commits, every one bit-identical or pure instrumentation:

| | before | after |
|---|---|---|
| whole frame, amortised | 30.10 ms | **26.16 ms** |
| frames over the 16.6 ms budget | 79.0% | **70.9%** |
| field phase | 20.74 ms | **15.22 ms** |
| field isolated (`field_cost`, 4 threads) | 15.06 ms | **7.54 ms** |

The field's two sky walks stopped fetching each tile eight times; its four
stencil passes moved onto rayon (they were already Jacobi, so nothing
reordered); the velocity/advection read snapshots build in parallel; and
`[profile.release]` exists at last.

**The release profile's split is the surprising part and is documented at the
setting.** All of the ~4% is `codegen-units = 1`; `lto = "thin"` **alone
measured no gain at all** (10.58 ms against a 9.84 ms baseline).

**Its cost line was measured wrong first, and the error is worth keeping.**
This report said, and the session claimed, that CI cost was zero. Independent
review overturned it. The mistake was not the conclusion but the instrument:
the noise bar used to dismiss a +56 ms slowdown in `cargo test (release)` was
taken from the *debug* test job — a different job, on a different profile,
that `[profile.release]` provably cannot affect. Measured on its own terms
the release job reproduces to **11 s** (two runs on identical code: 695 and
706), so the slowdown was never inside noise. And the same ±60 s bar had been
applied asymmetrically: as *noise* where it was inconvenient (+56 s) and as
*signal* where it was convenient (−52 s on `ascii`).

The isolated A/B — the profile's own commit against its parent, differing
only in `Cargo.toml` and three markdown files — puts `cargo test (release)`
at **763 s → 859 s**. A claim that `ascii` and `acceptance` got *faster* is
withdrawn entirely: the mechanism caps at ~6 s by arithmetic, and two runs
compiling byte-identical code differ by 70 s.

What stands: CI's **critical path** is unchanged, because the seven jobs are
independent and the longest (`cargo test (debug)`) never reads the profile —
though its slack fell from 245 s to 123 s, so one more change of this size
moves it. The extra compute is **unbilled, not free**; the repo is public.
Local rebuild is ~+14 s (±5; the measured table contains a `thin`-alone entry
that "rebuilds faster than no profile at all", which cannot be true).

**Two method rules came out of this**, both now in `CLAUDE.md`: a noise bar
belongs to the job it was measured on, and whichever bar you pick has to be
applied to both signs.

**Still over budget.** 26.16 ms against 16.6, with the field ~58% of what
remains, `step_organisms` ~28% and rising with plant biomass, and the sweep
~10%. The next lever is `apply_sky_to`: on the common moderate frames it is
now the single largest pass, because it is the one that walks the whole world
regardless of how few tiles are awake. Parallelising it needs a two-phase
split — walk transmission read-only to get each tile's entry values, then
write tiles in parallel — because `par_iter_mut` partitions by hash bucket and
this walk must partition by *column*. Worth roughly 1 ms of 26.

**Do not retry the column subset**: recorded dead end, measured 8.26 → 6.55 ms
and was measuring a bug (a skipped column *loses* its light); corrected, it
cost 8.26 → 9.06 ms.

## Gates, and one that could not be a comparison

`cargo test --release --locked` (943 + 9 + 2 + 44), clippy under `-D warnings`,
`docscheck`, and CI's own `acceptance.sh` and `ascii` jobs all green. The real
app was run once headlessly (xvfb + lavapipe) and renders correctly.

`seedsweep.sh` was run at `FRAMES="start=2 every=900 count=5"` — the budget
this file's own guidance requires, since the default stops mid-cascade — and
completed clean. Recorded so a later session has the reading:

```
cells lost over 24 runs:      total 3868, max 595,  p90 565, median 27, min -625
rock destroyed over 24 runs:  total 2544, max 1652, p90 213, median 0,  min -562
```

**It is a reading, not a comparison, and should not be quoted as one.** No
paired baseline was taken, because nothing here touches the load, bearing or
fracture models the sweep gates, and bit-identity is already established by
hash on both the field and the plant side. `cells lost` also rides a
±1,700-cell oscillation that has not been divided out, so it cannot compare
two models at any budget.

## How to retake it

```
cargo build --release --examples          # NOT --release alone; examples go stale
cargo run --release --example scale_probe -- size=8192x2560 phases=1 warm=1500 frames=7200
```

Nothing else compiling at the time -- a concurrent `cargo` skews this badly,
and a "regression" here was once entirely the machine slowing between two runs
an hour apart.
