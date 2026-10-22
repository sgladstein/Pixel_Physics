# Trail following in the colony: why it hurt, and a teacher session (2026-10-22)

The maze thread's record. The owner asked whether the new ant can follow a pheromone trail through a maze, and
asked for training to be built into the maze. When the trail lesson cost the colony food, he asked why it does not
help in the actual game. That question got an ant-by-ant trace and a comparison with real ants. He then asked
whether to fix it with targeted training or a teacher session, decided with the Teacher lane. Files are in this
folder ([README.md](README.md)).

## The answer in four lines

- **The trail the ants were taught to follow is a crowd scent, not a food trail.** c1 and hA1 lay channel B
  wherever they are crowded, so B peaks at the nest. "Follow the stronger B" sent ants home, and they starved
  there.
- **Switching off the crowd trail is the cheapest gain found.** hA1 with B laying silenced brought home 374 vs
  271 food a colony, more on 28 of 32 seeds. It is a one-row weight edit, now with the Teacher lane as a founder
  candidate.
- **A trail rule shaped on real ants works when it acts.** Ants lay less where B is already strong. An empty ant
  stays on the trail and takes the outward step. Against crowd-trail-off it has the same food home, field bites on
  30 of 32 seeds and fewer deaths, and ants roaming to the box walls halve.
- **That rule does not copy into today's brain.** Two DAgger copies were worse than the start. A climb-the-trail
  rule was wrong as a rule. The maze's built-in slope taught exactly that rule, so maze training is the wrong
  tool here.

## Setup

- **Bed:** the Teacher lane's in-run harness (compute branch `claude/project-thread-8y5ak0` at `ee38ae5b`, plus
  [patches/trail-teacher-vs-ee38ae5b.patch](patches/trail-teacher-vs-ee38ae5b.patch)) with their night2 judge
  settings and selection off, so these are colonies, not selection. One 200,000-frame round, 24 ants (the founder
  plus 23 trainer-style mutants of it), food 25 columns out. hA1 arms run with the pack move on.
