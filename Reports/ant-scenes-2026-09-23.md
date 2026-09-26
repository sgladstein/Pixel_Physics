# The ant's walk in controlled scenes: the baseline the chooser must beat

*2026-09-23. Results of step 3 of
[`ant-movement-plan-2026-09-22.md`](ant-movement-plan-2026-09-22.md): the
plan's scenes, run on today's code, each with its prediction written before
the run. The mechanism is described in [`how-the-ant-works.md`](how-the-ant-works.md).
Harness: `examples/scenes.rs`. S0–S3 are done, on today's code (§1, §2) and
under stage 1's chooser (§3). S4 and S5 are done (§4, §5): the baseline
stage 2 is judged on.*

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

## 4. S4: an empty ant at a fork, with trail B down one branch

**Setup** (asserted from the trace and the world):
- a one-high tunnel in solid stone, with a fork 12 cells ahead of the ant;
- a **level** branch runs 50 cells on, an **up** branch climbs diagonally
  for 25, and both end blind;
- trail B is laid down one branch (`trail=level` or `up`), rising from 30% to
  100% of a full deposit towards the blind end and topped up every 30 frames,
  or down neither (`trail=none`);
- the ant is empty, lays nothing, energy pinned at 0.5, clear sky, no food;
- 24 seeds per arm, 2,000 decisions each.

The predictions are in the harness header, written before the first run.

| | Shipped walk | Chooser (stage 1) |
|---|---|---|
| no trail: first branch level / up | 16 / 7 | 17 / 7 |
| level trail: first branch level / up | 18 / 6 | 10 / 11 |
| level trail: time on the level branch | **73%** (control 34%) | 41% |
| level trail: runs frozen 100+ decisions on a branch | 0 | **21 of 24** |
| up trail: runs that ever left the main tunnel | **0 of 24** | **0 of 24** |
| up trail: longest stand-still, median | **1,904 decisions** | 1,915 |

**Three findings**, the last one new:

- **The trail never changes which branch the shipped ant takes first**, as
  predicted: its walk has no trail term. It changes only where the ant
  stays. A trailed level branch holds it 73% of the time against 34% without.
- **Under the chooser the trail freezes the ant** (21 of 24 runs, a median
  1,750 decisions). Traced on seed 1: the ant walks up the trailed branch
  well (reading +0.02 to +0.9, `p_move` about 0.7). Five cells from the blind
  end, the sensor's sample point, 6 cells ahead on the ant's own row, falls
  past the end. The reading drops to −0.48 in one step and `p_move` goes to
  0. The chooser does not re-aim on a lost roll, so it faces the wall for
  good. The shipped walk escapes the same spot by tumbling. **This is the
  hypothesis of census report §13 for stage 1's collapse on the colony bed,
  now shown in a scene with a known answer.**
- **A trail on the up branch freezes every ant at the fork, on both walks**
  (predicted: no effect). The trail spreads into the fork cell (677 under the
  head), but the sensor reads the walker's row 6 cells ahead, where there is
  little or none. So the reading is negative whichever way the ant faces
  (−0.43 east and up, −0.73 west), and `p_move` is exactly 0 even at full
  `Stillness`. It is the census's frozen-on-a-local-peak case (79% of long
  stalls on the bed, census report §4), at the mouth of the branch that
  carries the trail.

**What stage 2 must do here**, and the bars it is judged on:
- read the trail at the cells a step would enter, not 6 cells ahead on the
  ant's row;
- retire the throttle, so a reading can turn the ant but never freeze it;
- **the first branch follows the trail** in both placements (at least 18 of
  24 into the trailed branch, against about 7 up and 17 level without a
  trail);
- **no run frozen 100+ decisions**, in any arm.

**Data:** `Reports/data/scene-s4-{shipped,chooser}-2026-09-23.log`.

## 5. S5: an open stone lattice, where every cell gives footing

**Setup:** stone pegs one cell wide at every third cell across and down, so
every empty cell has a peg beside it and an ant can step in all eight
directions: the nearest thing to a canopy that nothing grows in.
- **Laden** (`scene=s5`): a full crop, home 60 cells east inside the lattice,
  otherwise as S1.
- **Empty with trail B** (`scene=s5trail`): trail B along one row of the
  lattice, rising eastward and topped up, against a no-trail control. The
  ant starts on the row; 4,000 decisions.

24 seeds per arm; predictions in the harness header, written before the run.

| | Predicted | Shipped walk | Chooser (stage 1) |
|---|---|---|---|
| laden: got home | most, 80–120 decisions | **24 of 24**, median 103 | **24 of 24**, median 92 |
| empty, no trail: got 60 cells east | | 12 of 24 | 16 of 24 |
| empty, trail: got 60 cells east | further than the control | **21 of 24**, median decision 702 | **0 of 24** |
| empty, trail: furthest east, median | | 144 cells (control 65) | **3 cells** |
| empty, trail: decisions within a row of the trail | as often off it as the control | **13%** (control 1.5%) | 54% |
| empty, trail: longest stand-still, median | | 25 | **3,994 (every run)** |

- **Laden ants get home through open footing on both walks**, as predicted.
- **On the shipped walk the trail pulls an empty ant along**, through the
  throttle: it keeps stepping while it faces up the gradient and turns when
  it faces down. That is a crude run-and-tumble, and it keeps the ant on the
  row nine times as often as without the trail, which I did not predict.
- **Under stage 1's chooser the same ant freezes where it starts**, in every
  run, as S4 showed and as predicted: a downhill reading throttles `p_move`
  to 0 and the chooser never re-aims.

**What stage 2 must do here:** follow the row at least as well as the shipped
walk (21 of 24 reaching 60 cells east, 13% on the row), with no frozen runs.

**Data:** `Reports/data/scene-s5-{laden,trail}-{shipped,chooser}-2026-09-23.log`.

## 6. Stage 2, first form: the chooser reads the trail where it would step

**What it adds** (`PIXEL_PHYSICS_CHOOSER=trail`; `creature::trail_presence`,
`brain_inputs`), on top of stage 1:
- **The trail is read where a step would go**: the scent in the cell the
  head would enter and the one beyond, the larger of the two, as a presence
  from 0 to 1 (half at a tenth of a full deposit). Trail B for an empty ant,
  trail A for a laden one.
- **Presence multiplies going on**: a heading onto a full route scores up to
  4 times what its turn alone would. Turning round still scores 0, so a
  route that runs both ways does not make the ant reverse more.
- **The throttle retires**: the brain gets the two trail-gradient readings
  as 0, where its mirrored hidden-unit pairs cancel exactly.
- Not yet: the running averages of plan §4c, the away-from-home gain for
  empty ants of §4d, and new brain outputs for the gains.

**Predictions, written 2026-09-23 before the first run:**
- **S4, trail up the climbing branch: at least 18 of 24 take it first**, from
  the arithmetic at the fork (about 80%). **Trail on the level branch: at
  least 20 of 24.** No trail: as stage 1 (17 and 7).
- **No run frozen 100+ decisions, in any arm of S4 or S5.** Nothing reads
  the gradient any more, so nothing can set `P(move)` to 0.
- **S5 with trail: the ant stays on the row far more than the shipped walk's
  13%**, at least half its decisions, and at least 21 of 24 get 60 cells
  east.
- **S0–S3 unchanged from stage 1**, since those scenes lay no trail.

**Results, 24 seeds per arm, as S4 and S5 above:**

| | Predicted | Shipped walk | Stage 1 | **Stage 2** |
|---|---|---|---|---|
| S4 no trail: first branch level / up | as stage 1 | 16 / 7 | 17 / 7 | **17 / 7** |
| S4 level trail: first branch level / up | 20+ level | 18 / 6 | 10 / 11 | **22 / 2** |
| S4 up trail: first branch level / up | 18+ up | never left the tunnel | never left | **6 / 18** |
| S4 time on the trailed branch, level / up | | 73% / 0% | 41% / 0% | **40% / 22%** |
| S4 runs frozen 100+ decisions, any arm | 0 | 0 level, 24 up | 21 level, 24 up | **0** (longest 17) |
| S5 empty, trail: got 60 cells east | 21+ | 21, median decision 702 | 0 | **21, median 1,189** |
| S5 empty, trail: furthest east, median | | 144 cells | 3 | **97** |
| S5 empty, trail: within a row of the trail | half or more | 13% | 54% (frozen on it) | **23.5%** |
| S5 no trail: got 60 cells east | | 12 | 16 | **16** |
| S5 laden: got home | as stage 1 | 24, median 103 | 24, median 92 | **24, median 92** |
| S0–S3 | unchanged | | | **unchanged, line for line** |

- **The trail now chooses the branch, both ways round, and nothing freezes.**
  Both S4 bars are met: 22 of 24 take a trailed level branch first, and 18
  of 24 take a trailed climbing branch that no ant on either earlier walk
  ever left the main tunnel for. No run stands still for more than 17
  decisions in any arm of S4 or S5, where stage 1 froze 21 and 24 of 24.
- **S0–S3 are the same to the line**, because they lay no trail (the run
  echoes `CHOOSER=trail`, so this is not a stale binary).
- **Time on the trailed level branch fell from the shipped 73% to 40%, and
  that is not a regression.** The shipped ant climbed the rising trail and
  stood at the blind end, held there by the throttle reading uphill. The
  stage-2 ant walks to the end, turns round, and walks back out, because a
  route draws it on in both directions. Here there is nothing at the end; in
  the colony the end is food.
