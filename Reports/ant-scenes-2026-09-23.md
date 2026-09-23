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
