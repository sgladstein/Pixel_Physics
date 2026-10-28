# Evolvable pheromones: what the scents mean, a better B, and an emergence test (2026-10-25)

*Lane report, thread "Let ants evolve their own pheromones" (branch `claude/project-thread-iff53x`). Status:
**reopened 2026-10-25 15:00Z at the owner's request.** Decode done; B fade 0.25 rejected under selection (§5); the
first emergence test found no convention that beats deafness (§7), but the follow-ups (§9) found why, and that a
deleted food trail **re-evolves** once its emitter starts dense. A new-channel emergence run (§9.5, 2 replicates) found signals that emerge and are relied on, yet lose to deaf colonies (best genome vs best genome 1.4-2.2x; the review in §9.8 puts the arm-level gap at ~1.1-1.2x for replicate 2, n = 2), and opening a channel to a good forager cost it 13% (§9.6). Verdict §9.7: signals emerge, foraging does not improve; the owner chose to continue (2026-10-27); review and plan in §9.8; counted in energy rather than larder cells, replicate 2 is a tie and replicate 1 a 1.8x loss (§9.9). The PF4 positive control failed (§9.12); rechecked (§9.13), its 1.8x was mostly a farmed score (re-carried food), the real gap 1.16x, and ants with no scent relearned to forage. Companion: [the adversarial review](evolvable-pheromones-review-2026-10-25.md). Pictures:
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
  starvation unchanged, with selection off, **but loses under selection**: trained under it, colonies bring home
  more food and dig half the nest, and more ants starve. B stays at 0.03.
- **Why** (§6): slow-fading B piles up in the nest five times over, and foragers coming home stop to eat from the
  store and shuffle food instead of digging. Faster fading keeps the nest from smelling of food.
- **Emergence test** (§7): with the unused alarm channel opened to evolution under colony-level selection, the best
  brains came to rely on it, but the colonies got worse than in the deaf control (median 9 against 284), and the run
  lost its last good lineage. No convention that beats deafness formed, so the palette stays parked.
- **Not a dead end** (§9, owner asked 15:00Z): an independent review found the first test could not have passed
  (the alarm plane reaches ~3 cells, food sits 30-110 away), and that the free arm's reliance was real
  information, not a level. Deleting today's food trail and letting evolution rebuild it **fails from a zero
  start (2 of 2) and works from a small random start**: in ~40 generations the rebuilt trail is laid only by
  laden ants and colonies that hear it beat deaf ones 4/4. The blocker was search, not the idea.
- **The real test** (§9.5-9.7, owner approved 2026-10-25 19:06Z): on a fresh channel that reaches the food, two
  replicates of 150 generations each invented real signals (a "searching" mark; a traffic map) that their colonies
  came to depend on, yet deaf controls foraged better (1.4-2.2x best vs best; ~1.1-1.2x arm-level for replicate 2, §9.8), and opening the channel to a good forager cost it
  13%. Signals emerge; foraging does not improve. Not the emission bill.

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
- **Under selection it loses.** Teacher lane, price25b: 32 colonies x 24 rounds from colony 8, link price 0.25,
  ramp capped at 120, all colonies alive throughout. Garden medians against the shared control, paired over 32:

  | round | nest | nest bigger | food home | food higher | starved |
  |---|---|---|---|---|---|
  | r7 | 440 vs 458 | 13/32 | 11,920 vs 12,369 | 17/32 | |
  | r15 | 476 vs 454 | 21/32 | 12,010 vs 12,905 | 14/32 | 27 vs 12 (fewer 9/32) |
  | r23 | 214 vs 501 | 2/32 | 15,988 vs 12,230 | 28/32 | 22.5 vs 8.5 (fewer 8/32) |

  By r23 selection under the fast fade traded the nest for food home and the starvation guard fails. **B stays at
  0.03.** Re-gardening colonies 1-8 at r23 with events (`scratch/pheromones/storeeat.py`) shows a different cause
  from §6: store eating per forager is unchanged (~76 each), churn unchanged; the trained colonies have more
  foragers (63.5 -> 72.5, 6/8) that each dig about half as much (forager cuts 234 -> 146, 3.7 -> 2.0 each).
  **Lesson**: a constant judged on an untrained pool says nothing about a pool trained under it.

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

## 7. The emergence test

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
- **Run**: 16 colonies x 200 generations x 100,000 frames per arm at `8b51c3f`, about a minute a generation on 4
  cores (`scratch/pheromones/emerge.py`). Free arm in this lane's container, deaf and drift on runner threads; the
  deaf arm is read to generation 185 (it finished minutes later). Raw `gens.csv` / `listen.csv` stayed in the
  project's shared folder.

**Results** (listening replays: 4 seeds per checkpoint, one checkpoint every 10 generations; extinct = score 0):

| arm, generations | hearing beat deaf | hearing beat misplaced | extinct: hearing / deaf / misplaced | best (median over gens) | median colony | scoring colonies |
|---|---|---|---|---|---|---|
| free 0-44 | 4/11 | 2/12 | 5 / 1 / 1 of 16 | 454 | 196 | 12.1 of 16 |
| free 45-124 | **20/7** | **20/8** | 8 / **20** / 22 of 32 | 412 | **9** | 8.0 |
| free 125-199 | 15/11 | 17/8 | 8 / 13 / 15 of 32 | 36 | 12 | 11.4 |
| deaf 0-44 | 11/5 | 9/7 | 1 / 3 / 5 of 16 | 552 | 274 | 12.7 |
| deaf 45-124 | 12/19 | 17/13 | 5 / 2 / 3 of 32 | 552 | 284 | 12.5 |
| deaf 125-185 | 9/14 | 10/14 | 2 / 1 / 3 of 24 | 545 | 316 | 13.2 |
| drift | | | all colonies dead by gen 20 | | | |