- **S5 missed half its bar, and for a reason in the design, not the tuning.**
  21 of 24 get 60 cells east, as the shipped walk does, but at a median of
  1,189 decisions against 702, and on the row 23.5% of the time against a
  predicted half. **Presence says "this is a route"; it does not say which
  way along it is home or food.** On the row both directions score the same,
  so the ant goes along it both ways at random. The shipped walk had a
  direction, crudely: its throttle stopped it when it faced down the rising
  trail. Which way along the route is better is what plan §4c's running
  average is for (is the scent getting stronger as I go?), and S5 is the
  scene that will show whether it works.

**What this does not yet say** is whether the colony eats more. S4 and S5 are
one ant and a trail laid by hand. That is the bed (§8).

**Data:** `Reports/data/scene-stage2-{s0,s1,s2,s3,s4,s5,s5trail}-2026-09-23.log`.

## 7. Crumbs were ending up sealed underground

**Found by the owner from a review card** (2026-09-23: *"Some of your crumbs
are being placed underground..."*, then *"they look buried under soil, not
down a tunnel"*). Both were right, in order. `crumbwatch` (every crumb, every
frame, with its eight neighbours) traced one seed cell by cell:
- ants dig a narrow diagonal tunnel down from the nest;
- crumbs went down it, some carried and put down inside, most put down at
  the mouth and **sliding** down it: `crumbs` was an ordinary powder, and
  `update_powder`'s diagonal move is unconditional (it does not read
  `friction_angle`);
- loose soil then came down the same tunnel after them and closed it, and
  each crumb ended with none of its eight neighbours open. No ant digs
  there again, so that food is gone from the colony for good.

**The fix is `Material::rolls`**, `false` for `crumbs` only: a crumb still
drops straight down through open air (and through an organism), and
otherwise stays where it was put. That is how the fruit it replaces behaves:
a plant cell never moved. Guarded by
`update::tests::crumbs_stay_where_they_are_put_and_sand_in_the_same_place_slides`
on both drivers, with sand at the same tunnel mouth as the arm that must
slide, and a crumb in open air that must still land.

**Measured on the bed**, gap 90, 24 seeds, frame 24,000, the same seeds on
both sides, counting crumbs with no open neighbour (`CRUMBS BURIED`):

| | Crumbs slide | **Crumbs stay put** |
|---|---|---|
| crumbs sealed in, of all crumbs | 93 of 157 | **10 of 142** |
| runs with any sealed in | 22 of 24 | **8 of 24** |
| food sealed in | 10,612 J (about 11 fruits) | **277 J** |
| food delivered to the nest, sum | 12,383 | 13,237 (higher on 14 of 24 seeds) |
| starved, sum / colonies wiped out | 433 / 23 | 434 / 24 |

The ten left are crumbs put down *inside* a tunnel that later filled; this
does not stop soil falling on food, and nothing needs it to. **It does not
move the loop**: the colony starves as before. It stops the loop's own food
being lost where nobody can reach it, which would otherwise be a leak under
every later measurement. The sliding arm reproduces §Z33's fixed shipped bed
at gap 90 line for line, so the only difference between the columns is the
one flag.

**Data:** `Reports/data/bed-crumbs-{slide,stay}-2026-09-23.log.gz`.

## 8. The colony bed: stage 2 against the shipped walk

**Setup:** the §Z33 bed (gaps 90, 140 and 200 cells, 24 seeds each, arm
`hand`, 24,000 frames, `RAYON_NUM_THREADS=2`), both arms on one binary. The
shipped arm came out byte-identical across the two runs below, so everything
that moved is stage 2.

**First run, stage 2 as in §6: the colony changed kind, so the paired
totals mean nothing.** At gap 90, 21 of 24 stage-2 colonies bred, a median of
**1,311 births a run against 0**, nearly all off the comb, at the food pile.
The two arms' totals are over populations 70 times apart (the pooled-`n`
rule). Tracing every decision of every ant on seed 1, frames 0–12,000, found
the first route to the pile:
- **A frozen ant at the pile.** Ant 17 picked food up 7 cells short of the
  pile and climbed a shaft facing away from home. Its patience fell ×0.9 a
  step, 1.0 to 0.08 in 25 steps, because climbing gets no nearer home. At
  the top, beside the pile, its `Move` sum was `Bias` 2 + `Energy` −1.75 ×
  0.6 + `FoodAdjacent` −1.16 + `HomeAligned` 3 × 0.06 ≈ 0, and `p_move` was
  **exactly 0 for 1,200 frames** while it ate and topped up its crop. The
  shipped walk re-aims on a lost roll; the chooser pauses, and the facing
  changes only on a step, so a 0 stays 0. It is S4's deadlock through a
  different input. S0–S3 could not show it: no food beside the ant, and
  energy pinned.
- **Fix:** under the chooser, `HomeAligned` reads 1 while the ant carries
  food, whichever way it faces (how-the-ant-works §6d). Guard:
  `a_fed_laden_ant_beside_food_facing_away_from_home_still_walks_under_the_chooser`,
  **watched red first: 0 steps in 600 frames**.
- **Every laden scene got faster and none slower**, 24 of 24 arriving
  throughout; the empty-ant scenes (S0, S4, S5 with trail) are
  byte-identical:

| Laden scene, median decisions to arrive | Stage 2 as in §6 | With the fix |
|---|---|---|
| S1 facing away from home | 55 | 51 |
| S2 wall of 12 | 98 | 90 |
| S3 U-bend | 849 | **371** |
| S5 lattice | 92 | 86 |
| longest stand-still, any laden scene | 32 | 7 |

**Second run, with the fix: the breeding did not stop.** I predicted births
near 0; the median at gap 90 is **909** (23 of 24 runs), and at gap 140 now
35 (21 of 24). So the freeze was one route to a fed ant at the pile, not the
cause. The rest is untraced. The likely reading: stage 2 gets ants to the
food (reached 17 → 915 at gap 90, counting newborns), any fed ant breeds
where it stands, and the bed refills the pile to 400 cells every 400 frames.

**Comparable ants only: those born on the comb**, the 20 founders plus any
born at the nest. The loop counts are round trips, nest to food and back:

| Born on the comb | Shipped | **Stage 2** | Stage 2 higher / lower, of 24 seeds |
|---|---|---|---|
| gap 90: reached the food | 17 | 9.5 | 0 / 24 |
| gap 90: round trips | 14 | 8.5 | 1 / 21 |
| gap 140: reached the food | 5 | 6.5 | 13 / 8 |
| gap 140: round trips | 1 | **5** | 21 / 2 |
| gap 200: reached the food | 0 | 4 | 22 / 1 |
| gap 200: round trips | 0 | **4** | 24 / 0 |

- **At 140 and 200 cells, stage 2 is the first change on this line that
  makes founders complete round trips**: higher on 21 and 24 of 24 seeds,
  where the shipped walk manages 1 and 0.
- **At 90 cells it is worse for the founders.** Fewer reach the food, and
  more die early: by frame 6,000 a median of 11.5 are alive against 19
  (fewer on 18 of 24 seeds). Deaths by then, all ants: **394 against 33**,
  of which 255 against 20 starved and 130 against 8 were killed. Untraced.

**Stage 2 does not ship.** The question it leaves is not about walking:
whether an ant should be able to breed away from the nest. While any fed ant
can, a colony whose ants reach the food breeds there, and this bed stops
measuring the loop.

**Two instruments were wrong along the way, both fixed:**
- **The paired parser filed every run's food budget under the previous
  seed.** trailfollow prints a run's budget lines *before* its summary row
  and its loop funnel after. The loop figures were filed correctly, so no
  number in §Z33 or census report §13 moves. The shipped budget still
  closes to 17 cells over 24 runs.
- **A run where no crop ever held food priced a cell at ε**, so its
  "chewed" read 8 × 10⁹. It now falls back to a fresh cell's worth.

**Data:** `Reports/data/bed-stage2-{shipped,trail,v1-trail}-2026-09-23.log.gz`,
`bed-stage2-deaths-{shipped,trail}-2026-09-23.log.gz` (gap 90, 9,000 frames,
with deaths by cause), and `scene-stage2-fixed-{s1,s2,s3,s5}-2026-09-23.log`.

## 9. Breeding only at the nest, and a bed that was killing ants

**The owner's ruling, 2026-09-23:** an ant should breed only at the nest,
and in the end *where* should be something a lineage evolves. First as a
switch, measured before anything is made heritable:
`PIXEL_PHYSICS_BUD_SITE=nest` (or `World::bud_at_nest`). A species that
names a nest material buds only while at it (the same read as `AtNest`);
a species with none is untouched. `CreatureStats::buds_held_for_nest`
counts the ticks an animal could have budded and was held. Guard
`a_nesting_ant_buds_only_at_its_nest_when_the_switch_is_on`, watched red:
with the gate's `return None` removed, all six founders budded, four of them
40+ cells from any nest.

**The bed's pile refill wrote over whatever stood on the pile**, ants and
crumbs included (`trailfollow`'s `place_food`). A head cell written over is
a death booked as `Killed`, crumbs became fresh fruit, and each write was
booked as food taken. The shipped walk had 15 such writes over 24 runs at
gap 90; stage 2's ants crowd the pile, and it had a **median of 744 a run**.
The refill now skips an occupied slot. On the shipped walk every run that
never wrote over anything is identical to before, and the 8 `Killed` deaths
it had by frame 6,000 at gap 90 are gone. **It was not what made stage 2's
colonies breed at the pile**: with the refill fixed and no nest rule, they
still do (median 1,592 births a run at gap 90).

**Four arms, 3 gaps × 24 seeds, the refill fixed in all of them.** Medians
a run; "higher / lower" is per seed.

The nest rule, each walk against itself:
- **Shipped walk: almost nothing changes, as predicted.** It held the 4
  births at gap 90 that happened away from the nest; gaps 140 and 200 are
  identical.
- **Stage 2: the boom is gone.** Births at gap 90 **1,592 → 0** (23 of 23
  seeds that had any), ants alive at the end 1,612 → 20; breeding at the
  pile is gone at every gap. Founders' round trips 8 → 9 at gap 90 (12 /
  5), unchanged at 140 and 200.

