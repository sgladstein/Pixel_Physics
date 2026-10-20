# stage2/dig -- the dig rung for the new brain (2026-10-18)

Design: ../../ladder/dig-rung-design.md (rev 2, ladder lane), review ../../ladder/review-dig-rung-design.md.
Code: PR 720 (branch claude/project-thread-eewfjj, head b5cd311b): minds can cut and put a pellet down (ground only);
`challenge=soil step=1|2|3` scores each pellet: cut out of the original ground 1, put down 2, put down away from
the hole (no hole cell within 3) 4. Every stage needs the cut to be original ground; the founding shaft is part of
the hole; stages under `step` pay for 10 pellets an ant at most; from step 3 each hole cell refilled after the
settle costs 4. One ant, 6,000 frames + 2,000 settle, budget 1,200 J, 2-row shaft, flat bed.

## Controls (12 hold-outs, step 3; probes/soil-step3-controls-12holdouts.txt, per layout in soil-step3-per-layout.txt)
| mind | score | cut | put down | away | refilled | energy left (of 1,200) |
|---|---|---|---|---|---|---|
| rest | 0 (every layout) | 0 | 0 | 0 | 0 | 1,068 |
| blank (random) | -12.33 | 13.5 | 13.3 | 0.08 | 10.7 | 868 |
| hand-built forager (no dig wiring) | 2.25 | 13.25 | 12.4 | 0.5 | 6.9 | 652 |
| c33-trim-k450 (450 links), Dig on | 16.67 | 5.75 | 5.33 | 1.75 | 1.7 | 392 |
| 10-30 fetch winner (1,189 links), Dig on | 21.67 | 7.83 | 7.67 | 1.92 | 2.1 | 104 |
| colp30-on-seed52 (532 links), Dig on | 16.25 | -- | -- | -- | -- | -- |

- Steps 1-2 are passed by a blank mind on every layout (it cuts and puts down at random), so they test nothing
  once the verbs are on; training starts at step 3.
- The small-brain start already puts 1-3 pellets away on every layout (39% of its put-downs): step 3's bar needs a
  count or a share (ladder lane to set).
- Tripwire: colp30-on-seed52 scores byte-identically on fetch, trail and scarce before and after the change
  (probes/train-*-{fetch,trail,scarce}.txt).

## Runs
Modal, ref b5cd311b, suite soil,fetch,trail keep=fetch,trail forget 0.8 dist 10,30 step 3 pop 24 layouts 8 wire 2e-3
grow 0.2 patience 30 holdout 12: soil3-s52-seed71, -seed72 from colony/starts/colp30-on-seed52-holdbest.txt.

## Round 4 result: step 3 with nothing laid (scored 09:20, 12 hold-outs; probes/round4/)

Both ran 100 of 100 generations (still gaining at 85-91). Hold-bests, scored with the PR 720 build:

| | soil score | away (S3) per run | away >= 5 | pooled away share | hole cells refilled per run | blinded soil | fetch (10-30) | trail |
|---|---|---|---|---|---|---|---|---|
| start colp30-on-seed52 | 16.25 | 2.17 | 0 of 12 | 0.81 | 0.08 | not run | 22.0 | 0.76 |
| soil3-s52-seed71 | 48.67 | 6.00 (min 3) | 8 of 12 | 0.47 | 1.00 | 24.58 | **23.57** | **0.870** |
| soil3-s52-seed72 | **51.92** | **6.75** (min 4) | **10 of 12** | 0.56 | 0.92 | 38.50 | 22.29 | 0.803 |
| ladder's step 3 bar | | | 11 of 12 | >= 0.75 | <= 0.5 on every layout | | inside kept floor | inside kept floor |

- **Not passed.** Both triple the soil score and put 3x as much soil away, and keep fetch and trail. But they put about
  half their pellets down near the hole and refill about one hole cell a run, and the blind control keeps half to
  three quarters of the score (seed72 blinded 38.5 of 51.9): much of what they learned does not come from seeing home.
