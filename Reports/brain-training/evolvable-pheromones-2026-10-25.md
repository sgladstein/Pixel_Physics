# Evolvable pheromones: what the scents mean, a better B, and an emergence test (2026-10-25)

*Lane report, thread "Let ants evolve their own pheromones" (branch `claude/project-thread-iff53x`). Status:
**decode and tuning done; the emergence test is running** (free / deaf / drift arms, 200 generations each,
results to be added in §7). Companion: [the adversarial review](evolvable-pheromones-review-2026-10-25.md). Pictures:
the "Ant Scent Lab" artifact page (https://claude.ai/artifact/XiD6BBQtmyZ9iF9qJYGDV3).*

## The short version

- **The owner's ask** (2026-10-25 03:54Z): let the genome set the scent trails' physics so ants can evolve their
  own signals. Owner's rulings (04:02Z): *a fixed chemistry where the gene picks the molecule* is inside the
  one-physics rule; ship **off** until a paired test passes; the goal is **both** better foraging **and** seeing
  signals emerge.
- **The plan** (§2) was a fixed palette of six molecules on today's two spread laws, with a gene per emit output
  picking the molecule. **The adversarial review** (companion file) found the physics sound, but put a genuinely
  new convention at under 10% under today's training: selection inside a colony pays each ant for its own food, all
  founders share one convention, food sits in two fixed heaps, and a run has too few generations.
- **The owner chose** (05:12Z): decode what today's three scents mean, tune spread and fade as plain constants,
  then run an emergence test under the conditions the literature names. The palette comes back only if a
  convention forms there.
- **Decode** (§4): scent A, the "an ant walked here" map, carries the colony: deafen it and the nest shrinks from
  432 to 51 cells (32 of 32 seeds). Scent B is a real "I am carrying food" signal; ants that cannot read it store
  more and starve more. The alarm plane is laid mostly by empty ants away from home and changes nothing: a free
  channel.
- **B fading 0.03 -> 0.25 to 0.40** (§5) stores more food (441 -> 495) and builds a bigger nest (432 -> 528),
  starvation unchanged, with selection off. Handed to the Teacher lane to test under selection before any default
  changes.
- **Why** (§6): slow-fading B piles up in the nest five times over, and foragers coming home stop to eat from the
  store and shuffle food instead of digging. Faster fading keeps the nest from smelling of food.

## 1. The physics today

Three world-sized `u16` planes at cell resolution (`src/sim/pheromone.rs`), one pass every 12 frames
(`PHEROMONE_INTERVAL`):

- **Ground law** (`Spread::Diffuse`, planes A and B): each cell blends toward its 3x3 mean by `blend`, then loses
  `fade` of what is left. It conserves scent, so many ants' trails add up. For a thin trail the blend is itself a
  loss: the line smears sideways and its peak drops.
  - A: fade 0, blend 0.25. B: fade 0.03, blend 0.25 outdoors, **0.05 in the lab** (`LAB_B_DIFFUSE`, the one
    existing per-world physics difference).
- **Air law** (`Spread::ActiveSpace`, the alarm plane): each cell takes the louder of itself and its best of 8
  neighbours minus `fall` (12/255 per cell), then loses `fade` 0.35. It describes a cloud around a source, spreads
  at most **one cell per pass**, and makes **square** clouds (best of 8 = chessboard distance).
- A mind ant's three emit outputs write A, B and alarm (`EMIT_CHANNELS`, `src/sim/creature/mind.rs`), at most one
  `DEPOSIT` (40/255) per decision, billed 0.0625 of a move per unit. It reads each at its head, the change since
  its last decision, per-option scent features (`options.rs`), and the dig head's A feature (`targets.rs`).

**One walk of trail, laid once** (`examples/molecules.rs`, `scripts/molecules_sheet.py`): 120 cells x `DEPOSIT`,
never re-laid. Peak in old 0..255 units; width = rows at mid-trail over 1 unit.

| Molecule | Law | fade | blend / fall | peak @60 | @180 | @1200 | @4800 | width @1200 |
|---|---|---|---|---|---|---|---|---|
| road | ground | 0 | 0.05 | 33.9 | 25.5 | 8.7 | 3.0 | 7 |
| today's A | ground | 0 | 0.25 | 19.3 | 10.4 | 3.6 | 0.6 | 13 |
| today's B | ground | 0.03 | 0.25 | 16.6 | 6.6 | 0.1 | 0 | 0 |
| fresh | ground | 0.10 | 0.25 | 11.4 | 2.1 | 0 | 0 | 0 |
| flash | ground | 0.25 | 1.0 | 2.0 | 0.1 | 0 | 0 | 0 |

- On the ground, **faster blending does not reach further**: the 3x3 mean loses ~9x per cell, so a fast-blending
  scent just disappears sooner. Ground scents differ in **lifetime and width**, not reach. The lifetime ladder
  spans 60x: flash ~100 frames, fresh ~400, B ~1,200, A ~4,500, road >6,000. A colony round trip is ~2,200.
- **Reach needs the air law**, and less of it than the first draft claimed: the air rows were first measured with a
  240/255 bite, but an ant emits at most 40, so an ant's air cloud reaches ~2-6 cells, not 31 (review H4).

## 2. The palette that was proposed (parked)

Six fixed molecules on the two laws, ordered by one volatility number (road, trail = A, news = B, fresh, alarm,
call), the same in every world. One gene per emit output picks a position on that scale and the deposit splits
between the two neighbouring molecules; the ant's three scent inputs read the same molecules, so no new brain
links. Today's ant would be one setting of the genes (except in the lab, where B's 0.05 blend is not on the
scale: review M1). **Parked** by the owner's 05:12Z choice until an emergence test shows a convention can form.
If it returns: genes in the **mind genome**, not body traits (review H2); a fresh control for extra genome slots
(M2); ~500 MB for six planes at full outdoor size (M4).

