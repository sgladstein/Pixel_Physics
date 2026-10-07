# Storeroom parts A (`sky`) and D (`meal`): 150k results and trace (Nest race, 2026-10-07)

Build: claude/nest-race-store-stack 9a4914af, with NEEDS_FIRST, CARRY_HOME and DOOR_COLUMN on. Seeds 1-4, 150k frames, read over 100-150k unless a line says otherwise.

| Arm | Switch |
|---|---|
| off | store off |
| skyoff | NEST_STORE=sky (soil part only, store off) |
| fixb | NEST_STORE=on,pick=20,jaws,sated (arm 1b, store on) |
| skymeal | NEST_STORE=on,pick=20,jaws,sky,meal (store on, `sated` dropped) |

Tables are in this folder: `scorecard.txt`, `flow.txt`, `storesize.txt`, `starvewhere.txt`, `neardeath.txt`, `laying.txt`, `lastpull.txt`. Tools are in `tools/`, and Laying's heap tools are in `../arm3/why/`.

## Soil part alone (skyoff against off): it works, with one untraced cost

- **Ants holding a soil pellet** (mean): 11 / 25 / 17 / 23, against 51 / 87 / 56 / 62 off. In seed 1, 30 nest workers held pellets on the mound for a median 490 frames, against 53 workers for ~1,875 frames with the store off.
- **Mound height** at 150k is 15-19 rows, against 15-17 off, so the mound did not grow taller. Column cleared: 440-598, against 303-471 off.
- **Starved:** 1 / 1 / 3 / 0, against 6 / 0 / 0 / 213 off.
- **Deep ants** (mean, deeper than 10 rows): 3.6 / 23.4 / 4.1 / 4.5 of 512 / 939 / 555 / 596. Unchanged from off, as expected with the store off.
- **UNTRACED: trip deliveries are lower,** 2.5k-4.5k against 5.5k-7.1k off.

## Soil + keep-home-food with the store on (skymeal against fixb)

### Food fate (`flow.txt`)
- **Door pile** (mean food cells on the mound and doorstep): 12-27, against 120-152 in fixb.
- **Store loads:** 414-1,380 picked up and 230-794 arrived, against fixb seed 1's 655 and 162.
- **Store size** 100-150k (median, max): 3 (10), 3 (10), 9 (19), 4 (8).
- **Meal part fires:** 95k holds on seed 1 by 150k.

### Ants in the nest (`scorecard.txt`)
- **Deep ants:** 70.5 / 93.5 / 332.8 / 29.2 of 928 / 635 / 1,278 / 688.
  - Door column: 9.6 / 12.5 / 106.5 / 9.5.
  - Fed and deep 1,000 frames earlier: 4.2 / 41.1 / 33.1 / 10.6. Off is 0.4-0.9.

### Starvation
- Starved 20-150k: 816 / 112 / 2,338 / 4, against fixb 2,239 / 1,980 / 877 / 1,075 and off 6 / 0 / 0 / 213.
- Larvae starved: 323 / 96 / 269 / 116, against off 119-183.
- **Seeds 2 and 4 are good. Seeds 1 and 3 boom, then starve.**

## Why seeds 1 and 3 starve (traced, measured)

The chain is the one Laying traced on arm 3 (`../arm3/read-300k.md`).

1. **The nest reaches the food heap.**
   - Seed 1: between 108k and 112k, nest workers cutting up from the room's east side (`cut` rows at x 272-279, rows 166-172, about 89 per 10k at 110-120k, against 2-16 per 10k store off) opened a pocket under the heap. Heap food fell into it.
   - Seed 3: a second hole opened east of the door between 60k and 90k.
2. **Ants at the heap lay eggs** (`laying.txt`, heaplay). The laying bar counts food within reach, and the egg is charged to the layer's body.
   - Seed 1: from 0 to 679 eggs per 10k laid at the heap at 120k.
   - Seed 3: from 241 to 388 per 10k from 90k.
   - Seeds 2 and 4: 0.
3. **The colony booms past what the heap supplies.** The top-up gives at most 216 cells per 1k frames.
   - Seed 1 went from 707 to 1,291 ants between 120k and 140k. Seed 3 went from 980 to 1,514 between 90k and 120k.
   - The heap was empty (heapcount 54) from 95k on seed 3 and from 130k on seed 1.
