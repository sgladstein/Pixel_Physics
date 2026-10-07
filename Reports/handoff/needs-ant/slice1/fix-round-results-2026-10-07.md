# Slice 1 fix round: results

Redesign lane, 2026-10-07. The reviewed proposal ([`fix-proposal-2026-10-07.md`](fix-proposal-2026-10-07.md),
review beside it) built and run overnight under Scott's go. Code: branch `claude/project-thread-ns0j6p` at
`067dad72`, no PR. Every fix is a named part (`deeptrace needsparts=`), all off by default; with every part off
the walk is slice 1 exactly (seed 1, every map, colony row, walk count and death identical to 55k).

Runs: deeptrace goal box, mutation off, evolved founder, switch to the walk at 50k, seeds 1-4 to 300k, against
slice 1, the stack and shipped from [`results.md`](results.md). Numbers are measured unless marked *inferred*.

## Verdict: the round fails the gate, and the kill rule's conditions are met

All four fixes together (fix round / slice 1 / stack / shipped):

| | seed 1 | seed 2 | seed 3 | seed 4 |
|---|---|---|---|---|
| ants at 300k | **155** / 171 / 562 / 573 | **129** / 124 / 498 / 616 | **66** / 324 / 447 / 560 | **187** / 223 / 550 / 587 |
| lowest count after 100k | **66** (190k) / 170 / 511 / 527 | **70** (245k) / 112 / 490 / 539 | **23** (287k) / 171 / 422 / 461 | **109** (164k) / 138 / 458 / 526 |
| starved, 20k-300k | **986** / 338 / 8 / 42 | **889** / 682 / 25 / 8 | **1,440** / 799 / 28 / 15 | **828** / 588 / 12 / 12 |
| food carried in from outside, 100k-300k | **4,344** / 3,934 / 12,599 / 7,556 | **3,809** / 6,650 / 10,681 / 7,589 | **4,150** / 6,016 / 8,385 / 10,128 | **3,081** / 4,908 / 14,055 / 7,347 |
| births, 100k-300k | 1,489 / 1,218 / 2,696 / 2,703 | 1,177 / 1,970 / 2,540 / 2,684 | 1,556 / 2,126 / 2,400 / 2,630 | 1,418 / 1,505 / 2,555 / 2,613 |
| door shut (maps of 295) | **64** / 73 / 6 / 23 | **99** / 57 / 16 / 6 | **99** / 90 / 14 / 7 | **104** / 74 / 14 / 5 |
| ants deeper than 10 rows, 100k-300k | **22.7 of 235** / 6.5 of 274 / 4.0 of 599 / 2.9 of 566 | **31.1 of 164** / 15.4 of 383 / 7.7 of 585 / 2.5 of 573 | **33.8 of 178** / 19.3 of 403 / 5.8 of 522 / 3.0 of 537 | **27.5 of 221** / 10.7 of 278 / 4.5 of 523 / 2.7 of 560 |
| ...fed and still deep 1,000 frames later | 4.1 / 1.1 / 0.6 / 0.2 | 7.0 / 1.7 / 0.9 / 0.2 | 8.0 / 2.2 / 0.8 / 0.3 | 3.7 / 1.3 / 0.8 / 0.3 |

Food carried in is trip deliveries (food brought in from outside), never `forage_trips`.

The kill rule (design doc): after this round the new ant is dropped if any of these hold.
- **A late collapse: yes**, all four seeds (lowest 23-109 ants, against shipped's 461-539).
- **Less food carried in from outside: yes**, all four seeds against shipped (3.1k-4.3k against 7.3k-10.1k),
  and three of four against slice 1.
- Deep time below twice shipped: no. 22.7-33.8 ants deep against 2.5-3.0, so 8-12 times.
- Deep time below the stack: no. 4-6 times the stack's 4.0-7.7.

## Which fix did it: each fix alone, 4 seeds to 300k

Each arm is slice 1 with one fix's parts on and nothing else, otherwise the same runs (one ant in twenty
traced, digging record off; neither changes the game). Seeds 1 / 2 / 3 / 4.

