# The ant's walk in controlled scenes: the baseline the chooser must beat

*2026-09-23. Results of step 3 of
[`ant-movement-plan-2026-09-22.md`](ant-movement-plan-2026-09-22.md): the
plan's scenes, run on today's code, each with its prediction written before
the run. The mechanism is described in [`how-the-ant-works.md`](how-the-ant-works.md).
Harness: `examples/scenes.rs`. S0–S3 are done, on today's code (§1, §2) and
under stage 1's chooser (§3); S4–S5 run just before stage 2 and will follow
in this report.*

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
- **Four low-wall runs froze the same way**: held by its tail on the wall's
  top corner, the ant had its head out over the home side, with the only
  usable heading pointing back. So `p_move` was 0 for the remaining ~3,980
  decisions.

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
decisions. **The body is held, not hanging.** In all four runs the tail
rests on the wall's top corner: at height 1 the head is at (178, 85) and
the tail at (179, 86), diagonal to the wall cell (180, 87). The whole-body
support rule counts that as supported. The head sticks out over the home
side with nothing under it to step onto, so the only way on is back over
the wall, and that points away from home. (This paragraph first guessed the
ant was hanging unsupported; the final body, printed by `dump=`, says
otherwise.)

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

## 3. Stage 1: the chooser on the same scenes

**What stage 1 is** (plan §4a, §4e; `creature::chooser_step`, behind
`PIXEL_PHYSICS_CHOOSER=on`):
- `Move` only decides whether the ant steps this decision. A lost roll is a
  pause; the tumble's re-roll is gone.
- Every step picks one heading from **all** the usable ones, by weighted
  draw. Each scores `Persist x (1 + cos turn) / 2` (going on scores 1,
  turning round 0), plus, while carrying food, `home gain x patience x
  cos(heading, home)`.
- `HomeAligned` reaches `Move` as `(1 + cos) / 2`: facing home still walks
  at the shipped 0.76, facing away walks at an empty ant's pace instead of
  stopping.
- The support check runs every decision, and a fall is not a move.

**One addition the plan did not have: patience.** No rule that reads only
the cells around the ant can tell a U-bend from open ground. Facing away
from home in a passage that turns back later looks exactly like facing away
on an open slab. So the home term is multiplied by a patience that decays
(0.9 per step) while the ant gets no nearer home than it has already been
on this carry, and recovers (+0.25 per step) once it does.
`PIXEL_PHYSICS_CHOOSER=nopatience` holds it at 1, as the ablation.

**Predictions, written 2026-09-23 before the first run:**

- **S0.** On the slab only east and west are usable, so every step goes on
  (weight 1.21) or turns round (0.01): one reversal in about 120 steps.
  Steps still come at 0.20 and 0.53 per decision. **Energy 1: found food in
  24 of 24, median about 450–700 decisions. Energy 0.5: 24 of 24, median
  about 170–260.**
- **S1.** Facing home: the same 0.76 pace, and the home heading weighs 4.41
  against 0.01, so **a median of about 51, unchanged**. Facing away: its
  first step turns it round, but that step waits on an empty ant's 0.20
  roll, so **about 5 decisions slower, median about 56**.
- **S2. Every height crosses, 24 of 24.** At the foot, north scores 0.5
  and turning back east scores 0. Up the face, going on scores about 0.95
  and going down about 0.05. The pace on the face is about 0.63. **Median
  about 60 at height 1, rising about 3 decisions per cell of height, to
  about 100–110 at height 12.**
- **S3. Escapes in most runs, at least 16 of 24, median about 700–1,500
  decisions.** At the blind end it is forced west, turns back east, and
  repeats: 10–15 round trips until patience is below about 0.1. Then it
  follows the passage. A corridor reversal happens at about 0.8% per step
  whatever the patience, so some runs double back once.
- **S3 without patience: 0 of 24.** At the blind end, home and going on
  score 1 each against 0, so it turns round on about 99% of steps, for
  ever.
- **No frozen runs in any scene.**

**Results**, 24 seeds per arm, the same scenes and seeds as §1 and §2:

| Scene | Shipped walk | Predicted | Chooser |
|---|---|---|---|
| S0, energy 1: found food | 2 of 24 (median 2,787 decisions) | 24 of 24, 450–700 | **24 of 24, median 984** |
| S0, energy 0.5: found food | 19 of 24 (median 1,335) | 24 of 24, 170–260 | **24 of 24, median 237** |
| S0: steps between reversals | 1.0 (energy 1), 4.4 (0.5) | about 120 | **84, 67** |
| S1, facing home | 24 of 24, median 51 | about 51 | **24 of 24, median 51** |
| S1, back to home | 24 of 24, median 53 | about 56 | **24 of 24, median 55** |
| S2, wall 1 / 2 / 3 | 22 / 22 / 24 of 24 | all 24 | **all 24, medians 55 / 59 / 65** |
| S2, wall 6 / 12 | 0 / 0 of 24 | all 24, up to 100–110 | **all 24, medians 75 / 98** |
| S3, U-bend | 0 of 24 | at least 16, 700–1,500 | **24 of 24, median 849** |
| S3 without patience | — | 0 of 24 | **0 of 24** |
| Longest stand-still, any run | ~3,980 decisions | none frozen | **42 decisions** |

**Every stage-1 bar in §2 is met.** Two predictions missed:

- **S0 at energy 1 was slower than predicted** (median 984 against
  450–700). The ant turned round about once in 84 steps rather than once
  in 120, so more runs doubled back once before reaching a pile. Every run
  still found food.
- **S3 needed a fix found by tracing S2, and paid for it** (median 725
  before the fix, 849 after). See below.

**What patience is for, measured.** Held at 1 (`nopatience`):
- the U-bend is never escaped: 0 of 24, the ant shuttling at the blind
  end, as predicted;
- the 12-cell wall still crosses, but at a median of 225 decisions against
  98.

**The fix the trace found.** The first chooser run crossed the 12-cell
wall in 78–146 decisions in 20 runs, but four took 1,098–2,727. One
(seed 10, `Reports/data/scene-s2-chooser-h12-seed10-before-reset-2026-09-23.log`)
read decision by decision:

1. It climbed 11 cells up the face. Patience decayed on every step, since
   no step up the face gets nearer home: 1.0 to 0.35.
2. At the top it drew turning back down (weight 0.07 against 1.12 for over
   the top: 3.4%), and going on carried it to the foot.
3. At the foot, with patience at 0.11, turning east (away from home) beat
   climbing again. It then walked east for more than 900 decisions at an
   empty ant's pace. Nothing pulled it home, until a chance reversal and a
   long walk back.

**The fix:** a way round that comes back to where it started has failed.
Once the ant has been 6 or more cells from where it set its best distance,
coming back to within a cell of that spot restores patience to 1
(`EXCURSION_CELLS`). The shuttle at a U-bend's blind end never gets 6 cells
away, so it does not reset there. After the fix, the 12-cell wall's slow
runs are two (621 and 885 decisions) against four (up to 2,727), and the
median is unchanged at 98. The U-bend median rose from 725 to 849, and the
worst run fell from 3,762 to 2,962: a chance turn back to the blind end now
resets patience that had run out.

**Frame cost** (`examples/ascii` on the final code, paired and alternated, 2
threads): its one timed ant scene, the foraging loop, averaged **0.825 and
0.821 ms/frame off against 0.822 and 0.850 on**, about +0.01 ms (+2%) over
12,000 frames, inside the spread between two runs of one arm. Its worst
frames (16.3, 11.6 off; 9.8, 10.0 on) are not pinned by the mean (mean ×
frames is about 10 s against a worst of 10–16 ms), so they are noise, not
a cost. A run before the excursion reset read +0.04 ms.

**One warning from the same scene.** `ascii`'s foraging loop is 15 ants
(its title says 60), not a bed. Over its 12,000 frames the chooser cut
deliveries from **1,310 to 506** and raised deaths from **1 to 5** (441 and
8 before the excursion reset). Deliveries there are mostly
ants eating beside their own door (`Reports/instruments.md`, `nesthome`
row), so fewer of them may be less churn rather than less food. But eight
deaths against one is not explained by that. Stage 1 was not built to be
judged on a colony (plan §4i: until stage 2, trail B still throttles every
empty ant), and it does not ship. **So it stays behind its switch until a
colony-bed run says what those deaths are.** The first suspect was open bug
§Z33: the shipped walk's put-down-and-pick-up churn at the door creates
food, and the chooser churns less (drops 1,310 against 506). **The
colony-bed run, with §Z33 fixed, says stage 1 alone is much worse on the
colony** (ants reaching food: median 17 -> 3 at gap 90; second trips 127 ->
18). Census report §13 has the rows and the hypothesis to trace.

**Data:** `Reports/data/scene-chooser-{s0,s1,s2,s3}-2026-09-23.log` and
`scene-nopatience-{s2,s3}-2026-09-23.log`: every run's row and the
summaries, on the final code.
