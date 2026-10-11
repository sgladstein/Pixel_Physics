# Slice 1 results: the new ant on 4 seeds to 300k

Redesign lane, 2026-10-07. Branch `claude/project-thread-ns0j6p`; slice 1 itself is commit `dad7cbe9`
(switch `PIXEL_PHYSICS_NEEDS=walk`, off by default). Scene `nest_goal`, the walk switched on at 50k,
seeds 1-4 to 300k, against the shipped walk and "the stack" (today's walk with NEEDS_FIRST, CARRY_HOME
and NEST_REST on). Mutation off, evolved founder, `RAYON_NUM_THREADS=1`, so every run is repeatable.

## Verdict: the gate fails

The new ant does what it was built for, and then its colonies collapse.

- **Deep time passes on every seed.** Ants deeper than 10 rows are 4.6-9.2x the shipped walk's share
  of ant-time and 2.4-4.4x the stack's. It still passes with the fed-and-staying check.
  - It is not the collapse shrinking the count it is divided by. In the boom (60k-85k), with colonies
    as big as shipped's or bigger, 20-35 of 552-639 walk ants were deep against 1-2 of 457-518
    shipped; 4-10 of them fed and staying, against 0.1-0.2 (table below).
- **Every colony collapses late.** 124-324 ants at 300k, against 560-616 shipped.
- **Ants starve:** 338-799 per seed, against 8-42 shipped.
- **Less food is carried in from outside** on all 4 seeds.
- **The nest's door is shut on far more maps:** 57-90 of 295, against 5-23 shipped.

The design doc's rule allows **one round of fixes** before the new ant is dropped. The fixes are
proposed in [`fix-proposal-2026-10-07.md`](fix-proposal-2026-10-07.md), for another lane to review
first. Nothing is built.

Five gate items were not run, because the gate had already failed on three counts:
- carries that lose the way home;
- the feared-colony counters;
- next cut at the face;
- deaths up a wall;
- the 24-seed founding bed.

## The gate, 100k-300k (walk / stack / shipped)

| | seed 1 | seed 2 | seed 3 | seed 4 |
|---|---|---|---|---|
| ant-time deeper than 10 rows | **2.37** / 0.67 / 0.51% | **4.03** / 1.31 / 0.44% | **4.77** / 1.11 / 0.55% | **3.83** / 0.87 / 0.48% |
| ants deeper than 10 rows (mean) | 6.5 / 4.0 / 2.9 | 15.4 / 7.7 / 2.5 | 19.2 / 5.8 / 3.0 | 10.6 / 4.6 / 2.7 |
| ...of them fed and deep 1,000 frames before | 1.7 / 0.6 / 0.2 | 3.4 / 1.4 / 0.2 | 5.1 / 0.9 / 0.3 | 2.6 / 0.8 / 0.3 |
| ants at 300k | **171** / 562 / 573 | **124** / 498 / 616 | **324** / 447 / 560 | **223** / 550 / 587 |
| lowest count (frame) | 170 (163k) | 112 (273k) | 171 (150k) | 138 (157k) |
| starved, 20k-300k | **338** / 8 / 42 | **682** / 25 / 8 | **799** / 28 / 15 | **588** / 12 / 12 |
| food carried in from outside (trip deliveries) | **3,934** / 12,599 / 7,556 | **6,650** / 10,681 / 7,589 | **6,016** / 8,385 / 10,128 | **4,908** / 14,055 / 7,347 |
| births | 1,218 / 2,696 / 2,703 | 1,970 / 2,540 / 2,684 | 2,126 / 2,400 / 2,630 | 1,505 / 2,555 / 2,613 |
| door shut (maps of 295) | **73** / 6 / 23 | **57** / 16 / 6 | **90** / 14 / 7 | **74** / 14 / 5 |

