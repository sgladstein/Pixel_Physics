# Depth test: idle nest workers slow down by depth (Redesign thread, 2026-10-06 evening)

Scott's card at 18:26 chose "Stack + depth test". This is the depth test. Everything below is measured unless it says
inferred.

**Plain answer: no. No form of the slowing gets ants to live deeper in the nest.**

- Deeper than 10 rows stays at about 2.5-5.0 ants out of 486-876 on every arm, under 1% of ant-time. That compares
  with 2.3-3.1 on hunger-first alone and 4.0-4.7 with the rest pull alone. Added to the rest pull, neither form of the
  slowing beats the rest pull alone by more than about one ant.
- The fade reaches the half-fed ants that are actually deep, and it moved nothing either: 2.6-4.1 of 525-725. It did not
  hold half-fed ants deep and hungry.
- The free space off the door column stays empty: 220-375 free cells hold 1-3 ants. The door column below the doorway
  stays a brood pile.
- One colony crashed: seed 3 of the fade plus the rest pull, 611 -> 250 ants, 156 starved, almost all west of the door,
  after its food intake fell by three-quarters with the door open (traced below; why the intake fell is not traced).
- Inferred: slowing an ant only keeps it where it already is, and almost no ant is deep to be kept. The fed nest workers
  stand in the mound, and the few deep ants are half-fed. Recommendation: leave `DEPTH_SLOW` off. Score it
  inconclusive rather than failed, with no dead-ends entry, since what blocks it is upstream: ants don't come down.
  Re-check it once something else brings ants down and keeps the space off the door column in use.

## Build and protocol

- Branch `claude/project-thread-ns0j6p`, head `fda888d7`, on `claude/pack-behind` 4ca0631c. The switch is
  `PIXEL_PHYSICS_DEPTH_SLOW`, built OFF: unset reads as off. `workers` turns it on; `workers,lean` adds the fade.
- Off game: seed 1 with `NEEDS_FIRST=on` and the switch unset is identical to hunger-first alone's run over 6k-59k
  (stats, colony rows, maps). The lean build (binary 48780648281b, the code of `fda888d7`) reproduces the `workers` run
  over 6k-29k, and with the switch unset gives 0 map cells different from hunger-first alone up to 30k.
- Runs: hunger-first (`NEEDS_FIRST=on`) plus the switch, seeds 1-4 to 300k. `deeptrace scenario=nest_goal ants=0
  mapevery=1000 hungry=1 dig=0`, `RAYON_NUM_THREADS=1`, evolved founder. Windows are 100-300k, starved from 20k.
- Compared within seed against hunger-first alone (`nest-race/lane3/needs-first/runs/on-s1..4`) and the rest pull
  alone (C1, `nest-race/inside/c1/score.txt`). Seeds part within 8-19k frames, so each seed is its own world.
- The raw runs stay in the Redesign container, by the coordinator's 18:48 rule for the shared folder. Tables and scripts are in this
  folder.

## The rule, and what the ant senses

- **Who it reaches:** a nest worker (the 1 in 4 by id), at home (its head within a cell of the dug nest), carrying
  nothing (no crop food, no pellet, no store load), and not latched to walk back to a face or the store.
- **What it senses:** only its own memory. That is the row its head stood on at its last decision away from home
  (`last_out_row`), against the row it stands on now. It reads no map, no scent and no count of other ants.
- **What it does:** it multiplies the ant's chance to step by 10 / (10 + rows deeper than that memory). An ant ten
  rows down steps half as often; forty rows down, a fifth as often. It picks no heading and pulls nowhere.
- **Hunger.** Under `workers`, the slowing is off below the 200 J grant. Under `workers,lean` it fades instead: all
  of it at the grant, none at the 100 J lean line, and none for an ant hunger-first calls hungry for being shut in.
- **Biology.** Workers sort by age with depth "by active movement and choice" (Tschinkel 2004, J Insect Sci 4:21,
  doi 10.1093/jis/4.1.21). The CO2 gradient is not the template: venting or reversing it changed nothing (Tschinkel
  2013, PLoS ONE 8:e59911, doi 10.1371/journal.pone.0059911).

## Results, 100-300k (s1 / s2 / s3 / s4)

Deep means deeper than 10 rows below the old ground line, as mean ants at a census, out of the mean live ants.