- Energy: no layout ends under 10% of the 1,200 J budget (lowest 337 J), so the budget is not the limit.
- This is the design's "nothing laid" arm in the real rig: the baseline for the hidden-unit start (dig-rung-design.md 5),
  which the brain lane is building next.

## The hidden-unit start (09:45; hidden-start/truth-table.md; draft PR 721 at 82c59ff7)

PR 721 adds 8 hidden units to the job head and lays the soil gate on two of them when a forage champion enters
the dig rung (dig-rung-design.md 5). colp30-on-seed52 entered that way, **untrained**, 12 hold-outs at step 3:

| | soil score | away per run | away >= 5 | pooled share | refilled per run | blinded | fetch / trail |
|---|---|---|---|---|---|---|---|
| gated start, gen 0 | **54.75** | 7.75 (min 3) | **11 of 12** | 0.65 | 0.83 (<= 0.5 on 5 of 12) | **10.25** | 23.69 / 0.752 |
| same start without the gate | 16.25 | 2.17 | 0 of 12 | 0.81 | 0.08 | - | 23.96 / 0.752 |

- The gate alone beats 100 generations of training without it (48.7 / 51.9), and its blind control is low,
  so what it does comes from reading home. Still short of the bar on share (0.65 < 0.75) and refill.
- Training from it: soil3h-s52-seed71/72 on Modal at 82c59ff7, settings identical to soil3-s52-seed71/72 (paired).

## Round 5 result: step 3 trained from the soil gate (scored 10:30, 12 hold-outs; probes/round5/)

soil3h-s52-seed71 stopped flat at 45 (hold-best gen 14); soil3h-s52-seed72 stopped flat at 88 (hold-best gen 81).
Hold-bests scored with the PR 721 build (82c59ff7). Per-world soil numbers checked against trace mode (identical).

| | soil score | away per run | away >= 5 | pooled share | pellets back in the hole per run | soil fallen back in per run | blinded | fetch / trail (10-30) |
|---|---|---|---|---|---|---|---|---|
| round 4, no gate, seed71 | 48.67 | 6.00 (min 3) | 8 of 12 | 0.47 | 1.00 (0 on 3 of 12) | 0.33 | 24.58 | 23.57 / 0.870 |
| round 4, no gate, seed72 | 51.92 | 6.75 (min 4) | 10 of 12 | 0.56 | 0.92 (0 on 5 of 12) | 0.17 | 38.50 | 22.29 / 0.803 |
| gated start, untrained | 54.75 | 7.75 (min 3) | 11 of 12 | 0.65 | 0.83 (0 on 5 of 12) | 0.67 | 10.25 | 23.69 / 0.752 |
| **gate + trained, seed71** | 66.25 | **10.50 (min 5)** | **12 of 12** | **0.88** | 0.50 (0 on 6 of 12) | 0.83 | 17.08 | 23.89 / 0.762 |
| **gate + trained, seed72** | **66.67** | **10.25 (min 7)** | **12 of 12** | **0.79** | **0.08 (0 on 11 of 12)** | 1.00 | **10.08** | **24.42 / 0.788** |
| ladder's step 3 bar (as of 10:25) | | | 11 of 12 | >= 0.75 | <= 0.5, mean over the 12 | printed, not gated | | inside kept floor |

- **Seeds 71 and 72 meet the ladder lane's step-3 numbers** (bar as written in dig-rung-design.md at 10:25: pellets
  back in the hole as a mean, falls printed not gated). **Seeds 73 and 74 do not** (table below), so it is 2 of 4
  against the 3-of-4 rule: **step 3 is not passed.** Pictures: pictures/dig-step3-gate-2026-10-18.gif (world 0).
- **New on the gated side: loose soil slides back into the hole** about once a run (0.17-0.33 without the gate,
  0.67-1.00 with it). The score already charges it (-4 a cell, same as a pellet). Ladder lane ruled 10:25: falls are
  printed, not gated, at step 3; from step 4 print which heap each fall came from.