| arm | lowest ants after 100k | starved, 20k-300k | food carried in, 100k-300k | births, 100k-300k | door shut (maps of 295) | ants deeper than 10 rows, 100k-300k |
|---|---|---|---|---|---|---|
| Fix 1 alone: only the dig job digs, the dig job, the door cut | 159 / 113 / 31 / 231 | 2,053 / 1,648 / 1,161 / 1,317 | 6,880 / 5,766 / 2,685 / 6,933 | 3,120 / 2,307 / 1,503 / 2,868 | 119 / 97 / 161 / 83 | 71.2 / 25.5 / 11.4 / 58.2 |
| Fix 2 alone: foragers keep walking, give up, walk home to lay | 155 / **2** / 155 / **0** (died out 206k) | 598 / 836 / 652 / 604 | 5,822 / 3,919 / 5,085 / 464 | 1,442 / 1,234 / 1,490 / 165 | 80 / 81 / 73 / 41 | 10.7 / 11.1 / 10.9 / 2.6 |
| Fix 3 alone: keep food taken to eat | 82 / 171 / **0** (died out 152k) / 15 | 436 / 212 / 304 / 346 | 2,718 / 5,256 / 713 / 1,603 | 715 / 1,611 / 185 / 414 | 67 / 89 / 42 / 46 | 4.1 / 11.8 / 5.7 / 6.5 |
| Fix 4 alone: escape only after real stalls | **0** (died out 289k) / 141 / 50 / 149 | 523 / 373 / 538 / 473 | 2,619 / 4,793 / 5,888 / 7,859 | 901 / 1,341 / 1,864 / 2,556 | 97 / 50 / 80 / 90 | 6.6 / 11.1 / 24.1 / 26.8 |
| all four (the round) | 66 / 70 / 23 / 109 | 986 / 889 / 1,440 / 828 | 4,344 / 3,809 / 4,150 / 3,081 | 1,489 / 1,177 / 1,556 / 1,418 | 64 / 99 / 99 / 104 | 22.7 / 31.1 / 33.8 / 27.5 |
| no fixes (slice 1) | 170 / 112 / 171 / 138 | 338 / 682 / 799 / 588 | 3,934 / 6,650 / 6,016 / 4,908 | 1,218 / 1,970 / 2,126 / 1,505 | 73 / 57 / 90 / 74 | 6.5 / 15.4 / 19.3 / 10.7 |
| shipped | 527 / 539 / 461 / 526 | 42 / 8 / 15 / 12 | 7,556 / 7,589 / 10,128 / 7,347 | 2,703 / 2,684 / 2,630 / 2,613 | 23 / 6 / 7 / 5 | 2.9 / 2.5 / 3.0 / 2.7 |

**Every fix fails the gate on its own as well.** No single fix broke the round, and dropping any of them
would not have passed it: slice 1 collapses without them, and each fix alone collapses too.
- **Fix 1** carries the round's deep time (11-71 ants deep, against slice 1's 6-19) and most of its
  starvation: 1,161-2,053 starved, the most of any arm, with the door shut on 83-161 maps. Its food carried
  in stays below shipped's on every seed.
- **Fix 2** lost two colonies (seed 4 died out by 206k; seed 2 was down to 2 ants at 299k) and **Fix 3** one
  (seed 3 died out by 152k; seed 4 fell to 15). Their starvation stayed near slice 1's (Fix 2 598-836, Fix 3
  212-436, slice 1 338-799); their births fell instead (Fix 2 165-1,490, Fix 3 185-1,611, slice 1
  1,218-2,126). Why their births fell is not traced.
- **Fix 4**, the smallest change (when a stalled ant starts cutting its way out), also killed a colony (seed 1
  died out by 289k) and dropped seed 3 to 50 ants. Yet its seed 4 recovered to 778 ants and carried in more
  food than shipped (7,859 against 7,347), the only new-ant run on any seed to do so.
- In all, 5 of the 16 single-fix colonies died out or fell below 20 ants; shipped's lowest was 461. Outcomes
  this spread out from one small change suggest the new ant sits close to collapse on every seed (*inferred*).
- Together the four starve fewer than Fix 1 alone on three seeds, and keep more ants deep than Fix 2, 3 or 4
  alone on every seed.

## Why the colonies die (traced on seed 1, every ant traced, 50k-120k)