| arm | deep: ants of all ants | of them in the door column | off it | dug-nest time | starved 20-300k | mean ants |
|---|---|---|---|---|---|---|
| hunger-first alone | 2.3 of 562 / 3.1 of 569 / 2.3 of 546 / 2.7 of 554 | 1.5 / 1.8 / 1.5 / 1.5 | 0.7 / 1.3 / 0.8 / 1.2 | 4.1 / 6.0 / 4.2 / 4.9% | 21 / 3 / 4 / 9 | 562 / 569 / 546 / 554 |
| rest pull alone (C1) | 4.3 of 562 / 4.7 of 555 / 4.1 of 694 / 4.0 of 534 | 2.4 / 2.3 / 1.9 / 2.2 | 1.9 / 2.4 / 2.2 / 1.7 | 5.7 / 6.6 / 7.3 / 5.1% | 7 / 4 / 1 / 4 | 562 / 555 / 694 / 534 |
| + slowing (`workers`) | 2.5 of 530 / 3.2 of 560 / 3.7 of 618 / 3.3 of 615 | 1.4 / 1.5 / 2.0 / 1.8 | 1.0 / 1.7 / 1.7 / 1.5 | 5.2 / 6.4 / 7.5 / 4.5% | 2 / 2 / 7 / 127 | 530 / 560 / 618 / 615 |
| + slowing + rest pull | 4.2 of 551 / 4.9 of 876 / 3.0 of 533 / 4.4 of 581 | 2.2 / 2.2 / 1.8 / 2.2 | 2.0 / 2.7 / 1.2 / 2.2 | 6.1 / 7.3 / 4.4 / 6.7% | 10 / 47 / 27 / 5 | 551 / 876 / 533 / 581 |
| + fading slowing (`workers,lean`) | 4.1 of 725 / 2.9 of 548 / 2.6 of 525 / 2.9 of 530 | 2.0 / 1.7 / 1.5 / 1.7 | 2.2 / 1.2 / 1.2 / 1.2 | 6.7 / 5.4 / 6.0 / 5.8% | 22 / 6 / 2 / 2 | 725 / 548 / 525 / 530 |
| + fading slowing + rest pull | 4.8 of 569 / 4.9 of 543 / 3.4 of 486 / 5.0 of 547 | 2.6 / 2.7 / 2.0 / 2.6 | 2.2 / 2.2 / 1.3 / 2.4 | 5.9 / 5.7 / 4.9 / 7.5% | 6 / 22 / 156 / 6 | 569 / 543 / 486 / 547 |

C1's door column and off-column ants are its ant-time shares times its mean ants. Starved counts are the scorecard's
(stats); the where-split in `westdeaths*.txt` reads death lines and runs one or two off.

- **Still near zero on every arm.** The most is 5.0 ants of 547, 0.92% of ant-time.
- **The slowing adds about nothing on top of the rest pull.** The rest pull alone holds 4.0-4.7 deep. With the slowing
  it holds 3.0-4.9, and with the fade 3.4-5.0 (+0.5 / +0.2 / -0.7 / +1.0 ants against the rest pull alone, seed by
  seed).
- **Chambers:** one chamber on every map at 100k, 200k and 300k on every arm, none to spec. The one exception is the
  slowing plus rest pull's seed 2 at 100k: two chambers (17x30 and 7x7), neither to spec.
- **Size and booms.** Booms happen on some seed in most arms: homing's seed 2, the slowing's seeds 3 and 4, the
  combined arm's seed 2 and the fade's seed 1. Seed 3 of the combined arm ended at 415 ants: births fell from about
  270 to 144 per 20k frames over 240-300k while old age stayed about 250 (untraced).
- Starvers by where they died are in `westdeaths*.txt`. The combined arm's seed 2 boomed to 1,165 at 100k, and then 36
  ants starved inside the nest in the doorway jam (traced below). Its seed 3 starved 27, 18 of them in 280-300k: 7 in
  the nest, 7 at the door, 9 far west and 3 at the food.

## Did ants reach the free space off the door column? (bandfill, mean per map, 100-300k)

| arm | off column, deeper than 10 rows: free cells / ants | door column, deeper than 10 rows: full |
|---|---|---|
| hunger-first alone | 316 / 0.7, 260 / 1.3, 281 / 0.8, 316 / 1.2 | 96%, 100%, 99%, 100% (mostly brood) |
| + slowing | 306 / 1.0, 281 / 1.7, 324 / 1.7, 248 / 1.5 | 99%, 100%, 97%, 90% |
| + slowing + rest pull | 323 / 2.0, 240 / 2.7, 352 / 1.2, 371 / 2.2 | 98%, 84%, 91%, 99% |
| + fading slowing | 302 / 2.2, 261 / 1.2, 223 / 1.2, 277 / 1.2 | 95%, 99%, 100%, 100% |
| + fading slowing + rest pull | 308 / 2.2, 374 / 2.2, 321 / 1.3, 344 / 2.4 | 99%, 97%, 92%, 99% |