- **The free arm's best brains did come to rely on the alarm scent** (gens 45-124): deafened or reading it 40
  columns off, they died out in 20-22 of 32 replays against 8 hearing. The deaf arm shows no such reliance.
- **But relying on it made the colonies worse, not better.** Over the same generations the free arm's median
  colony scored 9 against 284 in the deaf arm, and only half its colonies scored at all. At generation 128 the last
  strong lineage drew a seed it could not survive (its elite clone scored 0) and nothing above ~40 scored again.
- **Reading**: std_pool already reads the alarm plane (trained when it meant nothing). Once its emit row mutates,
  every emit mutation changes what every nestmate hears at once, so most are harmful, and what selection kept was
  brains wired to whatever their own lineage happened to emit: dependence, i.e. fragility, not a convention. A real
  convention would show as the free arm out-scoring the deaf one; it never did. Drift collapsing in ~10 generations
  says unprotected standard mutation is itself heavy; colony selection holds it off in the deaf arm.
- **Verdict for the owner's rule** (palette only if a convention forms): **no convention formed; the palette stays
  parked.**
- **If retried** (done, §9 -- the follow-ups repaired these and more): a founder that does not yet read the channel (zero its alarm read weights),
  two or more seeds per evaluation so one extinction cannot erase the best lineage, 32+ colonies, and replicate runs
  per arm. About 4x this run's cost.

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
| `scratch/pheromones/emerge.py` | the emergence test's generation driver (§7) |
| `scratch/pheromones/storeeat.py` | per-class store eating, churn, deliveries and digging from `events.csv` |
| `PP_SCENT_CONST=CH:V,...` | the mind reads that plane as one constant level everywhere: separates information from level (§9.1) |
| `PP_EMIT_FREE=1` | mind ants' scent deposits cost nothing: prices a signal (§9.5) |
| `PIXEL_PHYSICS_ALARM_GROUND=RHO:DIFFUSE` | the third plane on the ground law at that fade and blend instead of the air-law call (§9.5) |
| `emerge.py EVALS= SCORE= ZERO= FILL= CHAN= ALARM=` | seeds per colony, score, zero/dense-start an emit row, which channel the replays deafen, alarm row open or frozen |

`PP_SCENT_BLIND=1` (older) zeroes only the head scent inputs, not the per-option features or the dig head, and so
is a **partial** blinding; use `PP_SCENT_DEAF=all` for a listening test.

## 9. Follow-ups: is this a dead end? (owner asked 2026-10-25 15:00Z)

The owner asked for an independent review and a straight answer, preferring to continue. The review (a fresh
sub-agent reading the code, this report and the run data) said **the first test was a dead end by construction,
but the line is not**, and named three defects, each of which the follow-ups below either confirmed or repaired:

1. **The channel could not reach the food.** The alarm plane spreads on the air law and an ant's own call carries
   about 3 cells (§1); food sits 30-110 columns from the door. Nothing worth saying could be said on it.
2. **One seed per evaluation was noise.** A colony's score on one seed predicts its score on the next at r = 0.23,
   so ranking 16 colonies on one seed each mostly ranked luck, and one bad draw erased the best lineage (gen 128).
3. **The score paid for a huddle.** Stored food + nest cells let a four-ant colony that dug and sat score well.

### 9.1 What the free arm's best ant actually learned (gen-49 colony 4, `scratchpad nx/`)

- **The reliance was information, not a level.** Read alarm as a constant everywhere (new switch
  `PP_SCENT_CONST=alarm:V`), at any level from 0.005 to 0.4, and 5-8 of 8 colonies go extinct; read 40 columns off,
  8 of 8; reading the real map, 0 of 8. Its ants were using *where* the scent was, not a bias it set.
- **It is social, not private.** In a chimera (half the founders silent on alarm), silent hatchlings survive as
  well as emitters (24% vs 19%), but an all-silent colony dies like a deaf one (7 of 8 extinct). The scent is a
  shared map that free-riders can read.
- **Decode** (`PP_MIND_LOG_EMITS`, mean emit strength, founder -> gen 49): A turned into a laden trail (empty ants
  away from home 0.544 -> 0.176, laden 0.647 -> 0.587); alarm became the empty-searching mark (0.384 -> 0.508).
  So the run did evolve a division of labour between two scents; it just did not pay at the colony level.

### 9.2 Does today's food trail pay on the patch bed?

On the emergence bed (one heap that moves every 20,000 frames), deafening the founder to B costs stored food
231 -> 128, lower on 12 of 16 seeds. So there is something worth saying on this bed, and B is saying it.

### 9.3 Check 3: rebuild the deleted food trail (default start) -- failed, 2 of 2

The founder's B emit row (slots 2375-2409) zeroed and left to mutation, readers intact, alarm row frozen;
2 seeds per evaluation, score = stored food only, 16 colonies x 50 generations. Without its B row the founder
stores 131 vs 231 and goes extinct on 8 of 16 seeds vs 1, so the prize for rebuilding it is large.

- Replicate 1: colonies whose ants laid any B scored *lower* (median 133 vs 158), and the best genome never laid B.
- Replicate 2 (runner thread): listening gap 0 at the end.

