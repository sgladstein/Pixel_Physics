# Heap 90, seed 2, WAY_FOOT `on`: what opened the east of the nest (Deep trace, 2026-10-07)

Steps 1-2 of the plan I sent Nest race, on their `h90s2/on` and `h90s2/off` logs (`stats.csv`, `cells.csv.gz`, `cuts.csv`).
Everything here is **measured** from those files unless a line says **inferred**.

## The answer

- **The opening is NEEDS_FIRST's packed column, the chain Way home traced, not soil carriers on the costed way.**
  - One walled-in ant, **3146316**, packed a column of spoil 24 cells straight up from the room's depth to the founding ground, at 96,434-96,554.
  - It rose 1-2 cells outside the room's east wall (rows 174-183), then through ground east of the room's top corner, so nothing open stopped it before the surface.
  - Diggers widened the room up to the column (96,651-97,139).
  - At **97,246** ant 2098248 cut a cell of the column, (286,180).
  - The column and the loose ground beside it drained into the room. In the box x 268-310, rows 160-184, over the next 300 frames:
    - **267 cells of ground opened**, and 22 filled;
    - 1,548 uncaused fall steps;
    - the off run, over the same frames and box: 1 cell opened and 0 fall steps.
  - **The nest gets a second way to the sky.** The drained ground is open to the sky by 97,290, and it joins the whole nest at 97,370.
  - The soil still falling plugs it again at 97,750-98,000. From 98,250 it stays joined: 1,793 cells, and 2,032 by 112k.
  - The off run never has a second way: in 144 samples from 7k to 150k, nothing open from the sky gets below row 165.
- **The costed way is not needed for this.** Way home found the same chain on 3 of 3 pile runs of build 6 (304d2e80), which had no WAY_FOOT (`way-home/trace-east-brood-pile-2026-10-07.md`, "The plug").
- **Already built for this chain:** Nest building's NEEDS_FIRST `backfill` part (off; not on your branch).
  - It packs a pack's tail cell below the founding ground instead of leaving spoil.
  - Patch: `nest-building/backfill/backfill.patch`, on 304d2e80.
  - On Way home's three seeds the east drains went 99/2/7 -> 2/0/2 (`nest-building/backfill/results-2026-10-07.md`).
  - **Inferred:** it would have held this column too. h90 s2 `on` with it would say whether the die-off was all this chain.
- **Not known: why 3146316 was walled in and "hungry".** That is your step 3; ant and frames below.
- **Inferred:** the costed way did not steer the cuts that walled it in. Its last three cuts went down and east, away from every way out, and a cut is not aimed by the way. The rerun's `pull_why` will show it.

## The chain, cell by cell (on)

1. **Digging the east face.** 3146316 (age 29,654 at 96,424, a worker) had cut the room's east face at x 277-284 since 90,234.
   It took two loads out to the mound (dropped at (247,137) at 91,509 and (265,135) at 94,934). It put others down inside the nest, 5-50 frames after the cut (93,714-93,794 and 96,199-96,249).
2. **Walling itself in, 96,404-96,424.**
   - Cut (284,182), put the pellet down at (282,181) **5 frames later**.
   - Cut (283,183), put it down at (281,181) 5 frames later.
   - Cut (284,184), and kept that one.
   - Its `cuts.csv` row at 96,424: open8 1, joins 0, home 0.
   - **Not known: which rule put those two pellets down.** NEEDS_FIRST's `hungry` put-down (`need_drop_site`) would do this, but this ant also put pellets down 5 frames after a cut at 93.8k, so the cell log cannot tell. The rerun's `spoil_why` will say.
3. **The column, 96,434-96,554.** 24 packs, one every 5 frames, at x 285-286 from row 184 up to row 160.
   - `needs_packed` rose by 25 in 96-97k.
   - 23 of the 24 cells it cut through were `soil`, the native loose ground, never logged before. The room's packed skin was at x 284-285 and the loose ground began at x 286.
   - At 96,559 3146316 put its last pellet down at (287,159), in the mound's tunnel at ground level. It lived on; it cut in the nest again at 101,389.
   - The column's top, (286,160)-(286,161), was left open. Spoil stood in (286,162)-(286,183) and (284,184)-(285,184).