No. The space off the door column stays empty: 220-375 free cells hold 1-3 ants. The door column below the doorway is a
brood pile.

## Which rule fired, and on whom

- **`workers` fired often, but shallow.** Over 100-300k it slowed 89k-124k decisions per seed. The lost rolls it alone
  made were 2.0k-2.8k, one per 70-100 frames. The mean remembered depth at a slowed decision was 2.2-2.5 rows, while
  the founding ground line sat 0.9-1.1 rows above the ant. So the ants it slows stand within a row or two of the old
  ground line, where the cut is about a fifth. 51k-94k more decisions per seed came from nest workers never yet away
  from home, which have no memory and are not slowed.
- **The ants that are deep are not the ants it slows.** On hunger-first alone, the ants deeper than 10 rows are almost
  all between the lean line and the grant (100-200 J): in the door column 1.4-1.7 of 1.5-1.8, off it 0.7-1.2 of
  0.7-1.3. About 0.1 is at or over the grant, and none is under the lean line. That is why the lean fade was built.
- **The fed, empty nest workers stand in the mound** (seed 1 of the slowing arm: 32 in the mound's tunnels, 23 on the
  mound top, 13 on the surface, 1.6 in the dug nest, of 139). A rule that only acts at home cannot reach ants that
  aren't home.
- **The fade reaches more ants, and deeper ones, and still moves nothing.** Under `workers,lean` the rule slowed
  234k-418k decisions per seed over 100-300k, 60-62% of them on ants under the grant (`depth_lean_slowed`). Its lost
  rolls were 29.5k-42.5k, about 15 times `workers` alone. The mean remembered depth at a slowed decision was 6.7-7.5
  rows, 5.1-5.8 rows below the founding ground, so inside the top 10 rows. Deep stayed at 2.6-4.1 ants.