The two walks against each other, **both with the nest rule**, so the
colonies are the same size (20 founders, no births) and the numbers finally
compare like for like:

| Founders, stage 2 against shipped | gap 90 | gap 140 | gap 200 |
|---|---|---|---|
| round trips | 13.5 → **9** (3 / 20) | 1 → **5.5** (21 / 1) | 0 → **4** (24 / 0) |
| reached the food | 17 → 10.5 (0 / 24) | 5 → 7 (13 / 8) | 0 → 4.5 (22 / 1) |
| food delivered to the nest | 525 → 1,412 (23 / 1) | 9.5 → 472 (23 / 1) | 0 → 127 (23 / 1) |
| starved by frame 6,000, all seeds | 19 → **249** | 331 → 356 | 452 → 397 |

**Stage 2 brings more food home at every distance and makes more round
trips at 140 and 200 cells. At 90 cells it makes fewer, because half its
founders starve in the first 6,000 frames.** Traced: every founder's first
trip out, 4 seeds, gap 90 (`bed-budsite-first-trip-gap90`):

| First trip out, 80 founders | Shipped | Stage 2 |
|---|---|---|
| reached the food | **64** | **34** |
| never got there, and died | 16, 2 died | 46, **41 died** |
| progress toward the food per step, for those that got there | 0.70 cells | 0.53 cells |

The 46 that never got there took a median of 245 steps, **98% of them onto
the trail**, and ended **2.5 cells behind where they started**. They walk
the route faithfully, both ways, and starve on it. For those first 6,000
frames the bed lays a trail from nest to food, and the shipped walk follows
it one way through the throttle. Presence has no direction. This is S5's
miss (§6) on the colony, and it is what plan §4c's running average is for:
is the scent getting stronger as I go.

**And the lab's evolutionary clock says the rule cannot ship as it stands.**
`labforage scenario=played_bed`, 6 seeds × 120,000 frames, the shipped walk,
the measurement the queen rule was rejected on
(`evolution-lab-breeding-clock-2026-09-10.md`):

| Lab played bed | Breed anywhere | **Only at the nest** |
|---|---|---|
| deepest generation, median (per seed) | 13 (14 13 11 13 14 6) | **1** (0 4 0 2 5 0) |
| deepest generation that itself bred, median | 12 | **0.5** |
| seeds where no animal was ever born | 0 | **3 of 6** |
| ticks an animal could have budded and was held, median | 0 | 114,040 |
| deliveries to the nest, median | 2,278 | 2,491 |

The breed-anywhere arm reproduces the clock report's 13.5. The nest arm is
as bad as queen-only was. **Not traced yet, and the likely reason:** ants do
reach the nest (deliveries unchanged), but the ones that get rich are the
ones that gorge at the food, and only a *carrying* ant heads home. A rich
ant with an empty crop has no reason to go back, so it is held for tens of
thousands of ticks and never breeds. So the switch stays off by default,
and the next experiment is its companion: **an ant ready to bud heads home**
the way a carrying one does.

**Data:** `Reports/data/bed-budsite-{off,offnest,trail,trailnest}-2026-09-24.log.gz`,
`bed-budsite-first-trip-gap90-2026-09-24.txt`, `lab-clock-budsite-2026-09-24.txt`
(each run's `SUMMARY` line).

## 10. Why breeding only at the nest stalls the clock: traced

§9's clock result came with an untraced guess, and four other explanations
fit the same two counters. Each would need a different fix, so they were
traced before anything was built. `labforage budtrace=FILE` writes what
`try_bud` would weigh for every animal every 30 frames:
`creature::bud_readiness`, a read-only probe built from `try_bud`'s own
calls and checked against the gate in its guard. It records bank, food in
reach, bar, whether the ant is at the nest, the distance to the nest site,
and, for an animal that could bud, what stands on each of the three lines a
child can be placed on. Lab played bed, the shipped walk, 60,000 frames.

| Ready to bud (bank + food in reach ≥ bar) | seed 1, nest rule | seed 4, nest rule | seed 1, breed anywhere |
|---|---|---|---|
| samples, ants | 18,657, 18 | 24,678, 38 | 8,199, 33 |
| rich in the body alone (bank ≥ bar) | 17,917 | 23,339 | 7,218 |
| food in reach, median | 0 J | 0 J | 0 J |
| distance to the nest site, median (all samples) | 83 (55) | 64 (23) | 26 (73) |
| at the nest | **2,008 (11%)** | **565 (2%)** | 1,408 (17%) |
| of those at the nest, with any line clear for a child | **0** | **7%** | |
| away, with any line clear | 36% | 57% | |
| buds refused for lack of room (`births_denied_no_space`) | 9,983 | 2,808 | 40,218 |
| births seen | 0 | | 58, **none at the nest** (median 88 cells from it) |

- **Not the pile.** A ready ant is rich in its own body; there is no food in
  reach at all.
- **Not drained at the nest.** Ready ants stay ready there, tick after
  tick.
- **Mostly away from home, because nothing sends them there.** Only
  carrying does, so 89–98% of the time an ant could bud it is away from the
  nest.
- **And at the nest, the child does not fit.** `try_bud` places a whole
  adult-length body in a straight line east, north-east or north of the
  head. At the nest those lines meet, most often, water (seed 1's nest holds
  it), other ants, delivered crumbs, nest wall and packed soil. The count
  agrees: 2,008 samples, each about 5 ant-ticks, is about 10,000 ticks at
  the nest ready, against 9,983 refusals for no room. **Every tick a rich
  ant spent at the nest was a refused bud.**

**Even breeding anywhere, no ant bred at the nest.** The shipped colony's
births all happen out in the open, and 40,218 buds were refused for room on
that seed alone. Room is a birth bottleneck everywhere, not just under the
rule.

**So a fix needs both halves:** a reason for a ready ant to go home, and a
way for a child to fit where ants live: placed along the free cells of a
tunnel rather than in a straight line, or born small (an egg or brood) and
grown.

**Data:** `Reports/data/lab-budtrace-{any-1,nest-1b,nest-4b}-2026-09-24.csv.gz`.

## 11. Stage 2 with a direction: away from home along a route

§9 traced stage 2's loss at 90 cells to empty founders walking the laid
trail both ways. The plan's §4c proposed a "scent getting stronger" sense.
**Its §4d says why that is wrong for the empty leg**: the trail an empty ant
reads is B, laid at a constant rate by laden ants, so the most lies where
they stall near the nest, and its gradient points home. A rising-scent sense
would lead empty ants back to the nest. The bed's hand-laid B ramp rises
toward the food, so that mistake would have measured well here and failed
in the game. §4d's answer is what recruited ants do: a weak pull **away from
home**, from the home vector.

**Built as `PIXEL_PHYSICS_CHOOSER=trailaway`**: stage 2, plus, for an empty
ant, `AWAY_GAIN × presence × cos(heading, away from home)` on each heading
(`AWAY_GAIN` 1). **Scaled by presence**, so it acts only on a route: off a
trail an empty ant explores as before, and cannot be pinned against a wall
that lies away from home the way a laden ant was in a U-bend. Guard
`on_a_route_an_empty_ant_turns_away_from_home_under_trailaway_and_not_under_trail`,
on an evenly laid trail so no gradient can supply the direction; watched red
with the gain at 0 (the ant walked 84 cells toward home). S4 and S5 now set
the ant's start as home (it was the world's corner); the other arms are
byte-identical with or without that.

**Predictions, written before the runs:** S0–S3 byte-identical to stage 2;
S5 trailed row 21+ of 24 at a median well under 1,189, near or below 702;
bed, nest rule on every arm, gap-90 round trips from 9 toward 13.5, first
trips from 34 of 80 toward 64, 140 and 200 no worse than 5.5 and 4.

**Scenes:** S0–S3 and S5 laden byte-identical to stage 2, as predicted.

| | Shipped | Stage 2 | **Away from home** |
|---|---|---|---|
| S5 trailed row: got 60 cells east | 21, median 702 | 21, median 1,189 | **24 of 24, median 297** |
| S5: furthest east, median | 144 | 97 | **159** |
| S4: first branch follows the trail, level / up | 18 / never | 22 / 18 | 22 / 17 |
| S4: time on the trailed branch, level / up | 73% / 0% | 40% / 22% | **96.5% / 60%** |

In S4 the ant now goes out the trailed branch and stays near its blind end,
which is what a trail's far end is for; in the game it is the food.

**The colony bed, nest rule on every arm** (20 founders, no births in any
arm; today's walk and stage 2 reproduce §9's runs byte for byte). Medians a
run; higher / lower of 24 seeds against stage 2 in brackets:

| Founders | gap 90 | gap 140 | gap 200 |
|---|---|---|---|
| round trips: shipped → stage 2 → **away** | 13.5 → 9 → **14** (23 / 1) | 1 → 5.5 → **14** (23 / 0) | 0 → 4 → **11** (23 / 0) |
| reached the food | 17 → 10.5 → **15** | 5 → 7 → **15** | 0 → 4.5 → **13** |
| cells taken from the pile | 31 → 45 → **85** | 7 → 22.5 → **67.5** | 0 → 11 → **39** |
| starved by frame 6,000, all seeds | 19 → 249 → **95** | 331 → 356 → **150** | 452 → 397 → **219** |