The mutation operator's add events (4 per child, anywhere in 3,285 slots) almost never land on the one input a
laden-only emitter needs, and a random emitter is noise every nestmate hears, so it is selected out. **The
failure is search, not value.**

### 9.4 Check 3b: the same, from a dense random start -- passed (1 replicate)

Same as 9.3, but the zeroed row starts with every weight `N(0, 0.05)` (`emerge.py FILL=0.05`), so sigma
mutations can tune what is already wired. Best colony's mean B laid, by 10-generation block:
13,671 -> 5,982 -> 456 -> 18,363 -> 122,836: first the noise is selected out, then a useful emitter grows.

| best genome | normal beat deaf-to-B | normal beat B-read-40-off | stored, normal / deaf (4 seeds) |
|---|---|---|---|
| gen 39 | 4/4 | | 143/130, 250/9, 204/135, 153/52 |
| gen 49 | 4/4 | 4/4 | 262/214, 230/188, 306/212, 236/197 |

Decode of the gen-49 best (mind log, mean B emit): **laden 0.248 (away 0.309, at the nest 0.016), empty 0.000.**
The dense-start founder emitted 0.012 laden / 0.016 empty; the original trained founder 0.365 / 0.007. So
evolution rebuilt a **laden-only food trail that colonies listen to**, in about 40 generations, once the row
started dense. Grafted back onto the original founder alone it does worse than the original row (stored 205 vs
231, 5 vs 1 extinct of 16): it is co-adapted with the rest of the gen-49 genome, as expected.

**Lesson for the ant line:** an emit row opened to unprotected add-mutation from zero will not find a
state-conditioned signal in tens of generations; a dense small-weight start will. Keep the alarm row frozen on
the ant line until a run like 9.5 says otherwise.

### 9.5 The new-channel run (in progress)

Check 3b rebuilt a signal whose *readers already existed*. The owner's question is whether ants invent a new one.
Set-up, repairing all three defects:

- **A channel that reaches**: `PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05` puts the third plane on the ground law at
  B's fade and the lab's thin-trail blend -- a fixed molecule, so the one-physics rule holds. On the founder at
  the start it changes nothing that matters (stored 191.5 vs 196.5 on the air law, 8 seeds), so any gap that grows
  is new.
- **A dense random emitter** on that channel (alarm row zeroed, `FILL=0.05`), open to mutation (`ALARM=train`).
- **2 seeds per evaluation, score = stored food only.**
- **Free vs deaf**, 16 colonies x 150 generations, 2 replicates (seed 1 in this lane's container from 17:45Z;
  seed 2 on two runner threads). Finished 2026-10-26 ~02:30Z. Data: `/mnt/project-files/evolvable-pheromones/newchan/`.

**Result: signals emerged in both replicates, and in both they lost to deafness.**

Median colony score (stored food) by 25-generation block, with the best genome's listening replays (hearing beat
deaf to the new channel / beat it read 40 columns off):

| run | g0-24 | g25-49 | g50-74 | g75-99 | g100-124 | g125-149 | hearing > deaf, g100+ |
|---|---|---|---|---|---|---|---|
| free, rep 1 | 122 | 154 | 199 | 147 | 159 | 182 | 14/16 |
| deaf, rep 1 | 139 | 302 | 387 | 406 | 408 | 441 | (native: deaf) |
| free, rep 2 | 208 | 216 | 243 | 264 | 308 | 362 | 20/20 |
| deaf, rep 2 | 170 | 270 | 341 | 352 | 376 | 404 | (native: deaf) |

Head-to-head, each arm's best genome (free g149 c00; deaf g129 c00 for rep 1, g149 c00 for rep 2) on 16 shared
seeds, stored food median, extinct of 16:

| | free best, hearing | ...deaf | ...read 40 off | ...constant 0.02 | ...constant 0.10 | deaf best, deaf | deaf best, hearing |
|---|---|---|---|---|---|---|---|
| rep 1 | 224, 1 | 122, 7 (14/16 lower) | 159, 3 (12/16) | 148, 6 (14/16) | 60, 9 (11/16) | **497**, 1 (16/16 higher) | 471, 0 |
| rep 2 | 334, 2 | 22, 2 (14/16 lower) | 67, 7 (16/16) | 43, 2 (15/16) | | **478**, 0 (15/16 higher) | 475, 0 |

- **The signals are real.** Every control that removes what the map says while keeping the channel (deaf, misplaced,
  flattened to a constant at either level) costs the free arm's best colony 2x to 15x its food, on 11-16 of 16
  seeds. At the start the same founder showed no such reliance.