4. **The room widens to it, 96,651-97,139.** 2098248, 1048908, 4194600, 1049116, 1049436 and 2097784 cut x 284-285 at rows 178-184.
   - By 97,240 the room's east edge was x 285 on rows 178-183, so the column's spoil was the room's wall there.
   - A cut packs its 8 neighbours into wall, but not spoil (Nest building's code read), so the column stayed loose.
5. **The drain, 97,246.** 2098248 cut (286,180), a spoil cell of the column.
   - `cuts.csv`: zone nest, depth 20, flags 0, dig_p 0.825, home 1: an ordinary dig.
   - Uncaused ground-to-open moves in the box x 268-310, rows 160-184, per 50 frames from 97,000: 9, 6, 11, 2, 4, **428, 238, 288, 216, 179, 199**, 3. These are fall steps: a grain falling 10 rows logs 10. Cells actually opened, 97,245 to 97,550: 267.
   - By 97,460 the ground east of x 274 was gone from row 160 down to about row 174, and its soil was in the room below.
   - From 97,266 brood falls into the drained cells: 62 brood arrivals in x 268-310, rows 160-200, by 100k, against 0 off. That is the "brood columns falling to y 190" on your 100k map.

## Pictures (rebuilt from `cells.csv.gz`)

Legend:
- `#` soil, or ground the log has not touched by this frame. Its material then is unknown: lining is not logged.
- `=` packed soil and `s` spoil, as the last logged change left them.
- `P` a cell a pack filled, still ground.
- `.` open: air, tunnel, an ant or brood.

Row 160 is the founding ground.

**On, 96,420: before the column.** The room's east edge is at x 283-284. Column x 286 is ground from the room's depth up to the surface.
```
--- frame 96420   x 266..306 (tens on top), y 156..188
          7         8         9         0      
  156 ..###.####....##...##.####...............
  157 .........................##.##...........
  158 ............#....###.......#####..#......
  159 #.#........###..###=......#.########..##.
  160 #########################################   <- founding ground
  161 #########################################
  162 #########################################
  163 #########################################
  164 #########################################
  165 #########################################
  166 ##..........#############################
  167 .............############################
  168 ..............###########################
  169 ................#########################
  170 ................#########################
  171 .................########################
  172 .................########################
  173 .................########################
  174 ..................#######################
  175 ...................######################
  176 ..................#######################
  177 ..................#######################
  178 ..................#######################
  179 ..................#######################
  180 ...................######################
  181 ...............s..#######################
  182 ...................######################
  183 ................#.#######################
  184 ..............#.#########################
  185 .................########################
  186 ...............##########################
  187 ...............##########################
  188 ................#########################
```

**On, 96,560: the column (`P`), from row 184 to an open top at row 160.**
```
--- frame 96560   x 266..306 (tens on top), y 156..188
          7         8         9         0      
  156 ..###.####....##...##.####...............
  157 .........................##.##...........
  158 ............#....###.......#####..#......
  159 #.#........###..###=.s......########..##.
  160 ####################.####################   <- founding ground
  161 ####################.####################
  162 ####################P####################
  163 ####################P####################
  164 ####################P####################
  165 ####################P####################
  166 ##..........########P####################
  167 .............#######P####################
  168 ..............######P####################
  169 ................####P####################
  170 ................####P####################
  171 .................###P####################
  172 .................###P####################
  173 .................###P####################
  174 ..................##P####################
  175 ...................#P####################
  176 ..................##P####################
  177 ..................##P####################
  178 ..................##P####################
  179 ...................#P####################
  180 ...................#P####################
  181 ..................##P####################
  182 ...................#P####################
  183 ..................##P####################
  184 ................##PP#####################
  185 .................########################
  186 ...............##########################
  187 ...............##########################
  188 ................#########################
```

**On, 97,240: the room dug up to the column (rows 178-183).**
```
--- frame 97240   x 266..306 (tens on top), y 156..188
          7         8         9         0      
  156 ..###.####....##...##.####...............
  157 ..................#......##.###..........
  158 ............#....##.P......#####..#......
  159 #.#........##...##.......#..########..##.
  160 ####################.####################   <- founding ground
  161 ####################.####################
  162 ####################P####################
  163 ####################P####################
  164 ####################P####################
  165 ####################P####################
  166 ##..........########P####################
  167 .............#######P####################
  168 ..............######P####################
  169 ................####P####################
  170 ................####P####################
  171 .................###P####################
  172 .................###P####################
  173 .................###P####################
  174 ..................##P####################
  175 ...................#P####################
  176 ..................##P####################
  177 ..................##P####################
  178 ....................P####################
  179 ....................P####################
  180 ....................P####################
  181 ....................P####################
  182 ....................P####################
  183 ..................#.P####################
  184 ...................P#####################
  185 ..................#######################
  186 ................#########################
  187 ................#########################
  188 ................#########################
```

**On, 97,460: 210 frames after the cut at (286,180).**
```
--- frame 97460   x 266..306 (tens on top), y 156..188
          7         8         9         0      
  156 ..###.####....##...##.####...............
  157 ..................#......##..............
  158 ............#....##......................
  159 #.#........##............................
  160 ########...##............................   <- founding ground
  161 ########................................#
  162 #########..............................##
  163 ###########......#....................###
  164 ############........................#####
  165 #############......................######
  166 ##..........###.#.................#######
  167 .............##..................########
  168 ..............###...............#########
  169 ................##.#...........##########
  170 ................###...........###########
  171 .................#...........############
  172 .................#..........#############
  173 .................##....##..##############
  174 ..................##.#####.##############
  175 ...................#P.###################
  176 ..................##P.###################
  177 ..................##..###################
  178 ...................#P.###################
  179 ...................#P####################
  180 ................####P####################
  181 ................####P####################
  182 ..............####.#P####################
  183 ...............##.##P####################
  184 ...............#..PP#####################
  185 ............s.#.##s######################
  186 .............####.#######################
  187 .............############################
  188 ............###.#########################
```

**Off, 97,460, the same box: the ground over the room is intact.** The `P` at x 272 is off's own column from 71.9k. It rose under the room and broke into it at row 175.
```
--- frame 97460   x 266..306 (tens on top), y 156..188
          7         8         9         0      
  156 .....###.#..#.##........#................
  157 #.#................########..............
  158 ####..............##########.............
  159 ####.#.......#.#.#############.#.....##..
  160 #########################################   <- founding ground
  161 #########################################
  162 #########################################
  163 #########################################
  164 #########################################
  165 #########################################
  166 #########...#############################
  167 .###.........############################
  168 ..............###########################
  169 ...............##########################
  170 ................#########################
  171 .................########################
  172 .................########################
  173 ...#....####.#..#########################
  174 .###....#################################
  175 .###...##################################
  176 ######P##################################
  177 ######P##################################
  178 ######P##################################
  179 ######P##################################
  180 ######P##################################
  181 ######P##################################
  182 ######P##################################
  183 ######P##################################
  184 .#####P##################################
  185 ######P##################################
  186 .#####P##################################
  187 .#####P##################################
  188 ..####P##################################
```

## Packed columns in the two runs

From the cell log: a pack is a blank-cause ground-to-ant change at the cell ahead, with a blank-cause ant-to-spoil change 2 cells away in the same frame (the tail).

| | off | on |
|---|---|---|
| packs, engine's `needs_packed` (to 150k) | 92 | 110 |
| packs found in the cell log | 83 | 94 |
| packs before 95k (`needs_packed`) | 67 | 34 |
| columns ending below ground | 16 (72 packs) | 16 (48) |
| columns in the mound (started at row 160 or above) | 4 (11) | 11 (22) |
| **columns from below ground to the surface** | **0** | **1 (24 packs, the one above)** |
| second way to the sky, other than the door | never (144 samples, 7k-150k) | 97,370; for good from 98,250 |

- Off also has a long column: 22 packs at x 271-272 at 71,871-71,976, rows 195 to 174. It rose under the room and broke into it at rows 174-175, so it joins two parts of the nest, not the nest and the sky. It is still standing at 100k.
- **So what makes a column dangerous is that nothing open lies above the carrier.**
  - Under the room, "up" meets the room.
  - Above its ceiling (Way home's three, from rows 167-171 up to rows 159-162) or beside its wall (this one), "up" meets only the surface.
  - **Inferred** from these 5 columns.
- **One surface column in one pair of runs says nothing about frequency.** The check is below.

## Step 3: the one-ant rerun

- **Ant 3146316**, the `on` env, `ants=all only=3146316`, frames to at least **96,600**.
- **Check first that the rerun is this run.** Its cuts at 96,199, 96,404, 96,414 and 96,424, the 24 packs at 96,434-96,554 (one every 5 frames), and its put-down at (287,159) at 96,559 must all match.
- **Read its rows from 96,150 to 96,560:**
  - `pull_why`;
  - energy against its grant, and the crop: was it `hungry` by being under half the grant, or by `shut_in` under the grant?
  - `shut_in`, and the probe's `shut_now` if your build has it: a stale "shut in" in a fresh cut is what I found behind 97% of seed 1's hungry put-downs above half a grant;
  - `spoil_why` on the two put-downs (96,409 and 96,419) and on the packs (`packed`);
  - why it cut at 96,404-96,424 while hungry: `weak`'s kept dig roll, or something else.
- **Optional:** 2098248 at 97,246 (the drain's cut) looks ordinary from `cuts.csv`, so I would not rerun it.

## Frequency (cheap, if you still have the other runs' cell logs)

Run these on all 16 batch-1 runs. They take seconds each.
- `second_way.py` says when the nest first opens to the sky other than by its door.
- `chains.py` counts surface columns.

```
T=/mnt/project-files/nest-race/store-arms/way-foot/tools
python3 $T/packs.py RUNDIR > RUNDIR/packs.txt
python3 $T/chains.py RUNDIR/packs.txt | tail -1
python3 $T/second_way.py RUNDIR 50000 75000 100000 125000 150000
```

## Tools, and the checks on them

In `nest-race/store-arms/way-foot/tools/`:

- **`packs.py`** finds the packs in a cell log.
  - **Check against the engine's counter** (`needs_packed`, per 1k frames): it finds 83 of 92 (off) and 94 of 110 (on).
  - Every pair it finds sits in a 1k bin where the counter rose, except two: one in the mound at 45,349 (on), and one moved across a 1k boundary (off, 54k/55k).
  - Ahead-to-tail distance is 2 on 174 of 177 pairs.
- **`chains.py`** chains the packs into columns.
- **`rebuild.py`** draws the ground at chosen frames. A cell the log touches is drawn, before its first change, as ground or open by that change's `from`. A cell the log never touches is ground below row 160 and open above, so the door shaft shows open only where the log touches it.
- **`second_way.py`** floods open cells from the sky, never through the door's columns.
  - It reports the deepest row reached and how much of the room (rows 166 and down).
  - **The founding shaft was cut before the log starts**, so the rebuild holds the door as ground. A flood that reaches the room went another way.
  - **Specificity:** the off run's flood never gets below row 165 in 144 samples (7k-150k). Rows 164-165 are a pit in the mound floor at 56-65k that never reached the room.
  - **Sensitivity:** the on run's flood goes from row 161 at 97,260 to row 166 at 97,290 and the whole nest at 97,370.

The `h90s2` files: I have copies, so I'm done with them. I've left them for you to delete. They may be the only copies of the run, and the project's rule is that deletions get Scott's tap.

## Addendum, 10:2x: reading Nest race's step 3 (their rerun of 3146316)

**Their rows match the original run** (cuts, the packs every 5 frames, the put-down at 96,559). What they show, with my reading:

- **The ant was fed.** It ate to its grant at 96,304-96,339, and was at 0.998-1.0 when it packed. Under NEEDS_FIRST it was "hungry" only because it read shut in, under its grant.
- **The two put-downs at 96,409 and 96,419 did not seal it.** They are `need` drops at energy 1.0 with `shut_in` 0, on the walk back to its face (the store `job` drop, inferred by Nest race). The rebuilt ground shows rows 181-183 still open from the room to x 283-284 after them.
- **At 96,429 it stepped into its own fresh cut, (284,184)** (the cut at 96,424). (285,184) was still packed soil until the first pack cut it at 96,434.

**The first packs fired on a stale "shut in", and its own pellet then sealed it.**

| | `shut_in` (engine's way) | `air_then` | `shut_now` (a way built after the step) |
|---|---|---|---|
| 96,434, first pack | 1 | 0 | **0** |
| 96,439 onwards | 1 | 0 | 1 |

- **Ways are rebuilt every 30 frames** (`REST_REFRESH`), at 96,420 and then 96,450. The cut at 96,424 came after the 96,420 rebuild, so the packs at 96,434 and 96,439 both read a way built before the ant's cell existed.
- **`air_then` 0 does not mean sealed.** It says the head's cell was not joined to the open air in the world the way was built from. At 96,420 that cell was still packed soil.
- **`shut_now` is the current reading** (the probe reads it after the row's step).
  - After the first pack it reads 0: the ant could still have walked out.
  - It turns 1 after the second pack. That pack put its pellet into (284,184), logged at 96,439, the one cell joining the pocket to the room; (283,183) beside it stays open.
- **Inferred: the ant was sealed by its own second pellet**, laid on the stale reading.
  - The first pack's pellet does not appear in the cell log, so I cannot say where it went.
  - From 96,444 the ant was truly shut in, and the remaining 23 packs climbed to the surface.

**Why its gate opened.** Read from the code on the WAY_FOOT branch:
- `walled_in` checks only the 8 cells beside the **head**: its own body, or bare ground.
- A carrier whose head is in a one-cell dead end and whose tail is in the room passes it.
- So the first link is the same stale "shut in" as seed 1's hungry put-downs (`shut-in-probe-s1-deep-trace.md`), here on a fed ant in its own fresh cut.

**Not answered: whether the costed way walked it in.**
- Their row at 96,429 says the step was on `soil way out`, with a ground-held target.
- Two checks on that row would answer it:
  1. If their build has probe v4's `k1_hit`, it says whether the old way aimed at the same cell (1 = same aim).
  2. `pull_x`, `pull_y` against the step: a step down and east, into a dead end, on a pull that aims up and west to the door means the ant took the only free cell. The way did not choose it.

## Addendum: Nest race's frequency table (`results.md` §6)

**Their tallies, re-counted:**
- 6 pairs differ, not 7: h30 s2 and h90 s3 tie.
- `on` is earlier in 5 and later in 1. A two-sided sign test gives p 0.22.
- `needs_packed` is up on 6 of 8, p 0.29.
- So the table is suggestive, not shown.

**It is not yet controlled for how much each colony dug.**
- A bigger colony digs more, and every dig is a chance for this chain.
- Their own list of candidates has "bigger colonies digging more".
- The free control: packs per 1,000 digs, from `stats.csv` (`needs_packed` and `digs`), paired by seed, counted up to the earlier of the pair's two opening frames. Counting stops there because a collapse feeds back into both counts.
- Here (h90 s2, to 95k):
  - off: 67 packs over 7,052 digs, 9.5 per 1,000;
  - on: 34 over 9,370, 3.6 per 1,000.
  - The costed arm packed less per dig before its opening.
- If that holds on most seeds, the extra openings follow the extra digging, and this chain is the stack's: NEEDS_FIRST's `pack` gate, plus a column that does not hold. The costed way is then not the cause.