**The food budget closes** (residual −11, 3 and 27 cells over 24 runs, against
2,130, 1,698 and 977 taken), so the extra food is food.

**Traced, every founder's first trip out** (gap 90, 4 seeds, the §9 seeds):
54 of 80 reach the food, against 34 on stage 2 and 64 on the shipped walk.
Those that make it are faster and straighter than the shipped walk (median
frame 1,893 against 2,478; 97.5 steps against 119; 4 turn-backs against
12). **What is left:** 26 still never get there (16 on the shipped walk),
and early starvation at 90 cells stays above the shipped walk's.

**So the loop now works at every distance on this bed**: founders make a
median of 11–14 round trips at 90, 140 and 200 cells, where the shipped walk
makes 13.5, 1 and 0. Still behind a switch, and not yet seen by eye.

**Data:** `Reports/data/bed-trailaway-2026-09-24.log.gz`,
`bed-trailaway-first-trip-gap90-2026-09-24.txt`,
`scene-trailaway-{s4,s5trail}-2026-09-24.log`; the shipped and stage-2 arms
are §9's `bed-budsite-{offnest,trailnest}`.

## 12. Without the hand-laid trail: the colony finds the food and builds the road

*2026-09-24.* Owner: *"This is still starting with the hand laid trail or are
they finding the food themselves?"* Every bed result in §8–§11 started from
it: `arms=hand` lays trail B from nest to food every 60 frames until frame
6,000 of 24,000. **`arms=self` lays nothing**, so the colony has to find the
pile by wandering and lay its own trail home. Same bed otherwise (20
founders, nest rule on, 24 seeds a gap).

**Predictions, written before the run:** discovery about equal in all three
walks (the away term is scaled by B presence, and before anyone finds food
there is none); any gain from trailaway only after a first finder walks home;
140 and 200 near zero round trips; at 90, trailaway > trail ≥ shipped in a
minority of seeds.

**Wrong on the first and third.** Founders' round trips, median a run
(ants that ever reached the food in brackets; colonies with 17+ of 20 dead):

| No hand trail | gap 90 | gap 140 | gap 200 |
|---|---|---|---|
| today's walk | 0 (0), 24 of 24 dead | 0 (0), 24 | 0 (0), 24 |
| stage 2 | 7 (8.5), 21 | 4 (4.5), 24 | 2 (2), 24 |
| **away from home** | **10.5 (12)**, 9 | **4.5 (6)**, 23 | **2 (2)**, 24 |

Against the same walk *with* the hand trail (§11), paired within seed:

| Away from home, round trips | gap 90 | gap 140 | gap 200 |
|---|---|---|---|
| hand-laid trail | 14 | 14 | 11 |
| **no trail** | **10.5** (lower on 22 of 24) | **4.5** (23) | **2** (24) |

So close in, the ants mostly do it themselves; further out the hand trail was
doing most of the work. Today's walk depends on it entirely: 13.5 round trips
at 90 cells with it, 0 without.

**Traced, every ant, 6 seeds** (`decisioncsv`; the trace's arrival count
matches the harness on every seed):

- **Finding it alone.** With the new walk some ant finds the pile **in every
  seed at both 90 and 140 cells**, at a median frame of 972 and 1,551. With
  today's walk one ant does, in 3 of 6 seeds at 90 and 1 of 6 at 140; its
  other ants never get more than 20 cells from the nest (median). So the
  chooser's straighter walk is what finds food; it is not the trail term.
- **Recruitment after the first find.** At 90 cells, 61 of the 63 later
  arrivals were on another ant's trail for at least part of the way out (33
  for most of it). At 140, 22 of 25, but 20 of them only partway: they wander
  out and meet the trail somewhere along it.
- **The self-laid trail does reach the nest.** Sampled cell by cell every 500
  frames (`btrail`, 140 cells, seed 1): nothing until frame 2,500, then
  continuous from x = 50 (inside the nest strip, 26–70) to the food by 3,500.
  The run summary's `B nest->food [8, 1088, …]` reads low at the nest only
  because its first probe sits at the nest's centre, past where laden ants
  stop laying. Not a finding; recorded so nobody chases it.