- **The two replicates invented different conventions** (`PP_MIND_LOG_EMITS` census, seed 700, mean emit on the
  new channel). Rep 1: an **"I'm empty / searching" mark**, emitted by empty ants 0.65 against 0.12 when laden
  (the same meaning run 1's alarm drifted to, §9.1), the mirror of B. Rep 2: emitted by everyone, everywhere
  (0.73-0.89 in every state), so its information is *where ants are*: a second traffic map like A. The
  dense-start founder emitted 0.007.
- **They lost anyway, by 1.4x to 2.2x, 15-16 of 16 seeds.** The deaf arm's evolved best does not care whether it
  hears (471-475 hearing vs 478-497 deaf), so the gap is not deafness helping; it is what the two populations did
  with 150 generations of search.
- **It is not the cost of emitting.** The free best lays about twice the deaf best's scent; replayed with the bill
  waived (new switch `PP_EMIT_FREE=1`, off = bit-identical, checked on 16 seeds), rep 1's free best stores 229 vs
  224 and the deaf best 473. The bill is ~2% of the gap.
- **Reading:** once nestmates read a channel, every emit mutation changes every listener's input, so the free arm's
  selection is spent holding a convention together, and the colony comes to depend on it (deaf costs it more than it
  ever gained). The deaf arm spends the same search on foraging. The signal is a crutch, not a tool -- the same
  verdict as §7, now with a channel that reaches, two seeds per evaluation, a food-only score and two replicates.

### 9.6 The add-on test -- failed

The last open question: can a signal add on top of a colony that already forages well? Founder = rep 2's deaf best
(478) with the new channel's emit row reset dense (`N(0, 0.05)`), free vs deaf, 16 colonies x 60 generations
(seed 3), then each arm's g59 best on the same 16 seeds (stored food median, extinct of 16):

| founder (deaf) | free best, hearing | free best, deaf | deaf best |
|---|---|---|---|
| 478, 0 | 410, 0 (below the founder 16/16) | 417, 0 (hearing higher 7/16) | **499**, 0 (above free 13/16) |

- **No signal formed** (hearing vs deaf 7/16 at the end, and 2/4 in the last listening replays), and the free arm
  ended **below its own founder on every seed**. The deaf arm held its level and edged up.
- So opening a channel to a colony that already forages well cost it 13% of its food in 60 generations and bought
  nothing. Together with 9.5 (and §7), four runs on this bed agree: under colony selection on this patch bed, a
  signal channel that every nestmate hears is a burden on evolution, whether it ends up relied on (9.5) or not (9.6).

### 9.7 Verdict

- **Emergent signals: yes.** A fresh channel acquires real, information-bearing conventions within ~100
  generations, and different runs invent different ones (a searching mark, a traffic map).
- **Better foraging: no.** In every run the deaf control forages better (1.4-2.2x from the founder, best genome vs best -- see §9.8: n = 2, rep 2 ~1.2x and closing; 1.2x from an
  evolved forager), and the gap is not the cost of emitting.
- **For the ant line**: keep the alarm emit row frozen and the third plane on its shipped air law; the gene-chosen
  palette stays parked. Switches stay off: `PIXEL_PHYSICS_ALARM_GROUND`, `PP_SCENT_CONST`, `PP_EMIT_FREE`.
- **What could change this** (the conditions the rejection depends on): a bed where one ant's private knowledge is
  worth more to nestmates than their own search (food that is rare, brief and far, so most foragers never find it
  alone); a reader that is protected from emit mutations while a convention forms (a separate, slower-mutating
  emitter); or selection over many more generations than 150. None of these is cheap.

### 9.8 Independent review, and the first two steps of its plan (2026-10-27)

The owner did not want to stop (2026-10-27 15:03Z) and asked for a review and a better plan. Full write-up:
https://claude.ai/artifact/Vt8E28wJ3A5vjSfSuYPXHX. The review (fresh sub-agent; code, all CSVs, 56 short runs) found
**the line is not answered**, and corrected four claims above:

- "1.4-2.2x on 15-16/16" is one genome per arm; at the arm level n = 2. Rep 2's population medians over g125-149
  were 362 vs 404 (1.12x), its second-best free genome stores 399 vs the best's 338, so rep 2's gap is ~1.2x and
  closing. Rep 1's is real.
- The deaf arm is not a no-signal control: it evolves A and B, and its best relies on B (470 -> 380 deaf to B,
  8/8). The result is "a third channel did not beat two evolvable ones".
- "The deaf best is indifferent to hearing" proves nothing: it never uses the channel.
- Listening tallies reuse the same 4 worlds at every checkpoint: the independent n is 4.

Harness traps it found, fixed in `emerge.py` (`HARNESS=2` turns on the protocol changes; the bug fixes apply to
both): a `.raw.txt` founder ignored ZERO/FILL; a resume duplicated `gens.csv` rows; elites carried no score history;
an extinct colony scored 0 despite stored food; gen 0 had no unmutated founder; listening worlds never changed.

**E1, where the mutation load comes from** (6 evolved genomes x 12 standard mutants split into their channel part
and the rest, + 12 gentle mutants at sigma 0.25 / 1 event, 4 paired worlds; median mutant/parent food stored):

| genome | parent | standard | channel part only | everything else | gentle |
|---|---|---|---|---|---|
| free rep 1 c00 | 211 | 0.67 | 1.08 | 0.74 | 1.02 |
| free rep 1 c01 | 214 | 1.01 | 1.00 | 1.01 | 1.16 |
| free rep 2 c00 | 396 | 0.50 | 0.72 | 0.54 | 0.70 |
| deaf rep 1 c00 | 466 | 0.92 | 1.00 | 0.97 | 1.14 |
| deaf rep 2 c00 | 485 | 0.87 | 1.00 | 0.88 | 0.98 |
| deaf rep 2 c01 | 464 | 0.94 | 1.00 | 0.94 | 1.01 |

Smelling genomes are more fragile, but mostly to mutations **outside** the scent links (2 of 3), consistent with
a colony that runs on a map it lays itself: change behaviour anywhere and the map changes for everyone. The scent
links themselves matter in 1 of 3. The gentle operator removes most of the load in 2 of 3 but misses its bar
(>= 0.95 on all six).

**E2, room for a signal** (founder and deaf rep 2 best, 16 worlds per bed; per food epoch, the share of empty
out-of-nest ants outside the door-to-heap corridor after a nestmate has already found the heap): 56-65% for the
evolved deaf best on every bed (today's 62%), 75-90% for the founder; other finders reach the heap a median 5,900
frames after the first on today's bed. The review's bar (>= 25% on some bed) passes on today's bed, so the bed is
not the blocker. Caveat: no positive control yet (a colony with a perfect signal), so this is an upper bound.

**The plan from here** (the review's, adjusted by E1/E2; every bar fixed before the run, comparisons paired with a
sign test):

| Step | Question | Cost | Pass bar | On fail |
|---|---|---|---|---|
| E3 | Do the harness fixes speed selection up? Deaf arm only, `HARNESS=1` vs `2`, seeds 31/32, 40 generations (`scripts/e3_run.sh` in the data folder) | ~3.5 h on 4 cores | >= 1.5x gain per 10 generations, judged on a common garden of each run's elite (`gNNN/c00`) at g10/20/30/40 | revisit before E4 |
| E4 | The decisive test: free vs deaf on today's bed, `HARNESS=2`, 6 replicates x 16 colonies x 60 generations, `EVALS=3`, gentle operator `MUT=0.013:0.25:1` | ~240 core-hours | free beats deaf in 6/6 replicates on a common garden of each replicate's top 4 (p = 0.031), and the free best loses >= 20% deaf on >= 12/16 worlds | close the third channel; record the dead end with bed, operator and replicates |
| E5 (optional) | Do evolved signals on A and B beat frozen ones? 4 replicates, evolvable vs frozen emit rows | ~160 core-hours | evolvable wins 4/4 | signals stay a demonstration |

**State at the account move (2026-10-27 18:30Z).** E3 was running and a second independent review, of E4's
readiness (environment, scent set-up, harness, statistical power), had been started at the owner's request
("This is a very resource heavy test. Are we confident..."). E4 was **not approved** and should not run before that
review's open points are settled. Known gaps the lane named itself: E2 has no positive control; the gentle operator
has never run a full evolution; the bed has not been checked against the training-environment review
(`teacher-training/training-environment-review-2026-10-27.md`: harness-only rules, 40-row floor, no windfall); the
founder is `std_pool` (layout 5), not the current ratchet pool or the gated layouts 6-8. Data, genomes and scripts:
`Reports/data/evolvable-pheromones-2026-10-27/`.

### 9.9 E4 readiness review: not ready, and the score counted crumbs (2026-10-27 evening)

The owner asked before approving E4 whether the set-up was right (18:16Z). A second independent review
([evolvable-pheromones-e4-readiness-2026-10-27.md](evolvable-pheromones-e4-readiness-2026-10-27.md)) found it was
not, and its first finding rewrites part of §9.5:

- **"Food stored" counted larder cells, and at frame 100k nearly all are part-eaten crumbs** (`larder.csv`:
  `prov_cells` 0, the rest `crumb_cells`). Re-scored in energy by the lane (each arm's best, 16 shared worlds, the
  §9.5 head-to-heads): **run 1 deaf 216 vs free 118 kJ (deaf higher 16/16, p < 0.001; cells said 497 vs 224);
  run 2 deaf 157 vs free 158 kJ (free higher 7/16: a tie; cells said 478 vs 334).** The deaf arm's energy per
  stored cell fell over evolution (0.55 -> 0.38 in run 1), so part of what selection found was smaller crumbs.
  Run 1's loss stands at 1.8x; run 2 is no evidence either way.
- **PF2, does a food trail pay on this bed for today's founder?** (`std_pool` member 0 as replaced on 10-26, third
  row filled, 16 fresh worlds 7001-7016, data in `Reports/data/evolvable-pheromones-2026-10-27/pf2/`):

  | Arm | Larder kJ | Food delivered home | Tank (mean) | Extinct |
  |---|---|---|---|---|
  | normal | 173 | 2,324 | 10.2 | 0/16 |
  | deaf to B | 205 (higher 14/16, p = 0.004) | 2,164 (lower 10/16) | 6.3 | 2/16 |
  | deaf to A and B | 48 (lower 15/16) | 641 | 0.0 | 12/16 |

  The food trail keeps ants fed and alive; it does not raise what is stored, and raises deliveries ~7%. A stored-food
  score cannot reward a new food trail on this bed, so E4 on stored food could only fail. **The owner was asked
  what "forage better" should mean** (food delivered home, colony fed and alive, or larder energy).
- The review's other blockers: the binary is not pinned (a fresh build makes the layout 6/7 gate and reach rows
  mutable, ~40% of add events); 60 generations with the gentle operator barely moves the channel (it recommends
  gentle off-channel and standard on the channel's own slots, and EVALS=2 x 90 generations for the same cost); the
  6/6 bar detects only ~35% gains, so it proposes pass / dead end / inconclusive rules fixed in advance and a
  committed garden script; and a positive control (PF4: B deafened in both arms, so the prize for a new trail is
  B's whole value) before the big run.
- `emerge.py` now logs `kj, tank, deliver, churn, home` per colony, ranks on `SCORE=kj` or `SCORE=home` on request,
  and no longer double-counts a generation's scores on resume (`hist.json` entries carry their generation). E3 was
  stopped at generation 4 because it ranked on cells.

### 9.10 The goal restated: communication, not foraging (owner, 2026-10-27 18:49Z)

The owner: *"I am not sure that I said 'forage better' was our goal. The goal is if ants can develop their own
pheromone signals to communicate information between ants (who have no other form of communications)... there
does need to be a way to measure it."* Food delivered home is the guard measure (the owner's pick from the card;
chosen, by the owner and by the lane, before the numbers below were computed).

**What counts as communication** (the biology definition: a signal carries information about the sender and
receivers act on it), as four measurable criteria:

1. **Information:** what an ant lays depends on its state (laden or empty, has found food or not). Measure: the emit
   census by state (`census_*.txt`); a convention needs a state ratio of at least 2x.
2. **Response:** receivers act on it. Measure: the colony deaf to the channel loses >= 20% of food delivered on
   >= 12/16 worlds.
3. **Meaning:** it is the real map that matters, not a level the brain leans on. Measure: the map read 40 columns
   off, and a constant level, each lose >= 20% on >= 12/16.
4. **Social:** ants act on other ants' marks. Measure: the half-silent chimera (silent hatchlings survive as well as
   emitters; an all-silent colony fails like a deaf one).

