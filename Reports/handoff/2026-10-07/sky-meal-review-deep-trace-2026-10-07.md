# Review of the sky-meal read (Deep trace, 2026-10-07 03:10)

Asked by the coordinator for Scott ("Has anyone reviewed nest races finding?", 02:32). Four questions: do the numbers
hold up from the saved runs; is the seeds 1 and 3 death chain traced or inferred; what is the share of eggs from hungry
layers; and why do food trips drop with the soil part alone. Measured unless a line says inferred. Proposing nothing.

**Correction (04:33): "encased" in this review was the tool, not the nest.** `starvewhere.py` flooded the walkable cells only down to
row 175, so every ant deeper than that read "encased" whatever was round it (`body-s1-trace-deep-trace-2026-10-07.md` §1;
the fix is PR 652). The two places this review leans on it are struck below.

**Sources**

- Nest race's tables in this folder.
- My four store-off runs (build c9e8a860, same command). They match this folder's `off` arm stat for stat: trip
  deliveries 5,503 / 7,145 / 5,570 / 5,631 and bites 22,383 / 33,878 / 24,607 / 31,736 over 100-150k.
- Reruns on Nest race's build 9a4914af with its command: `skyoff` seeds 1-4 and `skymeal` seeds 1 and 3. They
  reproduce its tables:
  - skyoff trip deliveries over 100-150k: 2,502 (s1) and 4,532 (s2);
  - skymeal s1: 651 ants at 100k, 1,291 at 140k, and 4,081 eggs over 40-150k in the same energy bands;
  - skymeal s3: 980 ants at 90k and 1,514 at 120k.
- A traced rerun of `skyoff` seed 1 to 92k (`ants=120 bornafter=66000`), beside my store-off trace made with the same
  picks rule. Its stats match the untraced rerun on every 1,000-frame row but the last.

## In short

1. **The numbers match the tables.** One slip: "fed and deep... off is 0.4-0.9" should be 0.5-0.9. The 0.4 is skyoff
   seed 1.
2. **The death chain is measured step by step, but each step is a colony count.** No starver or layer is followed in
   the folder's tables, so the heading "traced, measured" says more than they show. On my reruns, the link from heap
   laying to the starvers holds ant by ant, in timing:
   - **761 of 826 starvers on seed 1, and 2,291 of 2,381 on seed 3, were born after heap laying began** (110k and
     60k);
   - 229 and 453 of them had laid an egg at the heap themselves.
3. **Seed 3's starvers mostly died beside food, while the colony as a whole was short of it.**
   - 1,201 of 2,295 starvers died within 5 cells of food inside the nest, and none where the map showed no food
     inside.
   - ~~74% ended encased.~~ The tool's fault. Nest race's re-read with the fixed flood: 1,909 (83%) in the door
     system with the door open, 382 off the map, 2 encased (README, Correction).
   - The heap's ground was bare on 18 of 25 maps at 100-150k, with the top-up at its cap.
   - Foragers still made 19,135 trip deliveries in 100-150k, 3.4x store off.
   - So both hold: food was capped colony-wide, and the ants that died were stuck in the crowd beside what there was.
     The README's step 4 says the crowd part, and the `smell=10` rerun is aimed at it.
4. **The egg table does not show the store tripping the laying rule.**
   - Its store-off baseline is seed 1 only.
   - Over all four store-off seeds, 3 / 36 / 10 / 19% of eggs needed food in reach. Seed 2 reaches 84% at 140-150k,
     with no store, no heap laying, and no die-off (2 starved by 300k).
   - What the dying seeds share is laying **at the heap**.
5. **The soil part alone** (`skyoff`):
   - pellets and starvation are as stated;
   - deep ants are unchanged on seeds 1 and 3 only;
   - the drop in food trips is real and partly traced. Food trips take about three times as long, and traced
     foragers spent 38% of their time far west, against 21% with the store off.

## 1. Numbers against the tables

Checked against `scorecard.txt`, `flow.txt`, `storesize.txt`, `starvewhere.txt`, `neardeath.txt`, `laying.txt` and
`lastpull.txt`:

- pellets held;
- starved and larvae starved;
- deep ants and the door-column split;
- door pile and store loads;
- store size;
- the egg bands;
- the boom's ant counts.