- In the GIF the trained ant scrapes a trench along the surface beside the door rather than going down: step 3 pays
  for carrying soil away, not for depth, so this is what it was asked for. Depth is step 4 (up and out 2-12 rows).
- Training on top of the gate adds about a fifth (54.75 -> 66.5) and fixes the share; the gate did most of the work.

Same ants on 12 **sealed** worlds (never used to pick anything; noise=1 sealed=1, probes/round5/sealed-*.txt):

| | soil score | away per run | away >= 5 | pooled share | pellets back in the hole | soil fallen back in |
|---|---|---|---|---|---|---|
| round 4, no gate, seed71 / seed72 | 46.00 / 40.92 | 5.75 / 4.67 | 9 / 6 of 12 | 0.44 / 0.39 | 1.33 / 1.67 | 0.42 / 0.08 |
| gated start, untrained | 57.42 | 8.92 (min 6) | 12 of 12 | 0.76 | 0.75 (0 on 6 of 12) | 1.00 |
| gate + trained, seed71 | 62.25 | 9.50 (min 8) | 12 of 12 | 0.86 | 0.17 (0 on 10 of 12) | 1.00 |
| gate + trained, seed72 | **67.33** | **10.33 (min 6)** | **12 of 12** | **0.81** | **0.17 (0 on 10 of 12)** | 0.83 |

- The gate's lead holds on fresh worlds (67 vs 41-46), and both trained seeds meet the step-3 numbers there too
  (pellets back in the hole 0.17 a world on average, against the 0.5 bar).

### All four gated seeds against the step-3 bar (11:25; 73/74 on Modal at 82c59ff7, same settings)

| seed (stopped, hold-best gen) | hold-out soil | away >= 5 | share | pellets back / world | sealed soil | sealed away >= 5 | sealed share | sealed pellets back | fetch / trail |
|---|---|---|---|---|---|---|---|---|---|
| 71 (45, 14) | 66.25 | 12/12 | 0.88 | **0.50** | 62.25 | 12/12 | 0.86 | 0.17 | 23.89 / 0.762 |
| 72 (88, 81) | 66.67 | 12/12 | 0.79 | **0.08** | 67.33 | 12/12 | 0.81 | 0.17 | 24.42 / 0.788 |
| 73 (55, 24) | 63.17 | 12/12 | 0.79 | 0.75 | 60.67 | 12/12 | 0.74 | 0.83 | 23.54 / 0.855 |
| 74 (41, 27) | 61.33 | 11/12 | **0.73** | 0.83 | 63.67 | 12/12 | 0.80 | 0.67 | 23.48 / 0.782 |
| bar | | >= 11/12 | >= 0.75 | <= 0.5 | | | | | kept |

- Every seed carries soil away on (nearly) every world; what separates them is pellets put back in the hole: 73 and
  74 stay where the untrained gated start was (0.83), and both stopped flat early (gens 55 and 41).
- Falls about 1 a world on every seed (printed, not gated).

### Where the pellets and the falls go back in (11:45; ladder lane's ask; probes/round5/refills/, a scratch print in soil_trip)

Every refilled hole cell at the end of the run, 12 hold-out worlds (and 12 sealed for the trained gated seeds):

| ant | pellets back | ...in a surface scrape | ...within 2 cells of the scraped stretch's outer end | distance from the door | falls | ...into the founding chamber | ...into the cut under its floor | ...elsewhere |
|---|---|---|---|---|---|---|---|---|
| no gate, round 4 seed71 | 12 | 11 (1 in the chamber) | 3 | 0-75 (5 within 4 cells) | 4 | 3 | 0 | 1 |
| no gate, round 4 seed72 | 11 | 11 | 2 | 3-83 (3 within 4 cells) | 2 | 1 | 1 | 0 |
| gated start, untrained | 10 | 10 | 6 | 14-47 | 8 | 7 | 1 | 0 |
| gated seeds 72/73/74, hold-out + sealed (72 worlds) | 40 | 40 | 26 | 12-53 | 70 | 38 | 24 | 8 |

