# WAY_FOOT results (Nest race, 2026-10-07)

**What WAY_FOOT does.** The nest's ways prefer walls to the backs of the crowd. A way step onto a cell held up only by ants costs k steps instead of 1. It is built from the proposal `nest-race/way-over-ground-proposal-2026-10-07.md`, with Deep trace's review taken in full, and is off by default.

**Code:** branch `claude/nest-race-way-foot`, d5a2350d, on 26752d47.

**Test binary:** the feature plus Deep trace's probe v3 (measuring only).

**Base for every arm:**
- LAY_BAR=body;
- NEST_STORE=on,pick=20,jaws,sky,meal,smell=10;
- NEEDS_FIRST, CARRY_HOME and DOOR_COLUMN all on.

The off arms are b30smell and b90smell (same base, no switch). Seeds 1-4, to 150k, RAYON_NUM_THREADS=1.

**The off arm is the old game.** WAY_FOOT=off on the test binary, heap-90 s4: stats.csv and events.txt are byte-identical to b90smell s4.

## Batch 1: `on` (way + store, k=4)

### 1. The traced problem: falls (measured)

Share of hungry decisions deep in the nest (more than 10 rows under the old ground line, zone nest) that were a fall, 20-150k:

| | s1 | s2 | s3 | s4 |
|---|---|---|---|---|
| heap 30, off | 20.8% | 19.0% | 18.6% | 21.8% |
| heap 30, on | 10.8% | 7.1% | 16.4% | 7.9% |
| heap 90, off | 21.7% | 22.2% | 17.1% | 20.3% |
| heap 90, on | 10.6% | 10.3% | 13.3% | 12.6% |

Falls were lower on 8 of 8 seeds, about halved. Deep hungry decisions were also fewer on 7 of 8 seeds. The exception is heap-30 s3, a die-off: 3.1M against 0.75M.

### 2. Colony at 150k (ants / starved 20-150k)

| | s1 | s2 | s3 | s4 |
|---|---|---|---|---|
| heap 30, off | 593 / 90 | 692 / 87 | 660 / 4 | 661 / 117 |
| heap 30, on | 1,528 / 90 | 721 / 2 | **438 / 1,970** | 1,478 / 241 |
| heap 90, off | 996 / 201 | 563 / 86 | 660 / 131 | 1,608 / 154 |
| heap 90, on | 669 / 4 | **2 / 775** | 606 / 99 | 1,461 / 113 |

There are two die-offs under `on`; both are traced below.

Elsewhere:
- **Better:** fewer starved on 4 seeds (h30 s2, h90 s1, s3, s4) and a bigger colony on 2 (h30 s1, s4).
- **Worse:** h30 s4 starved 241 against 117.

### 3. The die-offs

Both fit the known boom-then-famine patterns. Neither was the falls.

**Heap 90, seed 2: a collapse at 100-110k (754 ants to 74).**
- 95% of the starvers were in the door system with the door open (starvewhere_deep, 95-115k).
- Eggs jumped to 430 in 95-100k, against about 100 per 5k before.
- Laying moved east, to x 270-305, as nest, mound_in and surface parents. It had been at x 250 throughout, and stays there in the off arm.
- Trip deliveries halved: 304, then 143, then 97 per 5k.
- The 100k map shows the mound east of the door open into the room at x 274-286 and x 297-302, with brood columns falling from them to y 190. The off arm's 100k map has no such opening.
- This is the east opening / second entrance that Way home traced (`trace-east-brood-pile-2026-10-07.md`).
- **Not known:** whether WAY_FOOT makes the opening more likely, or this seed just drew it. The soil way out now walks the costed way, so where spoil carriers go has changed (inferred).

**Heap 30, seed 3: boom, then famine.**
- The colony reached 1,577 by 80k, against 681 off.
- Starving began from 50k: 183 by 50k, 516 by 70k, 1,970 by 150k.
- Laying spread east to x 290-305 from 70k. That is beside the heap (30 east), the "laying at food" factor common to earlier die-offs.