All match. Not checkable from the folder: the seed 1 nest-worker pellet spells (490 against ~1,875 frames), which read
a walk-rows file that stays in Nest race's container.

**One comparison left out.** The skymeal section is titled "against fixb", but its deep ants are compared only with
store off. Against fixb, skymeal had fewer fed ants staying deep on three seeds (4.2 / 41.1 / 33.1 / 10.6, against
19.8 / 11.3 / 60.9 / 49.5), and fewer deep ants on three (70.5 / 93.5 / 332.8 / 29.2, against 265.8 / 260.5 / 297.2
/ 249.7). It also starved fewer.

## 2. The seeds 1 and 3 chain: what is measured, what is inferred

| Step | In the folder | Ant by ant |
|---|---|---|
| 1. The nest reaches the heap | No table. The `cut` rows at x 272-279 are quoted, not shared | No |
| 2. Ants lay at the heap | `laying.txt`: eggs per 10k by layers standing at the heap (zone food) | Counted per egg, not linked to later ants |
| 3. Boom, then a bare heap | Ant counts; heap box at 54 (the fresh drop with nothing on the ground; see my arm2c note) | No |
| 4. Famine | Where starvers died and what pulled them | Where, not why |

**What the reruns add (measured).**

- **Seed 1: the starvers are the boom's young.**
  - Heap laying began at 110k (146 eggs per 10k at 110-120k, then 546-679).
  - The heap's ground was bare on 3 of 25 maps at 100-125k and 16 of 25 at 125-150k (`heapground.py`).
  - 826 starved, at 125-150k, 818 of them in the nest. 761 were born after 110k (median age 5,964 frames).
  - Laying's `layerdeaths.py`: 243 starvers had laid an egg, and 229 of those had laid one at the heap.
  - **Not linked:** whether each starver hatched from a heap egg. The brood log gives the adult a new id, so egg and
    ant cannot be matched by id.
- **Seed 3: the same, earlier and larger.**
  - Heap laying began at 60k.
  - The heap's ground was bare on 5 of 25 maps at 75-100k, and on 18 of 25 in each of 100-125k and 125-150k.
  - 2,381 starved: 405 at 75-100k and 1,975 at 110-150k. 1,950 died in the nest and 408 on the surface.
  - 2,291 of them were born after 60k, and 1,803 after 90k (median age 11,989 frames).
  - 560 had laid an egg, and 453 of those had laid one at the heap.
- **Counts here come from the death log**, which can name the cause wrongly when two ants die in one frame: 826 and
  2,381, against the stats' 816 and 2,339.

**Corrections to step 4.**

- **Seed 1 "40% no pull"** is 40% of rows where no walk was scored. The outcomes in those rows were the move roll
  failing (21%) and falling (19%). `none` (a scored row with no pull) was 0%.
- **Seed 3 "no store food below ground (`f` = 0 on every map)".**
  - `f` is drawn only for whole provisions cells. Food put down at home is crumbs, drawn `c`.
  - `f` is 0 underground in every arm at 100k.
  - Seed 3 had 10 crumb cells and 7 corpses underground at 100k (scorecard, f/c/x 0/10/7). `flow.txt`'s 100-150k
    mean is 17.5 food cells underground, and the store's own median was 9 cells.
- **Seed 3's heap laying began at 60k** (51, 33 and 23 eggs per 10k at 60-90k), before the 90k rise.

## 3. The egg table ("10-35% of eggs from layers left under 200 J")