- **Food home** counts deliveries within 8 columns of the nest only (the foraging lane's delivery-count trap).
  "Field bites" counts bites at the food. Every comparison is paired by seed, and the counts in brackets are seeds
  better/worse.
- **The trace:** a per-decision log of every ant (`PP_DLOG`, from the starve-trace lane's patch, extended here).
  Each row holds what the ant was offered, what it picked, and B, B there-minus-here and food at the pick. It also
  dumps the A and B fields every 4,000 frames (`PP_PHERMAP`).

## 1. B is a crowd scent

c1's B-laying weights are crowd +0.86, its laden memory +0.87, crop +0.37 and bias +0.29. Nothing ties laying to
having found food, and the bias and crowd terms lay it at the nest from the first frame. Strongest B per column at
frame 4,000 (c1, seed 6), from the nest outward:

| columns from nest | -20 | -10 | -5 | 0 | +5 | +10 | +20 | +25 (food) |
|---|---|---|---|---|---|---|---|---|
| B | 0 | 17 | 1,502 | 7,251 | 14,496 | 6,668 | 757 | 26 |

## 2. Ant by ant: the graft keeps ants home

The trail lesson delivered to the Teacher lane earlier was a graft into c1's free scorer unit 15 (B there-minus-
here +16, B two-cells +16, output +20). In the colony, on seed 6, no graft ant went more than 4 columns from the
nest except one, which went 9. They climbed B on about half their steps (c1: about a third), and all 24 starved
between frames 2,095 and 2,115. Plain c1's ants ranged 7 to 62 columns out.

Over 16 seeds:
- Ants past 15 columns by frame 2,200: 0-2 of 24 with the graft against 9-19 for c1.
- Alive at 200k: median 0.5 against 10, with 8 of 16 graft colonies dead out.

[graft-ants-stay-home-seed6.png](graft-ants-stay-home-seed6.png) shows every ant's position over the first 3,000
frames, c1 above and the graft below.

## 3. Who lays B decides what following it does

16 seeds. In noB nobody lays B. In ladenB, B is laid only while carrying food (emit row `crop*12 - 2`), as a
recruiting forager does. In the empty-gated graft, the graft reads B only while empty, gated on c1's carrying
memory.

| arm | alive at 200k | field bites | food home |
|---|---|---|---|
| c1 | 10 | 2,212 | 279 |
| c1 + graft | 1 | 284 | 36 |
| c1, noB | 21 (15/0) | 4,122 (15/1) | 404 (13/3, p 0.02) |
| c1 + graft, noB | 22 | 5,833 (median) | 523 (median) |
| c1, ladenB | 21 | 3,628 | 254 |
| c1 + graft, ladenB | 17 | 4,632 (13/3 vs ladenB) | **29 (0/16)** |
| c1 + empty-gated graft, ladenB | 22 (10/3) | **6,098 (15/1, p 0.001)** | 346 (10/6, n.s.) |

- **The trap is the crowd scent, not the reading.** With nobody laying B, the same graft is harmless.
- **c1's own crowd B hurts c1, with no graft at all.**
- **On a recruitment trail, an ungated follower is pulled back to the food pile even while laden, so food home
  collapses.** Gated to empty ants, the same reading finds food far more often.

The same test on hA1, which keeps c1's B-laying weights and has no free scorer unit:

| arm | alive | field bites | food home |
|---|---|---|---|
| hA1 | 19 | 2,922 | 251 |
| hA1, noB | 19 (6/7) | 2,882 (8/8) | **363 (14/2, p 0.004)** |
| hA1, ladenB | 22 (8/6) | 3,679 (12/4, p 0.08) | 256 (7/9) |

## 4. Real ants

These sources were checked on PubMed:

- **Recruitment trails are laid by successful foragers, and crowding turns laying down, not up.** In *Lasius
  niger*, deposition fell 5.6-fold from the least to the most crowded trails. Contact with nestmate odour alone cut
  it by 45% ([Czaczkes, Grüter & Ratnieks 2013](https://doi.org/10.1098/rsif.2012.1009)). c1 does the opposite,
  which is what turned B into a nest beacon.
- **A trail does not say which way to go.** "Previous research has found no evidence that ants can detect polarity
  from the pheromone trail alone." Pharaoh's ants read direction from the geometry of trail forks, about 60
  degrees, and a laden forager should walk toward the nest
  ([Jackson, Holcombe & Ratnieks 2004](https://doi.org/10.1038/nature03105)). The maze stamped a slope toward the
  goal, which real trails lack.
- **Ants follow a trail by keeping its edges between their antennae (tropotaxis), not by climbing along it**
  ([Draft et al. 2018](https://doi.org/10.1242/jeb.185124)).
- **Experienced foragers lean on route memory and use pheromone as a supplement**
  ([Czaczkes et al. 2012](https://doi.org/10.1242/jeb.076570)).

The old hand-built ant already reads B only when not carrying food.

## 5. The teacher session

Agreed with the Teacher lane (2026-10-22 15:34Z). The crowd trail switched off is a weight edit for them to judge
at 64 seeds. Trail following would come from a hand rule copied DAgger-style onto hA1, judged here at 32 seeds
first. The copier is the foraging lane's, with one change: `target_only` reshapes only *which step* a walking ant
takes, never the verb, so digging, eating and carrying stay the start's own.

32 seeds, judged against hA1-ladenB:

| arm | what | alive | died | field bites | food home | digs |
|---|---|---|---|---|---|---|
| hA1-ladenB | B laid only while carrying | 21.2 | 2.8 | 3,590 | 271 | 339 |
| hA1-noB | no B laid | 19.6 | 4.4 | 2,968 (8/24) | **374 (28/4, p 2e-5)** | 338 |
| actT | rule 1 acting: empty, off the nest, step up B | 16.8 | 7.2 | 2,535 (2/30) | 148 (4/28) | 282 |
| tA1 | rule 1 copied | 5.0 | 19.0 | 1,269 (0/32) | 92 (0/32) | 207 |
| tA1ctl | the copy pipeline with the lesson's weight 0 | 22.1 | 1.9 | 3,563 | 241 (11/21, n.s.) | 312 |
| sup | ladenB, laying less where B is high (B-here -8) | 21.1 | 2.9 | 3,545 | 283 | 341 |
| supOut | sup + rule 2 acting: stay on the trail, step outward | 21.5 | 2.5 | **4,303 (22/10)** | **391 (28/4)** | 342 |
| oA1 | rule 2 copied | 16.8 | 7.2 | 2,569 (4/28) | 239 | 324 |
| oA2 | rule 2 copied, untouched choices anchored 4x harder | 18.7 | 5.3 | 2,996 (9/23) | 262 | 302 |

supOut against hA1-noB: food home 18/14 (n.s.), field bites 30/2 (p < 1e-4), deaths fewer on 22 seeds against 9
(p 0.03).

What the traces show (decision log, 3-4 seeds each):

- **Rule 1 traps climbers on a runaway peak.** Empty ants climbing B pile onto a peak 10-20 columns out, short of
  the food band (15-35). The crowd there keeps the peak growing until it saturates at 65,535. This is the
  positive-feedback trap real ants avoid by laying less where a trail is strong, and by not climbing.
- **Rule 2 keeps empty ants on the way out.** Rule 2 says: of the steps whose B there is at least half the best,
  take the one farthest from the nest's column; with no trail in reach, abstain. Empty walks past 60 columns fall
  from 40% to 21%, and walks at 10-30 columns rise from 34% to 60%. Those far walks belong to the roamers that the
  starve-trace lane found to be half of all starvation.
- **The rule-2 copies learned a blurred version.** They fit 55-64% of the teacher's steps. They dither on the
  trail, stepping back on 59% of sideways steps against 45%, and their ants die both at home and far out. Two copies
  failed the same way, so the approach is wrong for this rule, not the weights.
- **The brain has a home-direction feature, but it fades with distance.** `f::HOME` is `cos * squash(dist/32)`, so
  "outward" is weakest exactly where it matters. The Teacher lane's come-home-when-far copies fail with the same
  symptom.

## 6. The rock-blocks-trail switch, dropped

`PIXEL_PHYSICS_TRAIL_WALLS` (stone and soil block trail spread; off by default and bit-identical when off) was built
behind a switch at the owner's ruling. It is in commit `3501f682` on branch `claude/project-thread-wijq9s`.

- **In the arenas:** it kept a maze trail readable (follower 0.67 against 0.15 after 12 spreading passes).
- **In the played lab box:** 12 seeds, 120k frames. Intake went 2.21M -> 2.02M (better on 5 of 12 seeds) and net
  food home was lower on 10 of 12. Underground time rose from 29% to 37%. Adding a fade to A was clearly worse:
  intake -34%. Tables are in [trail-walls-labbench-2026-10-22.txt](trail-walls-labbench-2026-10-22.txt).
- **Why, as a hypothesis not traced ant by ant:** the box's home trail A sits near the top of its range along most
  of the corridor, and its only loss is soaking into ground. Walled, it keeps more mass in tunnels, which become
  the brightest A.

It was taken out of the landing PR (a dead-ends entry records it).

## What to do next

1. **Judge hA1 with its crowd B silenced** (genome `g-hA1-noB`) in the Teacher lane's 64x6 judge. This is queued
   there.
2. **Keep rule 2 as a ready second lesson for when the ant gets a far-from-home direction sense that does not
   fade.** Its labels are field 16 of the copy log (`PP_TRAIL_LABEL=1:0.2 PP_TRAIL_OUT=1`), and `copier_t.py`
   copies it with `target_only`.
3. **Otherwise, `g-hA1-sup` is a cheap founder variant.** It lays B only while carrying, and less where B is
   strong. Selection could find a use for the trail from there. Acting alone it is no worse than ladenB.
4. **Do not retry climb-the-gradient lessons, in a maze or out of one.**
