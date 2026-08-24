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

## How to retake it

```
cargo build --release --examples          # NOT --release alone; examples go stale
cargo run --release --example scale_probe -- size=8192x2560 phases=1 warm=1500 frames=7200
```

Nothing else compiling at the time -- a concurrent `cargo` skews this badly,
and a "regression" here was once entirely the machine slowing between two runs
an hour apart.