**The new-channel runs scored this way** (each arm's best, 16 shared worlds, food delivered home, sign test against
the free best hearing):

| | Run 1 | Run 2 | Add-on (§9.6) |
|---|---|---|---|
| Free best, hearing | **2,514** | 1,010 | 1,060 |
| ...deaf to the channel | 1,552 (lower 15/16, p = 0.001) | 347 (15/16, p = 0.001) | 1,230 (6/16) |
| ...map 40 columns off | 1,968 (14/16, p = 0.004) | 466 (15/16, p = 0.001) | |
| ...constant level 0.02 | 1,626 (13/16, p = 0.02) | 335 (15/16, p = 0.001) | |
| Deaf arm's best | 1,405 (**lower than free 15/16, p = 0.001**) | 1,224 (higher than free 11/16, p = 0.21) | 1,656 (higher 14/16) |
| Emit ratio by state | empty 0.65 vs laden 0.12 (5.4x) | 0.73-0.89 in every state | none formed |

- **Run 1 meets criteria 1-3, and on food delivered home its signalling colony beats the deaf control 1.8x on 15/16
  worlds.** The convention is an "I'm empty and still searching" mark. Cells (§9.5) and larder energy (§9.9) said
  the opposite because this colony eats what it brings home (fed better and laid more eggs, per the readiness
  review) instead of storing it. Criterion 4 was tested only on the first emergence test's alarm convention.
