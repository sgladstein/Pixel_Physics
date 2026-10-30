# Why trained colonies starve in the famine boxes: the wall trap (2026-10-30)

*Famine lane, deep trace. Measurement and diagnosis; nothing fitted, nothing switched. Built on `main` 678a4fb4.
Picture page: https://claude.ai/artifact/KtWqo1JPAGu1GTtCftSUWt. Tools: [`famine-wall-trap-2026-10-30/tools/`](famine-wall-trap-2026-10-30/tools/).*

**The question.** In every Teacher exam since 10-28, every trained pool dies in `two_larders`, `scattered_larder` and
`feast_and_famine` 2,000-6,000 frames after landing, where the hand-built ant lives. Selection cannot rank colonies
that all die, so this traces individual ants: hand-built (`founders=old`) against std_pool (`founders=std`).

**The answer.** The trained ant walks away from home, harder the hungrier it is, until an end wall stops it, then
stays at the wall until it starves. The hand-built ant reaches the same walls and comes back. Every trained ant that
reached an end wall stayed there: **548 of 548**, over five boxes and nine seeds; hand-built **412 of 427 came back**.
The food in these boxes is near the nest (herbs, a trickle 20-34 columns out, scattered windfall), so the trained
colony walks past it. On top, the brain bill (0.100 energy/frame against 0.057-0.075) gives a one-tank trained ant
about 2,000 frames, and `two_larders`' first food drop comes 3,000 frames after landing.

## How it should work (written before the data)

A one-tank colony in a walled room with sparse food near home survives by (1) spending little while there is
nothing to eat (resting), (2) searching out *and back*, so near and far ground are both sampled, and (3) taking food
when beside it. The hand-built ant has each as a rule: rest when there is nothing to do, a scout that gives up and
turns home, hungry-home pull, and a pick-up beside food.

## Method

