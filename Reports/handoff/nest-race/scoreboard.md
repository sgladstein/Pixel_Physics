# Nest race scoreboard

Shared by the two nest lanes (Nest building = lane 3, Nest race). Append only; re-read before editing.
**Scott's setup:** `digbox ants=20 food=60 hungry frames=40000`, eggs at the nest, seeds 1-4, `RAYON_NUM_THREADS=1`.
**Home** = `HOME` line (dug cells that are home, NEST_HOME=dug; the founding cut alone is 26). Medians of 4 unless noted; per-seed values at 40k in brackets.

## Nest building (lane 3), 2026-10-03

Binary: 6e3bf2bc (= main 538cf248 + walk-through PR 595, the same sim tree as main 7adddd24) + PR 597's DIG_FACE draft. Walk-through on (shipped). `off` reproduced byte-identical on a82c2cca (main a707e0a1 + PR 595).

| theory | switch | 7k | 13k | 20k | 27k | 33k | 40k | per seed @40k | ants alive @40k |
|---|---|---|---|---|---|---|---|---|---|
| baseline (main) | `PIXEL_PHYSICS_DIG_FACE=off` | 46.5 | 48.5 | 53 | 60.5 | 62 | **70** | 67 154 65 73 | 29 69 27 52 |
| turn to nearest cuttable wall, nest workers (v1, superseded) | v1 `workers` | 44.5 | 47 | 52 | 59 | 65.5 | 79 | 64 91 111 67 | 27 86 65 27 |
| turn to nearest cuttable wall, every ant at home (v1, superseded) | v1 `on` | 45 | 46.5 | 50.5 | 56.5 | 64 | 73 | 78 65 90 68 | 34 26 31 36 |
| **turn to nearest UNDERGROUND wall, nest workers** (PR 597) | `workers` | 44 | 52 | 58 | 67 | 82.5 | **103** | 209 68 106 100 | 119 33 50 44 |
| turn to nearest underground wall, every ant at home | `on` | 47.5 | 53 | 62.5 | 75 | 77 | 84 | 88 70 80 121 | 56 38 33 46 |
| PR 597 `workers` + fed ants leave food piled at home alone | + `PIXEL_PHYSICS_STOREROOM=on,caste=4,workerhome,pile,keep` | 40 | 52 | 61 | 69 | 94 | **112.5** | 128 82 125 100 | 55 36 56 46 |

**Theory behind PR 597 (per-ant trace, `digbox antlife=`, frames 20k-40k, 4 seeds):** every ant at home with free jaws has `Dig` ~0.8 and wins the roll, but the cut is only the cell ahead of the head, which is open air 33-56% / a nestmate 17-39% / cuttable ground 14-24% of the time. Cuts per won roll: 0.015 on this setup. v1 failed because the nearest cuttable cell in a shallow nest is usually the roof crust, which the heap cue vetoes (~1 in 1,000 let through).
**Ruled out:** "nest workers don't stay home" as a big lever -- their trips out are short (median 3-5 columns from the door, 400-600 frames), about half to tip spoil; they almost never reach the food.

### Why the old test box dug bigger (one seed each, frame 40k, same binary, DIG_FACE off)

| setup | cells dug below ground since 0 | of them holding larvae/food ("other") | cuts | cuts per won roll |
|---|---|---|---|---|
| old box: `ants=40` fed (20,000 J, no food, never hungry) | 220 | 205 (150 larvae laid by 7k, never pupate) | 302 | 0.030 |
| `ants=20` fed | -- | -- | 261-268 (s1, s3) | 0.037 |
| `ants=20 food=60` (fed) | 145 (s1) / 52 (s3) | 9 / -- | 307 / 139 | 0.013 / 0.010 |
| Scott's: `ants=20 food=60 hungry` | 53 (s1) | 30 | 190 | 0.015 |
| `ants=40 food=60 hungry` | -- | -- | 250-253 | 0.012-0.014 |

Reading: the aim limit is present in every box; the old box was bigger because 40 ants with nothing to fetch spent all their time at home. **The old box is broken as a nest-size reading since eggs went nest-only**: its larvae are never fed (no food on the ground), never pupate, and fill the dug nest. `PIXEL_PHYSICS_BROOD=off` on the fed box explodes the colony (2.1M dig rolls) and is not a usable control either.

### Lane 3 addendum 20:55 -- a clean "old box", and DIG_FACE in it

Feeding the old box's larvae would not give a clean reading (with 20,000 J founders the colony explodes, as `BROOD=off` alone shows), and `cap=` alone does not stop laying (259 eggs by 13k, none ever fed). **The clean old box is `digbox ants=40 cap=40` with `PIXEL_PHYSICS_BROOD=off`**: 40 founders, no births, nothing to fetch. Same binary as above, one seed each, cells dug below ground (open in brackets):

| arm | 7k | 13k | 20k | 27k | 33k | 40k | cuts | per won roll |
|---|---|---|---|---|---|---|---|---|
| clean old box, s1 | 83 (52) | 187 (143) | 243 (168) | 280 (208) | 317 (224) | **345 (243)** | 1,138 | 0.016 |
| clean old box, s3 | 64 (43) | 176 (135) | 233 (148) | 276 (189) | 315 (216) | **348 (237)** | 1,126 | 0.014 |
| + DIG_FACE=workers, s1 | 91 (58) | 197 (150) | 254 (184) | 305 (221) | 336 (240) | 355 (246) | 1,406 | -- |
| + DIG_FACE=workers, s3 | 72 (42) | 166 (117) | 222 (152) | 255 (181) | 293 (216) | 323 (219) | 1,201 | -- |

Reading: with every ant home all the time, the same low cuts-per-roll still dig ~345 cells, and the face turn is **neutral** there (extra cuts are re-cuts). So in Scott's setup the limit is ant-hours at home first; aim matters there because those few hours are spent facing air. A theory for the race lane to knock down: "Scott's colony digs small because ~80% of forager time is spent outside" -- a trace that shows foragers at home with free jaws and a won roll cutting nothing would refute it.

Laying's layer trace (20:43, `/mnt/project-files/laying/layer-trace/notes-2026-10-03.md`) adds a lab-only factor: home anchors fill in over time (open -> soil -> water on seed 1). Not in digbox; lane 3 to look next.

## Nest race (lane 20), 2026-10-03

### Regression hunt: old code on Scott's setup (one binary per commit, `PIXEL_PHYSICS_BUD_SITE=nest` set explicitly)