The seed-1 smoke run (every part on, every ant traced, digging record on) is the same game as seed 1 above,
frame for frame, to 119k. Over 60k-120k it starved 330 ants, against slice 1's 131 and shipped's 1, most of
them in three die-offs: 99 in 1,000 frames at 76k, 109 in 2,000 frames at 82.5k-84.5k, and 100 in 1,000
frames at 110k. Of those 308, 288 died in the mound's tunnels, most within 4 columns of the door.

**1. The way out through the mound shut for long spells, and the ants inside starved together.**
- The door was cut off from the open air on every map from 63k to 76k (maps every 1,000 frames), and again
  80k-85k. That 14-map spell is the longest on seed 1 over 50k-300k in any of the three games: slice 1's longest
  is 7 maps and shipped's is 4.
- It was not sealed the whole time. Ants walked up the tube and out between maps (between 67k and 68k, for
  one, without cutting), and of the 56 ants in the shut space at 63k, 30 were outside on a later map.
- At 76k, 96 ants were in the shut space (the tube and the nest's top 15 rows, where the door test's flood
  stops); 88 of them starved by 77.5k, and 94 by 84k.
- What shut it: soil sliding into the tube through the mound, and pellets put down on the mound above it
  falling in. What opened it at 77k: starving ants cutting on escape up a column 4 east of the door
  (76.3k-76.5k, at 1-18 J of energy), and the soil above draining down through their cuts by 76.9k. By then
  the ants inside were dying. At 86k a corpse was taken away.
- The door cut opened the door three times before the long spell (54.5k, 57.9k and 61.5k, each after a seal
  of one or two maps). During the long spell it cut 29 cells in the tube, 20 of them at its top under the
  mound's crest. Loose soil slid back into 12 of those cells within 500 frames and into the other 17 later.
- Cutting above the ground, 60k-100k: 383 cells under the round, against slice 1's 1,119 cut by `act` (plus
  up to 132 on escape) and shipped's 597. Of the round's 383, 308 were escape cuts and packs by ants already
  starving (hunger at the first escape decision: median 1.0), 69 were the door cut and 6 the dig job.