**Deep time in two earlier windows** (Nest building's review asked for one before the crash). Ants
deeper than 10 rows as N of all ants, walk / stack / shipped; "fed and staying" is fed and deep 1,000
frames before too.

| window | | seed 1 | seed 2 | seed 3 | seed 4 |
|---|---|---|---|---|---|
| 60k-85k, the boom | deep | **32.8 of 552** / 1.7 of 517 / 1.7 of 477 | **34.9 of 631** / 12.3 of 592 / 1.4 of 518 | **20.6 of 595** / 4.1 of 515 / 1.3 of 457 | **19.8 of 639** / 2.0 of 476 / 1.4 of 495 |
| | ...off the door column | 21.7 / 1.0 / 0.5 | 23.7 / 6.0 / 0.5 | 12.3 / 0.9 / 0.3 | 9.1 / 1.2 / 0.5 |
| | ...fed and staying | 10.5 / 0.0 / 0.2 | 6.6 / 3.0 / 0.1 | 4.6 / 0.5 / 0.1 | 4.2 / 0.2 / 0.2 |
| 100k-150k, falling | deep | **5.5 of 350** / 3.1 of 586 / 2.7 of 553 | **22.4 of 497** / 14.5 of 636 / 2.3 of 560 | **7.9 of 439** / 7.0 of 570 / 2.2 of 481 | **4.1 of 210** / 3.3 of 476 / 2.0 of 542 |
| | ...fed and staying | 1.5 / 0.4 / 0.3 | 5.3 / 3.0 / 0.2 | 1.8 / 1.1 / 0.2 | 1.2 / 0.5 / 0.2 |

By 100k-150k the walk's colonies were already shrinking (mean 210-497 ants), so that window is not
before the crash; the boom window is.

Four more facts frame the table:
- **The walk boomed before it collapsed.** On seed 1 it beat shipped on food in, births and colony size
  from 50k to 80k: 634 ants against 495 at 80k. The collapse starts at 85k-100k.
- **Not every deep ant is fed.** Of the walk's deep ants, 56-83% were fed.
- **Total digging is not the difference.** Whole-run digs are 9.7k-13.9k for the walk and 11.2k-13.9k
  for the stack, which did not collapse; shipped is 8.3k-10.4k.
- **What differs is where and when the walk digs:** mostly in the mound's tunnels, during the boom.

## Why the colonies collapse: two faults, both traced on seed 1

Seed 1 is traced ant by ant: 39 ants under the walk and 30 under shipped, from 50k to 100k, plus a
record of every dig decision of every ant, and under the walk every ant's drive at every decision. The
counts behind each step were checked on all four seeds wherever the colony census carries them. Each
link is labelled measured or inferred.

**Which ants (corrected 03:40, after Nest building's review).** The two traced samples were not drawn
alike, so no step below compares them:
- the walk's 39: 29 picked by caste, drive, place and whether they stepped (12 of them picked because
  they stood still), plus 10 more;
- shipped's 30: drawn at random within caste, place and load groups at 85k (8 of them empty ants
  outside).

Every step below reads every ant: the colony census (all four seeds), the digging record (every
decision of every ant, seed 1) and, under the walk, every ant's walk record (seed 1). The first version
of steps 6-8 compared the two samples; its "12% against 49%" and "only 11% are above the egg bar"
overstated fault 2 and are withdrawn.

### Fault 1: ants that are not diggers dig, and their soil chokes the mound

1. **Measured: ants with no digging job cut twice the soil during the boom.**
   - Over 60k-100k, empty-jawed ants made 1,787 cuts under the walk against 882 shipped.
   - By drive: foraging 63%, idle ants walking home 23%, resting ants 12%. The dig job made 1%.
   - Slice 1 hands those drives the brain's own dig urge; the design says only the dig job and escape
     dig.
   - Where the cuts were, walk against shipped: 1,197 against 683 in the mound above the ground line,
     and 588 against 199 in the nest below it.
2. **Measured: in the mound's tunnels, walk ants cut 4.5 times as often per decision as shipped ants.**
   - Both win the dig roll about as often: 8% of decisions against 12%.
   - The difference is what lies ahead. Walk ants more often face the mound's own walls above the
     ground line, which the roof rule does not protect, and less often the protected rows under it.
   - Of won rolls at cuttable soil that the heap cue let through, the roof rule refused 71% of theirs,
     against 94% of shipped ants'.
   - Cuttable soil was ahead on 44% of decisions, against 33%.
3. **Measured: every cut makes a soil pellet that must leave through the mound, and pellets are held
   there for thousands of frames.**
   - This happens in both games. It is the shipped soil-on-mound chain the Nest race lane traced: the
     drop is refused under the mound's roof.
   - So soil holders standing in the mound's tunnels are 128-247 under the walk against 47-82 shipped
     (all four seeds, 55k-100k). The stack sits between, at 58-142.
   - The mound grew taller and denser. On seed 1 at 90k it is 17 rows of soil high against 14, and
     wider.