| commit | seed | 7k | 13k | 20k | 27k | 33k | 40k |
|---|---|---|---|---|---|---|---|
| a98363c0 (10-03 09:35, before walk-through) | 1 | 49 | 48 | 47 | 50 | 51 | **55** |
| a98363c0 | 3 | 33 | 33 | 35 | 44 | 43 | **43** |
| 7adddd24 (main now) | 1 | 47 | 50 | 53 | 59 | 62 | **67** (matches lane 3's s1) |
| 7adddd24 (lane 3 baseline) | 3 | | | | | | 65 |

Reading (early, 2 seeds): **no code regression since this morning; today digs a little more.** Supports lane 3's T2 (setup, not code). 5244c858 (10-02) running next.

### Theory T3 (lane 20): no positive feedback in where digging happens
Real excavation amplifies at the active face (Buhl et al. 2004 Naturwiss. 91:602; Toffin et al. 2009 PNAS 106:18616); fresh pellets decide where leaf-cutters start digging (Pielström & Roces 2013 PLoS One e57040). Our dig cuts the cell ahead or nothing; `dig_return` remembers only each ant's OWN last cut. Switch `PIXEL_PHYSICS_FRESH_CUT=aim|draw` on branch claude/nest-race-iit0pb: cuts inside the nest are remembered 3,000 frames; a won roll with nothing ahead cuts a wall beside a fresh cut (`aim`), else turns toward the nearest fresh cut within 10 cells (`draw`). Lane 3: knock-down target = "the turned rolls are re-cuts of the same face, like DIG_FACE in the clean old box".

### T3 first read (lane 20, 4 seeds, binary = main 7adddd24 + FRESH_CUT on claude/nest-race-iit0pb; `off` s1 = 67, bit-exact with main)
5244c858 (10-02) dropped from the hunt: no HOME census and no BUD_SITE there, so not comparable. **Regression hunt closed: no code regression on Scott's setup.**

| arm | 7k | 13k | 20k | 27k | 33k | 40k | per seed @40k | turned rolls (s1-s4) | digs (s1-s4) |
|---|---|---|---|---|---|---|---|---|---|
| main (lane 3 numbers) | 46.5 | 48.5 | 53 | 60.5 | 62 | 70 | 67 154 65 73 | -- | 190 (s1) |
| FRESH_CUT=aim | 45 | 54.5 | 57 | 65.5 | 76 | **89** | 151 84 60 94 | 145 117 56 87 | 376 317 200 196 |
| FRESH_CUT=draw | 46 | 49 | 52.5 | 62.5 | 65.5 | 75.5 | 79 82 67 72 | faced 93 104 83 84; drawn 5,373-7,716 | 293 334 246 297 |

Reading: aim 2/4 up (s1 doubled), draw near-inert (turns undone by the next step). Both behind DIG_FACE=workers (103). Re-running main s2-s4 on my binary to check the 154.

### Lane 3, 21:15 -- "dig around piles" knocked down (4 seeds)

PR 597 head e585b5a0 (DIG_FACE=workers default) + trial `PIXEL_PHYSICS_DIG_PILES=on`: of the underground faces in turn order, take the first with loose food or brood beside it. Scott's setup, seeds 1-4, home medians:

| arm | 7k | 13k | 20k | 27k | 33k | 40k | per seed @40k | ants @40k |
|---|---|---|---|---|---|---|---|---|
| DIG_FACE=workers (reproduces the table above exactly) | 44 | 52 | 58 | 67 | 82.5 | 103 | 209 68 106 100 | 119 33 50 44 |
| + DIG_PILES | 43.5 | 52 | 55 | 68 | 76.5 | **86.5** | 209 68 66 105 | 119 33 42 30 |

Seeds 1-2 byte-identical (the preference never changed a pick before the runs diverged); seeds 3-4 smaller home, fewer ants. Not adopted; patch kept in lane 3's scratchpad. Aiming the turn at piles is not the lever; don't retry it as stated without a reason the pile should matter (e.g. a pile with no room round it).

### 21:10 lane 20: yesterday's build, a census of the wasted rolls, and RECRUIT
**Yesterday 5244c858** (eggs anywhere, no HOME census), SUMMARY digs at 40k: 61 141 93 103 (median 98) vs main 190 428 163 202 (196); its colonies boom to 300-420 ants. **No build dug big nests on this setup.**

**Why won rolls cut nothing** (FRESH_CUT=aim, IDLEDIG census, s1-s4, won rolls in the nest with nothing cuttable ahead): no cuttable cell round the ant 1,079-3,104 | **only crust (roof/heap cue refuse) 2,516-4,757** | an underground face beside 76-268. So ~50-80% of idle diggers stand at the top of the nest by the crust; turning (aim, DIG_FACE) cannot help them, they must go deeper. Lane 3: this predicts DIG_FACE's turned rolls are a small share of won rolls -- check `digs_faced` vs `dig_rolls`.

**T3b RECRUIT** (`PIXEL_PHYSICS_FRESH_CUT=recruit`): idle won roll with no fresh face beside -> `dig_return` = nearest fresh cut within 16 cells (the existing walk back to the face).

| arm | 7k | 13k | 20k | 27k | 33k | 40k | per seed @40k | digs | live ants @40k | born |
|---|---|---|---|---|---|---|---|---|---|---|
| main (my rerun, = lane 3) | 46.5 | 48.5 | 53 | 60.5 | 62 | 70 | 67 154 65 73 | 190 428 163 202 | 29 69 27 52 | 24 74 20 51 |
| aim | 45 | 54.5 | 57 | 65.5 | 76 | 89 | 151 84 60 94 | 376 317 200 196 | 37 39 27 31 | 36 35 20 26 |
| **recruit** | 49.5 | 66 | 103 | 114 | 120.5 | **130.5** | 165 129 132 121 | 463 394 429 355 | 34 12 32 12 | 30 5 28 17 |

Recruit: home 3/4 up, but births fall on s2/s4 (measured harm). Tracing why next.

### Lane 3, 21:30 -- PR 597 in the lab (played_bed, 150k, eggs at the nest, walk-through on, seeds 1-4)

Binary: e585b5a0 (PR 597 head = main 7adddd24 + DIG_FACE). `off` matches Laying's layer-trace numbers on 7adddd24 (eggs 19/15/16/63, alive 2/0/2/13), so the arm is main. Home = labforage `NEST` line, home cells (dug).

| arm | home 24k | 48k | 72k | 96k | 120k | home at 150k (s1-s4) | eggs laid (s1-s4) | ants alive @150k |
|---|---|---|---|---|---|---|---|---|
| off (main) | 33.5 | 35 | 22 | 27 | 29 | 8 32 21 129 | 19 15 16 63 (med 17.5) | 2 0 2 13 |
| DIG_FACE=workers | 28 | 47.5 | 40 | **53.5** | **52.5** | 2 93 9 261 | **30 54 12 113 (med 42)** | 0 11 0 19 |
| + HOME_REAIM=loose | 32.5 | 59 | 48 | 38 | 67.5 | 11 111 0 87 | 19 58 10 71 (med 38.5) | 0 21 0 14 |

Reading (4 seeds, early): the turn roughly doubles mid-run home and eggs laid (3/4 seeds) without a visible survival cost (alive median 2 -> 5.5; 2 up, 2 down); the lab colonies still crash on 2 of 4 seeds in every arm -- that is Laying's / Birth brake's problem, not the dig. HOME_REAIM (re-aim off a buried anchor, Laying's finding) fired 343-907 times a run and adds nothing clear on top: not adopted.

### 21:30 lane 20: recruit for workers, the stack, and T4 (nest tracks colony size)
Binaries: main 7adddd24 + FRESH_CUT (12a428a9 on claude/nest-race-iit0pb); stack = that merged with lane 3's e585b5a0 (DIG_FACE=workers default).

| arm | 7k | 13k | 20k | 27k | 33k | 40k | per seed @40k | live ants | born |
|---|---|---|---|---|---|---|---|---|---|
| recruit (nest-bound only) | 42.5 | 49.5 | 52.5 | 66 | 74 | **89** | 73 177 71 105 (4/4 up) | 29 56 27 48 | 23 58 22 46 |
| DIG_FACE=workers alone (stack binary, reproduces lane 3) | | | | | | **103** | 209 68 106 100 | 119 33 50 44 | 123 29 48 41 |
| DIG_FACE=workers + recruit | | | | | | 85.5 | 116 74 64 97 (3/4 below DIG_FACE) | 63 29 32 38 | 57 24 25 32 |

**T4 (lane 20): on this setup the nest tracks colony size, ~2 home cells per live ant.** 35 runs, home vs live ants at 40k, r = 0.57; home/ant medians: main 2.3, DIG_FACE 2.1, recruit 2.6, stack 2.3. Only recruitall breaks it (4.1-10.8/ant) by taking foragers off the food. So a "bigger nest" in a 4-seed median is mostly a bigger colony (DIG_FACE s1: 209 cells with 119 ants vs 29). Knock-down for both lanes: **score arms on home cells per live ant, paired by seed, not home cells alone.**
Next: `recruitfed` (nest-bound, or any digger at/above start_energy).

### Lane 3, 21:45 -- T4 holds for DIG_FACE (12 seeds), and the turn's chain count
Binary: PR 597 head e585b5a0 + a scratch lineage counter (byte-identical to the head: seeds 1-4 reproduce digs 463/218/324/292). Scott's setup, seeds 1-12 paired, RAYON_NUM_THREADS=1.

| arm | 7k | 13k | 20k | 27k | 33k | 40k | p90 @40k | live ants (median) | born (median) | home per live ant (median) |
|---|---|---|---|---|---|---|---|---|---|---|
| off (main) | 41.5 | 47.5 | 53 | 62 | 64 | 69 | 92 | 37 | 32 | 2.06 |
| DIG_FACE=workers | 42 | 47 | 52 | 61 | 71.5 | **90** | 121 | 37.5 | 33 | **2.09** |

Per seed @40k, home (live): off 67(29) 154(69) 65(27) 73(52) 70(37) 68(41) 83(28) 67(21) 92(34) 56(38) 75(48) 65(37); workers 209(119) 68(33) 106(50) 100(44) 93(15) 71(32) 80(27) 91(68) 63(42) 89(32) 121(59) 62(33).

- Home cells 8/12 up, median +21, but **home per live ant 6/12 up, median 2.06 -> 2.09: lane 20's T4 knock-down stands.** Fitting home on live ants per arm, at a matched 37 ants: off 76, workers 84 (+8 cells, ~11%); r = 0.70 / 0.86. The 4-seed "70 -> 103" was mostly seed 1's colony booming (29 -> 119 ants).
- The two arms are identical to 27k; any gap opens after 33k.
- digs_faced vs dig_rolls (lane 20's ask): turns are 16-62 a run against 11-21k won rolls (0.1-0.4%), as T3's crust census predicted. Each turned cut is followed by 1.2-4.7 cuts made from inside it (lineage: a cut made while standing in a turned or turned-descended cell), median 2.6; under `on`, 147-156 turns and ~0.75 follow-ons each. So the turn does open faces that get worked, but it is too rare to move the nest much.
- Reading: on this setup **no rule so far (DIG_FACE, recruit, stack) has moved digging per ant**; nest size follows colony size. Lane 3 next: why does a home digger stop at ~2 cells per ant -- crust-limited (T3's census) or no reason to go on?

### 21:50 lane 20: recruitfed + the LAB box (nestdoor played_bed, 4 seeds, 60k)
Draft PR 599 (head a5074d14), `PIXEL_PHYSICS_FRESH_CUT=recruitfed` (nest-bound + any digger at/above start_energy).
digbox (Scott's setup): home 114 167 101 82 (107.5) vs 70, 4/4 up; home per live ant 3.25 vs 2.3; live 22 54 30 27 vs 29 69 27 52.
LAB, dug home at 10k..60k:
- main off: s1 35 54 57 18 19 17 | s2 10 26 30 43 45 51 | s3 34 51 41 33 23 9 | s4 38 31 29 53 80 105 -> @60k median 34; digs 244 617 387 435; births 8 10 12 31
- recruitfed: s1 21 50 71 49 38 37 | s2 26 42 51 49 47 70 | s3 16 21 30 44 53 53 | s4 41 14 11 13 35 24 -> @60k median 45 (3/4 up); digs 805 779 780 272; births 11 3 30 22
**T5 (lane 20, testing now): lab nests shrank from 230-460 (a98363c0, lane 3's flooding trace) because births fell when lab eggs went nest-only (593)** -- lab births 8-31 per 60k on main. Running a98363c0 nestdoor and main with BUD_SITE=anywhere, 4 seeds.

### Lane 3, 21:55 -- more nest workers does not grow the nest (12 seeds)
PR 597 head (DIG_FACE=workers), Scott's setup, seeds 1-12 paired; `PIXEL_PHYSICS_STOREROOM=on,caste=<k>,workerhome,pile` (shipped k=4, one ant in four nest-bound for life). `caste=4` set explicitly reproduces the shipped s1 exactly (digs 463, home 209).

| arm | 7k | 13k | 20k | 27k | 33k | 40k | p90 | live | born | digs | home per live ant |
|---|---|---|---|---|---|---|---|---|---|---|---|
| caste=4 (shipped) | 42 | 47 | 52 | 61 | 71.5 | 90 | 121 | 37.5 | 33 | 233 | 2.09 |
| caste=3 | 42.5 | 49 | 59.5 | 67 | 74 | 89.5 | 185 | 40.5 | 37 | 277 | 2.19 (7/12 up) |
| caste=2 | 42.5 | 50.5 | 55 | 59.5 | 64.5 | 77 | 126 | 37 | 32 | 238.5 | 2.83 (8/12 up) |

Reading: doubling the nest workers (1 in 2) leaves home cells flat-to-lower and live ants unchanged; home per ant rises only because the home is about the same while some colonies stay small. **So time-at-home of nest workers is not what limits the nest on this setup** -- knocks down lane 3's own earlier "ant-hours at home" explanation for Scott's setup (it still holds for the old fed box). Next: what a nest worker does between cuts (single-ant trace).

### 21:40 lane 20: T5 refuted, T7 opened -- LAB REGRESSION since a98363c0 (nestdoor played_bed, 60k, dug home 10k..60k)
- a98363c0: s1 23 25 24 26 10 8 | s2 63 184 287 246 150 152 | s3 53 37 56 174 304 305 | s4 29 26 188 139 124 128 -> @60k median 140; digs 198 373 516 283; births 8 43 58 56
- main 7adddd24: @60k 17 51 9 105 (median 34); digs 244 617 387 435; births 8 10 12 31
- main + BUD_SITE=anywhere: s1 ..76 | s2 ..97 | s3 ..23 | s4 ..78; digs 305 648 515 564; births 90 120 16 77
**T5 (births/egg site shrank lab nests) REFUTED for the 60k window**: lay-anywhere today has MORE births than a98363c0 and still a small home. Same digging, much less home => **T7: something merged since a98363c0 refills or disconnects dug space.** Bisecting 98c5f60c / 8c7346b3 (588) / a707e0a1 (593), seeds 2-4.

### 21:50 lane 20: T7 RETRACTED -- metric change, not a regression (coordinator's check was right)
nestdoor `dug home / ever dug` at 10k..60k. a98363c0's home often EXCEEDS everything ever dug (s2 287/148, s3 304/212): its home flood-filled undug open ground joined to the door. Today home <= ever dug in every sample. On the geometry-only count (ever dug, `World::dug_cells`, same in both): a98 @60k 97 217 238 153 (median 185) vs main 130 184 189 180 (median 182). **No dig regression in the lab.** Today's gap is dug space that does not stay open/joined (lane 3 tracing refill). Rule for both lanes: never compare `dug home` across a98363c0..7adddd24; use ever-dug or open-dug cells.

### Lane 3, 22:10 -- recruitfed (PR 599) over 12 seeds: bigger nest, smaller colony
Binary: PR 599 head a5074d14 (main 7adddd24 + FRESH_CUT), digbox, Scott's setup, seeds 1-12 paired. `off` on this binary reproduces lane 3's main numbers exactly (all 12 seeds).

| arm | 7k | 13k | 20k | 27k | 33k | 40k | p90 | live ants | born | digs | home per live ant |
|---|---|---|---|---|---|---|---|---|---|---|---|
| off | 41.5 | 47.5 | 53 | 62 | 64 | 69 | 92 | 37 | 32 | 211.5 | 2.06 |
| recruitfed | 44 | 54 | 63.5 | 80 | 92.5 | **106** | 124 | **26** | **20.5** | 383 | **3.99** |

Per seed @40k, home (live): recruitfed 114(22) 167(54) 101(30) 82(27) 124(19) 115(33) 107(38) 90(23) 105(19) 98(13) 114(28) 103(25).

- **Home bigger on 12/12 and per ant on 11/12 -- the first rule from either lane that moves digging per ant.** It survives T4.
- **But live ants fall on 9/12 (37 -> 26) and births 32 -> 20.5 (-36%).** The 4-seed "births fall on s2/s4" was real. Likely the same trade as dead-ends' DIG_DOWN entry: a fed forager recruited to the face is a forager not foraging. Knock-down target for lane 20: does `recruit` for nest-bound only keep births (its 4-seed read did: 23 58 22 46 vs 24 74 20 51) while recruitfed's extra comes from the fed foragers?

### Lane 3, 22:25 -- lab seeds 1 and 3: what takes the home, and the nest never goes deep
nestdoor played_bed, main 29124668 + PR 597, scratch probe `homeevery=1000`: every 1,000 frames, each cell that was home (`World::nest_dug`) and is not now, by what stands in it now. Totals over 7k-70k:

| filled by | seed 1 (335 cells lost) | seed 3 (350) |
|---|---|---|
| cut off, still open (crumbs, ants, bare) | 34% | 32% |
| living plant tissue: grass blade, grass root, leaf, wood, root wood | 35% | 35% |
| soil / packed soil fallen back | 22% | 8% |
| water | 3% | 9% |
| pips and seeds | 2% | 16% |
| dead wood, windfall | 4% | 0% |

- **The nest stays in the top 11-12 rows of 80 rows of soil** (deepest cell ever dug: row 171-172, surface 160, both seeds, whole run). So everything that grows or falls reaches it.
- **Seed 1's collapse** is one event: at 57k-58k, 19 cells of `wood` plus a plug of crumbs cut the whole nest off from the door; home 47 -> 3 and never comes back (2-5 cells to 66k). A smaller one at 52k (home 44 -> 19 -> 45 next sample) cleared itself.
- **Seed 3** erodes instead: grass blades grow into the shallow nest (92 cells over the run) and pips land in it (46); home ~40 -> ~12.
- **Living plant tissue is something ants walk through** (soft tissue is parted, wood is crossed: `is_partable`, `trunk_crossing`), but the home fill treats it as a wall. So a grass blade across the shaft cuts everything behind it out of home, and with it the dig urge, laying and the home pull. This is the same bug the home fill already had for crumbs and then brood.
- **Testing now (lane 3)**: `PIXEL_PHYSICS_HOME_PAST_TISSUE=on`, which makes the home fill pass what a body passes, seeds 1-4, 70k. Lane 20: the shallow depth is open if you want it; I'm not on it.

### 22:05 lane 20: does it keep growing? 300k, main 7adddd24, 2 seeds
digbox Scott setup, HOME : live ants: s1 50k 79:29, 100k 222:40, 150k 521:125, 200k 638:48, 250k 617:0, 300k 627:0 | s2 50k 218:105, 100k 629:285, 150k 685:0 ... 685. Grows ~10x with the colony boom, then the colony crashes to 0 and the nest stands empty.
LAB nestdoor, dug home / ever dug every 25k: s1 61/51 19/121 21/140 10/143 8/149 then 8/156 flat | s2 15/106 45/178 31/208 38/232 35/234 26/234 then 9/234 flat. births 18/15, deaths 68/60: lab colony never grows, digging stops ~150k.

## 22:20 NEW GOAL (Scott via coordinator): steady lab colony + separate chambers for food/brood/workers
**Bed + readout for every lane** (branch claude/nest-race-iit0pb, commit 4a3339ac; PR 599):
- scenario `assets/lab_scenarios/nest_goal.ron`: played_bed's box, NO plants, colony 52 @6000 at x 256, one heap of `provisions` (player FOOD brush, never rots) at x 286.
- `cargo run --release --example nestgoal -- seed=N frames=300000 every=12500 shots=DIR` (run from the repo root): tops the heap up to `food=` (120) cells every 250 frames; prints POP (live ants, births, deaths, heap), NEST (open dug space, regions, chambers = 4-connected pieces of cells with >=7/9 open in their 3x3, >=6 cells), CHAMBERS (each: cells, centre, food/brood/ant cells, and separation = 1 - sum min(f_i/F, b_i/B)). `control=selftest` is the positive control.
Baseline running: main c3a7dac1 (+FRESH_CUT off), seeds 1-4 to 300k.

### Lane 3, 22:30 -- home past plants in dug tunnels (HOME_PAST_TISSUE, dug cells only): 12 seeds, lab nests stop collapsing
nestdoor played_bed, 70k, PR 597 head e812aa72 + switch (same as main c3a7dac1 for this), seeds 1-12 paired. The home fill passes living plant tissue a body passes (soft tissue it parts, wood it crosses) **only in cells that were dug** (`World::dug_cells`). `off` reproduces the earlier binary exactly.

| | home @20k | @40k | @70k | worst home 40k-70k | ever dug | live @70k | births | deaths |
|---|---|---|---|---|---|---|---|---|
| off | 35 | 49.5 | 53.5 | 28 | 206 | 25.5 | 26.5 | 40 |
| on | 48.5 | 62 | **85** | **53.5 (12/12 up)** | 214.5 | 23.5 | 25.5 | 42 |

- Seed 1's 57k collapse (home 47 -> 3) does not happen: worst 57. Seed 3's erosion: worst 10 -> 40.
- Colony: live ants down on 7/12, births down 7/12, both within the seed spread (s1 23 -> 7, s9 7 -> 17). The lab colony dies on both arms (deaths > births); this does not change that.
- **The first version passed tissue anywhere and was wrong**: home leaked through the root zone into undug ground (home > ever dug), births lower on 3/4. The dug-only rule is what ships; a unit test fails if the dug-cells condition is removed.
- Shipping ON as a draft PR (lane 3). Goal box (no plants) is not affected by it.

### Lane 2 (laying/survival), 22:28 -- why colonies die beside endless food: foragers starve at home carrying dirt (EARLY, 4 seeds)
Main c3a7dac1. Page: https://claude.ai/artifact/6PLWH3vPbdscyWKm5PVUvD
- **Food box** (digbox ants=20 food=60 hungry, 250k): every seed booms to 208-321 ants then crashes (ends 3/142/0/31 at 200k) while the pile stays 52-60 of 60 full. Food brought in per ant falls as the colony grows (s1: 389 kJ/10k at 120 ants -> 168 kJ at 289).
- **Who starves** (every ant traced, s1 75-90k, s3 115-130k): 9 in 10 spent their last 5,000 frames at the nest, none at the food; 3 in 4 were foragers who had delivered before; they held a dirt pellet 70% of that time. Foragers overall hold a pellet 40-59% of the time from 40k on; 2/3 of underground cuts are by foragers. `Dig` reads no hunger and the walked spoil cycle keeps a pellet inside while patience lasts.
- Ruled out: door throttle off (ends 0/359+/78/77, still crashes), forage drive `returns` (crashes 348->92, 345->162), spoil `keep` off (crashes to 0/13), spoil out off (all dead by 30k).
- **Switch `PIXEL_PHYSICS_LEAN_FORAGE=on`** (**draft PR #601**, head 053acc2e, default on, handed to the desk 23:25; off is bit-identical): under half its grant an ant skips its dig roll, puts its pellet down beside it, and at the door goes out on its own hunger (Blanchard et al. 2000 doi:10.1006/anbe.1999.1374; Bernadou et al. 2020 doi:10.1242/jeb.219238). Food box ants at 200k: main 3/142/0/31 -> lean 230/354/129/94; lowest after peak median 17 -> 112. Still declines on s3/s4.
- **Goal bed** (nestgoal, 150k): main ends 408/0/4/4 (3 of 4 dead or nearly); lean 0/37/148/267. s1 got worse: mass starvation at home at 60-70k, then the rich survivors scattered over the box and never laid again (nest-only laying, nobody home).
- Parts alone, food box 200k: `nodig` 37/237/21/361; `out` 0/82/270/0. All three together is the only arm with no seed under 94.
- Next: the brood column Scott saw (23:25, measuring), then why rich ants away from home stop coming back to lay.

### Lane 3, 22:50 -- nestgoal readout counts every egg/larva as an ant (fix is two lines)
`Census::what` (examples/nestgoal.rs, 4a3339ac) tests "organism of a creature species" before "brood material". A brood cell is an organism of the laying species (brood.rs module doc), so it returns `Ant` and **`brood underground` is always 0, separation always n/a**. The selftest misses it because it places brood with organism id 0. Fix: test `Some(c.material) == self.brood` first. Same seed 1, main c3a7dac1, after the fix: brood underground 0 1 1 4 16 at 12.5k..62.5k, all in the one chamber, separation 0.00 (food and brood share it). Lane 20 owns the file; the selftest should place brood with an organism id.

### Lane 20, 22:45 -- goal bed baseline (main c3a7dac1, nestgoal, 300k): every colony dies beside a full heap, of starvation
Seeds 1-4, live ants peak -> end: s1 408 @150k -> 0 by 300k; s2 never grows (28-41), dead 137.5k; s3 228 @100k -> dead 162.5k; s4 121 -> dead 162.5k. Heap 120/120 at every sample.
- **Deaths by cause** (new `DEATHS` line, commit on claude/nest-race-iit0pb): s3's crash 112.5k->137.5k is 187 starved vs 65 old age; s3 total 255 starved / 252 old age. s1 to 150k: 176 starved, 469 old age. Agrees with lane 2's 22:28 "foragers starve at home" -- the crash is hunger, not age.
- **Nest**: one big chamber (s1 418-594 cells at 125-150k), brood 95-156 cells all in it, **food underground 0-4 cells** -- nobody stores food inside; separation is meaningless with no food in. Pictures: /mnt/project-files/nest-race/goal-baseline/.
- nestgoal brood fix (lane 3's note): brood material is tested first; NEST brood counts are right. POP's "brood" figure still reads 0 -- use NEST's.

### Lane 3, 23:00 -- goal box baseline, seed 1 and 2 pictures (main c3a7dac1, readout with the brood fix)
Picture: /mnt/project-files/nest-race/lane3/goal-box-baseline-maps.png (cell map: chamber, passage, ant, brood, food, packed soil).
- **One room round a column of brood under the door.** Seed 1: chamber 25 -> 88 -> 338 -> 566 cells at 12.5k/62.5k/100k/150k with 32 -> 135 -> 331 -> 408 live ants; brood 0 -> 16 -> 96 -> 155 cells, all in that room; food in the nest 17-23 cells early, ~0-4 from 75k (the brood eats it). A 28-cell second chamber appears at 150k: the room's right lobe behind a thin wall, empty.
- **For lane 2: brood stacks as a vertical column** (powder that does not roll, laid beside brood): at 150k it runs from the pit under the room up the door shaft to the surface. Not jamming traffic yet (PUSH_PAST), but it is the shaft.
- Seed 2: colony 37 -> 41 -> 29 -> 0 (dead by 137.5k, births 58 deaths 110) with the heap full all run; the nest stays one 40-75-cell room with food piled in its floor.
- Lane 3 next: flat rooms (`FLAT_ROOM=<rows>`, scratch): a cut that would raise a room's ceiling above `rows`, or lower the floor of a room 3+ wide, is refused; shafts still deepen. Running 3 and 4 rows on seeds 1-2.

### Lane 20, 23:20 -- why no food is stored underground: it goes in, and is eaten as fast as it arrives
nestgoal goal bed, main c3a7dac1, new `FLOW` line (commit 2nd after 685da2fd on claude/nest-race-iit0pb): loose food underground within 80 columns of the nest, counted every frame.
- Food **does** go underground: 194-2,192 cells arrive per 12.5k interval (seeds 1, 3, to 100k). It **leaves within the same interval** -- departures are within 12 cells of arrivals every interval, so the standing stock is 0-23 cells. Bites (844-5,069) track crop drops at home (674-4,347).
- So the store isn't missing because nobody carries food in. The colony eats its whole inflow. Nest-worker store pick-ups are 0-42 an interval against thousands of drops; the pile rule hardly runs.
- Existing storeroom parts don't change that (goal bed, seeds 1, 3, 125k): `+harvest`, `+keep`, `+harvest,keep` all end with 0-11 food cells underground (baseline 1-16).
- Per-ant delivery collapses as the colony grows (s1 4,347 drops at 62.5k with ~150 ants -> 1,090 at 87.5k with ~270) -- the same thing lane 2 traced as foragers starving at home holding dirt.
- **Reading: a store can only build once the colony delivers more than it eats.** Next: rerun this on lane 2's LEAN_FORAGE branch when it is pushed; if the colony is fed and food still doesn't pile, then it is a placement rule.

### Lane 20, 23:45 -- goal bed on lane 2's PR 601 (head 053acc2e, hungry-drop on), seeds 1 and 3, EARLY to 100k
- Live ants 25k/50k/75k/100k: s1 33/80/73/**13** (main 32/71/207/331); s3 42/47/154/109 (main 42/48/136/228). Starved by 100k: s1 96 (main ~40), s3 120 (main 44).
- **Food underground still 0-18 cells**, arrived = left every interval. Crop drops at home collapse as the colony dies: s1 2,508 (50k) -> 424 (75k) -> 0 (100k).
- So on these two seeds the switch does not feed the colony on the goal bed, and no store builds. Matches lane 2's own "s1 got worse". Runs continue to 200k; pictures in /home/claude/runs/lean-s*/ on lane 20's box.

### Lane 20, 00:15 -- PR 601 on the goal bed, to 200k (final for seeds 1, 3)
- s1: dead by 150k (main: alive to ~290k). s3: **alive and growing, 297 ants at 200k** (main: dead by 162.5k). One better, one worse, as lane 2 found.
- s3 at 150k held **143 food cells underground** -- the first store seen on the goal bed -- but all in the one chamber with the brood (341 cells, food 137 / brood 85, separation 0.00), and gone by 175k (1 cell).
- Reading: when the colony is fed, food can pile up underground. Keeping it apart from the brood needs a second chamber (lane 3) and a placement rule (lane 2).

### Lane 3, 23:40 -- three chamber rules compared on the goal box (Scott: "Do all three"), seeds 1-2, 100k
Page: https://claude.ai/artifact/EPQw2uBJj6ZXpLpnMonTFq (cross-sections + table). Binary: race branch 4a3339ac + lane 3 scratch switches; readout with the brood fix; all switches off reproduces main.
- **Two crowds** (`FLAT_ROOM=4` + `BROOD_CARRY=on` + scratch `BROOD_DEEP=on`, brood carried to the lowest floor in reach): **3 chambers on both seeds, separation 0.90 / 0.97**, brood in smaller chambers under the ants' room; 116 / 114 ants at 100k.
- Flat rooms alone: second chamber with brood on s2 (0.82); s1 colony dead by 100k.
- Two crowds with a stricter cap (no column of room > 4 anywhere): flatter, but brood goes into narrow pits; 1 chamber, n/a.
- `FRESH_CUT=aim` / `recruitfed` (lane 20's), pillars (`PILLAR=7`): one domed room round the brood, like today.
- Population swings 0-450 between seeds of one arm: read shapes, not head counts. Running two crowds vs today on seeds 3-6 now.
- Lane 2: the brood move is a scratch edit in `brood::carry` (depth ranks before touching). Yours to judge.

### Lane 2, 23:30 -- the goal-box crash is a sharing failure; fed ants resting inside keeps all four colonies alive to 150k (EARLY)
Main 192b7103 (601 merged, hungry-drop on). Chart: /mnt/project-files/laying/nest-rest/goal-box-ants.png
- **Why it crashes** (s1 traced, 601 arm 66k; main the same at 76k): the colony is not short of food, about 320 J an ant. Ants on the mound have a median of 313 J, wanderers 356-460, ants at the food 436-685. Ants in the nest have 91-155: a digging crew (31 of 46 held a pellet at 64k) goes lean together and starves in the chamber. Sharing passes only between neighbours, and no fed ant comes down. Only 5-18 of 130-216 ants are at the food at any time.
- A lean ant walking out to the door (`leave`, scratch, dropped) moved those deaths from the chamber to the shaft (39/10 -> 26/41) but did not save the colony.
- **`PIXEL_PHYSICS_NEST_REST=on`** (existing, off; fed foragers and nest workers rest deep along the passages), goal box, live ants at 150k s1-4: **173 / 274 / 358 / 325**. 601 alone: 0/37/148/267. Before 601: 408/0/4/4. Still swings (s2 dipped to 114 at 130k). Biology: food spreads from foragers by ant-to-ant sharing inside the nest (Greenwald et al. 2018 eLife doi:10.7554/eLife.31730).
- Running: goal box to 300k and food box to 200k with it on; then two crowds on top (lane 3's switches) on 2 seeds.

### Lane 3, 23:45 -- two crowds on more seeds and on the new main: brood leaves the door column, but separate chambers on only 4 of 12; correction to 23:40
Page (same link, updated): https://claude.ai/artifact/EPQw2uBJj6ZXpLpnMonTFq. Goal box, 100k. Old main c3a7dac1 seeds 1-6; new main 192b7103 (PR 601 in) seeds 1-4; new main + `EGG_DOOR=door` seeds 1-2; each paired with two crowds (`FLAT_ROOM=4 BROOD_CARRY=on BROOD_DEEP=on`).
- **Correction to 23:40:** separation 0.90 / 0.97 rested on 4 and 1 food cells. **Food underground is 0-15 cells in every run, every arm**, matching lane 20's 23:20 flow (eaten as it arrives). Food vs brood can't be judged yet.
- **Brood shape** (12 two-crowds runs): a layer of pockets under the room floor in 7 (old s1 s2 s4; new s1 s2 s4; door s1), filled the whole room in 2 (old s5, new s3), little brood in 3. Today: a column under the door wherever brood is plentiful (old s1 s5 s6, new s2 s4), and **with `EGG_DOOR=door` alone s2 still has a column in the room**.
- **Separate brood chambers** (readout): 4 of 12 (old s1 s2 s4: 3 chambers each; door+crowds s1: 3 chambers). New main alone + crowds: 0 of 4 (1 chamber each). Today: 1 chamber in 11 of 12 runs.
- Live ants at 100k, two crowds vs today paired: lower on 4 of 6 (old), 2 of 4 (new: 82/48/161/85 vs 13/168/109/179); door: 49/114 vs 127/71. Inside the seed spread both ways.
- Next (lane 3): the pockets are too narrow to be a room. Prototype "brood draws digging": a digger beside brood prefers cutting the wall next to it (Römer & Roces 2014), to widen the layer into a chamber.

### Lane 20, 00:05 -- where food is put down inside the nest, NEST_REST=on, main 192b7103 (+ lane 20 harness), seeds 1 and 3, EARLY to 62.5k
New `SITES` line in nestgoal: every food cell that newly appears underground, by what lay within 2 cells of it (brood only / food only / both / neither). Counts include a cell that slides one place, so read them as shares.
- While the colony is small (to ~40 ants), food goes down beside food: 70-90% "food only", brood-touching under 15%.
- Once it grows (s3 50k, 163 ants; s1-keep 62.5k, 119 ants): food lands beside brood in 55-60% of cases (brood only + both), "neither" rises to 20-35%, and underground food stock drops to 0-1 cells. Everything is in one chamber, so separation is 0.00 or n/a at every sample.
- `+STOREROOM keep` (fed ants leave piled food): no store yet (underground food 1-35, same as without).
- Next (lane 20): switch `PIXEL_PHYSICS_FOOD_SORT` -- a forager at home does not put its crop down with brood within 2 cells, and walks on (Franks & Sendova-Franks 1992 sorting; Römer & Roces 2014 deposit-where-deposited). Same seeds, against these.

### Lane 20, 00:40 -- NEST_REST=on to 150k (main 192b7103): colonies live, still no food store; STOREROOM keep kills them
- NEST_REST on, seeds 1/3 live ants 75k/100k/125k/150k: 91/186/212/173 and 198/274/252/358. Lane 2's survival result holds on my harness.
- **Food underground 0-7 cells at every sample, both seeds**, one chamber with all the brood (s3 537 cells, brood 152, food 0 at 150k). So a living colony still eats its whole inflow; there is no surplus for a sorting rule to act on yet.
- Late in the run food arrives mostly beside brood (s1 150k interval: brood only 804, food only 51, both 106, neither 275).
- **`+STOREROOM ...,keep` is harmful here**: both seeds die (s1 134 @75k -> 0 @125k; s3 154 -> 0 @150k), against 173/358 without it. Don't stack keep with NEST_REST.
- Running now: `PIXEL_PHYSICS_FOOD_SORT=on` (no crop drop at home with brood within 2 cells), seeds 1, 3, against the NEST_REST control above.

### Lane 20, 01:20 -- FOOD_SORT (no crop drop at home within 2 cells of brood), NEST_REST on, main 192b7103, seeds 1/3 to 150k
- Live ants 100k/125k/150k: s1 160/189/**381** (control 186/212/173); s3 56/18/**dead** (control 274/252/358). One much better, one dead: no read on survival from 2 seeds.
- Food underground 2-14 cells at every sample, same as control. The separation scores of 0.95-1.00 at 100k come from 0-2 food cells and mean nothing. **There is still no surplus to sort.**
- Holds fire (36k by 87.5k on s1) but the colony eats everything that comes in either way.
- **Why there is never a surplus:** an ant banks energy up to its breeding threshold (1,100 J against a 200 J grant; crop 5,760 J), so the colony stores its food in its bodies and turns it into brood. That is also why `STOREROOM keep` (fed ants above 100% of the grant leave piled food) killed both colonies: it stops ants banking toward breeding. A floor store competes with breeding as the engine stands. Asking Scott.
- FOOD_SORT stays OFF on branch claude/nest-race-iit0pb; not a winner.

### Lane 3, 00:20 -- two crowds halves the colony on the resting base (NEST_REST=on); dropping it. Brood-draws-digging: one room
Main 192b7103 + goal box + lane 3 scratch, `PIXEL_PHYSICS_NEST_REST=on` in every arm (coordinator 23:34). nestgoal, 100k, live ants (births/deaths). Page updated: https://claude.ai/artifact/EPQw2uBJj6ZXpLpnMonTFq
| arm | s1 | s2 | s3 | s4 |
|---|---|---|---|---|
| rest alone | 186 (263/129) | 308 (639/383) | 274 (546/324) | 265 (328/115) |
| + two crowds | **4** (113/161) | **97** (263/218) | **69** (211/194) | **135** (218/135) |
| + FLAT_ROOM=4 only | 156 | 142 | | |
| + BROOD_CARRY only | 428 | 181 | | |
| + CARRY + DEEP | **48** | 133 | | |
| + two crowds + BROOD_DIG | 87 | 68 | | |
- **Two crowds is lower on 4 of 4 seeds, by half or more**; the depth preference in the brood carry costs most (births 194/317 vs 263/639). Separate chambers: 1 chamber on all 4 seeds (one wide flat room, brood at its edges). Not a candidate; lane 2 need not adopt BROOD_DEEP.
- Rest alone: 1-2 chambers, a narrow room round a brood column (93-174 chamber cells for 186-308 ants).
- `BROOD_DIG=on` (scratch: a digger beside brood cuts the wall next to it, never with open space straight above): 1 chamber on both seeds. A spaced version (`=4`/`=8`, only 4/8 rows below anything open) fired 3-19 times per run: a non-test.
- Food underground at 100k: max 18 cells over all 58 lane-3 runs.

### Lane 20, 01:55 -- CORRECTION to 01:20: the colony's food is mostly in its crops, not banked in bodies
New `ENERGY` line (nestgoal), NEST_REST on, main 192b7103, seeds 1/3. Ants by body energy at 100k (s1): <200 J 33, 200-500 78, 500-1,100 49, >=1,100 26. Most ants sit at 1-2.5 grants, not near the breeding threshold. **In crops: 46,979-164,003 J (49-171 provisions cells at 960 J); in bodies 9,835-110,285 J; on the floor underground 0-14 cells.** So the store exists, carried in the ants' crops (the social stomach). A floor store needs ants to put food down rather than carry it.
- SATIATE (Feed urge tapers above the grant to 0 at f x threshold): f=1.2 slows births (57 vs 120 by 75k, s1) and leaves food underground at 9-27 cells; f=1.5 s3 died by 150k. Not a winner; off.

### Lane 20, 02:10 -- GOAL BED NOW DRAINS ITS SURFACE (lane 2's puddle finding)
Lane 2 (00:01): the mister's water pools in the dip between spoil mound and food heap (the heap dams it); ants can't cross liquid; all 5 dead goal-box colonies had 13-47 water cells there. My own baseline shot (s3 112.5k, crash starting) shows water against the heap's nest side.
- **nestgoal `drain=1` is now the default** (commit after e1b1f898 on claude/nest-race-iit0pb): every 250 frames, liquid on or above the ground row from 60 columns left of the nest to 30 right of the heap is removed and counted (`BED` line). Underground water is untouched. Mister stays on. `drain=0` restores the puddle for comparison.
- **Every goal-box number posted before this (lane 20's baseline, PR 601, NEST_REST, FOOD_SORT, SATIATE) was on the puddle bed.** Re-running main 192b7103 baseline, seeds 1-4 to 300k, drained.
- Correction: NEST_REST is an opt-in switch, not on main.

### Lane 20, 02:50 -- drained goal bed, main 192b7103, EARLY to 100k (seeds 1-4)
Live ants 50k/75k/100k: s1 95/337/568 | s2 46/150/149 | s3 49/137/128 | s4 55/125/138. All alive at 100k (puddle-bed baseline on c3a7dac1 had s2 dead by 137.5k). Drain removes 530-820 water cells per 100k frames. Running on to 300k.

### Lane 20, 04:10 -- drained goal bed, main 192b7103, 300k, seeds 1-4: ALL FOUR COLONIES ALIVE
Live ants every 50k (50/100/150/200/250/300k): s1 95/568/608/384/471/632 | s2 46/149/288/345/399/400 | s3 49/128/165/253/427/426 | s4 55/138/53/71/484/507 (s4 dipped to 17 at 175k and recovered).
- Puddle bed baseline (c3a7dac1, which lacks PR 601): 4/4 dead by 300k. So the puddle was the main killer on the goal box; the colony doesn't need NEST_REST to survive here once the dip is drained. (Two things changed, puddle and PR 601; lane 2's mister control separates them.)
- Still boom-and-wobble, not flat: s1 608 -> 384 -> 632. Starved/old age at 300k: s1 2,376/2,058; s2 854/1,247; s3 468/1,266; s4 230/1,025.
- Nest: ever dug 1,306-2,820 cells; mostly 1 chamber (147-832 cells) holding all brood (90-297 cells); a second small chamber (24-103 cells) appears sometimes with workers only. **Food underground 0-23 cells.** Separation 0.00 / n/a.
- Pictures: /mnt/project-files/nest-race/goal-drained/. Water still lines the nest chamber underground (s3 300k).
- Running now: CROP_UNLOAD=0.25 (fed ants at home drop crop food), with and without FOOD_BRAKE=on, seeds 1/3, 150k, drained bed.

### Lane 20, 05:00 -- CROP_UNLOAD changes nothing (bit-identical); FOOD_BRAKE hurts on the drained bed
Drained bed, main 192b7103 + switches, seeds 1/3, 150k.
- `CROP_UNLOAD=0.25` (drop chance >= 0.25 for a fed ant at home): **every line identical to the baseline** although the floor was consulted 31,797 times on s1. So fed ants at home already roll Drop at >= 0.25; the crop store (59k-483k J) is held by ants that are away from home or under their grant, and it is their digestion, not spare food. Not a lever. Off.
- `+FOOD_BRAKE=on`: live ants at 150k s1 116 (baseline 608), s3 88 (165). Food underground 1-32. Worse here.
- Nothing tried so far (harvest, keep, pile, FOOD_SORT, SATIATE, CROP_UNLOAD) has put more than ~30 food cells underground on a living colony. Leaving the food-chamber question with Scott (card in lane 20's thread).

### Lane 20, 06:00 -- STORE_CHAMBER (fed ants at home eat floor food only as readily as it is piled), drained bed, EARLY to 100k
Main 192b7103 + switch, seeds 1-4, against the drained baseline (same binary, switch off).
- Live ants at 100k, store vs main: s1 133 vs 568 | s2 256 vs 149 | s3 47 vs 128 | s4 240 vs 138. Mixed, 2 up 2 down.
- **Food underground at 12.5k steps to 100k: 0-27 cells, same as main (0-20).** No store builds. The switch fires (175,593 scaled feed decisions on s1), so the floor food is being eaten by ants under their grant, which this leaves untouched.
- Running on to 300k.

### Lane 20, 07:30 -- STORE_CHAMBER final (300k): costs survival, builds no store. OFF.
Drained bed, main 192b7103, seeds 1-4, switch vs off (same binary):
- Live ants at 300k: 387 vs 632 | 887 vs 400 | **0 vs 426** | **0 vs 507**. Two colonies lost (s3 dead by 150k, s4 by 250k), none on main.
- Most food ever underground in 300k: 23/45/27/17 vs 28/33/20/20 cells. No store.
- Reading: refusing fed ants at home doesn't leave food lying; ants under their grant eat it, and the colony loses the banking it breeds on. Commit 4th after 6f440db1 on claude/nest-race-iit0pb, switch off.

### Lane 20, 08:40 -- FOOD_HANDLE (food put down at home can't be taken for N frames; harvester husking), first build leaked
Drained bed, main 192b7103 + switch. First build tracked each put-down by exact position: 97,886 refusals by 100k (s1) but food underground still 0-36 cells at every sample (h2000 and h6000, seeds 1/2), because provisions is a powder and slides out from under its record. Live ants at 100k: h2000 420/274, h6000 242/58 vs main 568/149. Rebuilt to hold any food within a cell of a fresh put-down; rerunning both handling times, seeds 1/2, to 300k.
Citations (Consensus quota spent, PubMed instead): Oliveras et al. 2008 doi 10.1007/s00114-008-0349-0 (Messor barbarus carries seeds home and discards ~69% of one plant's as too tough to open); Wu et al. 2022 doi 10.3390/insects13080691 (stored seeds kept from moulding).

### Lane 3, 01:00 -- drained bed: every earlier chamber rule gives one room; "two digging modes" is the first to grow tunnels
Page: https://claude.ai/artifact/2gZLRf6fseqZhgvSGQ6V7y. Lane 20's drained bed 6f440db1 (main 192b7103 + harness) + lane 3 scratch (`/mnt/project-files/nest-race/lane3/two-modes-scratch.patch`, off by default). nestgoal, seeds 1-4, 100k, no NEST_REST unless named.
- **One room, 8-33 rows deep, on every seed**: today, two crowds, FLAT_ROOM=4 + FRESH_CUT=aim (with and without NEST_REST), DIG_FACE=off (also with NEST_REST, 8 seeds), BROOD=off. Readout "second chambers" are 6-13-cell slivers, or (today s1, 494+191) the room split by a water seep beside the brood column -- not a room.
- **Puddle-bed findings that did not survive the drain:** DIG_FACE=off's separate second room (4 of 8 -> 1 of 8). Two crowds' halving of the colony was measured on the puddle bed too; on the drained bed without rest it is 140/154/73/119 vs 568/149/128/138.
- **BROOD=off** (control): no deeper (15-23 rows vs 21-33), still one room -- the brood column is not what holds the nest to one room.
- **`PIXEL_PHYSICS_DIG_MODES=on`** (scratch; Reports/nest-biology-digging-signals-2026-09-19.md §6, Römer & Roces 2014): beside brood or loose food a digger widens round it; elsewhere inside the nest a cut is refused unless at most 6 of the 24 cells round it are open (a tunnel tip). With `DIG_TIP=on` a refused digger turns to the most tip-like cut instead. **A room ringed by tunnels on 4 of 4 seeds** -- the first change of shape. Live ants 154/106/159/32 vs today 568/149/128/138.
- Missing: nothing is put down at the tunnel ends (brood laid as a column in the room, food eaten on arrival), so no room grows there. With BROOD_DEEP the brood fills the space as a layer instead.

### Lane 20, 09:30 -- FOOD_HANDLE (fixed, holds within a cell) still builds no store, EARLY to 75k
- Food underground at 12.5k steps: h2000 s1 18/9/7/4/0/2, s2 10/16/35/14/3/4; h6000 s1 14/18/22/1/0/1, s2 14/15/7/0/1/3. Same as main.
- Why it still leaks (new `GONE` line: what stands where an underground food cell was, the frame after): the cell is mostly **empty** (main s1: 4,218 of 4,917 at 25k; h6000 s1 1,614 of 2,164 at 50k), i.e. food cells move constantly (powder settling, parted and put back by walkers), so any record kept by position loses them. Liquid takes 5-20%.
- A sound version needs the delay to travel with the cell: a new material ("unhusked food", inedible, that turns into provisions over time via decay.rs). That touches the material registry, so it's a bigger change than a switch. Not doing it without a yes. 300k runs continue for survival numbers.
- (Lane 3, 01:20 addendum) DIG_MODES + DIG_TIP + `DIG_DOWN=1.0` (turn down everywhere): deeper tunnel mazes (20-31 rows vs 19-25), still one room, ants 94/214/47/144. Adding BROOD_CARRY + BROOD_DEEP crashed it: 6/46/2/16 ants. Widening fires ~440 cuts a run, all round the brood column and food in the central room. **Rooms grow only where contents lie; separate rooms now wait on brood/food being put down at tunnel ends (placement, lane 2).** Patch handed to lane 2.

### Lane 20, 11:00 -- FOOD_HANDLE final, 300k: no store, seed 2 dies at both handling times. OFF.
Live ants at 300k (seeds 1/2): h2000 637/**0**, h6000 515/**0**, main 632/400. Most food underground: 18/35, 22/15 vs 28/33. Commit after 7521123e. Off.

### Lane 2, 01:20 UTC -- ants walk on water (draft PR 603); FOOD_BRAKE kills on the drained bed
- **Mister control** (scenario setting rain 0, same binary, main 192b7103): seed 1 200k 0 -> 45, seed 2 200k 1 -> 334, NEST_REST seed 4 300k 0 -> 375, NEST_REST seed 1 300k 697. The puddle is causal.
- **Game-side fix, draft PR 603** (`PIXEL_PHYSICS_WATER_FOOTING`, on): liquid under a cell is footing, so a pool's top is a floor; liquid still not enterable, no climbing streams. Mister on: seed 1 200k 0 -> 412; NEST_REST seed 4 300k 0 -> 552; seed 2 200k 1 -> 3 (dip crossed, colony shrinking for another reason). Off bit-identical to main. Scott's card is in the laying thread.
- **FOOD_BRAKE=on, drained bed** (lane 20's nestgoal at 6f440db1, base reproduces lane 20's 95 ants at 50k): s1 holds 30-41 ants to 62.5k, peaks 192, **dead by 225k**; s3 holds 31-35 to 100k, peaks 96, **dead by 200k**. Base on the same binary: s1 632, s3 426 at 300k (lane 20, 04:10). The brake pins the colony near its 30-adult exemption while founders age out. Not a candidate here; no graded version pursued.

### Lane 2, 01:35 UTC -- BROOD_SPREAD (crowded brood pile sheds items 5-10 steps away, clear of food), on lane 3's two modes: column gone, still one room
Drained bed 6f440db1 + lane 3's two-modes patch + scratch `PIXEL_PHYSICS_BROOD_SPREAD=on`, DIG_MODES=on DIG_TIP=on, seeds 1/2, 100k. Rule: a larva or pupa with 6+ brood in the 24 cells round it, a free nestmate beside it, is carried (through passable cells, 5-10 steps) to an empty floored home cell with at most 4 brood round it and no loose food within 3; the spot with the most brood round it wins, then the farthest.
- Fired: 300 / 591 moves. Live ants 164 / 309 vs base 154 / 106 -- no cost.
- **The brood column is gone** (Scott's 22:52 complaint): brood lies spread over the room floor on both seeds instead of a 1-wide pink column down the shaft. Brood underground 65 / 148 (32 / 93 in the readout's chamber) vs base 46 / 86.
- **Still one room**, separation 0.00 on both: shed items join the edge of their own pile (a cell 5 steps out with 4 brood round it wins), so the room widens and flattens rather than a second one starting. Food underground 5 / 4 cells; food is not stored below on this bed, so there is nothing to keep brood away from yet.
- Next: the same rule with the shed carried 10-20 steps (`PIXEL_PHYSICS_SPREAD_FAR=10,20`), so a second pile can start at a tunnel end. Running seeds 1/2.

### Lane 2, 01:55 UTC -- BROOD_SPREAD farther / beyond a tunnel / with DIG_DOWN: still one room every time
Page: https://claude.ai/artifact/KFWviTPXQD3XFhUJhdeBTu. Same bed and patch stack as 01:35, seeds 1/2, 100k. Ants s1/s2, brood moves, readout rooms (cells):
- Base two modes: 154/106, rooms 111+10 / 169+6+6.
- Shed 5-10: 164/309, 300/591 moves, rooms 112 / 165. Column gone.
- Shed 10-20 (`SPREAD_FAR=10,20`): 99/139, 192/125 moves, rooms 131+12 / 149+7. Brood goes to the room's two wings; they widen.
- Only beyond a tunnel (`SPREAD_PAST=8`: every shortest way to the spot passes a cell with at most 8 of 24 open): 128/231, 500/675 moves, rooms 112+29 / 156. Brood goes into the tunnel maze under the room; the widening there joins it to the room.
- Beyond a tunnel + `DIG_DOWN=1.0`: 87/131, 3/290 moves, rooms 75 / 39+11+8; s1 nearly brood-less (2 underground).
- Reading: placement alone can't separate rooms on this nest -- every tunnel a shed item can reach ends within a few cells of the room, and two modes widens round the brood until it meets the room. A second room needs a tunnel leading well away first (lane 3's shape), and then `SPREAD_PAST` puts brood at its far end. Patch (on top of lane 3's): /mnt/project-files/nest-race/lane2/brood-spread-scratch.patch. Recommend the plain 5-10 shed as the column fix.

### Lane 20, 12:30 -- goal-bed baseline re-stamped on main 0b3e264a (drained bed, seeds 1-4, 300k): identical to 192b7103
Every line matches the 192b7103 run exactly (live ants at 300k 632/400/426/507; most food underground 28/33/20/20; one chamber holding all brood, 147-385 cells). Expected: PR 602 lets home pass living plant tissue in dug tunnels, and the goal box has no plants (lane 3 said so at 22:30). So the 04:10 drained-bed numbers stand as the reference for 0b3e264a. Logs: /home/claude/runs/bed3/ on lane 20's box; branch claude/nest-race-iit0pb at f13b684c (main merged).

### Lane 2, 02:45 UTC -- BROOD_SPREAD on plain main: ends the column, but larvae out of the crowd get fed less; NOT shipped
Plain main 0b3e264a (lane 20's bed f13b684c), no two modes, same binary on/off, 200k. Ants at 200k (births):
- Goal box (drained), 5-10 step shed: s1 396 (1798) vs off 384 (2909); s2 377 (1174) vs 345 (1124). Column on main at 50k, gone with the rule.
- Food box (`digbox ants=20 food=60 hungry`), 5-10: live ants 16 / 339 / 0 vs off 230 / 354 / 128 [corrected 04:40 UTC: first posted as 352 / 158 / 525 vs 480 / 376 / 438, which are dug cells (digbox's SCORE n), not ants]; births 328 / 1191 / 461 vs 736 / 1402 / 798; larvae starved 44 / 327 / 368 vs 60 / 71 / 103.
- Cause: food passed mouth-to-mouth into larvae 327k -> 59k J (s1), 365k -> 163k (s3); nursing by touch likewise. Spread larvae lie out of the traffic, so nestmates brush them less. Food distance is not it: the same rule with no food check is bit-identical on the food box (the check never decided anything there).
- Short carry 2-5 steps: food box 254 / 419 / 440 (births 562 / 235 / 231); goal box s1 **40** vs 384.
- Reading: spreading brood and feeding it are coupled here -- larvae are fed by whoever happens to brush them, so brood away from the crowd goes hungry. A spread that does no harm needs nurses that go to larvae (the backlog's "nursing via larva scent"). Rule parked; PR-ready draft (doc, tests watched red) at /mnt/project-files/nest-race/lane2/brood-spread-pr-draft.patch.

## Lane 20, 2026-10-04: angle = rooms widen only where ants crowd (CROWD_DIG)
Scott chose crops as the food store, so lane 20 switches to separate chambers. Untried angle, distinct from lane 3's two modes (widen beside brood/food) and lane 2's brood shedding: **density-dependent widening**. `PIXEL_PHYSICS_CROWD_DIG=<k>` (off by default, branch claude/nest-race-iit0pb): underground, a cut that would widen a room (3+ open neighbours) is refused unless k or more ants stand within 3 cells; tunnel-tip cuts are always allowed. Idea: as a room fills, the crowd spills into tunnels and the next crowd forms further along, so a new room starts down a tunnel instead of the old one swelling (Rasse & Deneubourg 2001; Buhl et al. 2004 doi 10.1007/s00114-004-0577-x; Tschinkel 2004 J Insect Sci 4:21, chambers strung along shafts). Running k=3 and k=5, seeds 1-4, 300k, drained bed, on main 0b3e264a. Counter `crowd refused` on the SITES line. Results here as they come.

### Lane 3, 02:55 UTC -- long tunnels out of the room: five rules tried, none gets past ~15 steps yet (EARLY)
Branch claude/nest-building-8j7uwn (dcafc8cc + uncommitted scratch), drained bed, DIG_MODES=on DIG_TIP=on under every arm, seeds 1/2, 100k. New readout line `TUNNELS`: steps along the shortest open way from the largest chamber -- reach (farthest cell), cells 10+ out, tips 10+ out, and how far out each other chamber starts. Selftest: a room 14 cells down a passage reads 15 out.
- Base two modes: reach 18 / 13, cells 10+ out 29 / 7. On the map that is a fringe of ~15 stubs, 3-6 cells each, round one room -- no long tunnel.
- `DIG_AHEAD=4` (no cut toward open space within 4 cells ahead): reach 10 / 18. `DIG_STRAIGHT=on` (no side branches): 9 / 6. Both: 10 / 15. `FRESH_CUT=recruit` (+both): 2-16. `DIG_NARROW=8` (advance only from inside a tunnel): 9 / 13, with AHEAD 7 / 16. Still the fringe on every map.
- **Trace of every cut (seed 1, 878 cuts):** 282 different ants dug, a median of 3 cuts each; of 432 advancing cuts that followed the same ant's previous cut, only 92 were within 2 cells of it (median 8 cells away). Most refused diggers stood where 8-20 of the 24 cells round them were open, i.e. in the room. So no tunnel is carried on by the ant that started it, and almost nobody stands at a tunnel's end.
- Cause of the low fidelity: the shipped walk back to the face is cancelled for any ant with food in its crop, and on this bed nearly every ant carries some. `DIG_STAY=on` (keep the walk back, stay at the face until the next cut) restores it (median gap 2 cells, 60% within 2) but the faithful diggers' faces are round the room's contents, so the work went into widening (646 of 980 cuts) and tunnels got no longer; ants 137 / 86 vs 154 / 106.
- Running now: stay only at a tunnel's end (`DIG_STAY=tip`) together with `DIG_NARROW=8`, with and without AHEAD.
- Lane 2's `SPREAD_PAST` patch builds on this branch and is ready to go on top once a tunnel runs 10+ out.

### Lane 20 CROWD_DIG result (main 0b3e264a + 54841a2b, drained bed, 300k, seeds 1-4) -- DROPPED
Live ants at 300k, ever dug: main 632/400/426/507, 2820/1481/1306/1395. k=3: 579/459/**0**/257, 1622/1073/497/506. k=5: 162/611/346/**0**, 810/2182/1028/387. Counter fires (k=5 s2: 135k refusals). Still ONE brood room every run; the only "second chambers" are 20-56-cell side pockets with no brood, which main makes too (s2, s4). Refusing room cuts just means less digging, and 2 of 8 colonies died. Reading: widening is not what joins rooms; everything gets put down round the one brood pile, so the room is wherever the brood is.
- **Census trap (lane 20):** on main, CHAMBERS often lists a second room of 80-190 cells with 20-40 ants and no brood (s1 100k, 250k, 262.5k; s2 212.5k; s4 300k). The pictures show it is the far wing of the same room, cut off by the falling egg column. A second chamber counts only if a picture shows a passage between the two rooms.

### Lane 2, 04:20 UTC -- WATER_FOOTING kills colonies on the drained bed; PR 603 reworked to step-only, shipped OFF
Same binary on/off, main 0b3e264a (bed f13b684c), 200k, ants alive. Off bit-identical to main.
- Drained, off, seeds 1-5: 384 / 345 / 253 / 71 / 593.
- Both halves (PR 603 as first pushed), seeds 1-3: 0 / 0 / 6. Fall half alone: s1 598 -> 94, s3 dead.
- Step half alone: 532 / **0** / 337 / **0** / **0**. With the drain sparing water under standing ants (scratch drain=2), s1/2/4/5: 0 / 201 / 0 / 0 -- not the drain.
- Undrained (puddle bed), step half: s1/s2 0 / 1 -> 333 / 490; s3 548 (no off arm).
- Trace: s4 steps onto water 10-50 per 2,500 frames, almost all on the mound and in the door; the door sealed by packed soil at 39.5-40k with the colony outside. s2: nest empties at 75-87k, fat ants sit on the mound over the sealed door and never lay again. This is the "rich wanderers who stop laying" case: they are locked out, not lost.
- PR 603 head 68f251f7: fall half removed, step half default off, dead-ends entry. The lab's puddle moat stays unsolved in the game; the goal bed keeps draining.

### Lane 3, 03:10 UTC -- long tunnels + brood beyond them: separate brood rooms appear, then grow into one. Nothing ships.
Page: https://claude.ai/artifact/LiADsAkaK9Cc9vHgXV6wA8. Branch claude/nest-building-8j7uwn at 69295d61 (all switches off; off reproduces the bed: s1 568 ants at 100k). Two modes under every arm, seeds 1/2.
- Correction to 02:55: "nearly every ant carries food in its crop" was inferred, not counted. What was measured: lifting the crop gate on the walk back took advancing cuts within 2 cells of the same ant's last cut from 21% to 40-60%.
- `WIDEN_ON=brood` (loose food no longer draws widening): reach 6 / 12 at 100k vs 18 / 13. One room. Not the lever.
- Lane 2's spread (`BROOD_SPREAD=on SPREAD_FAR=10,20 SPREAD_PAST=8`) on two modes, to 150k: s1 at 100k has four brood rooms (101/46/11/10 cells, the 11 one 17 steps out); by 125k one 178-cell room. s2 one room to 125k, three at 150k (81/14/13, 2 steps apart). They widen round their brood until they meet.
- `WIDEN_AHEAD=<r>` (a widening cut refused with open space within r ahead, so rooms stop short of each other) + spread: r=3 one small room (31 / 63 cells at 150k), most brood outside it (11 of 66, 26 of 85); r=5 colonies crash (22 / 0 ants at 150k).
- With narrow tips (`DIG_MODES=4`) + widen only round brood + spread: one room (53 / 133).
- Reading (inferred): the cut rules aren't the lever, and nothing stops a room growing, so brood rooms that start apart join. Agrees with lane 20's CROWD_DIG result.

### Lane 2, 04:10 UTC 10-04 -- door locked by soil heaped over it; a waiver that reopens it (main 0b3e264a, goal bed drained, 200k)
- **Cause** (seed 4 with walking on water, cell grid every 50 frames + every dig try at the mouth): loose soil slides into the mouth one row ABOVE the founding surface (38.6k) and is packed by a cut beside it. All 63 cuts tried at it from inside were vetoed by the heap cue at chance 0.00 -- the heap over the mouth is soil (spoil that lost its footing), not pellets -- and the door-reopen waiver covers only the founding cut, from the surface down. Colony locked outside; no laying (nest-only); starves.
- **Fix (scratch)**: `PIXEL_PHYSICS_DOOR_HEAP=8` -- a cut in the shaft's columns up to 8 rows above the mouth counts as reopening the door. Patch on main: /mnt/project-files/nest-race/lane2/door-heap-reopen.patch (type-checks; nest lane's code).
- **With walking on water** (step half), ants alive at 200k, without -> with the waiver: s2 0->406, s4 0->292, s5 0->240, s8 0->619, s10 5->322, s11 0->335. **6 of 6 rescued.**
- **Without walking on water**: s1 384->0, s3 253->610, s4 71->0, s5 593->285, s6 38->284, s7 295->1055, s9 11->11. **Mixed**: median 253 -> 284, collapses 1 -> 3 of 7. s1's births stop dead at 50k (170) -- tracing.
- Walking on water alone (seeds 6-11 added): 6 of 11 drained colonies collapse vs 1 of 11 without; median 67 vs 345. PR 603 ships it off.
- **04:25 update**: walking on water + DOOR_HEAP=8, 8 of 8 alive at 200k (s1 595, s2 406, s3 337, s4 292, s5 240, s8 619, s10 322, s11 335; plain main on the same seeds 384/345/253/71/593/842/607/420). DOOR_HEAP alone killed s1 by a second lock-out: the mound over the mouth grows ~6 rows, the passage through it plugs 3 rows above ground at 48-50k, and only 4 cuts were even tried near it in 8k frames (surface ants never dig down; the inside ants had left). Both lock-outs = a locked-out ant has no way to dig back in. Sent to the nest lane.

### Lane 2, 05:15 UTC 10-04 -- brood spreading + larva-scent nurses: both cost colonies, shipped OFF as switches (draft PR 604)
Same binary every arm, main 0b3e264a, off bit-identical to main; ants alive at 200k, dead = under 50. Chart https://claude.ai/artifact/YPXa7NPbZmoREaRvcqdVA3
- Food box s1-9: plain 1 dead (median 354); spreading 4 dead; spreading + nest-worker nurses 4 dead; nurses alone 1 dead.
- Goal box s1-6: plain 1 dead (s6 38); spreading + nurses 3 dead; nurses alone 2 dead; spreading alone 396/377/436/0/0/311 (2 dead).
- Nurses barely fire: of walking decisions (food s1, 80k) 54% carry crop food, 14% a pellet, 6% unfed, 23% foragers; pull reaches 0.14% (workers) / 1.2% (all). `all` cuts eggs (6/6 paired seeds).
- Spread larvae starve more (10/18/43% of eggs vs 6/4/10%). Goal-box nest stays ONE room in every arm.
- Reading: nursing only moves energy between laying and larvae. Re-test once crop food reaches the brood.
- Switches: PIXEL_PHYSICS_BROOD_SPREAD=on, PIXEL_PHYSICS_NURSE_SEEK=on|all.

### Lane 20, 05:40 UTC -- next angle: stop rooms growing into each other
The brood-free "pocket" lead is dead (it is the room's far wing behind the egg column, see census trap above). New angle, per lane 3's 03:10 finding that brood rooms start apart and merge: rooms widen only where ants crowd (CROWD_DIG=3), which should let a room stop once its crowd thins, layered on lane 3's two modes + lane 2's far brood spread. Scratch worktree = lane 3 e7f7571a + PR 604 branch + 54841a2b + SPREAD_FAR env. Arms A (DIG_MODES=on BROOD_SPREAD=on SPREAD_FAR=10,20) vs B (A + CROWD_DIG=3), seeds 1-4, 150k. Judging by pictures, paired by seed.

### Lane 3, 05:45 UTC 10-04 -- door sealing: dig-back doesn't win over 12 seeds; the heap is built on top of the door (branch claude/nest-building-8j7uwn, all switches off)
- **Dig-back + lane 2's waiver** (`DOOR_DIG=on DOOR_HEAP=8`), 7 seeds 200k vs today's default: median 253 -> 337, but s5 593 -> 7. Traced: the colony moved into tunnels in its own heap over the door (~15 rows tall, 45 wide; ants under a heap read as inside the nest), mouth packed shut below, brood unfed, births stop.
- **Dig-back v2** (`DOOR_DIG=heap`: ants under the heap too, 24 rows, no waiver), 12 seeds 200k, same binary: door shut 58 -> 39 of 192 samples, under 100 ants 3 -> 2, **median 365 -> 253**. Not a win; off. (Base wins on healthy seeds and loses on dying ones -- regression to the mean is expected from any arm, so read the distribution.)
- **What builds the plug** (default arm, temporary line at every cut and pellet, seeds 1/5, 130k): heap over the nest is mostly packed + loose soil, spoil a small part (s1 105k: 392/178/35) -- pellets that lose footing fall as soil; 22-43% of ALL cuts are ants re-cutting that heap, each tamping loose soil round it. s1: 433 of ~3,700 above-ground pellets landed within 2 columns of the mouth, 14-26 rows up -- the heap is built ON the door. Hungry ants drop pellets in the door in bursts while it is shut (18-19 per run default, 51-96 with the door rules).
- **Crumbs dug as dirt**: 7-18% of all cuts take a food cell (s1 530/6,822, s5 341/1,864); crumbs (resistance 0.2) pass `jaw_can_cut` and go out with the spoil -- the crumbs Scott sees stuck in the patchwork. Buried crumbs at any moment are few (0-9).
- **Now testing prevention** (`DOOR_CLEAR=drop` no pellet set down over the door; `tamp` passage not tamped; `both`), seeds 1/2/5/9 to 120k vs default. Also built, untested: `LEAN_DOOR=keep`.
- **Result (06:30):** ants alive at 150k, A spread-only 33/67/153/6, B +CROWD_DIG=3 64/57/21/245. The census counts 2-3 brood "rooms" in B s1 (59/33/11 cells) and s4 (108/20), but the pictures show one shallow room cluttered with brood and leftover soil pillars, not rooms joined by tunnels. A merges as lane 3 found. Not a win. Both arms are weak colonies, because spreading costs larvae (lane 2). Pictures: /mnt/project-files/nest-race/lane20/.

### Lane 20, 06:50 UTC -- depth readout (nestgoal DEPTH line, main-equivalent f13b684c, drained bed, 200k)
Deepest open row under ground at 200k: 44/28/30/26 (s1-4). The room is rows 6-20 (s1: 269/252/177 open cells per 5-row band). Below it, the shaft is mostly brood: s1 rows 21-45 hold 235 open cells, 175 of them brood; s2-s4 rows 21-30: 69/58/25 open, 31/39/18 brood. So the "8-33 rows deep" rooms are a room plus a shaft plugged with fallen eggs. The only way down is full of brood. Next (scratch, lane 2's material): eggs that don't fall (brood kind Solid), 2 seeds, to see whether the shaft stays open and a lower room forms.
- **Sticky eggs result (07:40, scratch brood self_supporting, 200k, s1-4):** the nest got SHALLOWER, not deeper. Deepest row 14/20/21/12 vs main 44/28/30/26; one room, rows 6-15, brood clumped at the shaft top. Ants alive 313/144/398/234 vs main 384/345/253/71 (all alive both arms). Reading: **the falling egg column is the only thing that pulls digging downward** (ants dig beside brood, and the eggs lead them down the shaft). Nothing in the engine digs down for its own sake. s3 sticky held 47 food cells underground at 200k (one seed, not yet a finding).
- **Sticky eggs + crowded brood carried to the lowest floor in reach (08:50; scratch SPREAD_DOWN on combo worktree = lane 3 e7f7571a + PR 604 branch, SPREAD_FAR=3,20, 200k, s1-4):** the nest deepens again (deepest 14/26/30/24 vs sticky 14/20/21/12), and ants alive 320/182/668/434 vs sticky 313/144/398/234 (higher 4/4, but a different base build, so not clean). Still ONE room: a bowl 233-516 cells with brood through it (picture lane20/brood-down-s3-200k.png). Brood put down lower is dug round and the room grows down to it. Same wall as every other arm: nothing leaves ground standing between two piles.
- **Lane 2 on sticky eggs (05:55 UTC 10-04):** read. The two results above say sticky eggs make the nest shallower, so there is no brood-falling change for lane 2 to land in brood code yet; if a later arm wins, send the patch and lane 2 lands it. Lane 2 now: `PIXEL_PHYSICS_CROP_NURSE` (on PR 604, off by default while measured): a carrier touching a hungry larva feeds it from its crop, and with `on` carriers inside the nest follow larva scent. It changes how larvae are fed, not where brood lies.
- **+ lane 3 two modes (09:45):** one room on all 4 seeds, and 2 colonies dead by 200k (s3 3, s4 1 ant; ants in chambers 0, so check for a door plug). Dropped.
- **Next (lane 20, running): rooms space themselves out.** Scratch `ROOM_SPACING=<core>,<reach>` with DIG_MODES: an advancing cut that would be refused for widening goes ahead when no room cell (7 of the 3x3 open) lies between core and reach of it. A new room grows to about core across and stops; the next starts only reach away, down a tunnel. Lateral inhibition between building sites (Theraulaz & Bonabeau 1995; Khuong 2016); chambers spaced down shafts (Tschinkel 2004). Arms: 4,12 + far/down brood spread, sticky vs falling eggs, seeds 1-2, 200k.
- **ROOM_SPACING=4,12 never fired:** sticky arm bit-identical to the same build without it (s1/s2), because the whole nest sits within 12 cells of its room, so no tip is ever far enough away. Falling-egg arm: both colonies dead by 200k with 0 ants in chambers (looks like the door lock-out, not judged). Re-running at 3,7, sticky, seeds 1-4.
- **ROOM_SPACING=3,7 (sticky, s1-4, 200k):** fires on s1/s3/s4 (s2 bit-identical, never fired). Census shows 2-3 brood rooms held apart on s1 (94/56 at 150k) and s4 (146/59 at 200k), but the pictures show one wide room broken up by brood and leftover soil, as before. Colonies: 0/101/0/20 at 200k, with 0 ants in chambers from 100k on s1/s3 (lock-out or exodus, not traced). Dropped.
- **Lane 20 summary, 10-04 morning:** 6 rule families this lane (crowd-limited widening, alone and stacked; sticky eggs; brood carried down; + narrow tunnels; room spacing) give one room every time. The one solid finding: **falling eggs are what dig the nest deep.** Switching from guessing rules to tracing: every cut that joins two brood rooms, who made it and why (CLAUDE.md "trace individuals").

### Lane 20, 11:20 UTC -- merge trace: brood "rooms" join without a single cut
Combo build (lane 3 two modes + lane 2 far spread SPREAD_FAR=10,20, falling eggs), seeds 1-4, 150k; every cut logged (DIGC + brood 5x5 + laden), chambers compared every 250 frames. Of 124 merges of two brood rooms (each 8+ cells, 3+ brood), 17 have no new bridge cells and **99 of the 107 with a bridge had no cut within 1 cell of it in those 250 frames** (8 did: all widening cuts beside brood, by spoil-carrying ants). Rooms also split and rejoin repeatedly (s1: 32 merges, mostly the same 8-35-cell pocket 5-10 columns off the main room). Reading: the "separate brood rooms" seen by lane 3, lane 2 and me were never walled apart. They are one room divided by loose material, which moves away. Checking now what filled the bridge cells.
- **What filled the bridges (s1, s3, rerun to 110k with the cell's material at the previous look):** water 46 (38 in dug cells), already-open 37 (a neighbour was closed), brood 5, packed soil 5, ant 5, soil 2. **The brood rooms are split by mister water pooling on the nest floor**, which the census counts as wall (and ants cannot cross). When the puddle soaks away or moves, the "rooms" rejoin. Nearly every multi-chamber count on the drained bed (mine, lane 3's four brood rooms at 100k, lane 2's) needs re-reading in that light: the drain only clears the surface, not the nest. Lane 18's floor-drain card (waiting on Scott) removes this water at the source.

### Lane 20, 13:10 UTC -- dry goal bed (mister off, Scott's call) -- EARLY, control running
Branch claude/nest-race-iit0pb d358b527 = main 6964deb4 + goal bed with rain 0 and drain off by default. Seeds 1-4, 200k: ants alive 45/334/**0**/4; ever dug 553/308/282/318; deepest 24/18/18/19 rows; one room 170-243 cells. Against yesterday's wet bed on f13b684c (not the same main): alive 384/345/253/71, ever dug 2528/924/1002/876. **Digging falls to a third and 3 of 4 colonies collapse.** Same-binary mister-on control (PIXEL_PHYSICS_LAB_SCENARIOS=dir with rain 1, drain=1) running now to split "dry soil" from "today's main". Note: lab scenarios are read from assets/ at run time, so editing nest_goal.ron changes every run started from that tree, even with an old binary.
- **Control (same binary d358b527, mister Light via scenario dir, drain=1):** alive at 200k 384/345/253/71, ever dug 2528/924/1002/876, deepest 44/28/30/26 -- identical to yesterday's wet bed, so today's main changed nothing here. **Mister off is the cause**: digging a third (553/308/282/318), colonies 45/334/0/4, deaths mostly starvation. Why the dry box starves is not traced yet. Recommend lanes stay on the wet drained bed (drain=1, mister on) until Scott decides; asked him.
- **Lane 2, 07:08 UTC 10-04 -- crop feeding ON in PR 604 (head 650cc6208), and a dry-bed control.** `PIXEL_PHYSICS_CROP_NURSE=touch` (default): a fed carrier touching a hungry larva feeds it from its crop before any bank. Neutral over 20 paired seeds (better 10, worse 9, 6 dead each arm); it rarely fires because carriers are seldom at the brood (ant touching a hungry larva has an empty crop 64-83% of ticks). Chart https://claude.ai/artifact/UHKWPnACtsiRCVsRtf9ZAp. **Dry goal bed, today's game** (main 99e0be4f, scenario copy with `(subject: "the bed", field: "rain", value: 0)`, nestgoal drain=1, 200k, s1-6): live ants 45/334/0/4/0/75 -- 4 of 6 under 50. Same binary with crop feeding: 0/388/206/474/0/114. Compare against your dry baseline when it lands.
- **Dry-bed trace (lane 20, 14:10 UTC; DOOR line on my nestgoal: open path from nest to 20 rows over ground + where ants stand):** dry s3/s4 the mouth seals at 75k/100k and **never reopens**; ants under ground fall to 0, all on the mound/surface, births stop, starvation. Wet s3/s4 seal at 100k too but **reopen by 125k** (inferred: packed soil reverts to loose when waterlogged, so rain softens the plug). Mister off unmasks lane 2/3's door lock-out. Testing lane 3's DOOR_HEAP=8 + DOOR_DIG=heap (their branch 20759c93, rain=0) on dry s1/s3/s4, 200k.
- **Lane 2, 07:20 UTC 10-04 -- walking over puddles (WATER_FOOTING) on top of lane 3's unpacked doorway (DOOR_CLEAR=tamp), lane 3 head 665a5b142, goal bed mister ON, drain=1, 200k, s1-6.** Live ants, tamp alone 357/401/373/157/701/475; tamp + WATER_FOOTING 123/207/449/444/369/507. **0 of 6 died in either arm** (without the door fix WATER_FOOTING lost 6 of 11 vs 1 of 11). Paired 3 better / 3 worse. Door at 200k: shut on s6 in both arms only. Played bed (labforage played_bed, 120k, s1-4, same arms): births 50->48, intake 292k->278k, ant-frames 3.5M->3.1M (3 of 4 lower), starved 0->4 raw, colonies lost 1->0; no sign test under p 0.25. Running played-bed s5-12 to settle. If it holds, the WATER_FOOTING default flip should ship with tamp ON, on lane 3's branch.
- **Door fixes on the dry bed (lane 3 branch 20759c93, rain=0, drain=0, seeds 1/3/4, alive at 100k/150k/200k):** off 131/151/45, 75/3/0, 51/7/4 (door shut from 75-100k for good). DOOR_HEAP=8 + DOOR_DIG=heap: 28/25/1, 87/103/77, 110/25/70 (door flickers). **DOOR_CLEAR=tamp: 203/185/304, 251/138/339, 195/228/244**, door open in 22 of 24 looks, ever dug 874/1243/649 vs 553/282/318, room 410-587 cells with 78-127 brood and 85-147 ants in it. Clear direction, so per Scott (07:13) no mister on/off sweep: **the goal bed stays dry, and the unpacked doorway is the fix**. Lanes on the dry bed should run with PIXEL_PHYSICS_DOOR_CLEAR=tamp until it ships.
- **Lane 2, 08:20 UTC 10-04 -- WATER_FOOTING settled: safe with DOOR_CLEAR=tamp, patch handed to lane 3.** Same arms as 07:20 (lane 3 branch, tamp both arms). Played bed s1-12: births/intake higher on 7 of 12, colonies lost 2 vs 2, starved/M 0.3 -> 1.0 (p 0.55): no measured harm. Dry goal bed (rain=0) s1-3: bit-identical 304/875/339 (no water to stand on). Default-on patch (one commit on 20759c930, docs + wiki): /mnt/project-files/water-footing/water-footing-default-on.patch, to ship only alongside tamp default ON. Chart https://claude.ai/artifact/F5ue2fYxAtLd92se9ujp7a.
- **Lane 3, 08:25 UTC 10-04 -- unpacked doorway is draft PR 608, ON by default; the hollow mound traced.** PR https://github.com/sgladstein/Pixel_Physics/pull/608, branch claude/nest-unpacked-door, head 41b2db86 on main f894bb22. Switch name on main will be PIXEL_PHYSICS_DOOR_LOOSE (on; `=off` restores tamping) -- same rule as scratch DOOR_CLEAR=tamp. Hollow mound (Scott 06:53): ants make 6-39% of all cuts into their own spoil mound and 54-218 live in it; inside it they read as underground, and each cut tamps it into packed wall. Scratch MOUND=nodig (lane3-scratch, bundle /mnt/project-files/nest-race/lane3/lane3-scratch-mound-2026-10-04.bundle): solid cone with a vent on 4/4 dry seeds, holes in mound 69/162/132/4 -> 25/14/24/16, but ants at 120k 256/380/292/674 -> 164/152/208/382 (births lower 4/4). Picture /mnt/project-files/nest-race/lane3/mound-dry-120k.png. Decision card to Scott; 200k on dry seeds 3/4/6/7 running.

### Lane 20, 15:00 UTC -- dry goal bed baseline on main c55cd60e (DOOR_LOOSE on), 300k, seeds 1-4
Alive 408/383/301/245 (all four live; 100k 500/161/144/193). Ever dug 2980/1546/2190/701, deepest 47/33/39/22 rows. ONE room every seed (903/570/747/274 cells, brood 263/111/182/89 in it). Most ants live in or on the spoil mound (203/189/234/182 at 300k) against 68/71/16/36 under ground, as lane 3 found. Egg column still stands in the shaft. **This is the base to beat for chambers.** (Census counts liquid as open since de453e25; DOOR line's "sealed" reads 20 rows over ground and can say NO while mound tunnels still connect.)
- **Next angle (lane 20, running):** brood sorted by stage into two depths. Scratch `PIXEL_PHYSICS_PUPA_UP=<reach>` (brood.rs, lane 2's file, scratch only): when a larva pupates, a touching nestmate carries the new pupa to the highest floored home cell within reach, at least 4 rows up. Eggs keep falling to the shaft bottom, so brood lies at two depths, which should give digging two places to widen. Ants keep each brood stage at its preferred temperature and moisture, with pupae nearest the warm surface (Porter & Tschinkel 1993; Penick & Tschinkel 2008). c55cd60e + switch, reach 20, seeds 1-4, 300k.
- **PUPA_UP=20 result, EARLY (c55cd60e, dry, seeds 1-4, 300k):** still ONE room (522/510/408/316 cells), so not the chamber fix. But **colonies bigger at 300k on 4 of 4**: 429/584/503/480 vs 408/383/301/245, and higher at 200-300k on every seed (s2 735 peak). The brood column becomes a broad pile in the room (pictures lane20/pupa-up-s2-300k.png vs main-c55-s2-300k.png). Mechanism not traced; births mixed (s1 fewer, s2/s4 more). Running seeds 5-12 both arms to settle it.
- **PUPA_UP=20, 12 seeds (c55cd60e dry, 300k; off arm = same binary, switch unset):** alive main 408/383/301/245/0/504/700/465/1146/337/779/265, pupa 429/584/503/480/712/1/522/305/421/868/500/818. Better 7 of 12, worse 5; median 395 -> 502; collapses 1 vs 1; spread narrower (pupa 305-868 bar the collapse, main 245-1146). **Not the chamber fix (one room every seed), and only a weak colony lean.** Parked as a scratch in my tree (brood.rs is lane 2's): offering it to lane 2 as an off switch if they want it.
- **PUPA_UP=40 + LIFT=12 rows (s1-4, 300k):** fires rarely (brood carried 103/54/50/4 against 1.8-3.1k pupae: little floor 12 rows above a new pupa), one room every seed, s1 collapsed (8 ants). Dropped. Lane 20 out of new chamber angles that fit in a switch; summary to Scott with a pause card.
- **Lane 2, 09:10 UTC 10-04 -- WATER_FOOTING default ON as draft PR 609 on main c55cd60e (DOOR_LOOSE merged).** Re-measured on main (the nest-branch numbers did not carry over): goal bed misted s1-6 461/176/701/313/508/394 -> 165/602/723/299/368/437, 0/6 dead both arms, door shut 14 -> 13 of 48 looks; dry s1 identical (405). Played bed s1-12: births lower on 9 of 12 (p 0.15), colonies lost 3 -> 1; over 24 seeds of both builds births higher 10, lower 14. Chart https://claude.ai/artifact/F5ue2fYxAtLd92se9ujp7a.
- **Lane 2, 15:13 UTC 10-04 -- PR 609 (WATER_FOOTING on) rechecked on main 3f3aa06d (narrow trail), handed to the desk.** Played bed 24 seeds: births 12 higher / 12 lower, food eaten 13/11, ants alive at the end 11/13, colonies died out 2 -> 5. Seeds 1-12 alone read as harm (alive lower 11/12, p 0.006) and seeds 13-24 reversed it (p 0.04): a 12-seed end count on this bed misleads either way. Water steps 200-1,000 per run, mostly onto water lying on plants; not falling mist ("aloft" in labforage is food, not ants). Goal box dry: no change (no water). CI 15/15 on head d97f39ec6. Chart v3 https://claude.ai/artifact/F5ue2fYxAtLd92se9ujp7a.
- **Lane 20, 22:xx UTC 10-04 -- EVOLVED FOUNDER rechecks, goal box dry, mutation off, main cdfff228 + Laying's six founder rows (local scenario copy), seeds 1-4, 300k. Mean live ants 100-300k:** base 436/461/367/399 (mean 416); PUPA_UP=20 419/386/436/409 (413, 2 better 2 worse: **neutral, drop**); LEAN_FORAGE=off 23/176/276/243, 3 of 4 dead at 300k (**on stays, strongly**); sticky eggs 97/424/492/466 (s1 died, 3 better: mixed); sticky + brood down 414/395/426/380 (395, 1 better 3 worse: neutral-to-slightly-worse). Rooms: one main room every arm every seed; sticky+down shows 2-3 rooms at the median but the extras are brood pockets under 30 cells beside a 400+ cell room (a second room >=30 cells in 3 of 84 looks, all s2). **None of these makes separate chambers; none beats base on colony.** Logs /home/claude/runs/evo (lane 20 container).

## 23:xx 10-05 — Storeroom re-trace (lane 20, deep-trace method). Page https://claude.ai/artifact/GDaaTmozYdzAaQ1tAdc7nc, write-up storeroom/02-storeroom-retrace-2026-10-05.md
- Main 043e9104, evolved, mutation off, s1-4 300k: every storeroom arm lives (main 547/551/529/573; off 457/486/542/517; keep, chamber neutral). Food in nest 0-1 mean. Mound meals >99% by fed ants. Nest workers' pick-up blocked "away from home" 2,600 vs 35 reaching the draw (s1 60k).
- Old 10-04 deaths reproduced on 6edeba8d: births stop, home food 0 in one interval, starvation burst 81-104, then old age; pictures show shaft plugged (lockout, inferred), since fixed.
- Carry-in probe (STORE_IN=20, scratch patch storeroom/store-in-probe.patch): carried 39-72 -> 1,425-1,848/run, colony unchanged, food in nest still ≤1 mean. Drop log s1,s3 to 150k: 78% of loads let go ABOVE ground by carriers out of homing patience; rest put within 15 cells of door on bare floor and gone by next census. Room arm no deeper.
- Ants were never mostly inside: nest 4-18% at 10k; at 16-20k ants on top move into the spoil mound (0 -> 25-35%) and cuts follow.
- Rec: no carry-in ship yet; homing + stay-in first (probe = test), then depth cue, then surplus/not-edible-at-will.

## 23:3x 10-05 — Lane 20: goal-box rerun on main b081040e (evolved ant every box, 629 nest switches on, nurses off). Dry goal box, mutation off, seeds 1-4, 300k, `nestgoal food=120` (local copy of the lane's tool; scratch counters zeroed)
| seed | ants mean 100-300k (min) | births | starved 0-300k (100-300k) | rooms | underground % of ants, 100-300k mean (range) |
|---|---|---|---|---|---|
| 1 | 548 (496) | 3,749 | 41 (11) | 1 (906 cells) | 6.4 (2.2-12.8) |
| 2 | 550 (518) | 3,726 | 86 (7) | 1 (1,018) | 4.2 (1.6-7.2) |
| 3 | 527 (474) | 3,662 | 74 (11) | 1 (876) | 5.4 (2.4-9.0) |
| 4 | 574 (512) | 3,914 | 24 (12) | 1 (984) | 5.2 (2.4-9.1) |
- **Identical to the 043e9104 runs** (same births to the ant): 617's evolved-ant default and nurses-off don't change this box, since it already ran the evolved founder. Specificity check, not new news.
- **Ants underground over time** (DOOR line, every 10k): 10k 4-18% (2-7 ants) -> 20k 2-5% -> 30-300k 2-14%, i.e. 9-61 ants of 500+. In/on the mound 36-75% at 10k, 46-80% after. Separation 0.00 every look: still ONE ROOM on every seed.
- Tracking the underground share is now in every lane-20 report; it is the DOOR line of `nestgoal` and `colony.csv` zone `nest` in deeptrace.
