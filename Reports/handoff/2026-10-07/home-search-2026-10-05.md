# Why carriers can't find home, and what the lost-ant search does (deep trace, 2026-10-05)

Chart page: https://claude.ai/artifact/WKgozaSioRyn39YMZ6aTGy (the trace). The fix trial in §10: https://claude.ai/artifact/BNzm89YCCFmUYhrxTVmzf4

Lane 21 (deep trace), for Scott's 19:57 point 3 ("ants who are trying to get home cannot find it is a HUGE problem") and the 20:14 "Both now" card. The coordinator's question was why `HOME_SEARCH` shrank colonies.

## The answer

1. **Carriers mostly can turn home, but the walk keeps them going away.** In the 22 steps before a carrier gives up its pull home, a step toward home was open on 83-86% of them. It stepped away from home on 60-66% of those 22 steps. The way being shut (mound soil, or a cell holding four ants) is only 14-17% of them.
2. **Nest scent locks a carrier's heading.** Nest scent (trail A) lies evenly over the whole mound: 0.98-0.99 strength in every direction. A carrier reads it as a road and holds its heading about four times harder on it. Going straight scores 3.9, while the pull home adds at most 1, and the pull fades as the carrier fails to get nearer (0.37 on average in those last 22 steps). A carrier that is facing away, or walking past its door, cannot turn round. So the pull runs out.
3. **The cost is about half the colony's food.**
   - 84-89% of carries lose the pull home at least once.
   - 43-52% of carries reach home; 41-48% are eaten or shared on the way (counted as `emptied`).
   - 51-67% of all food trail is laid by carriers that have lost the pull: these are the false roads.
4. **The search does what it says, a little.** On today's game, with nurses off, over 4 seeds, `HOME_SEARCH` makes lost carriers:
   - reach home more often, in all 4 seeds (39-43% to 42-47%);
   - end sooner (median 2,095-2,445 frames to 1,720-1,995);
   - wander west less (37-48% to 33-38%);
   - lay 26-40% less false road (food trail laid while lost).

   It does not stop carriers losing the pull (84-89% either way). The colony is no smaller: mean ants 100-300k were 547/551/529/573 off and 554/541/550/527 on.
5. **The search still costs some food: the heap gave out 3-16% less food over 100-300k, in all 4 seeds.** Carriers also laid 5-22% less food trail overall, including toward the heap. That the missing trail is the cause is a guess, not traced. Starvation is mixed: 13/55/42/14 grown ants starved off against 86/28/29/97 on, over 20-300k, with both arms' deaths coming in bursts. The 10-04 colony loss (436/461/367/399 to 257/341/12/388) does not reproduce on today's game.

**Owner of a fix:** the Nest lane owns the home pull and `HOME_SEARCH`; Scott chose (23:44) that the deep trace lane builds the turn-for-home fix as an off switch, with the Nest lane reviewing. **Update 23:50:** over 12 seeds the search shrinks the colony (smaller on 9, below 300 ants on 5, none with it off), so it stays off. Bringing carriers home, by the search or by the fix in §10, cuts how much they eat at the heap; §11 has the trace.

## Build and setup

- **Game:** PR 617 head dbf324f9 with `PIXEL_PHYSICS_NURSE_STAY=off`. That is main 01320ff7's game: PR 617 landed differing only in the nurses' default, which is now off.
- **Recorders:** the deep-trace recorders (PR 631 head ef7e7e5a, plus a scratch recorder of every laden decision with its options and scores, commit ed16e6a7 in the scratch worktree). Recording draws no RNG: the rows are byte-identical with and without it, checked to 109k.
- **Runs:** dry goal box, evolved founder, mutation off, seeds 1-4, 300k frames. The arms differ only by `PIXEL_PHYSICS_HOME_SEARCH=off|on`.
- **Carries:** those started 20-260k, so each has at least 40k frames to finish.
- **Seed 1 run to 110k a second time**, with what was in every refused cell. It reproduced the first run row for row.

## 1. Each carry from the heap, booked at the furthest it got

| Run | Carries | Got home | Lost the pull | Lost, then got home | Lost: median frames to the end | Lost, wandered west of x 236 | Food trail laid while lost | Mean ants 100-300k | Heap intake 100-300k | Starved 20-300k |
|---|---|---|---|---|---|---|---|---|---|---|
| off-s1 | 6,666 | 45% | 89% | 41% | 2,445 | 46% | 67% | 547 | 25,159 | 13 |
| on-s1 | 6,168 | 47% | 87% | 42% | 1,830 | 35% | 52% | 554 | 21,053 | 86 |
| off-s2 | 5,500 | 46% | 89% | 42% | 2,365 | 41% | 66% | 551 | 21,435 | 55 |
| on-s2 | 7,244 | 52% | 84% | 47% | 1,720 | 33% | 51% | 541 | 20,869 | 28 |
| off-s3 | 6,482 | 43% | 88% | 39% | 2,245 | 48% | 67% | 529 | 22,753 | 42 |
| on-s3 | 6,545 | 45% | 88% | 42% | 1,895 | 38% | 52% | 550 | 21,104 | 29 |
| off-s4 | 6,758 | 48% | 87% | 43% | 2,095 | 37% | 66% | 573 | 23,235 | 14 |
| on-s4 | 5,761 | 49% | 88% | 45% | 1,995 | 35% | 53% | 527 | 20,328 | 97 |