- **The store-off row is one seed.** Its 2-4% is off s1 (2%) and skyoff s1 (4%), and fixb's 30-35% is seeds 1 and 3.
- **Laying's `bodyonly.py` on all four store-off seeds**, 40-150k, counting eggs whose layer's body could not pay
  (energy after laying + the 120 J egg, under the evolved bar of 946 J):

  | Store-off seed | Eggs | Needed food in reach | Of those, in the nest 0-9 rows down |
  |---|---|---|---|
  | 1 | 2,029 | 63 (3%) | 56 |
  | 2 | 2,270 | 828 (36%) | 750 |
  | 3 | 2,020 | 207 (10%) | 179 |
  | 4 | 2,833 | 527 (19%) | 301 (89 more at the heap, from 130k) |

  - Seed 2's share rises over 100-150k: 69/233, 75/223, 184/262, 211/251, 244/292 per 10k.
  - Those layers stood at the top of the shaft. There was no store, no heap laying, and the heap was never bare.
  - Seed 2 starved 2 ants by 300k (Laying's arm 3 table).
  - **So food-paid laying on its own did not make that colony out-breed its food.**
- **Skymeal's two good seeds laid no more than store off:** 2,180 and 2,276 eggs, against 2,270 and 2,833.
- **The two dying seeds laid at the heap:**
  - seed 1: 1,920 of 4,081 eggs;
  - seed 3: 2,174 of 5,690;
  - seeds 2 and 4: none.
  - On seed 1, 1,107 of the heap eggs came from layers whose bodies could pay (317 layers). A body-only bar would
    leave those.
- **"Standing in the nest" mislabels the bands.** Of the eggs from layers left under 200 J, 290 of seed 1's 808 were
  laid at the heap. On seed 3, 111 of 544 were laid at the heap and 46 in the mound's tunnels.
- **"Nearly starving"** means under 200 J after laying: under one grant, with 626 J or more of food in the 8 cells
  round the head.
- **"Beside the store" is inferred.** The log gives only the zone.
  - On my skymeal s1 rerun, food-paid eggs by where the layer stood:
    - at the heap: 813;
    - top of the shaft (0-9 rows down): 333;
    - 10-19 rows down: 228;
    - 20+ rows down, the store's depth: 240.
  - Fixb did lay about twice the store-off count without the heap (4,245 and 4,124, against 2,029 and 2,020). Where
    its layers stood is not recorded.
  - Fixb's deaths were traced as the eat-pull crowd (`../arm1b/README.md`: ~~61-89% encased~~ (the tool's fault, not re-read), deep, within 10 cells of
    food). So fixb does not show that its boom killed.

## 4. The soil part alone (skyoff against off)

- **Holds:** pellets held 11 / 25 / 17 / 23 against 51 / 87 / 56 / 62; starved 1 / 1 / 3 / 0.
- **"Deep ants unchanged from off"** holds on seeds 1 and 3 only.
  - Seed 2: 23.4 deep of 939 against 3.4 of 701. 17.6 of them were in the door column, and they spent 17.6% of their
    time in the dug nest against 10.4%. There were 3 rooms at 100k against 1.
  - Seed 4: 4.5 against 15.7, but off seed 4 was booming.
- **The food-trip drop** (trips home with food, `forage_returns`, and trip deliveries; `forage_trips` is mostly soil
  holders looping, per the coordinator, so it is not used):

  | Seed 1, per 25k | 0-25k | 25-50k | 50-75k | 75-100k | 100-125k | 125-150k |
  |---|---|---|---|---|---|---|
  | trips home with food, off | 353 | 474 | 470 | 944 | 871 | 970 |
  | ...skyoff | 278 | 381 | 477 | 391 | 406 | 356 |
  | cells per trip, off / skyoff | 3.2 / 3.7 | 4.0 / 4.1 | 3.6 / 3.3 | 3.1 / 3.0 | 3.1 / 3.0 | 2.9 / 3.6 |

  - On seed 1 the gap opens at 75k, when store off's food trips doubled and skyoff's did not.
  - **The sign is not the same early on.**
    - Seed 4's skyoff made more food trips at 25-75k: 904 and 1,027, against 491 and 806.
    - Seed 2's made more at 75-100k: 922 against 729.
    - Seed 3's was lower from 25k.
    - All four are lower at 100-150k.
  - **All four seeds, 100-150k:**
    - trips home with food: 762 / 1,758 / 1,322 / 1,420, against 1,841 / 2,162 / 1,915 / 1,754 (19-59% fewer);
    - trip deliveries: 20-55% fewer;
    - bites: 21-39% fewer.
  - Seed 2 also brought fewer cells per trip: 2.6 against 3.3.