**At larger gaps** (same arms, 24 seeds; today's walk 0 everywhere):

| Away from home, no trail | 260 | 320 | 400 |
|---|---|---|---|
| ants that ever reached the food, all seeds | 25 | 7 | 0 |
| seeds with any arrival | 14 of 24 | 7 of 24 | 0 of 24 |
| round trips, all seeds | 13 | 2 | 0 |

Predicted 1 / 0–1 / 0 arrivals a run and 16 / 10 / under 6 seeds: close, a
little low. **400 cells is out of a founder's range**: searching, an ant
covers about a cell per 11 frames (first finds above), and it lives about
3,300 frames on its starting energy. Stage 2 (no away term) is the same as
trailaway at every one of these gaps, as predicted: with so little trail there
is nothing for the term to act on.

**What the misses were doing** (the ants that never reached the food).
Almost none died before a trail existed: **80 of 90** at 140 cells and **50 of
50** at 90 were alive when the first finder got home. Most then died **on
the nest** (33 of 50 at 90, 50 of 90 at 140; 30 more at 140 on the ground
just east of it). Being underground does not separate them (26% of their
decisions against 23% for ants that made it); hauling spoil a little (19%
against 10%). One traced end to end, seed 1, ant 7: it dug and idled on the
nest until frame 1,400, drifted west to the box wall, climbed it and walked the
ceiling until it starved at 3,282, having never touched the trail, which
reached the nest at 2,268. That kind is 11 of 50 at 90 cells. **The common
kind died beside a working loop**: seed 3's ant 1 had 94% of its starting
energy when the first food came home and starved on the nest 3,500 frames
later. Why is §13.

**Data:** `Reports/data/bed-selfarm-2026-09-24.log.gz` (the three walks),
`bed-selfarm-wide-2026-09-24.log.gz` (260/320/400),
`bed-selfarm-trace-2026-09-24.txt` (the per-ant traces),
`bed-selfarm-btrail-140-s1-2026-09-24.log.gz`.

## 13. Why the ants at the nest starve beside a working loop

*2026-09-24, continuing §12 on the no-trail bed (trailaway, `arms=self`, nest
rule on, 3 gaps × 24 seeds, paired within seed against §12's trailaway arm).*

**The food is there and it does not reach them.** At 90 cells, seed 1, the
colony takes 57 cells from the pile over the run and 54.6 of them are eaten;
what reaches the nest is crumbs worth 2 cells in all, and **food on the nest
ground reads 0 at every 3,000-frame sample in every run**. The pile never
runs short (400 cells standing throughout). Taken at 90 cells is about 58
cells, ~55,000 J a run, against roughly 34,000 J that 20 ants burn over the
run at the traced rate, so this is distribution, not supply.

### 13a. The hunger gate, re-tested: fires, changes nothing

`dead-ends.md` holds `digest_hunger_weight` (digestion slows once an ant is
well fed, so a carrier keeps its load) at 0.0, measured twice as bad under
today's walk, with *re-test when colonies get rich*. Under the chooser the
foragers do get rich, so: `hungergate=1`. **Predicted** food on the nest
ground off 0 and fewer starved at 90. **Measured:** it fires (a median 1,564
J held in carriers' crops a run at 90 cells, 22 of 24 seeds; exactly 0 in the
base) and moves nothing: nest ground still 0, starved 14 → 14, wiped 9 → 11,
round trips 10.5 → 10.5. Why it cannot help is §13b: the carrier eats what it
delivers, whether or not its gut waited.

### 13b. The carrier eats its own delivery (traced)

At 90 cells a colony logs **about 2,260 drops a run for about 58 cells
taken**, so each cell is put down about 40 times. One carrier, seed 1 ant 12
(503 drops): it stands at x = 65 on the nest at full energy, never moves,
and for hundreds of decisions picks up, puts down and picks up the same
scrap, nibbling it the whole time (fill 0.079 → 0.074). The wiring does this
by itself: `Feed` is `squash(0.4 + 0.8 × FoodAdjacent)` = 0.55 next to food
for **every** ant, `Drop` is +1.09 at the nest for **every** ant, and being
next to food cuts `Move` (`FoodAdjacent −1.16`). A delivery puts the food
next to the carrier, so it stops and eats it. This is the thrash
`pheromone-trail-direction` §7.28 predicted for the granary, now measured on a
bed where the loop runs.

### 13c. Stopping the re-grab leaves the food uneaten

`(AtNest, Feed, −0.7)` + `(Energy, Feed, −0.7)`: in a linear sum that is an
AND, so a **full ant at the nest** cannot pick up (0), everyone else can
(hungry at the nest 0.33, full at the pile 0.33, hungry at the pile 0.55).
Control: `(AtNest, Feed, −1.2)`, nobody picks up at the nest. Harness rider
`wire=` (new, `labforage`'s spelling). **Predicted** fewer drops, food
standing at the nest, fewer starved. Gap 90, medians a run:

| | base | AND | block everyone |
|---|---|---|---|
| drops | 2,260 | 772 | 164 |
| cells taken from the pile | 58 | 54 | 43.5 |
| **cells eaten** | **52.5** | **39** | **27** |
| starved (of 20) | 14 | 17.5 | 19 |
| colonies with 17+ dead | 9 of 24 | 15 | 23 |

**Wrong where it mattered.** The cycle breaks and the food then sits as
crumbs at the nest, eaten by nobody. Traced (4 seeds): hungry ants on the
nest *are* next to food, 72% of their decisions below half energy, and pick
it up on 19% of those (41% in the base, where they were next to food on only
2.6–12%). The Energy term is too blunt, and **`Drop` puts back what a hungry
ant does pick up**: +1.09 at the nest for every ant, the "4 ticks held" of
§7.29, which applies to the hungry as much as the full. Sharing is not the
route either: `SHARES` (new) moves about 2,000 J a run (1,400 with the AND)
against ~50,000 J eaten.

### 13d. A hungry ant keeps what it holds: hunger on `Drop`

The division of labour §7.28 wanted, *a full ant carries and drops; a hungry
ant eats*, belongs on `Drop`: `(Bias, Drop)` −0.2 → **−2.0** and a new
`(Energy, Drop, +1.8)`, `AtNest` unchanged. Through `eval_brain`
(`mode=feedgate wire=…`, new), per-tick P(drop) at the nest next to food, by
energy 0 / 0.25 / 0.5 / 0.75 / 1: **0 / 0 / 0.022 / 0.14 / 0.25**, against 0.25
at every energy today; away from the nest still 0. A full carrier delivers
exactly as before and a hungry one eats until it is nearly full, since
digestion pays out as it chews. Graded, not a switch. Arm DF adds a milder
pick-up AND (`AtNest:Feed −0.4`, `Energy:Feed −0.8`).

**Predicted** at 90: starved 14 → ≤ 11, wiped down from 9, eaten > 55, drops
down, round trips ≥ base. Medians a run; seeds better / worse against base:

| Drop hunger (D) | gap 90 | gap 140 | gap 200 |
|---|---|---|---|
| cells eaten | 52.5 → **58** (12 / 11; sum +16%) | 14.2 → **21.2** (15 / 8; +46%) | 2.6 → 3.2 |
| cells taken from the pile | 58 → 64 | 19.5 → 24 | 5.5 → 4.5 |
| drops | 2,260 → 1,305 | 438 → 237 | 33 → 12 |
| reached the food | 12 → 13 (16 / 6) | 6 → 5.5 | 2 → 2 |
| reached it a second time | 6 → 7 (13 / 7) | 1 → 2 | 0 → 0 |
| round trips | 10.5 → 10.5 (11 / 8) | 4.5 → 5 | 2 → 2 |
| **starved** | **14 → 14.5** (9 / 7) | 20 → 19 | 20 → 20 |
| colonies with 17+ dead | 9 → 11 | 23 → 22 | 24 → 24 |

DF is no better than D anywhere (eaten 54.1 at 90, starved 15, wiped 12).
**So D feeds the colony more and does not feed the dying**: the extra food
goes to ants that already had some. Not adopted; the wires are a rider, not
in `ant.ron`.

### 13e. The dying are on the far side of the nest

Where the ants that never reach the food step, on and near the nest after the
first forager is home (6 seeds, 90 cells; x from the nest's centre, the strip
is −22..+22; share of steps landing on trail B):

| surface steps | −30 | −20 | −10 | 0 | +10 | +20 |
|---|---|---|---|---|---|---|
| misses | 502, 0% | 878, 8% | 715, 7% | 446, 24% | 282, 62% | 91, 89% |
| reachers | 407, 0% | 713, 6% | 896, 10% | 960, 23% | 1,407, 64% | 1,239, 63% |

**About 70% of the misses' surface steps are on the west half, where the
trail is not**: carriers come in from the east and put their load down at the
east edge, so the trail ends there, and an ant on the far side of a 44-cell
painted strip never crosses it. Part of this is the bed: founders are laid in
a band `ants × 4` wide centred on the nest, and a dug nest with one entrance
would have no far side. **Nest digging is its own open track** (the owner,
2026-09-24), so no mechanism is built around the strip here.

**What this leaves for the loop:** discovery works (§12), recruitment works
where ants can meet the trail, and the food exists. The next link is getting
nest ants to the trail's start: a nest shaped like a nest, or a search that
brings a hungry ant past the nest's mouth.

**Data:** `Reports/data/bed-selfarm-hungergate-2026-09-24.log.gz`,
`bed-selfarm-nestpickup-2026-09-24.log.gz` (AND and control),
`bed-selfarm-nestpickup-trace-2026-09-24.txt`,
`bed-selfarm-drophunger-2026-09-24.log.gz` (D and DF),
`bed-selfarm-westhalf-gap90-2026-09-24.txt`.

## 14. The new walk in the lab box

*2026-09-24.* Everything above is the colony bed. The lab
(`labforage scenario=played_bed`, 120,000 frames, colonies arriving on the
scenario's timeline, plants regrowing, budding anywhere, which is the lab's
default) is where the ants actually live, and nothing had measured the
chooser there. Three arms, 12 seeds, paired within seed: today's walk,
`PIXEL_PHYSICS_CHOOSER=trailaway`, and trailaway with §13d's drop wires.
Today's walk on seed 1 reproduces §10's `any` arm digit for digit (born 239,
alive 108, deliveries 749).

**Predicted:** deliveries up; births and the deepest breeding generation
within today's spread (a big fall would mean the walk costs the lab's clock);
the drop wires deliver a little less.

| Lab, 12 seeds, median a run | today | new walk | new walk + drop wires |
|---|---|---|---|
| deliveries (food carried home) | 1,398 | **5,164** (12 / 0) | 4,815 (11 / 1) |
| food intake, J | 371k | 961k (11 / 1) | 836k (11 / 1) |
| births | 146 | 420 (8 / 2) | 450 (11 / 1) |
| **deepest generation that itself bred** | 10 | 22 (10 / 2) | **26** (11 / 1) |
| columns of the box ever visited | 218 | 504, all of it | 504 |
| alive at 120,000 frames | 50 | 20 (4 / 8) | **96** (7 / 4) |
| went extinct | 0 of 12 | 1 | 2 |

(Better / worse than today in brackets.) **The clock was the wrong
prediction, in the good direction**: the lab evolves more than twice as
fast. On its own the new walk leaves colonies smaller at the end because they
boom and bust; with the drop wires they end twice today's size.

**The extinctions are overgrazing, not the walk failing.** Seed 5, every
9,000 frames: under today's walk the colony holds near 100 ants and the plants
between 650 and 1,250 standing; under the new walk with the drop wires the
colony grows to 334 and then 379 ants, the plants go from 1,247 standing to
54 by frame 72,000, and the colony starves out by 108,000. Today's walk
cannot reach enough of the box to do that. So the lab becomes a world that can
be eaten bare on some seeds, which is the owner's call rather than a bug.

**Seen by eye, one seed** (`labshot mark=ants`, seed 4, frames 30,000 to
120,000; posted as a card): under today's walk the colony stays near its nest
and the plants thrive (654 standing at the end); under the new walk the ants
range the whole box up to the ceiling, the ground is bare by 90,000 and 15
plants are left. The same seed also shows the colony **digging 13× as much**
(9,663 digs against 711) and a nest room of 514 roofed cells against 45.
That is one seed and nothing here measured digging across seeds; recorded
because the nest-digging line will want to know.

**Frame cost** (`ascii scene=foraging`, 15 ants, four alternating pairs,
`RAYON_NUM_THREADS=2`): mean **0.658 → 0.717 ms** a frame, slower in 4 of 4,
with the ants taking **4.2× the steps** (2,742 → 11,615) and delivering
1,306 against 729. Worst frames ran 6–25 ms in both arms with no pattern and
are not pinned by the mean, so they say nothing. Per ant that is roughly
0.004 ms, on the order of a millisecond at the lab's peak of 300+ ants; not
measured there.

**What a default flip would touch.** The whole suite run with
`PIXEL_PHYSICS_CHOOSER=trailaway` set (the flip, approximated without a code
change): worldgen, determinism, the druid and app tests all pass; **8 of
1,895 library tests fail**. Four pin today's walk and would set it explicitly
(the decision-trace identity and reconciliation, the homeward re-roll, the
lifetime counters: each needs tumbles or blocked moves, which the chooser
does not make). Two are scenes that depend on how ants meet (a lone grazer
on a moss lawn; armoured ants taking a median 528 frames to meet at reach 1).
**Two are real questions before any flip:** a floating flitter under the
chooser gets within 2 cells of the flower it must reach, so the chooser
reaches flyers and changes them; and in `a_released_animal_falls_like_a_founder_does`
the released ant is **killed** within 400 frames (one death booked `KILLED`)
where today it falls and lives.

**Where this leaves the walk:** on the bed it makes the loop work with no
trail laid (§12), and in the lab it nearly quadruples what the colonies carry
home and doubles the evolution clock. It is still behind
`PIXEL_PHYSICS_CHOOSER=trailaway`, off by default, because turning it on
changes every game's ants, costs about 9% of the ant scene's frame, and makes
some lab boxes go extinct; that is a ruling for the owner, put to them as a
card with this section's numbers.

**Data:** `Reports/data/lab-walk-2026-09-24.txt.gz` (every `SUMMARY` line),
`lab-walk-summary-2026-09-24.txt` (the paired table and the timing pairs),
`lab-walk-seed5-series-2026-09-24.txt`; the seed-4 sheets are review card `20260924T064546699Z-98e7d7`.

## 15. Made the default

*2026-09-24, the owner's ruling* ("go ahead with option 1, make it the
default"). An ant now walks `trailaway` with the §13d drop wires; `off`
restores the walk before it.

**What changed:**
- `chooser_from_env` defaults to `TrailAway`, and `PIXEL_PHYSICS_CHOOSER=off`
  is the way back.
- **Scoped to species that name a nest** (`chooser_for`): the ant and its
  variants, the beetle and the hopper. The flitter and the worm walk as
  before, whatever the switch says. This fixes the flitter of §14, and guard
  `a_species_with_no_nest_walks_the_same_whatever_the_chooser_says` holds it
  (every decision row equal for the flitter, unequal for the ant; watched red
  with the scope removed).
- `ant.ron`'s `Drop` row is `Bias −2.0, Energy +1.8, AtNest +1.0889,
  Carrying +0.2`. `ant.ron` only; the variants were never measured with it.

**The shipped default is the measured arm, digit for digit.** The default
build with no switch and no rider reproduces §13d's "drop hunger" arm on the
colony bed (seeds 1 and 2 at 90 cells, every column), and §14's "new walk +
drop wires" arm in the lab (seeds 1 and 2, identical `SUMMARY` lines). So
every number in those sections describes the game as it now ships.

**The suite.** With the default switched, 7 of 1,896 library tests failed,
and each tests something other than the default walk:
- Four assert rules of the old walk (tumbles, blocked moves, the homeward
  re-roll), and now pin `Chooser::Off` with the reason written at the pin.
- Three are scenes that assume two ants do not meet: the lone grazer, the
  armoured-ant reach arm, and the released specimen, which under the chooser
  meets a foreign founder that bites its head off (one attack, one cell,
  booked `KILLED`, found by probe). All pinned the same way.
- `the_decision_trace_changes_nothing_it_watches` is **extended rather than
  pinned**, because the walk the ant runs is the one whose trace matters most.
  It runs both walks, and the chooser arm for 9,000 frames, since in 3,000 none
  of its ants had eaten, so the laden half of the trace went untested. The new
  arm was watched red with a trace-only draw planted inside `chooser_step`.

**Costs, restated:** about 9% of the ant scene's frame (`ascii`'s foraging
scene, mean 0.722 ms with the default); 2 of 12 lab boxes eaten bare and
extinct over 120,000 frames, against 0 before (§14).

## 16. The loop counted ant by ant, and where the food goes

The owner asked three things of the shipped default (§15), with no hand-laid
trail, at 90 cells: what share of ants get through each step of the loop; does
food pile up at the nest, i.e. is the economy too hard or is the loop too weak
or do ants fail to eat what is there; and are the ants that are not looping
exploring, or stuck. Every number below is from one command, now
`scripts/antloop.py`, over 24 seeds × 20 founders (`arms=self gaps=90
frames=24000 food=400 refill=400`, the bed of §12–§15), and its starved count
matches the harness's `DEATHS BY CAUSE` on every run (342 = 342).

**The funnel.** Each ant is booked at the furthest point it reached; a loop
counts only if the ant went back to the food before the next one.

| | ants | of prev | of all |
|---|---:|---:|---:|
| reached the food | 287 | 59.8% | 59.8% |
| picked food up there | 276 | 96.2% | 57.5% |
| got home still holding it | 255 | 92.4% | 53.1% |
| put it down at the nest: 1 loop | 237 | 92.9% | 49.4% |
| reached the food a 2nd time | 161 | 67.9% | 33.5% |
| 2+ loops | 121 | 75.2% | 25.2% |
| 3+ loops | 55 | 45.5% | 11.5% |
| 4+ loops | 12 | 21.8% | 2.5% |
| 5+ loops | 0 | 0% | 0% |

**The loop itself is not where the colony is lost.** Once an ant reaches the
food, 83% of them finish a loop (237 of 287). Over the whole run 425 loops
were completed and 118 broken (83 loads eaten on the way, 35 brought home
and eaten there).
**The loss is the first step**: 40% of ants never reach the food at all. No
ant makes five loops because a loop takes about 4,840 frames, so five is
most of a 24,000-frame run.

**Who starved** (342 of 480, 71%):

| how far it got | ants | starved | share of the dead | median frame of death |
|---|---:|---:|---:|---:|
| never reached the food | 193 | 188 (97%) | 55% | 3,792 |
| reached it, no loop | 50 | 45 (90%) | 13% | 9,276 |
| exactly 1 loop | 116 | 82 (71%) | 24% | 13,488 |
| 2–3 loops | 109 | 25 (23%) | 7% | 18,612 |
| 4+ loops | 12 | 2 (17%) | 1% | 21,798 |

**Food does not pile up at the nest, and the ones who die first die before
there is any.** Food standing on the nest, as joules an ant would absorb, at
the median run: **0 J at frame 3,000, 152 J at 6,000**, then about 600 J from
frame 12,000 to the end (the best run peaks at 2,212 J). A never-looper dies at
a median of frame **3,792**. That is its starting grant (200 J at 0.0688 J a
frame is about 2,900 frames, staggered per founder) and nothing more. When
it dies the nest holds almost nothing to eat. Later, 600 J is about a third
of one ant's need for the whole run (1,652 J).

**The colony absorbs 42% of what it burns**, a median of 13,230 J a run
against a need of 33,034 J. **The food goes to the ants that fetch it**:
ants with a loop made up 49% of the colony and ate 88% of the food, 960 to
1,460 J each. That is close to one ant's own need, so a forager
mostly feeds itself. Each loop takes 3.8 cells from the pile. That is 905 J
to an ant at the shipped gut, where plant food is worth 0.25 of its face
value, and a loop's 4,840 frames burn about 333 J.

**Ants not on the loop are idle, not stuck.** Where a never-looper's decisions
go, for the typical ant: 32% standing on the nest, 20% moving about on it, 16%
digging or hauling dirt, **3% out exploring**, 2% standing off the nest. Only
2.8% of their decisions are ones where they physically could not move. So
they are free to move and choose not to leave the nest. Ants that loop
spend 48% of their time carrying food and 5% exploring.

**So, of the three explanations the owner offered:**
- *Food builds up but ants don't eat it:* **no.** The stock is small, and the
  largest group of the dead died before it existed.
- *The loop is too weak:* **the loop is fine once found.** Finding the food
  is the weak step: 40% of ants never go 90 cells out, and they die on the
  nest when their starting grant runs out.
- *The economy is too hard:* **partly.** Food eaten comes to 42% of food
  burned. Even a forager making 2–3 loops eats only about its own need, so
  the colony cannot keep non-foragers alive. Three levers would change
  that: the gut's 0.25 yield on plant food, the 200 J starting grant, and
  the 3.8 cells a loop carries. Each is a design choice, not a bug.

The owner's view, recorded here because it sets the bar: **not every ant has
to follow the same loop, and some should be out exploring; standing idle on
the nest is the defect.** On these numbers the fix is to send the idle ants
out, not to make every ant forage. A 3% exploring share is far too low for
a colony whose food is 90 cells away.

## 17. Three levers read with the funnel: a nest pickup rule, where the road starts, and what a load weighs

All on §16's bed (shipped default, no trail, 90 cells, the same 24 seeds),
each read with `scripts/antloop.py` and paired seed by seed against §16.
Logs and the command's output for every arm, and for §16 itself, are in
`Reports/data/bed-default-*2026-09-25.*`.

### 17a. A full ant at the nest cannot re-take its delivery: worse again

The pickup AND of §13 (`wire=AtNest:Feed:-0.7,Energy:Feed:-0.7`), now on top
of the shipped drop wires, which is half of the condition `dead-ends.md` set
for re-testing it. **Not adopted.** Ants that complete a loop **237 → 201**
(lower on 17 of 24 seeds), loops **425 → 360** (lower on 15), food absorbed
median **13,230 → 10,872 J** a run (−18%), starved 342 → 363 (12 seeds up,
10 down: no change). Predicted: no change up to the first loop. Wrong: the
funnel fell about 7 points from the first step on.

### 17b. The ants that never find the food never meet the road

§16's largest group of the dead is the 193 ants that never reach the food.
Traced one by one:

- **The trail is how most ants find the food.** 214 of the 287 that
  reached it got there after frame 2,000, and in their last 60 decisions
  before arriving the median one stepped onto a trail every time. Only the
  first 73 found it mostly by chance.
- **Half the never-reachers (48%) never once step onto a trail.** The
  median one gets no further than 17 cells east of the nest centre, inside
  the nest's own ±26-cell band. It spends 95% of its life on that band, and
  23% of its decisions in the 48-cell dead end between the nest and the
  box's west wall, 12% of them against the wall itself.
- **They are not starved of energy early or stuck.** At frame 1,500 they
  hold the same energy as the reachers (0.63 against 0.64 of the grant),
  their median `P(move)` is 0.54, and 2.8% of their decisions are blocked.
- **Birth position predicts it.** Born on the far side of the nest from the
  food (−20 to −11 cells): 37.5% reach it and 85% starve. Born on the food
  side (+10 to +29): 75–79% reach it and 59–63% starve.

The road starts at the east edge of a 53-column painted nest, and an empty
ant off the road walks without direction (§6d of `how-the-ant-works.md`:
the away-from-home term is scaled by the trail under it). A real nest has
one mouth every ant comes out of, so the road starts where everyone is.
That is the nest line's work (`Reports/lanes/nest-entrance-handoff-2026-09-20.md`,
`PIXEL_PHYSICS_NEST_SHAFT`, which cuts a shaft but leaves the painted strip).
**The ceiling it could reach on this bed is the food-side row:** everyone
reaching the food as often as the best-placed founders do. That still
leaves about 60% starving, so the road is one lever of two.

### 17c. A bigger crop starves the carriers: food weighs its joules

`cropcap=5760`, twice the crop, predicted to help a little. **It is a
disaster, and it names the second lever.** Starved **342 → 461 of 480**
(worse on 21 of 24 seeds, better on none), loops 425 → 135, and the median
colony is extinct by frame 18,000.

Traced: **the carriers starve with food in their crops.** Ant 12 of seed 1
loads at frame 3,366 (crop 99% full, energy 0.34 of its grant) and is dead
by frame 4,200 with the crop still 91% full. At death, 34% of all the
starved hold a crop over three-quarters full, against 2% on the default.

The arithmetic, from `carried_cells` and the digest block:
- A load weighs its **worth ÷ `body_energy` (480)**, so a fruit cell (960)
  weighs two body cells. A full shipped crop (2,880) weighs **6 cells on a
  body of 2**, and a step costs 0.125 × 8 = **1.0 J** against 0.25 J empty.
  A full doubled crop weighs 12 cells, and a step costs 1.75 J.
- **Digestion pays at most 3.3 × 0.25 = 0.825 J a tick**, before its
  overhead, whatever the crop holds.
- So on the shipped crop, walking home with a full load costs about what it
  feeds the carrier. On the doubled one it runs at a loss.

That is why §16 found a forager mostly feeds only itself. `carried_cells`'
own doc says *"one cell of food weighs one cell of body"*; that holds only
for food worth 480 a cell (flesh). Fruit weighs double, a flower (1,440)
triple, a leaf (40) a twelfth.

### 17d. Weighing a load by its cells: the foragers live

`PIXEL_PHYSICS_LOAD_BY=cells` (new, off by default, bit-exact unset: seeds
1–8 identical on every per-run, food-store, death and budget line) weighs a
crop by the cells in it, the part-chewed one by what is left of it. A full
crop of fruit then weighs 3 body cells instead of 6.

| arm | starved of 480 | seeds better / worse | loopers who starved | never reached the food, starved | loops | food absorbed (median a run) | burn a frame |
|---|---:|---:|---:|---:|---:|---:|---:|
| default (joules) | 342 | – | 109 | 188 | 425 | 13,230 J | 0.0688 J |
| by cells | 296 | 16 / 8 (p 0.15) | 66 | 203 | 438 | 12,826 J | 0.0557 J |
| by cells, crop doubled | **263** | **20 / 3 (p 0.0005)** | **38** | 192 | 353 | **15,956 J** | 0.0631 J |
| joules, crop doubled (§17c) | 461 | 0 / 21 | – | 226 | 135 | 4,470 J | 0.0723 J |

- **It saves the foragers and nobody else.** Deaths among ants that made a
  loop fall from 109 to 66, and to 38 with the doubled crop (better on 18
  seeds, worse on 4). Deaths among ants that never reached the food do not
  move (188 against 203 and 192), which is §17b's lever, not this one.
- **Alone it is suggestive, with the bigger crop it is clear.** By cells at
  the shipped crop is 16 seeds better and 8 worse (p 0.15). With the
  doubled crop it is 20 and 3 (p 0.0005). A loop then brings home 5.3 cells
  (1,275 J to an ant) against 3.8 (905 J), and the colony absorbs 51% of
  what it burns against 42%.
- **Predictions, recorded before the runs:** starved about 310 by cells
  (296, close); loops up to 460–520 (438, wrong, no change); never-reachers
  unchanged (right); by cells with the doubled crop within 10% of the
  default (263, wrong: 23% fewer deaths).

**Not made the default, and why.** The stated design (*one cell of food
weighs one cell of body*) is what by-cells implements. But crop capacity is
counted in joules, so under by-cells a crop of cheap food is heavy: 72 cells
of 40-J leaf would weigh 72 body cells. No shipped creature carries food that
cheap (a 40-J cell yields 10 J at a neutral gut, under the 12-J floor below
which a mouthful is not food), but a plant gut evolved in the lab could. The
two consistent rules are "weigh by cells, and count capacity in cells" or
"weigh by joules, at a lighter density than flesh". Which one is a design
call for the owner.

**Where this leaves the colony's starvation:** about 190 deaths a run of 24
(the never-reachers) belong to the road and the nest's mouth. About 110
(the loopers) belong to what a load weighs, and two thirds of those can go
with a lighter load and a bigger crop.

### 17e. In the lab box, weighing by cells kills the colonies

The owner's rule is that the lab is where the ants actually live, so before
proposing §17d it was run there: §14's setup (`labforage
scenario=played_bed`, 120,000 frames, 12 seeds), `RAYON_NUM_THREADS=1`, the
default and `PIXEL_PHYSICS_LOAD_BY=cells` from one binary. The default
reproduces §14's `awayd` arm on all 12 seeds (born, died, alive, deliveries
and intake identical).

| Lab, 12 seeds, median a run | default | by cells |
|---|---:|---:|
| food intake, J | 836,343 | **97,330** (lower on 12 of 12) |
| births | 450 | **25** (lower on 11) |
| deepest generation that itself bred | 26 | **3** (lower on 11) |
| alive at 120,000 frames | 96 | 0 |
| went extinct | 2 of 12 | **10 of 12** |
| plants standing | 88 | 1,027 |

**The reason is what the lab ants eat.** On the default, intake per bite
is 19–50 J. At the 0.25 gut that is food worth roughly 80–200 J a cell,
well under the 480 at which the two rules agree. So by cells makes the
lab's loads 2.5–5 times heavier, where on the bed it made fruit half as
heavy. Energy burned per move rises from 1.0 to 1.3–2.0 J on three of the
first four seeds, and the founders starve before the colony grows.

**So by cells is rejected as built.** What helped on the bed was a lighter
load, and fruit is simply the richest food per cell there is. The bed's
number is still real: carrying a full crop costs about what it pays, and
the colony's foragers die of it. But the lever to try next is **a load that
is lighter per joule for every food**, not one that re-prices foods against
each other. That is a new constant, which `carried_cells`' doc argues
against inventing, so it is a design call for the owner. Data:
`Reports/data/lab-loadby-2026-09-25.txt.gz` (every `SUMMARY` line, both arms).

### 17f. A forager has to earn more than it eats, and today it barely does

The owner's ruling, 2026-09-25: *"I cannot see any other way this works than
foragers getting enough food to feed themselves and extra to feed the
colony. Otherwise what is the point of the foraging loop?"* So surplus is
the loop's purpose, not a separate design step, and the bar is a number.

**What a loop pays today**, traced over §16's 425 completed loops (medians,
joules an ant absorbs): about **716 J** picked up at the food, **236 J**
digested by the carrier on the walk home, **440 J** put down at the nest.
The whole loop costs the forager about 333 J (4,840 frames at 0.0688 J a
frame). So a loop puts down **1.3 times** what it costs: enough for the
forager and a third of another ant. The proposed bar is **3 times**, so a
forager feeds itself and two nestmates, and a colony where a third of the
ants forage breaks even.

**A fed forager keeping its cargo (the shared stomach) cannot help yet.**
`hungergate=1`, on the same 24 seeds: the carrier still digests a median
236 J on the walk home, it held back 1–3% of all digestion, and loops
425 → 402, starved 342 → 353 (8 seeds up, 4 down). The gate keeps cargo
only for an ant above its starting energy, and the median carrier holds
**0.61** of it; 17% of laden decisions are at or above. A shared stomach
can only protect food the forager does not need, and on this bed it needs
all of it. **So the trip has to pay more first.** The shared stomach is
the second step, once there is a surplus for it to protect.

### 17g. A lighter load for every food: the foragers live, and the lab holds

`PIXEL_PHYSICS_LOAD_SCALE=<f>` (new, off by default, bit-exact unset: bed
seeds 1–8 and lab seed 1 identical) multiplies every food load's weight by
`f`, keeping foods in proportion to their joules. So it does not re-price
foods against each other, which is what broke the lab in §17e. At `f = 0.5`
a full shipped crop of fruit weighs 3 body cells instead of 6.

**On the colony bed** (24 seeds, paired against §16):

| arm | starved of 480 | seeds better / worse | loopers who starved | never reached the food, starved | put down per loop | ≈ × the loop's cost |
|---|---:|---:|---:|---:|---:|---:|
| default | 342 | – | 109 | 188 | 440 J | 1.3 |
| half weight | 293 | 16 / 8 (p 0.15) | 81 | 189 | 523 J | 2.0 |
| half weight, crop doubled | **277** | **16 / 5 (p 0.027)** | **50 (18 / 2, p 0.0004)** | 200 | **868 J** | **2.7** |

(The multiple charges each arm its own colony burn rate over the default's
4,840-frame loop, so it is approximate. With the doubled crop, the
command's own starved count misses the harness's by 2, 277 against 279,
on seeds 14 and 22.)

**In the lab box** (§14's setup, 12 seeds, one binary):

| lab, median a run | default | half weight | half weight, crop doubled |
|---|---:|---:|---:|
| food intake | 836k J | 1,362k J (9 / 3) | 1,158k J (8 / 4) |
| births | 450 | 716 (9 / 3) | 590 (7 / 5) |
| deepest generation that itself bred | 26 | 31 | 28 |
| went extinct | 2 of 12 | 3 | **1** |
| plants standing | 88 | 20 (lower on 11) | 28 (lower on 8) |

- **Both games move the same way.** The foragers eat more and die less,
  unlike §17e.
- **Half weight alone grazes the lab down hardest** (plants 88 → 20, lower
  on 11 of 12, p 0.006). That is §14's overgrazing again, now stronger.
- **With the doubled crop the lab is steadier:** fewest extinctions (1),
  more food and births than the default. None of the lab differences from
  the default is significant at 12 seeds (p 0.39–0.77).
- **Predictions, recorded before the runs:** on the bed, right to within a
  few ants. In the lab with the doubled crop, all wrong in one direction: I
  expected it to amplify grazing (plants ≤ 20, 3–5 extinct), and it
  moderated it.

**Where this leaves the owner's bar** (§17f: a loop should put down 3× what
it costs): the default is at 1.3×, and half weight with a doubled crop
reaches about 2.7×. **It does nothing for the ants that never find the
food** (188 → 200 deaths), which is §17b's lever, the nest's mouth. **What
it would take to ship:** `ant.ron`'s `crop_capacity` 2880 → 5760, and a load
density of half for food, the new constant. Both are the owner's call.
Data: `Reports/data/bed-default-loadscale-*-2026-09-25.*`,
`Reports/data/lab-loadscale-2026-09-25.txt.gz`.

### 17h. What is realistic: pay the trip, not the floor

Asked by the owner after §17g: *"debate and think deeper on this. What is
most realistic?"* The question is which economy is most like a real colony
and consistent with the rulings already made. What real colonies do, stated
as the literature has it and hedged where the figure is from memory:

- **A foraging trip pays many times its cost.** In the harvester-ant
  measurements energy is not what limits foraging (time, water and risk
  are), and a trip returns tens to hundreds of times what it costs.
- **Carrying costs roughly in proportion to the mass moved** (leafcutter
  measurements). Loads are commonly around the ant's own mass.
- **Solid food travels in the mandibles and is not eaten on the way.**
  Liquid goes into the crop, a shared stomach, where a valve passes only
  what the forager needs to its own gut.
- **Only a minority of workers forage.** Many are inactive in the nest at
  any time, fed by nestmates and burning little. A founding colony starts
  from stored reserves, not empty.
- **A worker survives far longer without food than a trip takes.**

**Measured against that, the ant's load cost is already realistic.** Over
the default's laden decisions the average crop is 47% full: 337 J carried,
weighing **1.4 times the ant**. A laden tick costs 0.45 J against 0.20 J
empty, **2.25 times**. That is the mass-proportional law at a
body-mass-sized load. §17c's "a full crop weighs three times the ant" is the
full-crop extreme, not the typical load.

**What is not realistic, ranked by what it costs the colony:**
1. **The trip pays too little.** Per loop the forager carries 716 J against
   its own ~333 J of living, 2.15 times in all, against a real forager's
   many times.
2. **Every ant lives like a forager.** All 20 founders search, mill and dig,
   and none rests. The ones who never find the food die of it by frame
   3,800, with nothing stored at home to be fed from.
3. **Cargo is digested in transit.** That is realistic for liquid food and
   not for a fruit in the mandibles. But moving where the forager eats its
   trip's cost creates no surplus, so it matters only for how food is
   shared out.
4. **A crop holds 2,880 J of anything**: three fruits or seventy-two leaves.
   A real ant carries one item sized to itself. That is why weighing by
   cells broke the lab (§17e).

**The rulings rule out paying the floor.** *"An omnivore should be viable"*
keeps the gut neutral (`hopper.ron`, card `20260823T104411499Z-963f8d`).
*"I don't want ants sitting in one spot eating fallen leaves"* is why food
value was restored to 4× and no higher, because a richer floor is the
sit-still attractor. E14 (*"let them starve"*) sized the grant so an idle
ant lives one scene. A richer gut, richer food or cheaper living all pay an
ant for sitting as much as for working, and cut against those rulings.
**Realism agrees: a real forager is paid by the trip.**

**So the lever is what one load is worth.** Half the weight per joule with a
doubled crop is that lever. Averaged over laden decisions it carries
**577 J at 1.2 times the ant's weight**, against today's **337 J at 1.4
times**. Only an ant that carries food home gains anything from it. Read
this way it is not "loads are lighter" but "harvested food is about twice as
energy-dense as the ant's own flesh". That is plausible for seeds, and fruit
is already authored at twice flesh per cell. It measured 342 → 277 starved
and looper deaths 109 → 50 on the bed, and 1 extinct of 12 in the lab.

**Not chosen, and why:**
- *One item per trip, weighed by size.* The most physical rule, but it
  needs the crop redesigned around items and ants able to choose rich ones.
  Later, if ever.
- *Cheaper living.* It would lengthen a lost scout's search, but it pays
  idling too and undoes E14.
- *Not digesting cargo in transit.* No surplus, as argued above.

**The second realistic fix is the colony's other half.** Most ants should
rest at home and be fed, not search and starve. That needs food and the
hungry to meet, which is the nest's mouth (§17b), and a colony that does
not start with an empty store. Idle at home is realistic; starving at home
is the defect.

## 18. Shipped: the bigger crop and lighter food

*2026-09-25, the owner's ruling: "Let's test it out."* The ant now ships
`crop_capacity: 5760` (was 2,880) and `food_weight: 0.5`, a new
`CreatureDef` field. A load of food weighs half what the same joules of the
ant's own flesh would. The field defaults to 1.0, so every other species is
unchanged. `PIXEL_PHYSICS_LOAD_SCALE` still multiplies on top of it for
experiments, and `PIXEL_PHYSICS_LOAD_BY=cells` stays off (§17e).

**Foragers still eat on the way home.** The owner asked whether this stops
them eating what they carry, and was worried they would starve. It does not.
Digestion is untouched, and a laden ant eats from its load exactly as
before. What changed is what the load weighs and how much of it there is.
§17f argued that stopping foragers eating in transit creates no surplus: the
forager has to pay its trip's cost somewhere. And §17f measured that the
carriers are too hungry for a hold to act on. And on the new default fewer
foragers starve, not more: 109 → 50 of the ants that made a loop. Of all the starved, 4% die with more than a quarter of a crop, against
2% before and 38% for the doubled crop at the old weight.

**The default is the measured arm, digit for digit.** The default build with
no switch and no rider reproduces §17g's `LOAD_SCALE=0.5 + cropcap=5760` arm
on the colony bed (seeds 1–8, all 40 per-run, food-store, death and budget
lines identical), and in the lab box (seed 1, every `SUMMARY` line identical).
So every number in §17g describes the game as it now ships.

## 19. The nest as a door: does one mouth stop the starving?

*2026-09-26.* §17b found that the largest group of the dead are ants that
never reach the food, and that where an ant is born on the 45-column painted
nest decides it. This asks, before anyone builds a dug nest, whether giving the
colony one small home pays. The nest session (`Reports/lanes/nest-mouth.md`
once it exists) builds the real, dug version on this switch.

**The switch.** `PIXEL_PHYSICS_NEST_DOOR=<d>` makes founding paint a door of
`2d + 1` columns, unbroken, instead of the strip, and sets every founder's
home (`forage_anchor`) to the cell above the door's centre. Without that, a
founder standing off the door would carry its birth cell as home for life, and
a laden ant would walk home to a spot where it cannot put food down.
`PIXEL_PHYSICS_NEST_DOOR_FOUNDERS=pile` also starts every founder heaped on the
door instead of spread along the ground. The two arms separate "home is one
point" from "everyone comes out of one point". Unset is bit-exact: the new
binary reproduces the shipped default on seeds 1–8, all 40 lines. Guard:
`a_nest_door_paints_its_width_and_anchors_every_founder_at_it`, watched red
with the anchor write removed.

**Positive control, frame 0, seed 1.** Default: nest ground x 26–70, founders
x 30–68, 20 of 20 born beside it. Door, `d = 2`: nest ground x 46–50, founders
x 30–68, 3 of 20 beside it and all 20 homed at x 48. Piled: founders x 46–50.
The pile lands as a heap by frame 60 and spreads out by frame 360; there is no
gridlock (a packed colony once logged 27,386 blocked ticks, before ants could
climb over each other).

**On the colony bed** (shipped default, no trail, 90 cells, 24 seeds, paired
against the default):

| | default | door, homed (`d=2`) | door, piled (`d=2`) | wider door, piled (`d=6`) |
|---|---:|---:|---:|---:|
| starved, of 480 | 277 | **209** (18 better / 4 worse, p 0.004) | 214 (17/4, p 0.007) | 210 (17/7, p 0.064) |
| never reached the food, starved | 200 | **125** (18/5) | 141 (18/2) | 122 (16/5) |
| reached the food | 271 | 344 (16/5) | 328 (16/3) | 339 (15/5) |
| completed a loop | 229 | 308 (18/4) | 266 (15/5) | 290 (15/7) |
| loops | 366 | **465** (19/4, p 0.003) | 392 (13/9) | 410 (15/9) |
| foragers who starved | 50 | 61 (12/10) | 42 (7/9) | 53 (13/8) |

- **One home point is what pays; starting everyone there adds nothing.** The
  homed arm, with founders still spread, is as good as either pile on
  starvation and best on loops.
- **The mechanism is the one §17b named.** Founders born 11–20 cells on the
  far side of the nest from the food reach it **51%** of the time against
  **30%**, and starve **58%** against **76%**. By birth position, starvation
  goes 76/54/55/52/46% → 58/43/33/41/52%.
- **It does not touch the foragers** (50 → 61 starved, 12/10: no change),
  which is the load's job (§17g–18).
- **Predictions, written before:** 170 never-reached deaths and 255 starved
  for the homed arm, loops within ±10%. Too cautious on all three: 125, 209,
  and loops **+27%**.