- **It does not hold half-fed ants deep and hungry (the coordinator's 19:11 check).** Ants deeper than 10 rows and
  under the lean line: 0.2 / 0 / 0 / 0, against 0 on hunger-first alone. Starved: 22 / 6 / 2 / 2, against 21 / 3 / 4
  / 9.
- **Its seed 1 boomed to 1,150 ants at 142k, and 14 starved inside the nest** (7 of them in 140-150k; traced,
  `starvetrace-lean-s1.txt`).
  - Each began its last hunger in the nest, about 16 columns west of the door, at 0.97 of its grant. None crossed the
    door's column.
  - The hungry way-out pull acted on only 6% of their decisions. 78% of their scored decisions were a given-up scout
    heading for a home point that was not the door (11 of 14). The trail six cells east was the stronger on 83% of
    decisions, and they did not follow it.
  - Whether the fade's slowing kept them there is not traced (no per-ant record of the cut).
- **That is not the boom jam.** The combined arm's seed 2 (1,165 ants at 100k; 35 starved in the nest in 100-110k) was
  the jam (`starvetrace-slowrest-s2.txt`): the hungry way-out pulled on 61% of their decisions, 35 of 36 crossed the
  door's column, and none got above the ground line. That is the same as homing's seed 2.
- **With the rest pull, the fade slows deeper still and still moves little.** It slowed 285k-424k decisions per seed,
  72-75% of them on ants under the grant, with 40k-57k lost rolls. The mean remembered depth was 8.3-9.3 rows, 6.6-7.5
  below the founding ground. Deep ants under the lean line: 0 / 0.2 / 0 / 0.

## Seed 3's crash under the fade plus the rest pull (traced; `starvetrace-leanrest-s3.txt`)

- **The colony went 611 (175k) -> 486 (220k) -> 269 (230k) -> 250 (233k) -> 519 (300k).** 135 starved in 220-230k and
  156 over the run. Births fell from 110-160 per 10k frames to 23 in 220-230k.
- **Its food intake fell first, with the door open.** Every map from 106k to 256k reads the door open. Food eaten per
  10k frames fell from 3,144 (160-170k) to 815 (210-220k), and trip deliveries from 475 to 95, while forage trips held
  at about 480-620. Ants carrying food in the crop fell from 143 to 31 over 190-215k, and ants at the heap from 41 to
  6. Then the whole colony went hungry at once: ants under the grant went 37 (210k) -> 76 -> 176 -> 335 (225k).
- **Where they died:** 147 of the 150 starvers over 175-260k died west of the door, 54 within 60 columns and 93 beyond.
  Each began its last hunger at 0.99 of its grant, a median 12 columns west of the door: 61 in the mound's tunnels, 54
  on the mound top, 28 on the surface and 4 in the nest.
- **Which rule:** 44% of their decisions were the scouting walk with no other pull, 43% lost rolls, 11% spoil haul, and
  the hungry way-out and the rest pull about none. Of the scored decisions, 39% were a given-up scout heading home, 38%
  scouting, and 22% no scout.
- **What it sensed:** its home point was at the door for all 147. It had trail under its heading on 42% of decisions,
  and the trail six cells east was the stronger on 48%. 64 of the 147 crossed the door's column during their last
  hunger, and 54% of their steps went west.
- **Not traced:** why the intake fell over 200-220k. Neither switch acts on a forager: the fade slows nest workers, and
  the rest pull pulls fed nest workers home. The fade alone (seed 3: 2 starved) and the rest pull alone (seed 3: 1
  starved) did not crash; the slowing plus the rest pull's seed 3 lost births late (to 415 ants at 300k) and starved 27.
  It is the only crash in the 16 runs of the four slowing arms, and the switch stays off until it is understood.

## Seed 4's die-off under the slowing alone (traced, every hungry decision; `starvetrace-slow-s4.txt`)

- **125 starved over 240-300k; 123 died west of the door** (108 more than 60 columns west, mostly at the west wall).
  The food is 30 columns east of the door. The colony went 543 -> 841 -> 963 -> 751 ants at 240k, 260k, 280k and 300k.
- **Who:** young foragers from that birth wave, median age at death 22k frames. 114 of the 123 were not nest workers;
  100 had foraged and 89 had delivered.
- **Which rule:** each began its last hunger about 49 columns west of the door (median), at 0.99 of its grant, on the
  surface (72), the mound top (26) or in the mound's tunnels (25). It had no pull acting: 51% of its decisions were the
  scouting walk with no other pull, 46% were lost rolls, 3% spoil haul, and none was the hungry way-out. The scouting
  walk sends an ant away from home; west of home, that is west.
- **What it sensed:** its home point was at the door for 99 of them. It had no trail under its heading on 76% of
  decisions. The trail six cells east was stronger than six cells west on 34% of decisions, and weaker on 15%, but it
  did not follow it. The door's food-side read never ran, because it never stood at the door.
- **What happened:** 63% of its steps went west and 22% east. Only 4 of the 123 crossed back past the door. Scouting
  ants west of home off a trail stepped away from home 88% of the time. Ants that had given up and turned home stepped
  home 65% of the time, too late.
- **Not the slowing:** it never slowed these ants in their last hunger. They were foragers, or under the grant, where
  `workers` is off.
- **Hunger-first alone has the same death.** Nest building traced 11 of its seed 1's starvers at 288-291k: they left
  the door area empty-handed with no pull acting, walked away from home to the world's edge, and turned back too late
  (`nest-race/lane3/needs-first/homing-arm-2026-10-06.md`, "Seed 1, all on"). By death lines that seed starved 22
  over 20-300k, 19 of them west of the door; 17 died in 280-300k as the colony fell from 603 to 487.
- **Homing's boom starved its ants in the nest, not out west.** Seed 2 reached 1,058 ants; 54 of its 65 starvers died
  in the nest and 10 just west of the door, none far west. The slowing arm's seed 3 boomed to 1,040 and starved only 7.
  So a boom alone does not make the west die-off (measured on these runs; why seed 4's wave did is not traced).

## Files

- `starvetrace.py`, `westdeaths.py`, `depthbands.py` (this folder). Scorecard, bandfill and starvewhere are the Deep
  trace tools (`deep-trace/tools/`).
- `scorecard-*.txt`, `bandfill-*.txt`, `depthbands-*.txt`, `westdeaths*.txt`, `starvetrace-slow-s4.txt`,
  `starvetrace-lean-s1.txt`, `starvetrace-slowrest-s2.txt`, `starvetrace-leanrest-s3.txt`. Arm names in the files:
  `on` = hunger-first alone, `slow` = slowing, `slowrest` = slowing + rest pull, `lean` = fade, `leanrest` = fade + rest
  pull.