- **Run 2 is a load-bearing map with little sender-state information** (criterion 1 fails), and forages level with
  the deaf control. The add-on test formed nothing.
- **Caveat:** three food measures have now been read on these runs (cells, kJ, delivered), so the run 1 result is one
  replicate under the third measure looked at. It is a reason to run E4, not a result.

**E4 redefined.** Same replicates (6 free + 6 deaf, fixed harness), but the question is *how often a convention that
meets criteria 1-4 forms and holds*, with food delivered home (`emerge.py SCORE=home`) as both the selection score and
a guard against the deaf arm. The readiness review's other fixes still apply (pinned binary, channel-weighted
operator, 90 generations at EVALS=2, pre-registered garden script, PF4 positive control).

### 9.11 The review's fixes, and the positive control (2026-10-27, owner: "Are you working on the reviewers fixes and positive control?")

- **Score:** `emerge.py SCORE=home` ranks on food delivered home; `kj, tank, deliver, churn, home` are logged for
  every colony whatever ranks. Resume no longer double-counts a generation's scores.
- **Operator:** `MUT=0.013:0.25:1 CHANMUT=0.013:1.0` = gentle mutation everywhere (E1: it removes most of the load in
  5 of 6 genomes) plus the standard step on the channel's own live slots (its emit row and every link reading it:
  177 slots, ~85 live in today's founder), so a forming convention moves at the old pace without the off-channel
  damage.
- **Binary pinned (PF5):** main at `09e338c` built with `PP_GATE=off PP_REACH=off` reproduces the lane's older binary
  bit for bit (`stock.csv` and `larder.csv` md5 equal on 2 worlds x 30k frames, deaf to A and B) and writes identical
  mutants, but in layout 8 (4,625 slots, indices moved). `emerge.py` maps the channel's layout-5 slots through the
  binary itself (a probe genome converted by a zero-rate `mutate_to`), so `CHANMUT` hits the right slots in either
  layout. **E4 runs on main with `PP_GATE=off PP_REACH=off`.**
- **Judge pre-registered:** `scratch/pheromones/garden.py`, committed before any E4 run. Each run's last generation:
  every genome on 4 fresh worlds, its top 4 on 8 more (seeds from 5,000,000, never a training or listening seed);
  each free run's best then tested against criteria 1-3 of §9.10 (information: alarm emit differs >= 3x by sender
  state with the higher >= 0.25, since a random N(0, 0.05) row already differs 2.5x at ~0.2; response, meaning:
  deaf, map 40 columns off and constant 0.02 each lower on >= 12/16 by >= 20% in median). Verdict: SIGNALS FORM if
  >= 3/6 free runs communicate and free/deaf food delivered (geometric mean over paired runs) >= 0.90; DEAD END if
  0/6; otherwise INCONCLUSIVE, next step more replicates of the same harness.
