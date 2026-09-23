# The ant's walk in controlled scenes: the baseline the chooser must beat

*2026-09-23. Results of step 3 of
[`ant-movement-plan-2026-09-22.md`](ant-movement-plan-2026-09-22.md): the
plan's scenes, run on today's code, each with its prediction written before
the run. The mechanism is described in [`how-the-ant-works.md`](how-the-ant-works.md).
Harness: `examples/scenes.rs`. S0 is done; S1–S5 follow in this report.*

## 0. The answer so far

- **A fed explorer jitters.** At full energy an empty ant steps on 20% of
  its decisions and reverses on 20%, so it turns around about once per step.
  Over a whole bed-length run (24,000 frames, 4,000 decisions) the median
  ant gets **50 cells** from where it started, and it found food 90 cells
  away in **2 of 24** runs.
- **A hungry one explores.** At half energy it steps on 53% and reverses on
  12%, runs about 4 steps between reversals, and found the food in **19 of
  24** runs, a median of 1,335 decisions in.
- **The per-decision mix matched the formula to within a point** at both
  energies, so the mechanism in `how-the-ant-works.md` §4 and §6 is the one
  running. Exploration is set by `Energy`'s weight on `Move` and by the
  tumble reversing half the time. It is not set by anything about the world.

This is the baseline stage 1 must change: an explorer should run far
before it turns, whether or not it is fed.

## 1. S0: an empty ant on a bare slab, no trail

**Setup**, asserted in code from the trace and the world:
- one ant on a flat stone slab, 400 cells wide, with walls far out of reach;
- a 3-cell pile of fruit 90 cells away on **each** side;
- no trail laid (every `EmitA`/`EmitB` weight zeroed), no births, no aging;
- energy pinned every frame, at 1.0 or 0.5;
- weather pinned clear;
- the app's own frame step;
- the run ends when the ant first stands beside food, or at 24,000 frames.

Checks on every run:
- the energy read at each decision is the pin;
- the ant never becomes laden;
- no trail reading;
- at least 99% of decisions see exactly east and west usable;
- decisions come one per 6 frames;
- nothing but the ant and its food ever stands on the slab.

**Two setup faults were caught by those checks** before any result was
read:

- **Rain.** The app's frame step runs the weather, and the first full run
  had 364 cells of rainwater standing on the "bare" slab. An ant cannot
  step into water, so half its decisions saw only one way to go. The scene
  now pins a clear sky.
- **Old age.** With a 40,000-frame half-life, an ant died mid-run. The scene
  now switches aging off.

**Food on both sides, not two mirrored arms.** The ant's random draws do not
depend on where the food is, so "food east" and "food west" runs are the
same walk until one arrives. A smoke test showed identical rows. So both
piles stand in one run, and a sideways bias in the scene would show as one
side winning more often.

**Result**, 24 seeds per energy, pooled shares over decisions, medians over
runs:

| | Step | Reverse | Same-heading re-roll | Nothing | Steps between reversals | Furthest from the start | Found food |
|---|---|---|---|---|---|---|---|
| energy 1, predicted | 20% | 20% | 20% | 40% | about 1 | — | well under 1% |
| energy 1, measured | 19.9% | 20.2% | 19.8% | 40.1% | 1.0 | 50 cells | **2 of 24** (median 2,787 decisions) |
| energy 0.5, predicted | 53% | 12% | 12% | 23% | about 5 | — | half to two thirds |
| energy 0.5, measured | 52.8% | 11.9% | 11.7% | 23.6% | 4.4 | 89 cells | **19 of 24** (median 1,335 decisions) |

Food was found on the east side 1 of 2 times at energy 1, and 12 of 19
times at 0.5: no strong side bias.

**The reach predictions were low, and the reason was my arithmetic.** The
prediction treated each reversal as fresh randomness in direction. But two
reversals restore the original heading, so successive steps are positively
correlated, and the walk spreads faster than an uncorrelated one:
- **At energy 1**, the chance that the next step goes the same way is 2/3,
  not 1/2. That puts reaching 90 cells at a few percent per run, which
  matches 2 of 24.
- **At energy 0.5**, it is about 0.84. That puts the reach near
  two-thirds-plus, which matches 19 of 24.

The per-decision shares were predicted from the same formula and landed
within a point.

**What it means for the loop.**
- A forager that has just eaten reads `Energy` near 1 and explores like the
  top row: in a bed-length run it is more likely to wander 50 cells than to
  find food 90 cells out.
- The colony's discovery therefore rests on its hungry ants, and on a trail
  laid by the few that found food.
- Stage 1's turning preference (plan §4e) is judged here first: an explorer
  should cover ground whatever its energy.

**Data:** `Reports/data/scene-s0-2026-09-23.log`, every run's row and the
summary.