4. **Measured: the door was cut off from the open air on 57-90 maps against 5-23. On seed 1, one
   pellet shut it from 87k to 93k, and a non-digger had cut it.** (Traced 03:30 with `sealcell.py`.)
   - The cell: the top of the way out through the mound, 15 rows above the door and 2 columns west.
     Reopening that one cell opens the door at 87k: the thinnest wall was one cell.
   - At 86,126 a soil carrier on the haul job put its pellet down there. It had cut that pellet itself
     at 80,481, 35 rows deep in the nest, while on the forage drive at 118 J (step 1's fault), and
     carried it 5,645 frames.
   - The nest and the mound's tunnels were shut off from the air until 93,900. Then a hungry ant
     inside (83 J), its way out stalled for 33 decisions, cut through on escape.
   - Nobody starved while it was shut: 93 died, all of old age. 10 starved in the 7,000 frames after.
   - Six seals over 50k-99k on seed 1. The four my tool could follow back to a pellet were each put
     down by a soil carrier; none was a cut. Of the five reopenings, two were cuts by non-diggers (on
     the forage and home drives), two were soil falling away, one was escape.
   - Every seal's thinnest wall had a cell within 5 columns of the door column, 3-17 rows above the
     ground line. Four of the six walls were one cell thick, one two, one four.
   - Pellets put down over the door (within 5 columns, above the ground), 60k-100k: 255 under the
     walk against 54 shipped. A soil carrier there puts its pellet down on the same share of its
     decisions in both games (0.12%); the walk had 3.4 times the carrier decisions there (155,615
     against 45,149). So the extra pellets over the door are extra soil traffic, not carriers letting
     go sooner.
   - Shipped's seals in the same span: four, two from pellets put down and two from grains whose
     source the tool could not follow.
5. **Measured: food carriers did not get into the tunnels late on.**
   - At 85k-100k on seed 1, 14% of carriers were inside the tunnels (7 of 49), against 36% shipped
     (41 of 113).
   - At 55k-70k both games had 15% inside, so this is a late change.
   - Carriers on the mound step as freely as shipped ones: 62-75% of decisions, with the same mix of
     ants and soil ahead. They are not jammed step by step; they circle on top.
   - On all four seeds, carriers inside were 7-21 against 14-46 shipped.

### Fault 2: late on, foragers stand still outside, so fewer reach the food

*(Rewritten 03:40 from every ant; see "Which ants" above.)*

6. **Measured: late on, empty ants outside step about half as often as shipped's. In the boom they
   were level.** Share of every empty-jawed decision that stepped, seed 1, walk / shipped:

   | | 55k-70k | 80k-100k |
   |---|---|---|
   | on the mound top | 39% / 40% | **20% / 51%** |
   | on the open surface | 34% / 37% | **25% / 41%** |

   So this gap opens with the collapse; it is not there while the colony booms.
7. **Measured: under the walk a forager's pace is its energy's, and the poor ones barely move.**
   - The walk hands the forage drive the brain's own step chance. Over every empty decision outside on
     seed 1:
     - forage-drive ants under 1,000 J stepped on 9% of decisions at 55k-70k and 5% at 80k-95k. The
       step chance the walk used had a median of 0.03, then 0.00;
     - forage-drive ants over 1,000 J stepped on 61-66%, median chance 0.75.
   - The poor foragers' share of all empty decisions outside grew from 52% to 66%.
   - This is the chance the walk used, from its own record, not the trace's brain readout, so the
     readout fault does not touch it.
   - Rich ants outside fell away late on. Empty ants over 1,000 J as a share of empty ants outside
     (census, seeds 1-4): level at 55k-70k (walk 44/30/20/40%, shipped 46/30/27/41%), lower at
     85k-100k (25/27/18/22% against 53/42/28/29%), most on seeds 1-2.
   - Code-read: a shipped ant ready to lay gets home's pull (`HomeAligned`) and walks home.
   - **Not traced:** why fewer walk ants outside are rich late on. They leave the food as rich as
     shipped ants do (median 1,653 J against 1,469 J at 50k-70k).
8. **Measured: the forage job never ends outside, and rich empty ants outside stay on the forage
   drive.**
   - Code-read: in slice 1 a forage job ends only at home. A forager that stops being paced stays on
     the job where it stands.
   - 89-90% of rich empty ants' decisions outside are on the forage drive, which steers out, not home
     (seed 1, every ant, 55k-70k and 80k-95k).
   - At 85k-86k, 94 forage-drive ants stood on the mound's top round the door, stepping on 12% of
     their decisions, mean 463 J.
   - **Not traced:** whether rich ones get home to lay.

### How the two faults end the colony (measured, seed 1)

- Per 10k frames, from 70k-80k to 90k-100k:
  - bites, anywhere, fell from 3,835 to 1,200;
  - food carried in fell from 533 to 121, while shipped rose from 315 to 649;
  - eggs laid fell from 240 to 49, then to 16 in 100k-110k;
  - births fell from 211 to 43, then to 33 in 100k-110k.
- The ants born in the boom then died of old age with no one to replace them: 656 ants at 90k, 297 at
  140k.
- 44 starved between 100k and 110k.

**Not measured:** how much of the collapse each fault causes. The fault-1 counts (soil holders, a shut
door) and the fault-2 counts (foragers standing still) rise together. The proposal tests each fix
alone and both together, to split them.

## Smaller faults found in slice 1 as built

- **Measured: bites taken at home are put back down.**
  - Under the walk, 77 of 78 bites taken in the nest were put down again, a median 15 frames later,
    gaining 8 J each (traced ants, 50k-100k).
  - This is the fault the Deep trace lane found in the shipped game. The design doc said slice 1 likely
    had it; now checked.
  - The bites were taken by eating, resting and foraging ants; most were put down by an ant that had
    switched to resting.
  - Scott's purpose rule (design doc rev 131) is the fix.
- **Measured: escape fires on ants that are barely hungry.**
  - Its stall counter rises while the ant chooses not to step, so a resting ant counts as stuck.
  - One traced case fired at hunger 0.06.

## Measurement traps found on the way (they affect other lanes)

- **The brain readout in every trace was wrong. Fixed today in `creature.rs` `probe_full`.**
  - The trace fed the brain the raw senses. The real tick hides the two "scent along my heading"
    inputs under today's walk.
  - On the walk's trace, 85% of the decisions whose step chance is the brain's own disagreed with what
    the trace printed.
  - Every `o_` and `h` column `deeptrace` wrote before today carries this fault.
  - So do the brain readouts of `digbox`, `nestdoor` and `trailfollow`.
  - Findings built on those columns should be re-checked. The inputs columns were always right.
- **Sampling ants by id remainder picks nest workers only.**
  - The shipped storeroom makes every fourth id a nest worker for life, and the walk's first trace took
    one ant in N by id. Every traced ant was a nest worker.
  - The walk trace now samples by a hash of the id, or by an explicit `only=` list.
- **The colony counter "forage trips" mostly counts soil holders.**
  - It books any ant that went 8+ cells from home and came back.
  - At 80k-100k on seed 1, 73 of 80 such trips under the walk were soil holders wandering with a
    pellet, and 41 of 85 shipped.
  - So the walk's "more trips" (1,325 against 673 per 10k) is not foraging. Read trip deliveries
    instead.

## What is not yet traced

- ~~What closes the door~~: traced for seed 1 (step 4). Seeds 2-4 not traced.
- Why fewer walk ants outside are rich late on.
- Whether the walk's rich empty ants ever get home to lay.
- Seeds 2-4 are checked by colony counts only. The ant-by-ant trace is seed 1.

## Files

- Runs and traces stay in the lane's container (`/home/claude/runs/walk`, `walk-trace2`, `digrec`).
- Tools: scratchpad `walkrun/tools/`:
  - `tripclass.py`: carry trips by how they ended;
  - `roundtrip.py`: what the trip counter counts;
  - `surfpace.py`: how empty ants move by place;
  - `inputs.py`: brain inputs side by side;
  - `cutdrive.py`: cuts by drive;
  - `drivezone.py`: where each drive's ants stand;
  - `sealcell.py`: for each map where the door shuts or opens, the thinnest wall and the cut, drop or
    slide that made each of its cells (uses `deep-trace/tools/doorseal.py`'s rule);
  - `outdrive.py`: every empty decision outside, by the walk's drive and the ant's wealth.