- **Traced, seed 1, 66-92k** (`tripfunnel.py`, `heaplegs.py`, `focalwhere.py`, `pmove.py`):
  - **Fewer food trips because each takes about three times as long.** Traced free ants finished 59 heap trips that
    brought a load home, against 139. The median trip took 4,921 frames, against 1,450. Each leg was slower:
    - to the heap: 1,710 frames against 520;
    - at the heap: 855 against 318;
    - home: 560 against 255.
  - **The same share of time went on heap trips** (21% in both).
  - **They left poorer:** a median 401 J at the start of a heap trip, against 797 J.
  - **On the mound, step rate follows energy and matches between arms.** For an empty-handed free ant, p_move was
    0.15 at 200-500 J in both arms, 0.25-0.31 at 500-1,000 J, and 0.77-0.78 above 1,000 J. So poorer ants walk out
    slower (inferred from the bands, not traced per trip). The move roll failed on 50% of the outward leg against
    38%.
  - **More time far west:** 37.5% of traced free-ant time against 21.2%, at a median 496 J against 550 J. The census
    agrees: free ants 40+ columns west of the door numbered 150-171 against 82-163 on seed 1 at 50-125k, and 206-281
    against 100-134 on seed 2 at 100-150k.
  - **Longer stays out there, not more ants leaving** (`westentry.py`, census, seed 1):
    - ants walking out far west: 519-746 per 25k, against 656-842 with the store off;
    - they stayed a median 3 census samples against 2 at 75-150k;
    - they were poorer while there: 452-585 J, against 504-756 J (seed 2 at 100-150k: 364-427 J, against 749-777 J);
    - they were not soil carriers: only 4-12 per 25k held a pellet at the sample before, against 23-49 with the store
      off.
- **Not traced:**
  - how the soil part starts this, since the ground's shape round the door and heap is much the same in both arms at
    50k, 100k and 150k (maps);
  - what started the gap at 75k on seed 1;
  - why the colony is poorer (bites 13,637 against 22,383 over 100-150k).
  - The two could feed each other: poorer ants walk slower and stay out longer, so they bring less home and stay
    poor. (Inferred.)

## 5. A fault in the trace columns: re-checked, my notes stand

Relayed by the coordinator from Redesign:

- **The fault.** Before 8298e59f (`claude/project-thread-ns0j6p`, one line in `probe_full`), every deeptrace `o_` and
  `h` column was computed without the PheroAAlong/PheroBAlong inputs. The `i_` input columns and the engine's own
  fields (`p_move`, `outcome`, `drop`, `drop_p`, energy, crop) are right.
- **Re-checked (03:25, measured).** I re-ran arm 2b seed 2's trace to 135k on c9e8a860 with that one line applied
  (`ochanged.py`, `ocompare.py`; tables in `review-tables-deep-trace.txt` §7).
  - It is the same game: stats, events and every engine field agree on all 840,012 traced rows.
  - In the evolved founder's brain those two inputs reach only the Move output, through hidden units h0-h3. Move
    changed on 83% of rows. Drop, Feed, Share and every other output are the same on every row.
  - Every trace in these notes uses the evolved founder with mutation off, and every ant in this run carries the one
    genome. So the same holds for the arm 2c and store-off traces. (Inferred from the shared genome; those two were
    not re-run.)
  - The put-down odds were also re-read on the engine's own `drop_p`, with the same bands in all three traces: a mean
    of 0.03 or less under 100 J, 0.16-0.27 at 100-150 J, and 0.39-0.42 at 150-200 J.
- **So the README's "Added 02:32" lines quoting me stand:** the put-down counts, the Drop output by energy
  (`../arm2c/meal-check.md` §2), and the use of `o_Share` to tell shares from other energy falls. The "two thirds"
  figure stays inferred, as it was, because it rests on one ant's crop-to-energy rate.
- **This review uses no `o_` or `h` column.**

## Files

- Here, in `tools/`:
  - `trips25.py`: trips and deliveries per 25k, from stats.csv;
  - `census25.py`: where ants stand, from colony.csv;
  - `tripfunnel.py` and `heaplegs.py`: heap trips ant by ant;
  - `focalwhere.py`: where traced ants spent their time;
  - `westentry.py`: ants walking out far west, and what they were doing just before;
  - `pmove.py`: step rate by energy;
  - `ochanged.py` and `ocompare.py`: the same trace before and after the probe fix;
  - `laywhere.py`: where food-paid layers stood;
  - `starverbirth.py`: starvers' birth frames against the heap-laying start.
- Laying's `bodyonly.py`, `paidby.py`, `heaplay.py` and `layerdeaths.py` are in `../arm3/why/`; `heapground.py` is
  in `../arm2c/tools/`.
- Tables: `review-tables-deep-trace.txt`.
- The reruns stay in Deep trace's scratch.