- **Pellets: the ladder lane's guess holds.** With the gate, no pellet goes back near the door or into the shaft; every
  one lands in a one-row surface scrape 12-53 columns out, mostly at the scraped stretch's outer end, where h1 reads
  "far from home, put down". What would tell the ant "this cell is the hole" is a feature on the put-down's target
  cell (`f::CUT`, never written; PR 2), more than the ant's own depth: it stands on the surface beside the scrape.
- **Falls are not heaps spreading into the trench: they are the nest filling.** 62 of 70 falls land in the founding
  chamber or the cell cut under its floor (the ant's first cut, Dig at the nest). Without the gate falls are rare
  (2-4 in 12 worlds). About one chamber cell a run, of about 18. That is the door/nest risk step 6's D exists for, so
  it should be watched from step 4, not left to step 6.
- **The dig entry costs no trail**: colp30-on-seed52 scores 0.752 on trail (12 hold-outs) before and after
  `enter_rung` with the gate, HOLDOUT lines byte-identical. The round-4 runs just gained more trail in training
  (0.80-0.87) than the gated ones (0.76-0.86) from the same 0.752.

### What drops soil into the founding chamber (12:00; ladder lane's ask 11:26; probes/round5/falls/)

A scratch print in `Soil::run` (`PP_FALLS=1`, `falls/scratch-patch.diff`) logs every founding-chamber cell, or cut
within one of it, that turns to ground, with the put-downs and cuts within 8 cells in the 400 frames before and the
ground cells that went missing just before (where the grain came from). `falls_class.py` sorts each fall by what set
it off. Every world has the same founding cut: a 2-wide shaft (x 128-129, rows 64-65) into a 7x2 chamber (x 125-131,
rows 66-67) under a two-row roof.

| ant (12 hold-out + 12 sealed worlds unless noted) | falls a world | the score's falls a world | set off by a cut on the surface beside the door | by a cut in the roof row | other cut | by a put-down |
|---|---|---|---|---|---|---|
| gated seed71 | 0.88 | 0.92 | 16 | 5 | 0 | 0 |
| gated seed72 | 0.75 | 0.92 | 16 | 2 | 0 | 0 |
| gated seed73 | 0.92 | 1.00 | 18 | 4 | 0 | 0 |
| gated seed74 | 0.92 | 1.00 | 16 | 4 | 1 (2 grains) | 0 |
| gated start, untrained (12 hold-outs) | 0.67 | 0.67 | 8 | 0 | 0 | 0 |

- **Not put-downs: 0 of 83 grains.** No pellet put down near the door rolls in.
- **Not the ant's first cut either.** That cut (frame ~5, one cell under the shaft floor) never sets a fall off; it is
  only where some falling grains end up (the "cut under its floor" column above).
- **It is the ant cutting the rim beside the door.** 65 of the 66 surface cuts are the same cell, (131,64), two columns
  right of the shaft. Three frames later the shaft's right shoulder (130,65) drops into the chamber. Why: a worked cell
  with air beneath stays up only with 3+ non-empty cells of the 8 around it (`update.rs` ~1220, the crumb rule beside
  `self_supporting`). Both shoulders of the founding cut have air on two sides (shaft and chamber), so they sit at
  exactly 3, and one cut next to them drops them. The ant there is beside the founding cut, which `AT_NEST` reads as
  home (`NestHome::Dug`), so Dig's job fires.
- **Moving Dig under cover does not fix it** (scratch `PP_DIG_COVER`, same untrained gated start, 12 hold-outs / 12
  sealed). With Dig reading `ROOFED` instead of `AT_NEST`, falls go 0.67/1.00 -> 0.17/0.42 a world, but cuts halve
  (13.6 -> 6.0), the soil score drops 54.75/57.42 -> 18.83/17.25, away>=5 drops 11-12/12 -> 0/12, and pellets back
  rise 0.83 -> 1.50. `AT_NEST`+`ROOFED` together: soil 24.75/21.67, falls 0.33/0.58. The rim cut still happens in both.
  The control arm reproduces 54.75 exactly.
- **What would fix it: a sense on the cut's target**, "cutting this leaves a roof cell above air with fewer than 3
  non-empty cells around it", which is the crumb rule read one cut ahead, from the 8 cells around each neighbour. Pair it
  with a scorer veto unit for Dig, the same shape as PR 2's `f::CUT` veto for Drop. The score already charges -4 a fall
  and the trained ants still make ~0.9 a world, because nothing they sense tells (131,64) from any other rim cell.
  A thicker founding cut (shoulders at 4+) would hide it in the arena only: every gallery roof edge an ant digs later
  has the same shape.

## PR 2: the soil vetoes, dead links, a colony that can dig (16:00; local branch off PR 721 73cb2c2f, push waits on Scott's 721 go)

What it adds (genome 2,797; 2,333/2,669 genomes pad in): the roof-drop sense on a cut's target (`f::ROOF_DROP`, the
crumb rule read one cut ahead, as the section above asked), a lined-wall sense, a depth sense, a "this cell is the
hole" flag on Drop (`f::CUT`), and two context bits (`ROOF_CUT`, `HOLE_DROP`). On entering Dig a start gets vetoes laid
by hand: a scorer unit that vetoes Drop into the hole, one that vetoes roof-dropping cuts, and a hidden gate that turns
Drop off when the ant holds soil and the only Drop on offer is the hole.