**Next:**
- batch 2 runs `way` alone (k=4) and `on,k=2` on the same seeds, to see whether the die-offs follow the store part, the k, or the seed;
- trace whether the east opening comes from soil carriers following the costed way.

### 4. Funnel: hungry episodes that began deep in the nest, 60-150k (measured)

**Method** (`/tmp/claude-0/wf/funnel.py`, Nest race; inferred method, a rough cut of releasefunnel):
- An **episode** is a run of an ant's hungry rows with no gap over 40 frames.
- It **began deep** if its first row was zone nest and more than 10 rows under the old ground line.
- **Top** means any row in the top 10 rows or out of the nest.
- **Starved** means the ant died STARVED within 200 frames of the episode's last row.
- **Fed** means the episode ended and the ant lived. It does not say where the ant ate.

| | episodes | reached top | ended fed | starved (of which after top) |
|---|---|---|---|---|
| h30 s1 off / on | 4,412 / 10,783 | 45% / 30% | 95.0% / 98.6% | 62 (12) / 0 |
| h30 s2 off / on | 6,407 / 5,925 | 35% / 36% | 97.0% / 98.4% | 47 (12) / 0 |
| h30 s3 off / on | 7,427 / 23,155 | 27% / 21% | 98.3% / 95.7% | 0 / **385 (188)** |
| h30 s4 off / on | 4,705 / 3,632 | 45% / 48% | 96.8% / 98.7% | 25 (4) / 0 |
| h90 s1 off / on | 5,538 / 9,422 | 39% / 25% | 94.8% / 98.7% | 121 (19) / 0 |
| h90 s2 off / on | 3,043 / 3,927 | 52% / 34% | 94.6% / 94.3% | 39 (8) / **180 (176)** |
| h90 s3 off / on | 4,691 / 8,980 | 43% / 26% | 95.1% / 98.6% | 73 (12) / 1 |
| h90 s4 off / on | 4,039 / 6,700 | 55% / 45% | 86.6% / 98.0% | 68 (7) / 6 |

**Reading (measured, except where marked):**
- **Off the two die-off seeds, deep hungry ants almost stop starving:** 0-6 per seed under `on`, against 25-121 off. A larger share of episodes end fed (98-99% against 87-97%).
- **Fewer of them climb to the top** on most seeds, yet more end fed. So more are fed without leaving. Inferred: from the store or from nestmates. The store bites in stats are higher under `on` on 7 of 8 seeds.
- **On the die-off seeds the deaths are not deep ants failing to climb:**
  - h90 s2: 176 of 180 starved after reaching the top (famine at the top);
  - h30 s3: 188 of 385 starved after reaching the top.
- h30 s4 shows 0 deep starvers in this funnel, yet stats count 241 starved over 20-150k. Its deaths are elsewhere or earlier; still to trace.

### 5. Heap 30, seed 4: the extra deaths are lost foragers far west (measured)