- **PF4, the positive control (running at the account move):** A **and** B deafened in both arms, so a trail on the
  third plane is worth what the founder's two scents are worth together (PF2: 2,324 vs 641 food delivered, 12/16
  extinct). The review proposed B alone; PF2 found B worth only ~7% of food delivered for today's founder, too small
  a prize for a positive control. 16 colonies x 40 generations x 2 worlds, `SCORE=home`, the operator above,
  `HARNESS=2`, seed 41, both arms (`scripts/pf4_run.sh` in the data folder). **Pass, fixed now:** at generation 40
  the free arm's garden value (stage B, `garden.py ... DEAFALSO=A,B`) is >= 1.5x the deaf arm's, and the free best
  meets criteria 2 and 3. If it fails, E4 cannot find a smaller prize: fix operator or length first.

**The E4 command** (per replicate R = 1..6, both arms; then `garden.py` over all twelve):

```
BIN=target/release/examples/inrun PP_GATE=off PP_REACH=off PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05 \
  python3 scratch/pheromones/emerge.py OUT/ARM_R ARM 16 90 100000 assets/minds/std_pool.txt P=4 SEED=$((100+R)) \
  EVALS=2 SCORE=home MUT=0.013:0.25:1 CHANMUT=0.013:1.0 ZERO=2410-2444 FILL=0.05 CHAN=alarm ALARM=train \
  EVERY=10 LISTEN=4 HARNESS=2
python3 scratch/pheromones/garden.py OUT/garden free=OUT/free_1,...,OUT/free_6 deaf=OUT/deaf_1,...,OUT/deaf_6
```

### 9.12 PF4, the positive control: FAILED its pre-registered bar (2026-10-27 23:30Z)

A and B deaf in both arms (§9.11), 16 colonies x 40 generations, food delivered home; judged by `garden.py` on fresh
worlds (data: `Reports/data/evolvable-pheromones-2026-10-27/pf4/`, `garden.json`).

| | Free (hears the third plane) | Deaf (no scent at all) |
|---|---|---|
| Garden value: top 4 x 8 fresh worlds, food delivered home | 1,981 | **3,569** |
| Larder kJ / tank / extinct of 32 | 134 / 10.3 / 0 | 91 / 6.7 / 1 |
| Median colony, generations 35-39 (training) | 1,635 | 2,934 |
| Third plane laid per colony, generations 35-39 | ~80,800 | ~1,900 (stopped laying at ~gen 30) |

- **Bar: free >= 1.5x deaf and the free best meets criteria 2 and 3. Result: 0.56x. Failed.** Per §9.11, E4 cannot
  find a smaller prize with this harness; the operator, the length or something else must change first.
- **The free best came close to communicating.** Information: emit 0.77 when empty at the nest vs 0.15 laden away
  (3.1x, passes). On 16 fresh worlds (food delivered, normal 2,017): deaf to the channel 1,619 (lower 13/16, -19.7%:
  misses the 20% bar), map 40 columns off 1,441 (13/16, -29%: passes), constant 0.02 1,648 (13/16, -18%: misses). So
  a state-dependent signal the colony leans on did form in 40 generations, but it is worth ~20%, while losing the
  channel entirely was worth 80%.
- **The striking number is the deaf arm.** With no scent at all it went from the founder's ~641 (PF2, deaf to A and
  B) to 3,569 food delivered: more than the founder delivers with both its scents (2,324). Its jump (median 1,564 ->
  2,699 at generations 20-34) coincides with it silencing the plane it cannot read (laid 140,944 -> 1,232 per colony),
  so **the emission bill may be far larger than §9.5's cell-scored check (`PP_EMIT_FREE`, 224 vs 229 cells)
  suggested** -- unmeasured on food delivered. The free arm cannot silence without losing what its nestmates read.
- **Next, if the line continues:** re-run this control with `PP_EMIT_FREE=1` in both arms (does the deaf arm's
  advantage vanish when laying costs nothing?), and measure the bill directly: today's founder and the PF4 bests on
  16 worlds with and without `PP_EMIT_FREE`, scored on food delivered.

### 9.13 PF4 rechecked: the deaf arm farmed the score; the real gap is 1.16x, and why the control failed (2026-10-28)