4. **Famine.**
   - Seed 1: starvers' last 1,000 frames were 56% walk-out pull and 40% no pull, all of them in the nest.
   - Seed 3: no store food below ground (`f` = 0 on every map from 80k to 150k). The store was mostly near the door.
   - Seed 3 starvers were 53% on the store eat pull (`lastpull.txt`). The store was above the colony-wide 8-cell line on 36 of 51 checks, so the eat pull called every hungry nest ant, and 1,701 starvers ended encased in the crowd. **This is the arm-1b crowding again. I left the 10-step limit (`smell=10`, Deep trace's arm 2b) out of this arm, and that was a mistake.** The rerun with it is running (`skysmell`).

**Every store-on arm lays more, not only these two** (`laying.txt`, paidby). Eggs from layers left under 200 J, standing in the nest:

| Arm | Share of eggs |
|---|---|
| fixb | 30-35% |
| skymeal | 10-21% |
| store off | 2-4% |

Fixb booms and starves without touching the heap. So the store trips the same laying rule as the heap does: an ant beside the store counts the store's food toward its egg bar and pays for the egg from its own body. This is the shipped laying rule (`try_bud` reads food in reach; `lay_egg` charges the bank).

## Still untraced
- Seed 1, 100-106k: a crowd in the shaft, 71 ants against fixb's 20. Most were empty and near their grant (median 0.93), with no pull. Nest workers held store loads on the mound, pulled at the door cell.
- Lower trip deliveries with `sky`.
- Why nest workers in seed 1 cut up toward the heap from 100k on.

## Added 02:32: notes from Deep trace (arm2c/meal-check.md, arm2c/farwest-who-300k.md)
- **heapcount 54 = the top-up's fresh drop with nothing else on the ground.** So "heap empty" above means the ground was bare. `heapground.py` splits the two counts.
- **What part D reaches.** D holds put-downs by ants at about 150-180 J. Ants under 100 J almost never put food down. About a quarter of hungry put-downs at home are by nest workers with something in their jaws, and D does not hold those.
- **Held ants pass much of what they digest to nestmates.** Trophallaxis moves about two thirds of the energy on to poorer nestmates. So whether D's held ants reach their grant needs a focal rerun (`ants=N`), read with sharecheck.py, mealcheck.py and dropband.py.
- **Far-west deaths in arm 2c also follow booms that ate the heap bare.** This fits the laying chain here.

## Added 02:55: rerun with the 10-step store limit (`skysmell` = skymeal + smell=10), seeds 1-4 to 150k (`scorecard-smell.txt`)
- **Starved 20-150k:** 1 / 3 / 1,801 / 2, against skymeal's 816 / 112 / 2,338 / 4.
- **Deep ants:** 29.9 / 21.4 / 117.1 / 35.0 of 631 / 590 / 1,092 / 659.
  - Fed: 26.7 / 20.7 / 64.9 / 30.0.
  - Fed and deep 1,000 frames earlier: 10.9 / 6.7 / 29.5 / 15.3. Off is 0.4-0.9.
- **Births 100-150k:** 779 / 725 / 2,680 / 769, against skymeal's 2,238 / 871 / 2,910 / 871.
- **Larvae starved:** 150 / 200 / 161 / 117.
- **Seed 3 is still the heap chain.**
  - Heap laying starts at 60k (39 per 10k) and reaches 573 per 10k at 120k.
  - 21% of eggs come from layers left under 200 J.
  - 627 of the starvers died deeper than 10 rows.
  - Not yet traced ant by ant. The laying switch is what targets it.
- **Seed 3 traced (03:00), events plus the hungry log.** These deaths happened in two waves.
  - **80-110k, heap still full** (120 cells on every map to 110k): 671 starved. 454 of them died on the surface more than 40 columns west of the door. In their last 1,000 frames they had no pull 49% of the time and were not scored 37%: lost foragers, the far-west death Deep trace traced in arm 2c.
  - Births were already booming then: ants went from 591 to 965 between 60k and 100k. 24% of eggs at 70-110k came from layers left under 200 J, in the nest and mound.
  - **110-150k, heap bare** (54 from 120k): 1,130 more starved, 69% of them in the nest, as walk-out pulls (36%) or no pull. Famine.
  - So seed 3 is the boom (store laying, then heap laying from 60k), which feeds both the far-west loss and the famine. The laying switch targets the boom. The far-west loss is the Way home topic.

## Corrections from Deep trace's review (03:00; sky-meal/review-deep-trace-2026-10-07.md)
- **Skymeal seed 3's deaths were mostly crowding, not famine.** 1,201 of 2,295 starvers died within 5 cells of food inside the nest, and 74% ended encased. Above I called it famine on the strength of the heap going bare. The crowd is the arm-1b eat-pull crowd, and smell=10 is aimed at it.
- **The death chain's links are inferred, not traced.** Colony counts were measured at the right times, but no starver or layer was followed. layerdeaths.py and birth frames are to follow.
- **"Every store-on arm lays more" overstated it.** Store off, food-paid eggs are 3 / 36 / 10 / 19% by seed, not 2-4%: my figure was seed 1 only. Skymeal's good seeds laid no more than store off. The die-offs share laying at the heap.
- **The 02:32 notes resting on o_ columns** (Drop by energy, Share) were re-read by Deep trace on the fixed probe build and **stand**. The missing trail inputs reach only Move (review §5). The "two thirds" share figure stays inferred.
- **Starvers' birth frames, traced** (Deep trace, starverbirth.py, on its reruns of skymeal s1/s3):
  - Most were born after heap laying began: 761 of 826 after 110k on s1, and 2,291 of 2,381 after 60k on s3.
  - 229 and 453 of them had themselves laid an egg at the heap.
  - So the boom-then-die link is now measured per ant.

## Added 03:25: heap at 90 (`foodgap=90`), store off (g90off) vs smell store arm (g90smell), seeds 1-4, 100-150k (`scorecard-g90.txt`)
- **Deep ants:** 68.3 / 65.7 / 28.2 / 53.0 of 554 / 581 / 1,343 / 576, against off 4.3-6.5 of 557-671.
  - Fed and still deep 1,000 frames later: 26.2 / 17.9 / 8.9 / 18.7, against off 0.5-0.7.
- **Starved 20-150k:** 186 / 49 / 136 / 8, against off 9 / 34 / 15 / 1.
  - s1: 180 starvers shallow, a median 28 columns west of the door.
  - s2: 44 deep.
  - s3: a median 137 columns west.
  - **None traced yet.**
- **s3 boomed to about 1,343 ants with no heap contact** (the heap is at x 346).
  - Eggs 100-150k: 1,907 against 978 off.
  - 76% of eggs come from layers still at 1,000 J or more after laying, mostly in mound tunnels. These are rich layers, not food-paid.
  - Starvation stayed low (91 in the window).
  - **Untraced.**
- **Food-paid eggs** (layer under 200 J after laying): 7-8% at 90, against 20-22% at 30 (skysmell).
- **Trip deliveries:** 2,441-3,993, against off 3,048-3,760.
- **g90smell s1: 180 starvers traced (03:40).**
  - **Pocket cluster, 115-130k: 50 ants.** They died in a shallow pocket just below ground, 25-30 columns west of the door (x 224-236, rows 159-166).
    - The pocket was a buried cavity at 104k. At 112k ants opened it to the surface on its west side.
    - All 50 were foragers (census: 138 of 146 samples were foragers with empty jaws and crop).
    - They sat in the pocket a median 7,700 frames before dying (max 27k). Their last 2,000 frames: no pull (74%) or not scored (26%), legs empty, energy median 0.25 of grant.
    - **Why nothing pulled them out (walk-out or store) is untraced.** It looks like the walk-out mapping only to the founding door (the second-hole bug, AIR_WAY), but that is not checked.
  - **Rest:** about 70 died on the surface far west, the lost-forager pattern.
  - **Pocket ants, more (03:50; hungry log, 4,000 to 500 frames before death, 33k rows):**
    - They were walking: step chance median 0.72, stepped 70%.
    - Scouting patience was 0.098, i.e. given-up scouts, who walk home.
    - Their zone was "nest" 84% of the time.
    - The pocket is not on the nest's way (`way_out_from` needs the head on `nest_ways`), so the walk-out pull gives them nothing.
    - **Reading (inferred):** given-up scouts walking home end in a dead-end pocket beside the nest that the way-out map does not reach. This is the second-hole family (Way home / AIR_WAY), not a store rule. Whether the store or sky made the pocket likelier is untraced: g90off s1 had no such pocket, but that is one seed.
- **g90smell s3 boom (03:55):** it is food-backed, not food-paid.
  - More food came home: trip deliveries 9,785 by 140k against 6,921 off, and all deliveries 28,092 against 17,723.
  - 76% of eggs came from layers still at 1,000 J or more after laying (off: 88%), mostly in mound tunnels, as off does.
  - Colony 1,216-1,366 from 80k to 140k, flat, with 136 starved in all.
  - Why this seed brings home more food is untraced. It is not a die-off.

- **Correction (04:30 UTC):** the "encased" counts above (1,701 starvers; 74%) came from starvewhere.py's flood fault: it filled only to row 175. Re-read with Deep trace's starvewhere_deep.py, skymeal s3 is 1,909 starvers (83%) in the door system with the door open, 382 off the map, and 2 encased. "Crowding" still holds: 1,201 died within 5 cells of inside food. The crowd was in an open room, not sealed in.