- **How carries that did not get home ended:** 41-48% eaten or shared on the way (`emptied`), and 7-10% died mid-carry, every one of old age.
- **Heap intake** is the food cells the heap had to replace (the log's `food_dropped`).
- **Losing the pull** means the walk's patience fell under 0.1. It falls by a tenth on every step that does not beat the carrier's nearest approach to home by a quarter cell.

## 2. Where the pull runs out

By where the carrier was when it lost the pull. Search off, 4 seeds; search on is alike.

| Where | Share of lost carries | Median nearest approach to home (cells) |
|---|---|---|
| Inside the mound, under cover | 36-42% | 5.0-6.1 |
| On the mound's open top | 27-34% | 9.4-14.4 |
| Back at the heap | 23-27% | 17.9-18.4 |
| Elsewhere | 4-5% | 41-46 |

- **By column:** about three quarters lose it between the door and 30 columns east, toward the heap.
- **How far they got:** only 10-12% had come within 3 cells of their home point, and 44-51% within 10.
- **When:** the median is about 450 frames after picking up, and about 330 decisions.

## 3. Why: the last 22 steps before each carrier gave up

Every carry that lost the pull, its last 22 scored steps before that moment. By definition none of those steps got nearer home. Each step is booked as one of three:

| | Away: a homeward step was open, it went another way | Shut: no homeward step was open | No nearer: it stepped homeward and still got no nearer |
|---|---|---|---|
| Search off, 4 seeds | 64-66% | 14-16% | 20% |
| Search on, 4 seeds | 60-61% | 16-17% | 23% |

- **Away is the larger share in every place:**
  - inside the mound: 54-59% away, 16-19% shut;
  - on the mound top: 66-70% away;
  - at the heap: 61-68% away.
- **Per carrier, away is the most common kind for 89-100% of lost carriers.**
- **What filled a shut homeward cell** (seed 1, the run that recorded every refused cell, 20-90k):
  - search off: ground 73%, a cell already holding four ants 22%;
  - search on: ground 78%, four ants 20%.
  - The ground is packed soil, soil and spoil: the mound over the door.

## 4. The walk's arithmetic (checked against the engine's own scores)

- **The score of each usable heading** is persistence x turn preference x hold, plus the pull home x cos(heading, home). A carrier has persistence 1, no turn bias and choice looseness 0.1 on every decision (483,633 checked).
- **Hold** = 1 + 3 x nest-scent strength. On the mound the strength is 0.98-0.99 in every direction, so hold is about 3.97. Straight on scores 3.94, a 45° turn 3.36, 90° 1.97, 135° 0.58, and turning back 0.
- **The pull home** = gain x cos. Gain is the pull's strength times patience, so it is at most 1, and it fades as patience falls.
- **Checks:**
  - The split reproduces the recorded scores: the trail gain solved from them is 2.999 on 1,383 of 1,383 decisions.
  - The chooser's odds, (0.1 + score)² normalised, predict how often carriers actually took a homeward step, to within half a point in every distance band, in all 8 runs.

How often the walk takes a homeward step when one is open, over every carry (not only the lost):

| Distance from home point | Homeward step open | Took it (engine's odds) | Facing away from home | Got nearer |
|---|---|---|---|---|
| 0-3 | 61-65% | 58-62% | 36-41% | 24-28% |
| 3-6 | 70-75% | 49-53% | 48-50% | 25-29% |
| 6-10 | 80-88% | 44-53% | 44-50% | 32-37% |
| 10-20 | 84-85% | 51-56% | 36-39% | 42-46% |
| 20+ | 87-91% | 53-59% | 35-40% | 51-57% |

## 5. How much a fix to the turn could move (inferred, one step, from recorded states)

Each recorded decision was re-scored with nest scent counting only where it is stronger than around it, so even nest scent holds nothing.
- **Over every carry,** the odds of a homeward step when one is open rise from 44-62% to 70-84% by band.
- **In the 22 steps before a carrier gives up,** they rise from 27-31% to 45-48%.
- **The faded pull limits the rest.** Re-scored with contrast-only nest scent and the pull at full strength as well, the odds would be 76-77%.
- **This is not a run.** Carriers that turn more readily might press against the mound instead of walking round it. The fade exists for exactly that case, a U-bend, so it needs a real test.

## 6. The search, in the colony

- **No colony loss on today's game.** Mean ants 100-300k, off against on: 547/554, 551/541, 529/550, 573/527. Births are alike: 2,549-2,683 off, 2,479-2,636 on.
- **Less food in.**
  - Heap intake 100-300k: -16%, -3%, -7%, -13% with search on.
  - Trip deliveries: 8,018/6,788/7,821/8,154 off against 7,446/7,578/7,386/7,195 on.
  - The heap's intake dips mid-run in both arms. Its lowest 20k window over 100-300k is lower with search on in all 4 seeds: 1,117-1,411 against 1,557-1,905 cells.
- **Less food trail.** Carries started 100-260k laid 5-22% less food trail. Toward the heap they laid 21% less, 6% less, 2% more and 14% less.
  - **Inference, not traced:** lost carriers' trail near the heap helps foragers find it, and the search takes some of it away.
  - **The Nest lane saw the same intake drop** at 01:00 on main 6e42f0fa: pile food -12 to -33%, births -9 to -13%, colony 0.85-0.90 of the arm without search. Its test with 1.5x trail did not confirm the trail link. Today the colony loss is gone and the intake drop remains.
- **Starvation is noisy.** It comes in bursts in both arms: 58 in on-s1 at 20-40k, 46 in off-s2 at 20-40k, 64 in on-s4 at 200-220k.

## 7. Individuals

- **Ant 445** (on-s1, about 50.8k).
  - It homed to 2 cells of its home point (259,159). Standing at (261,159), only east and north-east were usable.
  - It wandered off and lost the pull at 51,089. The search re-armed it, it got within 1 cell, and it delivered at 51,484.
- **Ant 5243145** (on-s1).
  - Its patience sat at 0.00 for thousands of frames, with the search's reach out to 32 cells.
  - It wandered the mound top and west to x 233, and delivered at 64,294, 7,700 frames after picking up.
- **Pictures:** `pic-off-s1-75k` and `pic-off-s1-200k`. Each marks the spot where every lost carrier last got nearer home, and the home points.
  - The home points are the top of the shaft, at row 159. That row lies under the mound and is ringed by ants.
  - The last-best spots scatter over the mound around and east of the door.

## 8. Corrections to my earlier lines

- **Shut way.** Earlier tonight I said the way down was shut 60-70% of the time, and that the pull lost to "keep going" 58-63% of the time when a way was open. Both came from steps taken right after each carrier's last approach. By construction those exclude the steps where it went home.
  - Over all steps, a homeward step is open 61-91% of the time, depending on distance.
  - In the run-out, shut is 14-17%.
- **Uniform nest scent.** This stands: 0.98-0.99 in every direction.
- **The 10-04 note** (`lost-homing-2026-10-04.md`) said carriers on the mound "cannot get closer: falls, a crowded neck, and loose spoil all stop it". That is the smaller part. The larger part is that they don't turn.
- **Lane 2's PR 617.** Its README and commit message label the `eats` counter as "heap intake". The numbers are 64.5k etc. I told lane 2.

## 9. Fix candidates (for the Nest lane, which owns the home pull; none built)

1. **Recommended: nest scent stops locking the heading where it is even.**
   - **The change:** a carrier's hold on trail A counts how much stronger the scent is ahead than around it, not its level. Trails that really vary still hold it as a road.
   - **Expected lift (inferred):** homeward odds when open from 44-62% to 70-84%, and from 27-31% to 45-48% before a give-up.
   - **Direct metrics:**
     - share of carries that lose the pull (84-89% now);
     - share eaten on the way (41-48%);
     - food trail laid while lost (51-67%);
     - frames to home (median 1,330-1,826).
   - **Then** a picture of where carriers stop, then the colony.
   - **Risk:** carriers pinned against the mound, since the fade exists for U-bends.
2. ~~Turn `HOME_SEARCH` on, after the 12-seed check that settles a default.~~ **Settled 23:50: it stays off.** Over 12 seeds it shrank the colony on 9 and took it below 300 ants on 5 (§11). Its cost is the same as the fix's: carriers leave the heap with less.
3. **A real way-in cue near the door.**
   - Homing desert ants follow the nest's CO2 plume upwind to find the entrance, but only when path integration tells them they are near home (Buehlmann, Hansson & Knaden 2012, [doi 10.1016/j.cub.2012.02.029](https://doi.org/10.1016/j.cub.2012.02.029), via PubMed).
   - Here nest scent has no gradient near the door. A scent rising out of the actual way in would give one. This is a bigger build.
   - Nest odour also pinpoints the entrance in Steck, Hansson & Knaden 2009 ([doi 10.1186/1742-9994-6-5](https://doi.org/10.1186/1742-9994-6-5)). Lost ants search in widening loops, as `HOME_SEARCH` does (Wehner 2002, [doi 10.1007/s00359-002-0340-8](https://doi.org/10.1007/s00359-002-0340-8)).
4. **Not alone: keep the pull from fading.** It would lift the odds most (76-77% combined with 1), but it removes the U-bend escape, so it only makes sense alongside 2 or 3.

## 10. Fix 1 built in scratch and run (22:00-23:30)

**Short version.** The fix brings carriers home: lost the pull 45-58% to 9-26%. But on its own it halves the early colony. About half of today's walks from the door to the heap are ants still holding food from the pile at the door, carried out by the same nest-scent hold that loses carriers. The fix keeps those ants at the door, and empty ants don't walk out enough to replace them. Applied only to food from a trip, the fix keeps most of the homing gain, and the colony matches today's on 3 of 4 seeds at 30k. The 300k runs are below.

**What was built.** A scratch switch, `PIXEL_PHYSICS_HOLD_CONTRAST=on|trip|off`, off by default. It lives in the scratch worktree and is not pushed.
- `on`: a laden ant's hold on nest scent counts only how much stronger the scent is on a heading than on the weakest open heading. The even fog over the mound then holds nothing, and a real route still holds.
- `trip`: the same, but only while the crop holds a trip's load (food taken away from the door and not yet booked home).
- `off`: today's game. Runs are identical row for row (checked to 30k on seed 1 and 16k on seeds 1-4).

**Direct metric: carries started 6-25k (early colony, 4 seeds, same game and setup as §1).** Early carries get home more often than the 300k carries in §1 because the mound is still small, so compare the arms with each other, not with §1.

| Arm | Lost the pull | Got home | Median frames to home | Eaten or shared on the way | Carries |
|---|---|---|---|---|---|
| Today | 45/54/49/58% | 82/86/80/86% | 405/600/410/535 | 9/7/12/10% | 206/180/280/170 |
| Full fix | 11/26/12/9% | 95/91/96/95% | 180/240/195/200 | 4/3/3/2% | 170/189/312/174 |
| Trip-only fix | 20/15/22/20% | 93/91/89/94% | 215/215/235/215 | 5/6/8/4% | 294/289/255/352 |

**The colony (ants alive, seeds 1/2/3/4).**

| Frame | Today | Full fix | Trip-only fix |
|---|---|---|---|
| 12k | 52/74/61/67 | 29/64/38/36 | 61/65/67/56 |
| 20k | 130/137/167/140 | 54/99/89/56 | 136/161/123/155 |
| 29k | 235/231/203/246 | 128/108/181/140 | 237/228/142/245 |

**Why the full fix slows the colony.** Traced every ant, every decision, 8-16k, 4 seeds (`ants=all dig=1 walk=1 laden=1`; recording changes nothing, checked against the unrecorded runs).
- **Today about half of all walks out to the heap are made holding food.** A walk out is a leg from within 8 columns of the door to 14 or more columns east of it. Today there were 140/151/247/121 of them, and 44/52/58/50% were made holding food.
- **That food mostly came from the pile at the door.** Of the walks out holding food that was neither a trip's load nor a packed lunch, the food was taken only in the door area on 40 of 44 such walks on seed 1 and 118 of 119 on seed 3. The nest-scent hold walks these ants straight past the door to the heap. There they top up, and they come back as carriers.
- **Door food is mostly not a packed lunch.** On seed 1, 92% of pick-ups above ground at the door (503 of 546) left the lunch flag off. The packed-lunch rule exists to send exactly this food out with an ant that behaves as an empty one. The cause is not traced; it belongs to the food lane.
- **The full fix keeps these ants at the door.** Walks out holding food fall 3.4-3.6x: 61/79/143/61 today, 17/37/42/17 with the fix. The drop comes first. In 8-10k the colonies are still the same size and the counts are already 5/18/28/7 against 3/5/3/1.
- **Empty ants don't replace them.** Empty walks out: 79/72/104/60 today, 52/65/103/17 with the full fix.
- **Less food in, fewer eggs.** Eggs laid 10-14k: 73/51/71/34 today, 6/23/30/19 with the full fix.
- **Picture:** the chart page's space-time strip (seed 1, 8-12k). Today, orange (holding food) tracks shuttle between the door and the heap. With the full fix they stay by the door, and the only orange in the heap is two ants that sit there eating.
- **Individuals.**
  - **Ant 81** (today, seed 1, frame 9,709). It held 1 cell picked up at the door and stood at (273,158) heading east, past its door. Under the hold, east scored 2.34 against 0.89 for west, toward home. It walked into the heap, topped up to 6 cells and delivered at 10,174.
  - **Ant 43** (full fix, seed 1). It held 1-4 cells of door food from 8,419 to 10,759 between x 255 and 268, idle on 311 of 480 walking decisions, and never left the door area.

**The trip-only fix.**
- Walks out holding food: 94/77/60/106 (today 61/79/143/61). Empty walks out rise on all 4 seeds: 114/99/123/80 (today 79/72/104/60).
- Eggs laid 8-16k: 126/122/118/140 (today 132/116/147/90; full fix 80/56/77/70).
- Seed 3 is the exception: fewer walks out holding food and a colony 30% smaller at 29k.

**300k.**

| Arm | Mean ants 100-300k | Fewest ants 100-300k | Grown ants starved 20-300k | Trip deliveries 100-300k |
|---|---|---|---|---|
| Today | 547/551/529/573 | 495/509/462/492 | 12/55/41/14 | 8,018/6,788/7,821/8,154 |
| Full fix | 245/216/313/92 | 87/30/66/0 | 405/562/246/409 | 4,209/4,260/4,847/1,243 |
| Trip-only fix | 478/469/398/400 | 420/314/222/1 | 120/68/419/323 | 7,795/9,034/6,876/6,362 |

- **The full fix starves the colony.** It halves the colony or worse on all 4 seeds, and the seed 4 colony died: 329 ants at 100k, 0 at 210k. Its trip deliveries were 105-240 per 10k frames over 100-160k (today's seed 4: 448-642) and 0-83 after 160k, while 391 grown ants starved over 100-210k. Seed 2 crashed from 442 ants to 38 over 180-210k and recovered to 362 by 300k. The deaths are read at the colony level only, not ant by ant, but they follow the early mechanism: less food comes in.
- **It stays off.** By the 13:05 rule, a fix that kills a colony outright stays built but off.
- **The trip-only fix costs the colony too, later.** Mean ants are 12-30% below today's. The seed 4 colony fell from 521 ants at 220k to 1 at 300k (150 starved in 250-260k alone), and seed 3 starved 33-53 per 10k frames over 150-210k, dropping from 563 ants to 265. Traced in §11. Births 100-300k: 2,203/2,178/2,089/1,783 against today's 2,549/2,622/2,575/2,683.

**What it means.** Today's colony leans on an accident for its food. Ants holding door food wander to the heap under the same nest-scent hold that loses carriers. Bringing every laden ant home removes that before anything replaces it, and both versions of the fix stay off (a fix that kills a colony stays built but off until the deaths are understood). The gap the fix exposes is outbound: empty ants rarely walk out to the heap, and door food is not the packed lunch it was meant to be. That gap belongs to the food lane. Once ants walk out without holding food, the homing fix can be tried again on top. Why the trip-only version collapsed seed 4 late is §11.

## 11. Why bringing carriers home starves the colony (23:00-23:50)

**Short version.** Today's lost carriers are also the ants that eat their fill at the heap. A carrier in the nest-scent fog lingers at the heap and comes back to it, so it stays about 100 decisions per carry and half of them fill their crop. A carrier that is pulled home leaves after one or two bites. The colony then takes 15-50% less food off the heap, its foragers grow lean, and the mound workers (who never walk to the heap) live on what reaches the door. When the door runs short, they starve together. The home search does the same, more mildly. So the turn-for-home fix needs a partner: fill up before leaving the food.

**The seed 4 crash, ant by ant** (trip-only fix, re-run with every hungry ant recorded; the re-run's colony counters match the first run exactly).
- **The spiral.** Hungry ants (energy under the founding grant) rose from 167 at 236k to 368 at 252k. Carriers' energy when they picked up at the heap fell from a median of 600-990 J (208-232k) to 223 J (252k); today's seed 4 carriers start at 880-1,900 J. Lean carriers digest their one cell before they get home (a cell lasts about 1,200 frames in the crop), so door deliveries fell to 1-2 per 2k frames over 252-256k, against 10-142 per 2k before. 108 ants starved in 254-256k and 247 over 250-270k.
- **Who starved.** 219 of the 247 had foraged and 194 had delivered food; 57 were nest workers. In their last hunger (median 1,790 frames, from 0.75 of the grant):
  - 86% of their decisions were on the mound: by the door 47%, west of it 30%, east 9%. Inside the shaft 7%.
  - Pulled by nothing 35%, carrying soil out 32%, standing 27%, sent out by `HUNGRY_OUT` 3%.
  - 117 started it holding a soil pellet, and 65 picked one up on the mound during it.
- **Who lived.** Of 385 ants hungry in 246-250k, 69 lived past 270k. 55 of those 69 had been to the heap in 236-272k, against 98 of the 194 that starved.
- **Where the hungry wait.** Today's seed 4 keeps its hungry inside the shaft by the door (62-80% of their decisions, 100-260k), where `HUNGRY_OUT` sends them out (21-31% of their decisions). With the trip-only fix, from 160k on, the hungry sit on the mound: 21-56% inside the shaft (5% in the crash window) and `HUNGRY_OUT` on 6-21% (1%). On the mound nothing turns a hungry ant toward the heap (`mound-hunger-2026-10-05.md`).

**The cause, on every seed: carriers leave the heap with less.** Heap carries started 150-250k, seeds 1/2/3/4:

| Arm | Decisions at the heap per carry (median) | Filled the crop (6 cells) during the carry | Left the heap holding 1 cell | Food off the heap per ant, 150-250k (cells) |
|---|---|---|---|---|
| Today | 101/107/99/98 | 52/50/48/44% | 32/33/30/39% | 21.1/18.8/21.6/21.6 |
| Home search on | 87/73/98/78 | 38/34/42/45% | 38/42/39/35% | 16.3/17.5/19.9/16.8 |
| Trip-only fix | 21/38/30/31 | 12/17/15/20% | 50/47/53/53% | 15.0/17.1/15.3/14.3 |

- Food off the heap in total, 150-250k: today 11,309/10,287/11,461/12,103 cells; trip-only fix 7,270/8,729/5,681/6,788 (15-50% less).
- Today 73% of heap carries come back to the heap before the carry ends (seed 4); with the trip-only fix, 48%. The lingering is the nest-scent hold of §2-§4: a laden ant at the heap is held about 4x on every heading, so the pull home seldom wins there either.
- **Inferred, not traced:** a full carrier also has energy to spare for the door crowd. Today's carriers start their carries at 880-1,900 J; the hungry at the door are fed mostly where they stand.

**The home search, 12 seeds** (colony counters; seeds 1-4 from `home2/`, 5-12 from `home4/`, same binary).
- Mean ants 100-300k, off/on: 547/554, 551/541, 529/550, 573/527, 530/447, 537/433, 556/521, 599/449, 576/393, 555/557, 591/469, 553/503. On is smaller on 9 of 12; mean ratio 0.89.
- Fewest ants 100-300k fell below 300 on 5 seeds with search on (297, 160, 93, 91, 71) and on none with it off (lowest 359).
- Pickups 100-300k were lower with search on in 10 of 12 seeds.
- **It stays off.** This is the measured harm the "default on" rule asks for.
- Three of the five dips are bursts like seed 4's: 122, 187 and 224 ants starved inside 10-20k frames (seeds 5, 8, 11). Seeds 6 and 9 declined over 40-70k frames instead. On seeds 5 and 8, the colony counters show ants holding food falling to 25-46 before the die-off, against 73-182 with search off on the same seeds.

**Tested next (scratch, 23:50; results in §12).** `PIXEL_PHYSICS_FILL_FIRST=on`: a carrier on a trip with room in its crop does not step away from food beside it, so it takes the next mouthful before the walk takes it home. Foragers in a starved colony fill their crops fuller and feed longer at the source (Josens & Roces 2000, *Camponotus mus*, [doi 10.1016/s0022-1910(99)00220-6](https://doi.org/10.1016/s0022-1910(99)00220-6), via PubMed).

## 12. The real switch, and why seed 4 died under it (2026-10-06, 00:00-01:15)

**Build.** `PIXEL_PHYSICS_CARRY_HOME` on branch `claude/deep-trace-yacs73` over main b081040e: `fill` (a trip's carrier with room stands beside food beyond the trip reach of every door) and `turn` (a trip's carrier's trail hold counts presence above its weakest option, so even nest scent holds nothing). Unset is off and reproduces today's game exactly (stats rows identical to 16k, seed 1). Dry goal box, evolved founder, mutation off, nurses off, seeds 1-4, 300k, `deeptrace ants=0 hungry=1`. Paired against today's game from `home2/`.

**Seeds 1-3 hold; seed 4 died.** Over 100-300k:

| Seed | Mean ants, today / switch | Food delivered from trips, today / switch (cells) | Grown ants starved, today / switch |
|---|---|---|---|
| 1 | 547 / 492 | 8,018 / 9,276 | 12 / 62 |
| 2 | 551 / 514 | 6,788 / 14,125 | 55 / 14 |
| 3 | 529 / 526 | 7,821 / 13,636 | 41 / 15 |
| 4 | 573 / 449 (8 at 300k) | 8,154 / 5,778 | 14 / 374 |

The scratch fill-and-turn (no door test) held seed 4 and lost seed 1; this build lost seed 4. Both are one seed of four, so the switch stays off.

**How seed 4 died (traced from the colony census every 1,000 frames and every hungry ant's decisions).**
1. **Trip carriers stopped reaching the door at 245k.** Trip deliveries ran 22-56 per 1,000 frames over 239-244k, then 0-6 over 247-253k. Ants with 3+ crop cells inside the mound's tunnels went from 3-18 to 0 at 247k; they stayed on the mound top (19-21) and at the heap (6-10). Today's seed 4 keeps 17-38 laden ants inside the mound at the same frames.
2. **The colony was living on door food.** Hungry ants took about 500 meals per 1,000 frames at the door over 200-250k (median); it fell to 202 at 251k and 14 at 252k. 324 ants starved over 251-256k, a median 1,430 frames after their last meal. 273 of the 328 dead died in the mound's tunnels, and 159 spent most of their last hunger in the tunnels by the door.
3. **The mound over the door was jammed with soil carriers.** Ants holding a soil pellet in the mound's tunnels ran at up to three times today's from 80k: seed 4 at 200-240k had 213-288 (today 75-102), and in the 25x15 box over the door 128-167 (today 49-70). Each pellet was held far longer: over 100-240k, a median of 8,000 frames per pellet against 2,000 (mean 11,700 against 4,000), with 67% held 5,000+ frames against 29%. The map every 1,000 frames shows the mound over the door packed with ants and almost no open cells (`home10/on-s4-252000-maps`).
4. **Why they hold:** the soil haul's target is the door, which the mound covers, and `SpoilOut`'s `keep` will not let a pellet go under cover until the carrier's patience runs out. A carrier there is pulled to where it may not drop. They milled a median 14 cells from the door (hungry rows, 230-246k: 72% stepped, 413 ants).
5. **Why they starved:** ~~a hungry ant holding a pellet can't eat (`haul_bite_blocks`) and~~ `HUNGRY_OUT` acts only on an empty ant. At 250k, 320 of the 327 pellet holders were hungry. *(Corrected 04:38 2026-10-06: `HAUL_BITE` ships `on`, under which `haul_bite_blocks` never refuses, so a pellet holder eats food beside it. What the pellet takes away is every pull that leads to food until the ant is lean: `HUNGRY_OUT`, the hungry walk home, and the soil's own pull asked first. Nest building's finding, in `nest-race/findings.md`.)*

**Seed by seed.** The pellets held under the mound are up on seeds 1 and 4 only (seed 1: median 3,000 frames per pellet against 2,000; pellet holders in the door box 62-108 against 47-66). Seeds 2 and 3 match today. Trip food delivered on seed 4 was already 14% below today's over 100-240k, before the crash; on seeds 2-3 it doubled.

**Not the same death as `turn` alone.** With `turn` alone seed 4 also died in 250-270k, but by the heap route of §11: carriers left the heap with a bite or two and the colony took less food. There, pellets were held only a little longer than today (median 3,000 frames, 36% over 5,000) and the door box held 75-95 pellet holders.

**Not traced yet:** why the switch raises the pellets held under the mound on two seeds of four, and what starts the jam. The next step is to record every decision of the soil carriers in the box over the door on seed 4 (the walk recorder widened to the mound, as Nest race did for its trace).

**One gap closed in the build.** The fill's hold could keep a carrier beside food `act` would never take: food of another material than its load, or a `Feed` urge of 0. It now checks both. The test scene with `windfall` beside a `provisions` load went red without the check (held 74 times, its crop never grew) and passes with it. It changes the game: the fixed build and the first part at 14-19k, where the first had booked 5-18% more holds (`home10/hard-*`). So the four runs above are of the first build, and the fixed build is re-running on seeds 1-4 on merged main (`home12/`, §13).

## 13. On today's main: the switch, today's own collapse, and the mound trap (2026-10-06, 01:30-03:00)

**Build.** PR 638's head (953efb1c): main 33389072 (after #637, `NEST_REST=workers` on) plus `CARRY_HOME`. Unset is today's game. Same box and setup as §12. Scratch `home12/` (12 arms) and `home13/` (maps every 1,000 frames; `NEST_REST=off`).

**What happened, seeds 1-4.**

| Seed | Today: ants at 240k / 270k / 300k | Switch on: ants at 240k / 270k / 300k |
|---|---|---|
| 1 | 498 / 194 / 57 | 368 / 296 / 478 |
| 2 | 601 / 640 / 593 | 489 / 513 / 494 |
| 3 | 627 / 584 / 616 | 490 / 435 / 160 |
| 4 | 588 / 141 / 17 | 1 / 0 / 0 (495 at 180k, 93 at 210k) |

- **The switch still kills seed 4, now earlier.** 201 died in 187.5-190k: starved in the mound's tunnels (137 over 175-200k) and at the heap (71), many still holding soil. Seed 1 under the switch dipped to 287 at 210k and recovered. **Seed 3 under the switch fell late too**: 435 ants at 270k, 160 at 300k, 354 starved over the run against 39 today (not traced). Seed 2 was smaller with it (mean 536 against 613) while it delivered more trip food (9,835 cells over 100-300k against 6,839).
- **Today's game collapses too, on seeds 1 and 4, after 240k** (seeds 2 and 3 hold). On the previous main (b081040e era, §12's `home2/`), the same seeds held 553 and 586 at 300k.
  - Seed 1: eggs per 10k frames fell 183 → 35 → 18 over 220-260k, and trip deliveries fell 298 → 133 → 37. Old-age deaths held at about 120 per 10k, so the colony aged out. The 250k and 275k maps show the shaft filled with soil and brood, and every ant outside.
  - Seed 4: trip deliveries fell from 468 per 10k (200-210k) to 104 (240-250k), then 25. 232 starved in 260-270k, and the door stayed open. 278 grown ants starved above ground after 240k (`deeptrace_hunger.py starved`): 145 died west of the door, away from the food, and 62 east. West of home, with their scout on, 73% of their steps went away from home. That is the mound hunger of `mound-hunger-2026-10-05.md`, late in the run.
  - **It is #637's rest pull, on both seeds.** #637 changed only `NEST_REST`'s default, and `NEST_REST=off` on this build is bit-identical to the previous main's runs on every 1,000-frame counter (ants, births, trip deliveries, starved, pickups; `home2/off-s1`, `-s4`, 295 rows each). With it off, seed 1 ends with 553 ants (41 starved over the run) and seed 4 with 586 (24 starved). With it on (today), they end with 57 (189 starved) and 17 (426 starved).
    - The two arms track each other to about 230k (seed 1: 554 against 537 ants; seed 4: 612 against 572), then today's falls and the off arm holds. Trip deliveries over 230-300k: seed 1 529 today against 2,934 off; seed 4 549 against 1,930.
    - **All four seeds** (the off arm for seeds 2 and 3 is `home2/off-s2`, `-s3` by the same identity):

      | Seed | Rest pull on (today): ants at 240k / 270k / 300k; mean 100-300k; starved 20-300k | Off (before #637): the same |
      |---|---|---|
      | 1 | 498 / 194 / 57; 449; 179 | 542 / 567 / 553; 547; 12 |
      | 2 | 601 / 640 / 593; 613; 34 | 558 / 587 / 568; 551; 55 |
      | 3 | 627 / 584 / 616; 604; 39 | 485 / 513 / 568; 529; 41 |
      | 4 | 588 / 141 / 17; 495; 402 | 621 / 595 / 586; 573; 14 |

      Two colonies of four die after 240k with it on, none with it off; the other two are bigger with it on. The arms part at 8-9k frames, so each seed is a different world, and 2 of 4 against 0 of 4 is not proof (Fisher p ≈ 0.43). Both deaths come after the 200k it had been measured to.
    - **Not traced: how the rest pull leads to the collapse.** That is Nest building's (it owns `NEST_REST`).

**Correction (03:50-04:20).** Until 03:50, `tools/doorseal.py` counted brood and every food cell as wall. The walk goes through brood and crumbs (`PushPast`, both shipped on); corpses and provisions are walls to it. deeptrace's maps drew all food alike, so the seed 4 numbers below were re-counted on a re-run whose maps draw crumbs and corpses apart (scratch patch `home15/mapfood.patch`; the re-run matches the first run on all 51 counters). The seal story holds; the counts changed: shut on 20 of 25 maps over 176-200k, not 22; 380 ants shut in, not 400, 364 of them holding soil, not 381; 17 rolls at the reopening cells, not 12, with `FaceTrip`'s `only` refusing 11, not 8. Today's seed 4 never stayed shut more than 2 maps, as before.

**Why the switch raises the soil holders under the mound (inferred from maps and the colony census; not traced decision by decision).**
- In both arms the holders under the mound rise with the size of the mound's inside: the covered, open cells above the ground within 40 columns of the door, read from a map every 1,000 frames (seed 4, `holdvsmound.py`).
  - Inside 300-349 cells: today 46 holders, switch 58.
  - Inside 400-449: 77 and 85.
  - Inside 550-599: 126 and 216, with 602 and 665 ants.
- The switch's colony gets there sooner. At 100k it had 712 ants against 615. Its mound's inside was 572-601 cells over 106-136k, against today's 459-526, and 328-361 ants stood inside it against 214-254. The mound's top over the door reached row 142 by 106k, a level today's mound reached only at 136k.
- So the switch's colony is roughly 30k frames further down the same road: a bigger mound sooner, with more ants and more pellets trapped inside it. Today's game reaches its own collapse later, on the same two seeds.
- **The door seal is not why the holders rise, but on this build it is the final blow.** With a map every 1,000 frames to 175k, the door was cut off from the open air on 9 of 170 maps today and 12 of 170 with the switch, in spells of 1-3 maps (`tools/doorseal.py`, the walk's rule: see the correction below). Today's stayed at 9 of 195 to 200k.
  - Then, with the switch, it shut at 176k and stayed shut almost to the end (`home13/spells.py` on `home15/on-workers-s4-200000-dig`): shut 176-179k (4 maps), open at 180k, shut at 181k, open at 182k, shut 183-196k (14 maps, the longest spell of either arm), open at 197-199k, and shut at 200k (the colony was gone by 245k; the 225k and 250k maps draw all food alike, and read shut if the food in the way is corpses, open if it is crumbs). Over 176-200k it was shut on 20 of 25 maps. Trip deliveries per 2,000 frames went 101 (174-176k) → 25 (176-182k) → 4-5 (182-186k) → 0, and 192 starved in 186-190k. Today's game never stayed shut longer than 2 maps (`home15/unset-workers-s4-270000-maps`, 10 spells over 6-270k: five of 1 map, five of 2). What shut and opened it, on the walk from the door to the sky (`home13/sealpath.py`): at 176k, six cells of a tunnel at x 243-246, rows 153-157, four of them under ants at 175k, turned to soil; at 180k a soil cell on the walk became an ant's cell (a cut from outside at 179.7k), and at 197k a soil cell on it became air; at 181k and 200k, cells under ants turned to soil again (and one to a corpse at 200k; inferred: pellets put down in the tunnel, the lean drop).
  - **Did anyone try to dig through it? Hardly anyone** (Scott, 02:19: ants can dig through a sealed door, so only a seal that never reopens explains a crash). Re-run with the digging record (`home15/on-workers-s4-200000-dig`, `dig=1 mapevery=1000`, maps drawing crumbs and corpses apart; its 51 counters match `home12/on-s4` row for row, and 32 of its 195 maps are sealed by the walk's rule); readers `home13/digseal_an.py`, `plugkey.py`, `plugdepth.py`.
    - **The plug was thin.** The fewest cells a digger would have had to cut, from the door to the open air: 1 on most sealed maps over 176-200k, 2-3 at 183k and over 187-191k (`plugdepth.py`). On the sealed maps up to 186k, 8-23 cells of loose soil each touched both the door's system and open air joined to the sky, so one cut at any of them reopened the door. The sealed door's rim was soil (294-489 cells a map), with 0-5 corpses (`home15/pocketwall.py`).
    - **Over 176-200k, 17 dig rolls were aimed at those cells, by 13 ants** (7 rolls from inside, 10 from outside). 11 were refused because the digger was walking back to its own face (`FaceTrip`'s `only`, shipped on). Of the 6 cuts, the one at 179.7k opened the 180k map; soil sliding in filled others again within 26-215 frames.
    - **380 ants were shut in** (in the door's system on a sealed map), over 392,242 decisions. **75% of those decisions were by an ant holding a soil pellet (364 ants)**, and a pellet in the jaws means the dig is never asked (`not_asked` on every one of them). These are the soil carriers caught in the mound trap above, walking their pellets toward the door the mound buried. 23% were by ants with empty jaws (257 ants): the dig roll lost on 67% of those decisions, they were lean on 15%, 15% had nothing to cut, and 129 decisions cut a cell. Their cuts went into the mound's inner walls: 76 cuts on the system's face from inside, almost none on the cells that led out. Nothing points a shut-in ant toward the open air (inferred from the code: the dig's target is the cell ahead).
    - **The mound is loose soil that keeps moving.** Over 176-200k, in the mound (40 columns either side of the door, rows 120-159), 4,775 open cells filled with falling or sliding soil and 4,690 emptied, against 546 pellets put down and 372 packed cells cut.
    - The shaft under the door was filled with brood from 186k (rows 161-166) and with food and soil in its top cells from 190k (`map_f186000`-`194000`). Bodies walk through brood (`PushPast`), and `doorseal.py` now reads it so; the sealed verdict is about the mound above.
  - When the holders died, their pellets fell into the tunnels: soil put down was 226 in 186-188k against 51-72 per 2,000 frames before.
  - The first build's seed 4 shut on 74 of 247 maps, in long spells.
  - **Today's own seed-4 collapse does not start with the door shut** (`home13/unset-s4-270000-maps`, a map every 1,000 frames; its counters match `home12/unset-s4` row for row). Over 220-270k the door was open on 45 of 51 maps. It shut for two maps at a time at 246k, 252k and 260k, and reopened each time. Trip deliveries had already fallen from 236k. Soil in the mound grew from 649 to 877 cells over 220-261k, and food lying in it from 42-77 cells to 92-115 over 262-270k. So on this build the colony collapses by two roads: a shut door with the switch on (from 176k), and an open door with mound hunger today (from 236k). With the rest pull off, the switch's seed 4 lives (below), so its death on this build needs the rest pull; the first build's seed 4 died without it (§12).
- **The trap itself is Nest building's finding, and it matches.** A carrier in the mound's tunnels reads as inside, so it may not drop or lift. When its patience runs out it may drop, but no tunnel cell has the headroom, so it holds the pellet until it is lean. A hungry holder ~~can't eat (`haul_bite_blocks`) and~~ isn't walked out (`HUNGRY_OUT` takes empty ants), though it eats food beside it (`HAUL_BITE` ships `on`; corrected 04:38, Nest building's finding). Nest building has a probe that lays the pellet where the carrier stands.
- **Below ground (a smaller effect):** over 90-130k on seed 4, the switch's hungry soil holders made 75k decisions below the ground against 15k, with 7,135 falls at the bottom of a deeper room (around x 249, rows 192-195), where they climb the wall and fall back. Fewer than 8 holders stand below ground in any 1k sample, so this is a few ants stuck for a long time.

**The switch on main's game again (rest pull off; 03:00).** #639 (main db14d10fe) turned the rest pull off and changed nothing else, so `NEST_REST=off` on this build is that game (`home2/off-sN` the off arm, `home12/onrestoff-sN` the switch; reader `home13/tally14.py`).

| Seed | Mean ants 100-300k, off / on | Ants at 300k, off / on | Trip food 100-300k, off / on (cells) | Starved 20-300k, off / on |
|---|---|---|---|---|
| 1 | 547 / 546 | 553 / 461 | 8,018 / 9,191 | 12 / 31 |
| 2 | 551 / 506 | 568 / 150 | 6,788 / 7,567 | 55 / 431 |
| 3 | 529 / 507 | 568 / 412 | 7,821 / 16,656 | 41 / 27 |
| 4 | 573 / 623 | 586 / 456 | 8,154 / 14,392 | 14 / 11 |

- More trip food on all four seeds, and seed 4 lives. Its door was open on all 12 of its 25k maps.
- **Seed 2 falls instead**: 650 ants at 190k and 200k, then 593, 551, 489, 428, 357, 264, 275, 192, 157 and 150 at 300k. Its door was shut on the 200k map and open on the four after it (a map every 25k, so a spell between them would not show). Traced below.
- The other three end smaller than the off arm (461/412/456 against 553/568/586).
- The fill held carriers 13,118-19,566 times per run; the turn touched 2.0-2.8 million decisions.

**Seed 2's fall with the switch on, traced (rest pull off; 03:30-04:25).** Build 98fc554dd (main db14d10fe plus the switch, which is on main, off, since #638), `CARRY_HOME=on` against unset, seed 2, 300k: `home14/on-s2-300000-h` and `unset-s2-300000-h` (`ants=0 hungry=1 mapevery=1000`; the switch arm matches `home12/onrestoff-s2` on every counter), and the same two worlds again with maps that draw crumbs and corpses apart, the switch's with the digging record (`home15/on-off-s2-300000-dig`, `unset-off-s2-300000-maps`; all 51 counters match row for row). Readers in `home14/an/` and `home15/`.

- **The arithmetic.** Over 200-300k the colony went from 650 ants to 150: 692 births against 964 deaths of old age and 228 starved. Unset: 1,323 births, 1,313 of old age, 5 starved (563 to 568). Per ant alive, births ran 21% lower and old age 10% higher (an older colony after the switch's boom, 1,661 births over 100-200k against 1,299: inferred), and starving added 0.6 deaths per ant. Eggs: 1,014 against 1,789.
- **Who starved.** 431 over 20-300k against 55, in two waves: 174 over 120-140k (none unset) and 202 over 220-300k. All 118 grown ants (not nest-bound) of the first wave, and 140 of the 178 over 200-300k, died in the mound's tunnels.
- **Step 1: a pellet held until lean** (`SpoilOut`'s `keep` and `SPOIL_HEADROOM`: the trap above). Of the starvers whose last hungry spell ended with empty jaws, 101 of 104 in the first wave and 105 of 127 over 200-300k had held a pellet in it. At equal mound size the switch had twice the holders (mound inside 550-599 cells: 206 against 96; 450-499: 183 against 94), and its mound grew sooner (soil in it: 512 cells at 140k against 424; 840-881 at 240-280k against 602-630).
- **Step 2: the lean drop** (`LEAN_FORAGE`'s `drop`: the pellet goes in the first empty cell beside the ant with two of the three under it filled, no headroom asked). 99 of the 101 and 92 of the 105 were lean (under half their grant) at their last pellet decision. Pellets let go this way: 779 over 20-300k with the switch, 45 unset; 384 over 120-140k and 337 over 200-300k, against 0 and 1.
- **Step 3: shut in a pocket of the mound.** On each decision's 1,000-frame map, by the walk's rule (`tools/doorseal.py`: crumbs and brood open; soil, corpses and the heap's provisions walls), 59% of the starvers' empty decisions over 200-300k were in a pocket joined to neither the door nor the open air, and 105 of 127 died in one (2 in the open; 20 in a cell the older map drew as ground). In the first wave the door itself was shut (130-133k, 136-140k): 39% of decisions in the door's sealed system and 37% in a pocket; 67 died in a pocket, 30 in the sealed system. Hungry, empty ants in the mound who lived were in a pocket on 9% of their decisions and joined to the open air on 76%.
  - Ants in pockets above the door row, on the maps every 10k: 84 (130k), 107 (170k), 116 (220k), 76 (260k) with the switch; 0-50 unset.
  - **What closed them** (`home15/pocketclose.py`, every cell that turned, 100-300k): 156 pockets holding ants closed between one map and the next. The cells that cut their last walk to the open air were soil that fell or slid (285 cells), a pellet an ant put down (67) and a corpse (41). So the mound's own sliding soil shuts most pockets, and pellets let go shut about a sixth.
  - The pockets were tight: 1-2 usable headings on 46% of decisions, and after its drop an ant moved a median 5 cells in a median 1,610 frames.
- **Step 4: no way out.** A lean ant does not take its dig roll (`LEAN_FORAGE`'s `nodig`, `act`, creature.rs:15556). In their last hungry spell with empty jaws (the digging record), the lean gate took the dig on 82% of the first wave's decisions and 55% of the second's: nearly every decision on which the brain wanted to dig (urge above 0 on 88% and 60%). They cut almost nothing (under 0.5%), though a cuttable cell lay ahead on 65% and 52%, and no food lay beside them on any decision. And `HUNGRY_OUT` pulls a hungry ant only along the nest's way in, which starts at the door row and runs down (`build_nest_way`, :12416): 85% of the starvers' empty decisions over 200-300k were above the door row, and none scored the hungry pull out. 64% scored no pull, and on 36% the ant stood. The scout's pull was live on 64%, aimed at an anchor on the door row 11-40 cells away (58%) or further (28%), and had given up and was heading home on 53%; in a pocket no heading leads out.
- **The births (measured, not traced).** Fewer of the switch's ants cleared the lay bar (946 J): 10-28% of the colony over 200-260k against 24-41%, and 5-8% from 270k against 38-43%. They crossed it upward less often per ant (0.44-0.69 per 20k against 0.63-0.78), and over 200-260k 40-88% of those above it held a pellet, against 26-38% (`ready_to_lay` refuses a holder). The unset colony had as many or more rich holders (34-108 against 1-66); what it had more of is rich ants with free jaws (87-157 against 2-83). Why fewer of the switch's ants clear the bar is not traced.
- **It sits in the mound, so it is Nest building's** (handed over through the coordinator, 04:2x). Four rules meet there: a holder can't put its pellet down under cover; the lean drop lets it go in the tunnel; a lean ant shut in a pocket can't dig; and the hungry pull out doesn't reach above the door row. The pockets themselves are mostly the mound's sliding soil.

**What this means for the switch.** It stays off. The homing it fixes is real (§10-§12), but the colonies it grows walk sooner into the mound trap that today's colonies reach later. The levers are Nest building's: the soil way out through the mound, the pellet laid in the tunnel, the lean ant that cannot dig out of a pocket, and the hungry pull out that stops at the door row. After those, the switch is worth re-testing.

## Files

All scratch: the deep-trace session's scratchpad, `home2/` (8 runs) and `home3/` (seed 1 with every refused cell).

- **Readers (Python 3, no libraries):**
  - `pass1.py`: the funnel and the distance bands;
  - `runout.py`: the 22-step run-out, by where the pull was lost, with odds;
  - `turnodds.py`: the bands, with a check against the engine's odds;
  - `colony2.py`: the colony counters;
  - `picture.py`: the door picture;
  - `ways.py`: what filled refused cells.
- **§10-§11 (scratch `home5/`-`home8/`, `home4/`):** `home7/crash.py` (hungry ants per 1k and where they were fed), `fatal.py` and `fatal2.py` (the starved ants' last hunger against the hungry who lived), `hungrywhere.py` (where the hungry wait, and `HUNGRY_OUT`'s share), `carrierseries.py` (carriers' energy at pickup), `leaveheap.py` (time at the heap and crop at leaving, the §11 table), `topups.py`, `doorfood.py`; `home4/tally.py` (the 12-seed search tally). The seed 4 crash re-run with every hungry ant is `home7/trip-s4-300000-h`, and a map every 1k frames is `home7/trip-s4-300000-m`.
- **Recorder patch:** `/mnt/project-files/laying/nurse-stay/deeptrace-617-runs/recorder/laden-recorder.patch`. The refused-cell columns are a further scratch edit that is not pushed.
- **§12 (scratch `home9/`, `home10/`):** `home9/on-s*-300000-c` (the real switch, both parts), `home10/on-s4-252000-maps` (seed 4 again with a map every 1,000 frames; its counters match `home9` row for row), `home10/fill-s*-300000-c` (`fill` alone); readers `home9/tally4.py` (the table), `home9/series.py` (counters side by side), `home9/spoilseries.py` (pellet holders under the mound), `home10/doorbox.py` (the box over the door), `home10/spells.py` (how long each pellet is held), `home9/carriers.py`, `home9/zonesE.py`, `home9/spoilrows.py`, `home7/crash.py`, `home7/fatal.py`.
- **§13, seed 2 and the corrected seed 4 (scratch `home14/`, `home15/`):** `home14/an/` (`series2.py`, `starvedwhy.py`, `afterdrop.py`, `emptywhy.py`, `trapped.py`/`trapped2.py`, `leanphase.py`, `rich.py`, `getrich.py`, `pockets.py`); `home15/` (`pocketwall.py`, `pocketclose.py`, `digwhy.py`, `run.sh`, `mapfood.patch`: deeptrace's map draws crumbs `c` and corpses `x`).
- **Reproduce:** runs are deterministic per build and seed. The command is `env RAYON_NUM_THREADS=1 PIXEL_PHYSICS_NURSE_STAY=off PIXEL_PHYSICS_HOME_SEARCH=<off|on> deeptrace scenario=nest_goal seed=<s> frames=300000 ants=0 founder=evolved laden=1 hungry=1 out=<dir>`.