## 3. The adversarial review, in brief

Full text in the companion file. Critical findings: **C1** in-run selection culls and breeds by individual score,
so a signal that helps nestmates pays the emitter only its cost; **C2** pairing emit and smell in one gene from
identical founders freezes the convention, and anything that moves is drift; **C3** two fixed heaps carry little
to say; **C4** ~50-150 lineage generations per run with emit rows mutating at a tenth of the rate is far below the
~500 the robot-colony studies needed (Floreano et al. 2007; Mitri et al. 2009, 2011). **H1**: this project's trail
records put the bottleneck in *who emits and how readers react*, not chemistry.

## 4. Decode: what today's three scents mean

Main `84988ab2` plus this lane's switches. `std_pool` (30 members), `examples/inrun`, **selection off**, mutation
off, 200,000 frames, `dist=100 food=mix`, breeding on, 32 paired seeds. Measures per how-we-test: stored food (nest
+ door at the end), mean tank over the second half, starved, alive, nest cells and crater (colsel's `nestcount`),
rooms, and units of each scent laid.

**When each scent is laid** (seed 101, 100k frames, 1.23M decisions, `PP_MIND_LOG_EMITS=1`; mean emit 0..1):

| Ant's state | A | B | alarm |
|---|---|---|---|
| laden (crop > 0.05) | 0.651 | 0.316 | 0.114 |
| empty | 0.555 | 0.010 | 0.386 |
| empty, away from nest | 0.560 | 0.011 | 0.412 |
| laden, at nest | 0.618 | 0.296 | 0.042 |

A is laid by nearly every ant nearly all the time. B is laid only by ants carrying food (the trail lane's
hand-written rule, still exact in the pool). Alarm is laid mostly by empty ants away from home; nothing in this
bed bites, so the pool has repurposed it as an "empty and searching" mark.

**What happens when ants cannot read a scent.** Deaf = the scent reads 0 everywhere a mind reads it. Scrambled =
read 40 columns away: a real plane in the wrong place.

```
arm                stored             tank          starved            alive             nest           crater            rooms             layA             layB            layAl
ctl                 441.0             68.2             41.0             85.0            431.5              9.0              2.0        1603905.0         473059.0         689989.3
deafA       199.0  0/32*       18.5  0/32*       22.5  5/27*       69.0  0/32*       50.5  0/32*        4.0  6/23*        1.0  4/17*   1242195.0  0/32*   268053.5  0/32*   708012.2 19/13
deafB       528.0 29/3 *       58.8  9/23*       68.5 28/3 *       84.0 12/17       541.5 29/3 *        7.0 14/17         2.0  8/13    1577976.0  8/24*   839323.0 32/0 *   586306.2  0/32*
deafAl      465.5 22/10        63.6 10/22        49.0 21/10        85.0 11/17       454.0 19/13        10.0 21/10         2.0 11/10    1586325.5 12/20    482376.5 22/10    638871.2  6/26*
deafAll     361.0  0/32*       81.9 31/1 *       11.0  0/32*       84.0  6/23*       41.5  0/32*        1.0  1/31*        1.0  5/19*   1420463.5  0/32*   784330.0 32/0 *   491648.7  0/32*
shiftA      271.5  0/32*       39.9  1/31*       30.5  9/23*       77.0  1/30*      123.0  0/32*        7.0 13/17         1.0  3/19*   1276784.0  0/32*   300665.5  0/32*   767025.6 31/1 *
shiftB      477.0 26/6 *       65.8 13/19        55.0 26/6 *       86.0 17/14       461.0 21/11         6.0  9/21*        1.0  8/15    1597546.0 17/15    797845.5 32/0 *   631361.6  2/30*
shiftAl     462.0 20/12        62.6 11/21        52.5 21/11        85.0 13/14       459.0 22/10         6.5 11/21         2.0 15/11    1583187.5  9/23*   477471.0 20/12    647683.6  2/30*
cells: arm median, then seeds where arm > ctl / arm < ctl (* sign test p<0.05)
```

- **A carries real, place-specific information and the colony is built on it.** Deaf: stored 441 -> 199, nest 432
  -> 51 cells, tank 68 -> 19, alive 85 -> 69, all 32/32. Scrambled: stored 272, nest 123 (32/32). A shared plane
  has no sender tag, so this cannot tell reading one's own trail from reading nestmates'.
- **B is listened to, and it trades feeding for storing.** Deaf: stored 441 -> 528 and nest 432 -> 542 (29/3), but
  starved 41 -> 69 (28/3). Scrambled B goes the same way, smaller.
- **Alarm carries nothing useful here**: ~690k units laid per run, billed, and no measure moves significantly when
  it is deaf or scrambled.
- **No scent at all**: nest 42, stored 361, but starved 11 and tank 82 (31/1): ants feed themselves and build
  nothing.
- Silent arms (deposit nothing, still billed) came out **identical** to deaf, as they must when ants are the only
  readers and writers of a plane; dropped after 23 seeds.

Caveats: selection off measures how this pool copes with a change it was never trained under; deaf is
off-distribution (the scrambled arms are the cleaner listening test and agree in direction). One bed.

## 5. Tuning spread and fade as constants

Same setup as §4. `PIXEL_PHYSICS_B_RHO` (B fade), `PIXEL_PHYSICS_A_DIFFUSE` (A blend, new) and
`PIXEL_PHYSICS_A_RHO` (A fade).

```
arm                stored             tank          starved            alive             nest           crater            rooms             layA             layB            layAl
ctl                 441.0             68.2             41.0             85.0            431.5              9.0              2.0        1603905.0         473059.0         689989.3
bF0         163.5  0/32*        3.5  0/32*      161.5 31/1 *        0.5  0/32*      133.5  0/32*       80.5 27/3 *        1.0  3/15*   638121.5  0/32*    64548.5  0/32*   247298.0  0/32*
bF10        484.0 24/8 *       71.0 18/14        47.0 19/13        86.0 16/14       463.0 17/15         7.0 15/16         2.0 10/10    1624681.5 22/10    585973.5 32/0 *   663362.4  8/24*
bF15        480.5 22/10        69.0 18/14        48.0 22/10        86.0 17/11       492.5 23/9 *        6.0 12/20         2.0  9/13    1629908.0 23/9 *   638428.5 32/0 *   645633.3  4/28*
bF25        495.0 27/5 *       70.8 20/12        46.0 19/11        87.0 15/14       527.5 25/7 *        5.0 13/17         2.0  8/12    1639134.0 27/5 *   711157.0 32/0 *   636655.8  2/30*
bF40        501.0 28/4 *       71.0 19/13        49.5 20/11        86.0 14/14       507.0 28/4 *        6.5  9/22*        2.0  6/12    1626404.5 22/10    763030.5 31/1 *   628348.1  5/27*
bF60        489.0 27/5 *       67.8 14/18        53.0 23/9 *       87.0 15/16       508.5 24/8 *        6.5 15/15         1.0  7/16    1614344.5 19/13    786936.5 32/0 *   638360.7  3/29*
aB05        396.0  4/28*       59.3  7/25*       48.0 19/12        82.0  8/22*      355.5  7/24*        9.0 14/17         2.0 10/13    1575823.5  8/24*   407104.0  6/26*   700073.7 19/13
aB10        409.5  9/23*       58.8  6/26*       52.0 23/9 *       84.0 12/18       390.5 11/21         9.0 17/12         1.5 10/13    1593036.5 12/20    426302.5  6/26*   709734.4 23/9 *
aF03        414.5  6/24*       70.2 18/14        10.0  0/32*       83.0  7/22*      124.5  0/32*        1.0  1/29*        1.0  2/15*   1432257.0  0/32*   452287.0 10/22    651071.7  5/27*
bFxx = B fade 0.xx (control 0.03); aB = A blend (control 0.25); aF03 = A fade 0.03 (control 0)
```

- **B fade 0.25-0.40 is a plateau**: stored 441 -> 495-501, nest 432 -> 507-528, starvation not significantly
  changed. At 0.60 starvation starts to rise (23/9). **B that never fades kills the colony** (alive 0.5).
- **A should stay as it is**: a narrower A (blend 0.05, 0.10) costs food and nest; a fading A (0.03) wrecks the nest
  (124 cells) while ants stop starving, the same "feed yourself, build nothing" pattern as no scent at all.
- **Status**: the Teacher lane is running B fade 0.25 against the control under in-run selection (32 colonies x 16
  rounds), judged on garden stored food and nest with starvation as a guard. No default changes until that passes.

## 6. Why faster B fading helps

8 paired seeds, `trace=` per ant every 25 frames plus `phero_dump=` maps of A and B, second half (100k-200k).
Control (B fade 0.03) against 0.25. Picture: the B map and these numbers on the Ant Scent Lab page.

| measure | B fade 0.03 | 0.25 | seeds 0.25 higher |
|---|---|---|---|
| B in the nest (home band, summed) | 13,617 | 2,632 | 0/8 |
| B in the middle band | 10,163 | 2,062 | 0/8 |
| food eaten from the store | 4,921 | 3,156 | |
| food picked up and dropped again at home (churn) | 4,282 | 2,543 | |
| soil dug | 233 | 278 | 8/8 |
| laden ants in transit (share of ant-time) | 11.3% | 17.9% | 8/8 |
| forays out of the nest | 1,602 | 2,115 | 6/8 |
| forays turned back (%) | 16.5 | 37.2 | 7/8 |
| frames from leaving to reaching food | 525 | 463 | 2/8 |

- With slow fading, B laid by ants coming home with food piles up in the nest: five times the scent. The nest
  smells like a food heap all the time.
- Traced per ant, the change is in **returning foragers**, not ants that stay home (8 vs 8 stayers' counts barely
  move): foragers ate from the store 4,222 vs 3,024 times, re-dropped food 3,352 vs 2,225 times, and dug 221 vs
  275. Pickups and deliveries in the field are about the same.
- So faster fading does not make B a better trail; it **stops B from saying "food here" inside the nest**, so
  foragers stop grazing the store and do more digging and carrying. The link from the ant's scent inputs to its
  choice is inferred from these counts, not traced decision by decision.

## 7. The emergence test (running)

The test the review asked for (E2), built on the conditions the robot-colony studies needed:

- **High relatedness**: every colony is founded from one genome (30 copies) and runs with in-run selection off,
  so hatchlings keep their layer's mind: a colony is a clone.
- **Colony-level selection only**: score = stored food + nest cells at the end of the round (0 if extinct); the
  next generation is the top 2 unchanged plus mutants of uniformly chosen top-half parents (standard mutation
  0.013 : 1.0 : 4 over `inrun::slots()`, no lesson protection).
- **Something worth saying**: `patches=20000:30:110`, one food heap that jumps to a random side and distance 30-110
  columns from the door every 20,000 frames.
- **A free channel, free to evolve**: `PP_MIND_ALARM=train` opens the alarm emit row (frozen on the normal bed).
- **Controls**: `deaf` (no ant can read alarm), `drift` (random parents). Same colony seeds per generation and
  index, so arms pair.
- **The emergence signal**: every 10 generations the best genome is replayed on 4 seeds normal / alarm-deaf /
  alarm read 40 columns off. A convention has formed if that listening gap grows in the free arm and not in drift
  or deaf.
- **Baseline**: on the patch bed `std_pool` is neutral to alarm deafness (6 better / 5 worse, 12 seeds) and 2 of 12
  colonies go extinct by 100k frames: the channel starts empty, and the world is harsh enough to select.
- **Run**: 16 colonies x 200 generations x 100,000 frames per arm, ~7 hours on 4 cores each, at `8b51c3f`.
  `scratch/pheromones/emerge.py`; results (`gens.csv`, `listen.csv`) to be added here.

## 8. Switches and tools added (all off by default; off is bit-identical)

| Switch | What it does |
|---|---|
| `PP_SCENT_DEAF=A,B,alarm\|all` | the mind reads 0 for those planes everywhere (head inputs and changes, per-option features, dig head) |
| `PP_SCENT_SHIFT=CH:DX,...` | the mind reads that plane DX columns away: the scrambled listening test |
| `PP_SCENT_SILENT=...` | ants deposit nothing on those planes (still billed). Equal to deaf by construction; kept for completeness |
| `PP_MIND_LOG_EMITS=1` | adds an `E` row per decision with the three emit strengths to the mind log |
| `PIXEL_PHYSICS_A_DIFFUSE` | plane A's blend (default 0.25) |
| `PP_MIND_ALARM=train` | opens the alarm emit row to mutation on the bed |
| `inrun patches=T:LO:HI` | one moving food heap instead of the two fixed ones; `patches.csv` logs each jump |
| `inrun phero_dump=N` | A and B planes around the nest every N frames |
| `inrun mutate_to=PATH:PM:SIGMA:EVENTS:MSEED` | mutate one genome and save it |
| `examples/molecules.rs` | one walk of trail per molecule, both laws, at five ages |

`PP_SCENT_BLIND=1` (older) zeroes only the head scent inputs, not the per-option features or the dig head, and so
is a **partial** blinding; use `PP_SCENT_DEAF=all` for a listening test.