New lane after the account move ("Find evolved pheromone signals"). Brief: verify PF4's verdict, find why it failed,
propose whether and how to continue. Each arm's g39 best (free `c08`, deaf `c13`) and the founder (std_pool member 0,
third row filled as PF4 built it) replayed on 16 fresh worlds (5,000,200-215, garden.py's stage C seeds) under PF4's
environment, **with `events.csv` kept**, and each delivery traced back to the pickup that began its trip. Picture page:
https://claude.ai/artifact/NcXbVeF8P1ndpiojacYnj7. Scripts and summary data: `Reports/data/evolvable-pheromones-2026-10-27/pf4/recheck/`.

**The replay reproduces PF4** (free best 2,016 food delivered, garden.py's stage C said 2,016.5; deaf best 3,588 against
stage B's 3,569). Means per world, sign tests deaf vs free over 16 paired worlds (6 measures: Bonferroni bar p < 0.008):

| measure | founder | free best | deaf best | deaf higher | p |
|---|---|---|---|---|---|
| food delivered home (PF4's score) | 730 | 2,005 | 3,605 | 16/16 | < 0.001 |
| food lifted at the heap (within 15 columns of the current patch) | 676 | 1,439 | 1,537 | 10/16 | 0.45 |
| deliveries whose trip began at the heap | 409 | 787 | 913 | 15/16 | < 0.001 |
| deliveries that re-carried a cell already delivered | 125 (17%) | 665 (33%) | 1,558 (42%) | | |
| alive at the end | 0 | 68 | 63 | 3/16 | 0.02 |
| larder kJ | 75 | 123 | 89 | 4/15 | 0.12 |

- **Traced, not inferred: the score counts re-carries.** A cell put down at the nest is a delivery when any body
  cell is within reach (`put_down`, `mind.rs`); a pickup is "away from home" by the picker's head (`pick_up`,
  `food_at_home`), and the spoil mound by the door lifts heads out of reach (the Nest lane's trace). So a delivered
  cell picked straight back up starts a new trip and is delivered again. World 5000200, deaf best: 1,428 away-from-home
  pickups were cells delivered a median 40 frames earlier. During PF4 the deaf arm's `churn` (non-trip drops at home)
  rose 536 -> 1,344 per colony alongside its score, the visible sign of selection learning the loop.
- **So the 1.8x was mostly the loop.** On deliveries whose trip began at the heap the deaf arm still leads, but by
  **1.16x** (15/16). Lifted at the heap the arms tie. The free colony keeps a few more ants and stores more energy;
  neither survives the correction.
- **The emission bill is not the cause** (§9.12's guess). With `PP_EMIT_FREE=1`, deliveries from the heap: deaf best
  906 vs 913 (8/16, p = 1.0); free best 717 vs 787 (lower 14/16, p = 0.004: waiving the bill perturbs a brain tuned to
  it). Founder unchanged.
- **Why the positive control failed** (inferred from the founder column). PF4 assumed losing A and B was a prize only a
  new trail could win back (PF2: 2,324 vs 641 delivered). But the deaf arm, with no scent at all, more than doubled the
  founder's heap deliveries in 40 generations (409 -> 913). Most of what deafening cost the founder was a brain wired to
  scents, which selection re-wires without any. On this bed a nestmate's trail adds little to private search.
- **A signal still formed, close to the communication criteria**: emit 3.1x by sender state (passes 1), map 40 columns
  off -29% (passes 3a), deaf -19.7% and constant -18% on 13/16 (just miss 2 and 3b; §9.12). So three runs out of three
  with a fresh channel produced state-dependent, load-bearing signals; none produced a colony better than one with no
  signal on this bed.
- **The loop is in training too** (exam garden 500001-500016, `scratch/inrun/exam.py` settings, my driver reproducing
  exam.py's w500001 exactly): re-carried 28% of std_pool's deliveries (median 1,160 of 4,040) and 34% of
  clean1_k3c23's (1,859 of 5,573). clean1 still beats std_pool on deliveries from untouched food, 3,008 vs 2,272
  (14/16, p = 0.004), against 5,573 vs 4,040 counted. Sent to the Teacher and Selection lanes 2026-10-28 00:30Z; the
  Selection lane adopts the Nest lane's `inrun homepick=K` scoring switch; a cell bit ("put down at home") that also
  changes what the ant believes is the Teacher lane's call.
- **Measurement caveats.** `loop.py`'s "untouched" counts a cell that slid after a drop as untouched, so the loop share
  is a lower bound. The heap window (15 columns) is a choice; deliveries whose trip began at the heap are the measure
  least open to the loop, since a re-carry starts at the door.

**Proposed next step** (card to the owner 2026-10-28): PF4 once more on a bed where private search is hard, scored on
deliveries whose trip began at the heap, ~4 h on the lane's container. A clear free win reopens E4 on that bed; a tie
closes the third channel. E4 on today's bed is not recommended.

**Choosing that bed: a founder cannot say how much a bed rewards scent.** First bed check (founder, hearing A and B vs
deaf to them, 8 worlds 5,000,400-407, deliveries from the heap): today's bed 881 vs 412 (8/8, 2.1x); heaps every 5,000
frames 60-120 out at `cap=30`, 674 vs 441 (8/8, 1.5x). The faster bed made scent worth *less* to the founder, partly
because old heaps are never removed, so a bed that moves faster leaves more food lying about. And by the lesson above,
the founder's gap measures how its brain is wired to scents, not what a trail could add. A second check uses the two
evolved bests instead.

**Second bed check, the PF4 bests** (both evolved on today's bed; deliveries whose trip began at the heap, mean of 8
worlds, free / deaf; free alive higher):

| bed | deaf best | free best | free / deaf | free higher | alive, free higher |
|---|---|---|---|---|---|
| today (`patches=20000:30:110`) | 931 | 750 | 0.81 | 1/8 | |
| far (`patches=20000:80:120 cap=30`) | 815 | 718 | 0.88 | 2/8 | |
| wide (`PP_BED_W=448 patches=20000:100:250`) | 578 | 545 | 0.94 | 1/8 | 7/8 (p = 0.07) |

The farther the food, the more the scent-free forager loses (931 -> 578) and the smaller its lead over the signaller,
even for genomes evolved where food was near; on the wide bed the signalling colony keeps more ants alive on 7 of 8.
Suggestive at n = 8, not a result. **The proposed rerun therefore uses the wide bed** (the exam garden's width, so the
same bed the trained pools are judged on): PF4's set-up, `PP_BED_W=448 patches=20000:100:250`, scored on deliveries
whose trip began at the heap, ~5 h on 4 cores. Pass bar, fixed now: the free arm's garden value (stage B on that score)
>= 1.2x the deaf arm's AND its best meets criteria 2 and 3. Fail: close the third channel on today's evidence.