- The colony grew from 620 at 120k to 1,478 at 150k (off: 711 to 661). Eggs were 220-380 per 5k from 120k, trip deliveries 1,068-1,324 per 5k.
- 239 of the 241 starvers died at 120-140k, **on the surface at x 0-99**, 160-250 columns west of the door (x about 256). The heap is east.
- These are the far-west lost foragers that WAY_HOME was built for (Way home's build 6 cut far-west starvers from 1,041 to 65). They are not deep in the nest; the funnel shows 0 deep starvers on this seed.
- Inferred: a bigger, faster-growing colony sends out more foragers and loses more of them west. WAY_FOOT does not touch the surface.

### 6. Second ways into the room: does WAY_FOOT make the east opening likelier? (measured)

Deep trace traced h90 s2's opening (`h90s2-opening-deep-trace-2026-10-07.md`). It is NEEDS_FIRST's packed column, the chain Way home found: an encased carrier packs a column up to the surface, and a later cut drains it into the room. It is not soil carriers walking the costed way.

Frequency, from Deep trace's `second_way.py` and `chains.py` on all 16 batch-1 cell logs. Cells are the first frame of 50/75/100/125/150k at which the sky reaches the room other than by the door:

| | off | on |
|---|---|---|
| h30 s1 | never | 125k |
| h30 s2 | never | never |
| h30 s3 | 100k (13 cells only) | **50k** |
| h30 s4 | never | 125k |
| h90 s1 | 150k | never |
| h90 s2 | never | **100k** (the collapse) |
| h90 s3 | 150k | 150k |
| h90 s4 | 125k | 100k |
| surface columns (all 8 seeds) | 4 | 9 |

**Reading:**
- A second way is common in both arms: 4 of 8 off, 6 of 8 on.
- Under `on` it comes earlier in 5 of the 7 pairs that differ, and later in 1 (h90 s1).
- So WAY_FOOT looks like it makes the east-opening chain likelier or earlier. This is measured on 8 pairs only.
- **The mechanism is not known.** Candidates (inferred):
  - more pack_behind (needs_packed is up on 6 of 8 seeds);
  - bigger colonies digging more;
  - stale shut-in readings in cells the costed way now sends carriers through.
- The one-ant rerun of the h90 s2 column builder (ant 3146316, Deep trace's step 3) is running, to read its pull, energy and shut_in/shut_now when it packed.

### 6a. Corrections and the dig control (after Deep trace's addendum, 10:20 UTC)

- **The column builder was not truly sealed when it began packing.** Its first two packs (96,434 and 96,439) fired on a stale "shut in": the way had last been rebuilt at 96,420, before its cut at 96,424. Its own second pellet then filled (284,184), the one cell joining the pocket to the room, and only after that did it read shut in on a fresh way. It was fed (0.998 of the grant).
- So the first link is the same stale shut-in reading as seed 1, here on a fed ant in its own fresh cut. `walled_in` checks only the 8 cells round the head, so a carrier with its head in a one-cell dead end passes it.
- **Its last pull was not what put it there.** At 96,429 the `soil way out` pull aimed at (283,180), up, and the ant stepped down and east into its own fresh cut at (284,184), the only free cell. My "the costed way walked it into the cut" is withdrawn.
- **The table in §6:** 6 pairs differ, not 7 (h30 s2 and h90 s3 tie). `on` is earlier in 5 and later in 1, sign test p = 0.22. needs_packed is up on 6 of 8, p = 0.29. Suggestive, not shown.
- **Dig control.** Packs per 1,000 digs, from stats.csv, counted to the earlier opening of each pair (150k where neither opened):

| | off | on | digs off / on |
|---|---|---|---|
| h30 s1 (to 125k) | 2.5 | 5.6 | 8,807 / 12,929 |
| h30 s2 (to 150k) | 4.9 | 1.7 | 8,778 / 13,089 |
| h30 s3 (to 50k) | 16.2 | 17.1 | 2,350 / 5,144 |
| h30 s4 (to 125k) | 3.7 | 10.2 | 7,361 / 5,682 |
| h90 s1 (to 150k) | 9.2 | 1.9 | 17,034 / 28,166 |
| h90 s2 (to 95k) | 9.5 | 3.6 | 7,052 / 9,370 |
| h90 s3 (to 150k) | 8.8 | 4.2 | 15,688 / 27,930 |
| h90 s4 (to 100k) | 3.4 | 3.6 | 9,704 / 12,727 |

- **Reading (measured):** packs per dig go up on 4 seeds and down on 4. Digs are higher under `on` on 7 of 8 (bigger colonies).
- **Inferred:** the extra openings follow the extra digging, not a higher packing rate per dig. The chain itself is NEEDS_FIRST's (a pack fired by a stale shut-in, climbing straight up through loose soil), which is Nest building's backfill topic, not WAY_FOOT.

### 6b. The other two bad seeds also opened first (measured with Deep trace's tools; the link to the deaths is inferred)

**h30 s3:**
- Surface columns (`chains.py`) at 15.8k (x 244, 16 cells) and 26.3k (x 266, 8 cells).
- `second_way.py` at 2k steps: the sky reaches the room other than by the door from **32k** (x 244 onward), and the opening widens to 2,000+ cells.
- The colony then boomed, 377 at 32k to 902 at 46k, and starvation began at 48-50k (36 to 183).
- So the opening came before the boom. No drain has been traced, so this is consistent with the chain, not shown to be it.

**h30 s4:**
- A surface column at 108.7k (x 231, 27 cells, west of the door).
- The sky reaches the room from **116k** (x 221-231), widening to x 215-279 by 126k.
- The colony grew 620 to 1,478 from 120k, and the far-west deaths came at 128-138k.
- **Per forager:** 238 far-west starvers (x < 150) over 20-150k under `on` against 0 off. That is 49 per 1,000 forage returns against 0. So it is not colony size alone.
- Inferred, not traced: ants leaving by the west opening get lost west.

**So all three bad seeds** (h90 s2 traced, h30 s3 and s4 timed) had a pack-column opening into the room before their deaths.

## Batch 2 and the overall read (11:05 UTC)

Same base and seeds. **Deep starvers** = ants whose hungry episode began deep in the nest at 60-150k and that starved (per ant). **Die-off** = more than 500 starved over 20-150k. **Opening** = first of 50/75/100/125/150k at which the sky reaches the room other than by the door.

| arm | deep-hungry falls (range over 8 seeds) | deep starvers, 8 seeds | die-offs | openings |
|---|---|---|---|---|
| off | 17.1-22.2% | 435 | 0 | 4 (all at 100k or later, 1 tiny) |
| on (way + store, k=4) | 7.1-16.4% | 572 (0-6 on the 6 seeds without a die-off) | 2 (h30 s3, h90 s2) | 6 |
| on, k=2 | 5.5-20.2% | 946 (0-1 on the 6 seeds without a die-off) | 2 (h30 s1, h90 s3) | 2 |
| way alone, k=4 | 7.9-13.2% | 433 (0-32 on the 6 seeds without a die-off) | 2 (h30 s3, h90 s1) | 2 |

Per seed: ants at 150k / starved 20-150k.

| | off | on | k=2 | way alone |
|---|---|---|---|---|
| h30 s1 | 593 / 90 | 1,528 / 90 | 1,982 / **713** | 558 / 18 |
| h30 s2 | 692 / 87 | 721 / 2 | 727 / 0 | 752 / 40 |
| h30 s3 | 660 / 4 | 438 / **1,970** | 751 / 8 | 1,173 / **1,147** |
| h30 s4 | 661 / 117 | 1,478 / 241 | 849 / 0 | 691 / 0 |
| h90 s1 | 996 / 201 | 669 / 4 | 690 / 44 | 183 / **823** |
| h90 s2 | 563 / 86 | 2 / **775** | 642 / 7 | 592 / 16 |
| h90 s3 | 660 / 131 | 606 / 99 | 1,109 / **1,097** | 721 / 5 |
| h90 s4 | 1,608 / 154 | 1,461 / 113 | 646 / 22 | 708 / 3 |

**Reading:**
- **Measured: the traced problem is fixed in every arm.** Deep hungry ants fall about half as often, and on the seeds without a die-off almost none starve deep (0-32 per seed against 0-121 off).
- **Measured: every switch arm has 2 die-offs in 8 seeds, 6 in 24, against 0 in 8 off.**
  - The die-offs move between seeds from arm to arm, so they are not one bad seed.
  - All 6 are on seeds whose nest opened to the sky other than by the door at 50-125k.
  - The off arm's openings came at 125-150k, or were tiny, and none died.
- **Measured: the store part helps a little.** Deep starvers on the 6 healthy seeds: way alone 10, 32, 13 and 0s; with the store (`on`) 0-6.
- **Inferred (Deep trace's dig control, §6a):** colonies under the switch grow bigger and dig more. That brings the NEEDS_FIRST pack-column opening earlier, and an early opening then leads to the boom-and-famine chain. This is not shown seed by seed.

**Decision (Nest race; framing agreed with Deep trace):**
- **WAY_FOOT stays off.**
- It is not ready for default-on: it is built on the NEEDS_FIRST stack, which is not on main; 6 of 24 switch runs died; and Scott's bar wants 12 seeds to 300k.
- No PR. Branch `claude/nest-race-way-foot` (d5a2350d) holds it.
- **What would unblock it:** the opening chain fixed (Nest building's pack/backfill work), then this rerun.

### Notes from Deep trace's final review (11:30 UTC), applied

- **"0 of 8 off" means 0 of 8 by 150k.** Off's openings came at 125-150k, and the traced crashes followed their opening by 10-20k (Way home's off s6 crashed from this chain at 208-212k). A faster-growing arm reaches the hazard sooner inside the window.
- **6 of 24 against 0 of 8 is p ≈ 0.15** (Fisher, one-sided). It fits the inferred link but is not shown.
- **Deep starvers paired on each arm's own 6 healthy seeds, against off on the same seeds:**
  - `on`: 7 against 396;
  - k=2: 1 against 300;
  - way alone: 42 against 314.
- **Digs** (whole run to 150k, not cut at the opening):
  - k=2 is higher than off on 5 of 8 seeds;
  - way alone is higher on 7 of 8;
  - `on` is higher on 7 of 8 (§6a, cut at the opening).
- **For the rerun after the opening fix:** go to 300k, and check the fix on off first.

## Rerun on top of Nest building's backfill (19:45 UTC, the narrowed task)

**Build:** branch `claude/nest-race-way-foot` at 74a2f08a, which is WAY_FOOT d5a2350d plus Nest building's `NEEDS_FIRST` `backfill` part (d1c4fa08 as a patch). Pushed.

**Runs:** heap 90, seeds 1-4, to 300k. The base is as above, with `NEEDS_FIRST=on,backfill`. The off arm is WAY_FOOT=off, the on arm WAY_FOOT=on (way + store, k=4). No probe.

**1. The backfill on the off arm first (measured).** No run, in either arm, opened to the sky other than by the door at any 50k mark up to 300k (`second_way.py`). In batch 1 that happened in 4 of 8 off runs and 6 of 8 on runs by 150k. So the opening chain is gone here.

**2. Paired by seed:**

| seed | ants at 300k, off / on | starved 20-300k, off / on | deep starvers (episode began deep, 60-300k), off / on | deep hungry falls, off / on |
|---|---|---|---|---|
| s1 | 701 / 796 | 646 / **24** | 362 / 10 | 15.1% / 10.4% |
| s2 | 709 / 771 | 729 / **9** | 446 / 3 | 16.5% / 12.4% |
| s3 | 647 / 748 | 1,040 / **148** | 624 / 77 | 10.4% / 10.8% |
| s4 | 722 / 736 | 434 / **269** | 295 / 179 | 18.8% / 11.8% |

- **Starved:** lower on 4 of 4 seeds, 2,849 off against 450 on.
- **Colony at 300k:** bigger on 4 of 4.
- **Falls:** lower on 3 of 4; s3 is level.
- **Die-offs:** none in either arm.
- **Store bites:** about 2x under `on` (43-52k against 18-28k).
- **When the off arm's deaths come:** they climb from 100-150k onward, and 516 of its 527 deaths at 200-300k on s3 were in the nest.
- **Residual under `on`:** s3 and s4 still lose 145 and 260 ants in the nest at 200-300k. Untraced.

**Verdict (Nest race; Deep trace checked it 19:04 UTC: holds):**
- On top of the backfill, WAY_FOOT is clearly better on every seed tested, without the crashes seen before the backfill. Falls are lower on 3 of 4 seeds, and deaths are lower on 4 of 4. That includes s3, where falls did not move, so the store part (about 2x the store bites) carries part of the gain (inferred, Deep trace's wording).
- It is **not** for default-on yet:
  - it and the backfill both live on the NEEDS_FIRST stack, which is not on main;
  - this is 4 seeds at heap 90 only, against Scott's bar of 12 seeds at heap 30 and 90;
  - the residual late nest deaths on s3 and s4 are untraced.
- Recommended next step, when usage allows: carry WAY_FOOT with the backfill as part of the NEEDS_FIRST stack, and run the 12-seed bar there.
- For the 12-seed bar (Deep trace): "no opening" was read only at the 50k marks. Run `second_way.py` every 5k to rule out an opening that formed and plugged between marks.
