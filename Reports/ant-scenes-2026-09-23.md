# The ant's walk in controlled scenes: the baseline the chooser must beat

*2026-09-23. Results of step 3 of
[`ant-movement-plan-2026-09-22.md`](ant-movement-plan-2026-09-22.md): the
plan's scenes, run on today's code, each with its prediction written before
the run. The mechanism is described in [`how-the-ant-works.md`](how-the-ant-works.md).
Harness: `examples/scenes.rs`. S0–S3 are done; S4–S5 run just before
stage 2 and will follow in this report.*

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

- **A laden ant on open ground gets home.** 40 cells over a flat slab:
  24 of 24, a median of 51 decisions, and starting with its back to home
  costs it two decisions.
- **Any obstacle that makes it face away from home stops it.** The brain
  cuts `Move` to zero whenever the ant faces away (`HomeAligned` weight
  3.0), and the homeward re-roll only picks among headings it can take
  right now. So a detour is impossible by construction:
  - **a wall of 6 or 12 cells: 0 of 24.** Climbing the face points slightly
    away from a home on the ant's own level, so every decision up the face
    is throttled to 0.097 and every re-roll points it back down. No ant got
    more than 2 cells up in 4,000 decisions;
  - **walls of 1–3 cells: 22, 22 and 24 of 24**, because from 2 cells up
    the step over the top is usable and points home;
  - **a U-bend: 0 of 24, and not one step taken.** The only way out faces
    away from home, where `p_move` is exactly 0.
- **Four low-wall runs froze the same way**: the head hung over the home
  side of the wall with the only usable heading pointing back, so `p_move`
  was 0 for the remaining ~3,980 decisions.

This is the baseline stage 1 must change: an explorer should run far
before it turns, whether or not it is fed, and a laden ant must be able to
take a step that points away from home when that is the only way on.

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

## 2. S1–S3: a laden ant getting home

**Setup**, asserted in code on every run:
- one ant with a full crop of fruit, pinned full every frame (digestion
  would otherwise turn it back into an empty ant that no longer homes);
- energy pinned at 1.0;
- its home point (`forage_anchor`) set by hand, and checked unchanged at
  the end: there is no nest material to re-anchor it and no trail A;
- no drop is ever placed (it has no nest to deliver to);
- one decision per 6 frames, clear sky, no aging;
- the scene's stone is unchanged at the end;
- the run ends when the head is within one cell of home, or after 4,000
  decisions.

The predictions are in the harness header, written 2026-09-22 before any
run.

### S1: flat slab, home 40 cells away

| | Arrived | Median decisions | Step | Tumble | Nothing |
|---|---|---|---|---|---|
| predicted | all | 50–60; facing away about 2 slower | | | |
| home east, facing it | **24 of 24** | **51** | 76.2% | 13.1% | 10.7% |
| home west, back to it | **24 of 24** | **53** | 74.4% | 14.1% | 11.4% |

As predicted. Facing home, `p_move` read 0.76 on every decision, which is
`squash(2.0 − 1.75 + 3.0)`. Starting with its back to home, the ant faced
away on only 2.9% of decisions: the first homeward re-roll turns it round.
Not one of the 339 re-rolls pointed it away from home.

### S2: home 40 cells west, a one-cell-wide wall halfway

| Wall height | Predicted | Arrived | Median decisions | Highest the head got |
|---|---|---|---|---|
| 1 | about 0.15 per decision up the face | **22 of 24** | 60 | 1–3 cells |
| 2 | about 0.09 | **22 of 24** | 70 | 2–4 cells |
| 3 | about 0.02 | **24 of 24** | 99 | 3–5 cells |
| 6 | only `Stillness` gets it over | **0 of 24** | — | **2 cells, every run** |
| 12 | same | **0 of 24** | — | **2 cells, every run** |

**The low walls went far better than predicted, and the high ones worse.**
Both come from the same fact, which the prediction missed: the re-roll can
only choose among headings usable at that moment.

**Walls of 6 and 12: the ant creeps up and down the bottom of the face.**
A dump of seed 1 at height 12, decisions 3,000–3,023
(`Reports/data/scene-s2-dumps-2026-09-23.log`), shows the loop:
1. At the foot, facing the wall, only east and north are usable. The
   re-roll picks north (cosine to home 0, the best on offer), and `p_move`
   is 0.200.
2. One cell up, facing north, the usable headings are N, S and SE. North
   points 1/21 away from home, which lies 21 cells west and one row lower,
   so `HomeAligned` reads −0.048 and `p_move` falls to **0.097**.
3. A failed roll re-rolls to south, which points 0.048 *toward* home.
   `p_move` rises to 0.282, and it steps back down.

West over the top is never usable, because the wall is in the way. So the
height the wall rises to does not matter: runs at 6 and 12 are
byte-identical, because the ant never reaches the part that differs.
`Stillness` never gets high enough to help, since the ant keeps stepping.

**Walls of 1–3: the step over the top is what gets it across.** From 2
cells up the face, north-west over a wall of height 3 or less is usable,
it points almost straight at home, and the re-roll picks it at once. Every
crossing happened with `Stillness` at 0.00, so `Stillness` never took the
credit.

**The four runs that failed at heights 1 and 2 froze, rather than got
lost.** Seeds 11 and 15 at height 1, and 8 and 17 at height 2. In the two
dumped (height 1 seed 11, height 2 seed 8), the head is at (178, 85), two
cells west of the wall and two cells above the floor, facing south-east.
The only usable headings are SE (and E at height 2), pointing back over the
wall with a cosine of −0.625 to home. So `p_move` is exactly 0 on every
decision, and each re-roll picks SE again. It stood there for about 3,980
decisions. How the body is posed there was not drawn, only the head read
from the trace. The shipped ant only checks its footing inside a step,
after the step roll succeeds, so an ant with `p_move` at 0 never checks it
at all. Plan §4e moves that check to every tick.

### S3: a U-bend

The ant starts at the blind east end of a one-high tunnel. The only way out
runs 70 cells west, up a shaft, and back east along a second tunnel to
home, which lies east of the start.

| | Arrived | Steps taken | Facing away |
|---|---|---|---|
| predicted | trapped | 0 | always |
| shipped | **0 of 24** | **0** | **100% of decisions** |
| `PIXEL_PHYSICS_REVERSE=off` | **0 of 24** | **0** | **100%** |

As predicted. The only usable heading is west, away from home, so
`p_move` is 0 on every decision. All 47,944 re-rolls picked west, the only
choice. The run with the reversal switched off is identical, because the
ant never takes a step for the reversal to act on.

### What it means for stage 1

The bars for stage 1, on these same scenes:
- **S1 at least as fast:** 24 of 24, median at most about 51.
- **S2 over every wall height, including 12.**
- **S3 escapes the U-bend.**
- **No frozen runs anywhere.**

Every failure here needs the same two changes from plan §4:
- the ant must be able to step while facing away from home (`HomeAligned`
  becomes a speed adjustment with a floor, not a veto);
- its heading choice must be allowed to prefer going on over pointing home
  (a turning preference), so that a climb or a detour, once started, keeps
  going.

**Data:**
- `Reports/data/scene-s1-2026-09-23.log`, `scene-s2-2026-09-23.log`,
  `scene-s3-2026-09-23.log` and `scene-s3-noreverse-2026-09-23.log`: every
  run's row and the summaries;
- `Reports/data/scene-s2-dumps-2026-09-23.log`: the three stuck runs,
  decision by decision.