`examples/labsurvive` with `trace=100` (per ant: energy, bites, position, crop), `food=N` (new: the food cells in
each half of the box), `png=DIR pngat=F,...` (new: the game's own render), `knock=F,...` (new: std_pool with the
scorer's weights on option features F zeroed, a diagnostic only). Decisions: `PP_COPY_LOG` for the trained ant's
own picks (every option, its features and score); `PP_SHADOW=1 founders=std` for the hand-built ant's choices on
the same option list (the shadowed colony matched the plain hand-built run's `RESULT` line exactly). Mutation off,
`RAYON_NUM_THREADS=1`, all in this container. Exam boxes were traced, never fitted; the knock-out was run on the
selection twins (`scratch/inrun/labscen/lab_famine_scatter.ron`, `lab_famine_trickle.ron`), not on the exam.

## Findings (traced unless marked)

**1. The wall trap, per ant.** An ant "came back" if after first touching an end wall (within 10 columns) it later
stood 40+ columns nearer its nest.

| Box (seed) | hand-built reached / came back | trained reached / came back |
|---|---|---|
| two_larders (1) | 72 / 71 | 98 / 0 |
| two_larders (2) | 69 / 67 | 99 / 0 |
| feast_and_famine (1) | 101 / 99 | 102 / 0 |
| scattered_larder (1) | 20 / 19 | 40 / 0 |
| twin scattered (1-3) | 88 / 86 | 122 / 0 |
| twin trickle (1-3) | 77 / 70 | 87 / 0 |

Bill-free (`tax=0`) trained ants: 99/0 in two_larders seed 1, 279/0 in the twins. On `played_bed` seed 1 (plants
every 25 columns), trained ants almost never reach a wall (2 of 66) and the colony lives (lived 43.6 vs 37.6).

**2. Where they were.** two_larders seed 1, left room, frames 1,000-3,000 after landing: hand-built ants span the
herb row (column deciles 52-202), 26% of samples within 2 cells of a plant, 55 bites; trained ants 80% within 10
columns of an end wall, 1% near a plant, 0 bites. feast_and_famine: trained ants 50-60% at the walls, 0 bites,
against 29 and 124 for the hand-built rooms.

**3. What the trained ant chooses** (two_larders seed 1, its own picks vs the hand-built ant's on the same offers).
Away from the nest, when a homeward and an outward step are both offered:

| energy | trained: outward / homeward | hand-built: outward / homeward |
|---|---|---|
| >= 0.4 | 0.75 / 0.12 | 0.48 / 0.35 |
| 0.2-0.4 | 0.81 / 0.07 | 0.31 / 0.60 |
| < 0.2 | 0.90 / 0.02 | 0.84 / 0.12 |

Rest share away from the nest: trained 9% (5% at walls), hand-built 51% (42% at walls). At a wall half the trained
ant's decisions are climbs and 41% are steps onto nestmates; it never picks turn-round (switched off in every std_pool member, as
the copy test found) and scores the back step near -0.99. Inferred from the training pool's header (food far and
moving, `PP_BED_W=448`): outward is what the training bed pays for.

**4. The bill.** Energy burn with no food: trained 0.100/frame, bill-free 0.067, hand-built 0.057-0.075 (0.058 in the
meat room, where it eats nothing). From ~195 at landing, a trained ant lasts ~2,000 frames.

**5. The knock-out (twins, seeds 1-3, lived = mean alive over 8,000 frames).** `knock=39,40` zeroes the scorer's
weights on the options' home and last-food direction (about 14 links per member). Wall arrivals fall from 209 to 8,
so those links drive the walk to the wall (traced by intervention). Survival does not improve:

| arm | scattered twin | trickle twin |
|---|---|---|
| hand-built | 52.3 / 52.3 / 51.9 | 41.5 / 40.4 / 42.9 |
| trained | 30.4 / 27.8 / 29.4 | 19.8 / 23.0 / 20.9 |
| bill-free | 35.5 / 34.0 / 36.0 | 27.0 / 29.7 / 26.6 |
| home links cut | 30.5 / 28.7 / 27.1 | 15.1 / 16.3 / 15.9 |
| both | 40.9 / 37.5 / 37.5 | 21.4 / 25.2 / 20.6 |

Paired against trained: bill-free 6/6 higher (sign test p 0.031), home links cut 2 up / 4 down (p 0.69), both vs
bill-free 3 up / 3 down. Three comparisons: none passes Bonferroni (0.017). Null control: `knock=18` (a feature with
no live links) reproduced the trained run's `RESULT` exactly. With the outward drive cut the ants hover near home
and eat less (trickle seed 1: 5 bites vs 29); walking out was their only way of finding food.

**6. Beside food** (small n, read with care). A hungry trained ant offered food picks it 23-39% and walks off 43-59%;
the hand-built ant picks 54-62% and walks off 2-11% (two_larders seed 1, n = 102 / 189 decisions).

**7. Side finding: two_larders' meat room kills the hand-built ant too.** Seeds 1 and 2: 52/52 dead by ~3,900
frames; 40-44 reached the corpse heap and none ate from it (crop never held it, `bites` 0). Both starting heaps are
gone before the colonies land (frame 3,000); the first food arrives 3,000 frames later. So the hand-built ant's
two_larders exam score is the herb room alone. Why it will not take the corpse heap: not traced.

## What it means

Std_pool's way of finding food is "go out and keep going". The training bed (food far out on open ground) and the
played bed (plants all along the way) reward it; a walled room with food near home does not, and nothing in the
pool brings an empty ant back. What is missing is a return leg after a fruitless trip, and resting when hungry with
nothing in reach. Cutting the outward links alone does not supply either. Selection on the same boxes will keep
failing if no training case pays for coming back. That is a training-box question for the Teacher lane.

## Not done

Only seed 1 (seed 2 for two_larders) of the exam boxes; the twins at three seeds. The beside-food gate has small n.
The meat-room refusal is untraced. No candidate brain was built or fitted.