- Shipped's door shuts too, but briefly: four seals over 50k-100k on seed 1, each open again within one to
  four maps. None of the four reopenings was a cut: three were soil falling away and one was food taken
  away (checked against every shipped ant's decisions beside the cells). Slice 1 had passing ants cutting
  (two of its five reopenings on seed 1) and still had its door shut on 57-90 maps.
- Below the ground the dig job cut 568 cells over 60k-100k, against shipped's 143 and slice 1's 536, so about
  four times shipped's soil was carried up through the tube to be put down on the mound over the door.
  *Inferred*: that loose soil is what keeps the door shut, more than who cuts. The cells that shut the door
  were mostly grains that slid, not traced back to their pellets.

**2. Ants outside could not get back in.**
- Carriers circled the mound, as in slice 1 (results step 5). One traced carrier held 3,610 J of food on the
  mound from before 55k, drifted west, and digested its whole cargo by 59.4k, ending with 6,965 J of its own
  and none delivered.
- Ants rich enough to lay, sent home by `lay_home`, walked on a pull to the door, which is buried under the
  mound. The pull fades as an ant fails to get nearer, so most wandered. The same carrier, now a lay walker,
  was 178 columns west of the door at 64k and still walking at 72k. Lay walkers' decisions 80k-100k, four
  seeds (one ant in five traced): outside 7.0k-19.8k, on the mound 1.5k-6.9k, in the nest 16-102 (0.1-0.4%).
- Foragers that gave up outside (`give_up`), 80k-100k, four seeds: of 112, 2,000 frames later 50 were
  still outside, 47 on the mound, 6 in the nest and 9 dead or out of the trace.
- Food carried in fell right after the switch: on seed 1, 392 trip deliveries over 54k-86k, against
  shipped's 1,079 and slice 1's 1,606.

**3. With little food coming in, the colony went hungry together.** Ants share food downhill, so energy
evens out across the nest and runs down at once (*inferred* from energies; the sharing itself was not
traced). The 110k die-off came with the door open. Just before it, 199 ants were in the tube and the shaft
(within 8 columns of the door), 146 of them hungry. In the tube, 79 of 132 were on the haul job; in the shaft
below, 44 of 67 were on the dig job and 56 of 67 were hungry and heading out. The 100 ants that starved
stepped on 114-121 of their last 121 decisions (600 frames), mostly heading out (a median 118), but
covered only 6-71 cells (median 40): they milled in the jam and never got up and out.

## Each fix's own behaviour

| part | what it was built to do | did it (seeds 1-4, 80k-100k unless noted) |
|---|---|---|
| `only_diggers` | only the dig job digs at the brain's urge | yes: every one of act's 3.9k-4.5k cuts per run was the dig job's; none by foraging, walking home or carrying |
| `dig_job` | take the dig job on soil ahead plus a crowd | yes: taken 29k-42k times per run; ended by the stimulus fading 16k-29k times, by patience 45-80 |
| `clear` | cut back in through a shut door | fired 327-516 times per run; on seed 1 it opened three short seals but not the long one, where loose soil refilled its cuts |
| `pace` | an empty forager keeps walking | yes: 49-50% of its decisions outside won the step roll, against slice 1's 12% and shipped's 49% |
| `give_up` | the forage job ends outside when patience runs out | fired 3.5k-5.4k times per run; 6 of 112 such ants reached the nest within 2,000 frames |
| `lay_home` | a forager rich enough to lay walks home to lay | fired on 0.38M-0.74M decisions per run; lay walkers were in the nest on 0.1-0.4% of them |
| `meal` | home food taken to eat is kept | kept on 137k-182k decisions per run; the put-back rate itself was not re-measured |
| `won_stall` | escape only after real stalls | yes: of 504 escape spells, 7 began under hunger 0.5 (median 1.0); escape no longer fires on a resting ant |

## What the round leaves

- **What worked:** more ants live deep. Under the round 22.7-33.8 ants were deeper than 10 rows over
  100k-300k, against shipped's 2.5-3.0, and 3.7-8.0 of them were fed and still there 1,000 frames later,
  against 0.2-0.3. An ant inside with no pull, slowing as it nears its own preferred depth, does gather deep.
- **What failed is getting food in.** Both of the round's failures are at the door: the way through the
  mound shuts and stays shut, and ants outside do not find their way back in.
- **Inferred, not tested:** the new ant's dig job brings up about four times shipped's soil, and that soil on
  the mound is what holds the door shut. Taking cutting away from passing ants (the round) and leaving it
  with them (slice 1) both left the door shut on 57-104 maps against shipped's 5-23.
- **Not traced:** why give-up ants and lay walkers end up far west rather than at the mound; whether the
  dig job's extra soil is what builds the seals; why the dig job digs so much more than shipped's ants; why
  Fixes 2 and 3 alone cut births; the meal part's put-back rate.

## An instrument trap found on the way

The digging record (`cells.csv.gz`) names a cut only when it is the brain's own dig (`act`). The walk's
escape cuts, packs and door cuts are logged with no cause, the same as a grain that fell, so `sealcell.py`
reads them as soil falling away. Under the round that hid the door cut's three early openings and the escape
cuts that opened the door at 77k; both were found by matching the walk record's `cut` column. Slice 1's
results (step 4) read two of its five reopenings as soil falling away: re-checked against the walk record of
every ant, no escaping ant was beside either cell, so step 4 stands. Not fixed in `deeptrace`.

## Files

- Chart page, every arm against shipped by seed over time: <https://claude.ai/artifact/PnGwDdHmeh6hV86XJNKftV>.
- Code: `claude/project-thread-ns0j6p` at `067dad72` (`src/sim/creature/needs.rs`, the parts and the module
  doc's *The fix round*; `examples/deeptrace.rs`, `needsparts=`).
- Raw runs stay in the Redesign container (`/home/claude/runs/fix`): `F-s1..4` (all parts, digging record
  on, one ant in five traced), `A-s1` (smoke, every ant traced to 120k), `P1..P4-s1..4` (single-fix arms).
- Tracing scripts: [`tools/`](tools/) (`gate.py`, `deep.py`, `perfix.py`, `shutspace.py`, `trapped.py`,
  `clearcuts.py`, `lastdays.py`, `drops.py`, beside `sealcell.py` and `outdrive.py`).