- **Vetoes, 4 seeds x 24 worlds (12 hold-out + 12 sealed):** pellets back in 0.51 -> 0.01 a world, falls 0.95 -> 0.25,
  food put-down unchanged. Each of the six veto faults (Dig job blind to the roof, Drop job blind to the gate, no exact
  cancellation, the "every Drop is the hole" read always 0, no scorer Drop veto, no scorer Dig veto) turns
  `the_soil_vetoes_stop_soil_into_the_hole_and_roof_dropping_cuts` red (probes/round5/vetoes-final-2026-10-18.tar.gz).
- **Dead links zeroed at Dig entry:** links on scorer units nothing reads, inputs nothing writes, verbs never offered,
  and hidden/memory units nothing reads. 101-120 a digger (live 533-562 -> 444-458), 0.040-0.050 J a tick back.
  Choices stay bit-identical (2,000 five-step runs in the unit test; positive control moves >100). Soil, same 4 x 24:

  | | score | s3 | away share | pellets | falls | cuts | energy left |
  |---|---|---|---|---|---|---|---|
  | vetoes only | 65.95 | 9.33 | 0.79 | 0.01 | 0.25 | 13.42 | 430 J |
  | + dead links zeroed | 66.61 | 9.45 | 0.79 | 0.00 | 0.23 | 13.67 | 488 J |

- **Two new soil prints:** `chamber_fill` (founding-cut cells ground again after the settle) 0.10 a run, against the
  step-4 bar of 0.5; `fall_putdown` (falls within 2 cells of a pellet that left its put-down site) 0.00.
- **A colony that can dig (`scarce-dig`, R12):** the scarce colony with Dig and the soil verbs offered, heads-only,
  kept during step 4 with a floor of ref - 2 SE (fetch and trail keep the share floor: a 2 SE floor was rejected on
  trail). Kept arenas are now calibrated on the start as saved, and a pre-flight prints how the entered start scores
  against each floor. Same genome, Dig offered or not (dist 10,30, 8 hold-outs):

  | | scarce | scarce-dig |
  |---|---|---|
  | seed71 | 4.93 (adults 12.3, kept 0.40, starved 12) | 1.09 (adults 6.5, kept 0.19, starved 14.8) |
  | seed72 | 7.38 (kept 0.58, starved 4.5, delivered 86) | 4.15 (kept 0.29, starved 9.5, delivered 38) |
  | blank start | 0.75 | 2.10 |
  | blinded | 1.41 / 2.13 | 1.19 / 1.88 |

  The step-3 diggers dig instead of feeding the colony when both are on offer. That is the choice step 4 has to train,
  and why the colony arena is kept during it.
