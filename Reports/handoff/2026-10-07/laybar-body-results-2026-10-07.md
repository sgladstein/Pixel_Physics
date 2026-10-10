# LAY_BAR=body: first results (Nest race, 2026-10-07 04:10)

Build 95d65cd6. Seeds 1-4 to 150k, read over 100-150k. Each arm is compared with its baseline at the same heap distance.

| Arm | Store | LAY_BAR | Heap |
|---|---|---|---|
| b30off | off | body | 30 |
| b30smell | smell store arm | body | 30 |
| b90off | off | body | 90 |
| b90smell | smell store arm | body | 90 |

Tables: `scorecard-b30.txt` (off, b30off, skysmell, b30smell) and `scorecard-b90.txt` (g90off, b90off, g90smell, b90smell).

## The switch does its job (measured)
Eggs from layers left under 200 J after laying, 40-150k: **0%** on every body arm checked (b30off s3, b30smell s1 and s3, b90smell s4). Without the switch: 7-22%. Eggs from layers that ended between 200 and 1,000 J: 0-1%.

## Starved 20-150k, against the baseline (traced = per ant)

| | s1 | s2 | s3 | s4 |
|---|---|---|---|---|
| off, heap 30 | 6 | 0 | 0 | 213 |
| b30off | 1 | 9 | **549** | 2 |
| skysmell, heap 30 | 1 | 3 | **1,801** | 2 |
| b30smell | **90** | **87** | 4 | **117** |
| g90off | 9 | 34 | 15 | 1 |
| b90off | 1 | 0 | 2 | 2 |
| g90smell | 186 | 49 | 136 | 8 |
| b90smell | 201 | 86 | 131 | **154** |

## Reading so far
- **Heap 30, store on, seed 3: the heap-laying boom is gone.** 4 starved against 1,801. Colony 650 against 1,092. No heap eggs (heaplay 0 every 10k). Fed and staying deep: 20.3 against 29.5.
- **Heap 30, store off, seed 3 booms with the switch on** (traced to the layer). The nest reached the heap at about 50k, and **rich** ants laid there.
  - 1,148 heap eggs, all from layers still at 1,000 J or more after laying.
  - Colony 2,086 by 150k; 585 starved.
  - This is the residual Laying's review predicted ("fat foragers still lay at the heap once the nest reaches it"). It is not self-limiting by 150k.
  - The store-off baseline seed 3 never reached the heap, so this is a different dig history. Whether the switch made heap contact likelier is untraced.
- **Store on (heap 30 s1, s2, s4; heap 90 s4): 87-154 starved, against 1-8. Mostly deep beside the brood pile** (starvewhere; its "encased" was a tool fault, see the 04:30 UTC correction):
  - b30smell s1: 89 of 90, trapped a median 6,300 frames, at 113 J when first found trapped.
  - b30smell s4: 89 of 116.
  - b90smell s4: 153 of 153.
  - **b30smell s1, 55-70k (when 51 of them died):**
    - Hungry ants deeper than 10 rows logged 266k decisions (416 ants), against 77k (316 ants) without the switch.
    - 25% of those decisions were a fall, against 17%.
    - The pull was the walk-out (23%) or none.
    - Colony 873 against 654 at 65k; brood 416 against 292.
    - In the maps, the crowd sits at the foot of the room beside a tall brood stack.
  - **UNTRACED:** why births went up with the switch on these seeds (eggs 40-150k on s1: 2,603 against 2,293), and why hungry deep ants could not climb out (falls).
- **Heap 90, store off:** starved 0-2 against 1-34. Colony 556-596 against 557-671.
- **Ants living deep** (fed, deep 1,000 frames earlier), body against no body:
  - Heap 30, store on: 26.4 / 21.6 / 20.3 / 24.7 against 10.9 / 6.7 / 29.5 / 15.3.
  - Heap 90, store on: 24.1 / 16.6 / 23.3 / 32.5 against 26.2 / 17.9 / 8.9 / 18.7.

## Verdict for now
- **Not a default.** It fixes the food-paid laying it targets, and the heap-30 store-on seed 3 die-off.
- **But** it is followed by new deep starvation (not encased: see the 04:30 UTC correction) on 4 of 8 store-on runs, and by a rich-layer heap boom on store-off seed 3. Neither is traced to its cause yet.
- **Next:** trace b30smell s1 ant by ant, before any change. Why are there more births? Why do the deep hungry ants fall and get sealed in?

## Trace: b30smell s1, the 53 starvers of 55-75k (04:30, hungry log, each ant's last 8,000 frames)
- **They tried to climb out and fell.** Median 423 falls per ant (max 539).
  - Highest row reached: median 170, the room's top. Ground is 160, and the shaft runs 160-166.
  - Pull: walk-out 23%, otherwise not scored or none.
- **The walk-out pull sent them up the room's left side.** Its step targets cluster at x 240-248, rows 180-192. They were in a lobe at the room's lower left (x 236-252, rows 187-202; map at 62k).
- **The brood sits under the shaft.** On the 62k map the brood is one column, 8-10 wide, at x 252-261, rows 170-211, directly below the mouth (x 254-258). The way up from the lobe:
  - climbs the open left wall (rows 173-186), then
  - crosses the open top of the room (rows 166-169) to the shaft.
  - Ants fall off on the way. (Inferred: the column blocks the direct climb. Not checked whether ants can stand on brood.)
- **The colony and brood are larger than without the switch** at the same time: 873 ants and 416 brood at 65k, against 654 and 292.
  - **More eggs from rich layers:** 1,464 against 501 eggs at 6-60k from layers still at 1,000 J or more after laying. Total eggs 1,476 against 1,014.
  - Deliveries home are higher too: 15,229 against 9,385 by 60k.
  - **Why removing poor layers gives more rich layers is untraced.** One reading (inferred): ants beside food no longer spend 120 J eggs at low energy, so they fatten past the bar and lay steadily from there.
- **So the deaths look like a room-shape problem.** Hungry ants in a big open room with the brood stacked under the mouth cannot climb out. The switch made it likelier by growing the colony and brood. Both links need a check: whether brood blocks climbing, and why there are more rich layers. Nothing proposed.
- **Correction from the fall census (04:45). The room-shape reading above is wrong in its detail.**
  - The support rule (`body_is_supported`) counts powder as footing, so ants can stand on brood. The brood column is not a wall to them.
  - **Where the 19,669 falls happened:** inside the crowd itself, at x 240-250, rows 183-195. Every fall drops one row.
  - A living nestmate is not footing for a species that does not climb. So an ant on top of the pile falls one row onto the next ant: this is a churning pile of ants, not a climb that fails.
  - The walk-out targets lie in the same lobe. The crowd sits on the way out and blocks it.
  - **Reading (inferred):** a pile of hungry ants jams the lobe the way out runs through. It is the same kind of jam as skymeal s1's shaft crowd at 100-106k.
  - **Untraced:** why the way out runs through this lobe, and why the crowd formed when it did (51 deaths at 60-70k).
- **Why the pile formed there (about 04:05 UTC, maps): the store is in that lobe.**
  - Food cells below row 166 sit at x 237-252, rows 193-206: 22 cells at 50k, 21 at 55k, 9 at 60k, 9 at 65k.
  - Ants below row 180 went from 33 at 55k to 160 at 60k as the store fell from 21 cells to 9.
  - Starvers' last 1,000 frames: walk-out pull 45%, not scored 50%, fell 32%.
  - **Reading:**
    - The store draw (smell=10) brings hungry ants into the dead-end lobe where the store lies. With the switch on, the colony is about a third bigger (873 against 654), and more of them are near the store.
    - When the store runs down, about 150 hungry ants are turned out at once through the lobe they fill, and the pile jams the way out.
    - This is the arm-1b eat-pull crowd at a smaller scale. smell=10 limits who is drawn, not how many.
  - **Measured:** store size, ant counts, the pull mix and where the falls happened.
  - **Inferred:** that the switch's bigger colony is what tips it. On skysmell s1 at the same frames, the store and ant counts were not read.
- **Same frames without the switch (skysmell s1), measured:**

  | Frame | Store cells, switch off | Store cells, switch on | Ants below row 180, off | Ants below row 180, on |
  |---|---|---|---|---|
  | 50k | 8 | 22 | 63 | 77 |
  | 55k | 7 | 21 | 22 | 33 |
  | 60k | 10 | 9 | 14 | 160 |
  | 65k | 6 | 9 | 42 | 153 |

  - With the switch on, the store grew bigger, then fell from 21 to 9 cells, and the crowd rose from 33 to 160 ants as it fell.
  - The larger store went with more deliveries home (15,229 against 9,385 by 60k).
  - Untraced: why the crowd arrives as the store falls. The store pull and its smell gate need checking at those frames.
- **Why the crowd formed (04:10 UTC, traced). It is a hatch wave of new ants, born deep beside the store. They are not hungry ants walking in.**
  - **Who:** 144 ants were in the deep west room at 60k. 78 of them were born at 56-60k (ledger). At 57k only 70 of the 144 existed, and 8 were deep.
  - **The wave:** births per 4k frames, with the switch on, ran 69-91 at 44-56k, then **160 at 56-60k**. Without it: 34, then 25.
    - At the same time larvae ate 147 kJ and were nursed 43 kJ, against 66 kJ and 7 kJ in the 4k before.
    - Larvae on the map went from 113 to 324 between 57.5k and 60k. The brood column under the mouth (x 252-262) widened from about 5 cells to about 10.
  - **Fed from the store (inferred):** store food fell from 29 to 9 over the same frames. Before the wave the switch's store held 16-36 food cells, against 2-8 without it.
    - So the switch's first effect is a full store, because eggs no longer spend food that is merely in reach.
  - **New ants start low and boxed in:** the crowd's energy at 60k was 171-182 J (quartiles). That is under start energy, so NEEDS_FIRST counts them hungry where they are shut in.
  - **They dig and cannot put the soil down:**
    - 101 of the 144 held a pellet at 60k.
    - Cuts in the store lobe were 283 per 2k frames at 58-60k, against 5 without the switch. They cut wall (214), store crumbs (121) and soil (74).
    - Load-down failures for want of a free cell (`needs_no_site`) were 102 in the 1k frames to 59k, against 16-49 without the switch.
    - About 36% of their moves pulled up the soil way out, and 25% were falls. Ants fell off ants, as found above.
  - **Who died:** 32 of the 53 starvers at 55-75k were born at 56k or later. The median age at death was 10.9k frames.
  - **Side finding (not caused by the switch; every store arm has it):** ants dig store crumbs as if they were soil. Crumbs name no spoil material, so the pellet stays food and is carried out on the soil way as spoil. Food cuts by 150k: store on, switch off 639-954; store on, switch on 1,746-2,481. Untraced where those pellets end up.
- **Is the hatch wave the general cause? No. It is seed 1's chain.**
  - **Other seeds:** starvers born within 8k frames before the biggest birth wave were 23 of 87 (s2) and 39 of 116 (s4). At heap 90 it was 0-8 per seed.
  - **The common pattern across every switch-plus-store arm:** deaths are in the nest (86-149 of 87-153 per seed), and many of the dead never foraged (37-91). At heap 90 without the switch, half died on the surface (84 of 180 on s1; 89 of 133 on s3).
  - **Reading:** with the switch on, young ants starve inside a nest whose store is fuller than without it.
  - **Next trace:** why never-foraged ants starve deep beside a store. Are they encased, crowded, or barred from eating (`keep`, the smell gate)? Class each cluster as crowd or famine. Nothing proposed.
- **Seed 4 at heap 30, the 74 deaths at 28-30k (04:20 UTC, traced). It is the same chain as seed 1: new ants fill a room they cannot leave.**
  - **Is the store's food too small to eat? No (measured).** A scratch probe counted store cells that clear the bite's own yield gate (`EAT_YIELD_THRESHOLD`), using the founder's gut. Rerun e30-s4 is identical to b30smell-s4 through 34k.
    - 3-15 store cells over 14-35k; all but 0-1 were edible. Worth: 860-10,500 J.
    - So the store was small, but edible.
  - **Who was in the room** (rows 166 and below, under the shaft):
    - With the switch: 79-112 ants at 22-28k. Without it: 3-14.
    - 57 of the 99 there at 26k were born at 20-26k, 42 of them at 24-26k. 64 of the 99 starved.
    - With the switch, births by 24k were 404, against 287 without. Eggs were fewer at 16k (199 against 236) and more by 24k (577 against 441).
  - **They could not leave:**
    - The shaft (x 254-258, rows 150-165) was packed in both arms: 55-73 ant cells of 80 in every map from 18k to 34k.
    - The starvers' last 2,000 frames: walk-out pull 51%, no store pull at all, falls 25%. They died at the shaft foot, rows 166-184, 10-25 rows from the store cells.
  - **Class: famine in a room joined to the door but jammed.** Food was 10-25 rows away, the store was small, and the shaft was full of ants. It is not a crowd at the food. (Not sealed: see the correction below.)
  - **Reading (inferred), seeds 1 and 4 together:** the switch lets more brood hatch, and the new ants hatch deep. They start under start energy in a room whose one exit, the shaft, is already jammed, and whose store is a few cells. Without the switch, the room held 3-14 ants.
  - **Untraced:**
    - Why births run ahead with the switch on seeds 1 and 4 (s4: 404 against 287 by 24k). That needs eggs and larval survival per 2k.
    - Why the shaft is always packed. That is the old shaft jam, in both arms.
  - Probe: `PIXEL_PHYSICS_STORE_EDIBLE=1`, scratch, on claude/nest-race-store-stack (uncommitted). Heap 90 seed 4 rerun (e90-s4) is still running.
- **Why births run ahead on seed 4 (04:25 UTC, partly traced).** Larvae do not starve in either arm before 40k: 0 per 4k. So the extra births come from extra eggs, not from more larvae surviving.
  - **Eggs per 4k,** with the switch against without:
    - 12-16k: 134 against 122.
    - 16-20k: 182 against 113.
    - 20-24k: 180 against 88.
    - 24-28k: 17 against 78.
    - 28-32k: 0 against 83.
    - That is boom then bust. Without the switch, laying runs steadily.
  - **Who laid.** Eggs are de-duplicated by egg id, and the parent's energy is read at the first sighting.
    - With the switch, layers were fat: median 1,388-1,688 J, against 813-1,444 without.
    - Most eggs were laid on the mound top: 117-146 per 4k.
    - At 20-24k, one layer laid 51 eggs.
  - **Reading (inferred):** under the body bar, an ant that fattens past 946 J lays until it is back under the bar, so fat ants lay in runs and the colony lays in bursts. Each burst hatches into the room below the jammed shaft.
  - **Untraced:**
    - Where the layers fattened: the heap is 30 cells east, and the store is a few cells.
    - Why laying then stops dead: no layer above 946 J? Their energy needs reading at 24-32k.
- **Correction (04:30 UTC, Deep trace's fixed flood, starvewhere_deep.py). No deaths in these arms were encased.**
  - The old tool flooded only to row 175, so every deeper ant read "encased".
  - Re-read with the fix:
    - b30smell s4: 116 of 116 starvers were in the door system with the door open.
    - b90smell s4: 148 of 153 (97%), and 4 encased.
    - skymeal s3: 1,909 (83%) in the door system, 382 off the map, 2 encased (the review had quoted 1,701 encased).
  - Every "encased deep" in this file and in README.md means the old tool's fault.
  - The seed 4 room was joined to the door. What blocked the way out was ants packing the shaft, not ground.
- **Why laying stops dead on seed 4 (04:35 UTC, traced). It is not a lack of fat ants. Fat ants are shut outside by the packed shaft.**
  - **Ants at or over the 946 J bar,** with the switch on: 89 at 24k, 123-149 at 26-34k. Most were on the mound top (79-115) with food in the crop, and almost none were in the nest.
  - Eggs at 28-36k: 0.
  - **The gate that held them:** `buds_held_for_nest`, meaning no empty home cell within 4 steps to put an egg (`brood::pile_site`). It ran at 36,885-58,324 per 2k frames at 26-36k, against 13,062-24,347 at 18-24k. `lays_declined`, `births_denied_no_space` and `food_brake_held` were all 0.
  - **Without the switch, layers inside the nest keep laying.** At 28-32k, 64 of 83 eggs came from ants in the nest. Those ants can lay on food in reach, under the 946 J bar. With the switch, only fat ants lay, and the fat ants are on the mound top.
  - **The picture at 28k on seed 4:**
    - Fat foragers sit on the mound top and cannot lay or get in.
    - The shaft between is packed with ants (55-73 of 80 cells).
    - About 100 newborns starve in the room under it, 10-25 rows from a small store.
  - **Untraced:**
    - Why the shaft packs, which happens in both arms.
    - Whether fat ants on the mound top would feed the room if they got through.
  - **Who was in the shaft** (heads at x 253-259, rows 150-165, colony census):
    - With the switch at 28k: 147 ants, median 184 J (quartiles 114-258), 37 holding a pellet.
    - Without it at 28k: 120 ants, median 311 J.
    - So with the switch the shaft is filled by hungry ants climbing from the room, and fat ants on the mound top do not come down. Inferred: two-way traffic in a 5-wide shaft. Deep trace's fallwhere.py found no falls in the door shaft on seed 1.
- **Heap 90, seed 4, with the edibility probe (04:40 UTC).** The rerun e90-s4 is identical to b90smell-s4 through 134k.
  - **Store food was edible again.** Over every census from 20k on, 1,122 of 1,313 store cells cleared the bite gate. The inedible ones are crumbs worth almost nothing: 99% of the worth was edible.
  - **The deaths at 118-128k (127):** the store held 309-1,979 J in 5-12 cells at 118-126k, then refilled to 8,670-12,651 J at 130-132k.
  - 93 of the 153 starvers died within 5 cells of inside food. A focal starver (died 129.8k) swung between the store pull and the walk-out pull in the store lobe (x 240-250, rows 185-197) for 3,000 frames. It never ate, while its energy fell.
  - **Class: a crowd at a store too small for it**, which then refilled after they died. Untraced: why the store ran low then (trip food at 110-130k) and why this ant never got a bite. Was the food under the crowd? Or eaten first by others? That needs the bite log.
- **Did the starvers feed themselves to the larvae? Not below their start energy (04:42 UTC).** In their last 10k frames, starvers gave larvae a median of 65-171 J per run (b90smell s4: 124 of 153 gave, 35 kJ in all). But both ways of giving pay only from surplus over start energy:
  - nursing from the bank: `brood::nurse` skips donors at or under start energy, and gives a fraction of the surplus;
  - the brain's share to a larva: an ant at or under start energy is redirected to adult kin, and the amount is a fraction of the surplus.
  - So this spends food that went into fed ants. It does not push a hungry ant under. Whether that surplus would have kept them alive longer is not measured. Over 118-130k on b90smell s4, larvae got 281 kJ from adults by share and bank, against a store of 0.3-2 kJ. Where food goes at the nest in that window is the next question.
- **The store pull's 8-cell edge, checked on the other death clusters (05:55 UTC, Deep trace's storeedge.py over each death window).**
  - **b30smell s2, 47 starved at 92-112k: the edge, cleanly.**
    - All 47 had a store pull in their last 15k frames. Their last one fell at 104,626-104,726 (p10-p90).
    - Energy then: a median of 0.43 of the grant. They died a median 1,800 frames later.
    - The store was 12 cells at 103k, after 461-570 bites per 1k frames, then 5 at 104k. Store pulls went 1,238 → 16 per 500 frames.
  - **b30smell s4, 81 starved at 16-34k: the edge, more spread out.**
    - 71 of 81 had a store pull. Their last one fell at 24.8-26.8k, with energy at a median of 0.85 of the grant. They died a median 3,780 frames later.
    - The store had dropped to 2-6 cells from 23k, after 287-368 bites per 1k frames. These are the ants under the packed shaft, above.
  - **b90smell s4, 125 starved at 110-140k: NOT the edge.**
    - All 125 were still on the store pull at the end: the last one came a median of 10 frames before death, at 0.01 of the grant.
    - The store hovered at 6-24 cells, with store pulls of 600-3,400 per 500 frames.
    - This is a crowd held at a store too small for it, as above.
  - **Reading:** two kinds of death share one gate.
    - Where the store drops under 8 cells, the colony-wide gate releases a whole cohort onto the walk-out at once, with 0.4-0.85 of a grant left.
    - Where it stays above 8, the gate holds a cohort at the store until they die.
    - Both are the colony-wide count deciding for every ant. The code's own comment calls 8 "a guess", and the rule it replaced (1-2 crumbs drawing everyone) starved 397 on main seed 4.
  - **Still untraced, and it decides any fix:** why released ants with 0.4-0.85 of a grant fail to get out and eat. On s4 the shaft was packed. On s1 (Deep trace) they climb and fall back in the open room. Nothing proposed yet.
- **After the release: what the s2 and s4 starvers did in their last 6k frames (06:05 UTC, hungry log).**
  - **They did not get out.**
    - s2: 2 of 47 came within 2 rows of the ground line. Their shallowest point had a median of 13 rows down, and they died a median of 24 rows down.
    - s4: 17 of 81 came that close. Shallowest median 5 rows, death median 19 rows.
  - **Last 2k decisions:**
    - Up-steps equalled down-steps: 16,265 against 16,121 on s2, and 25,040 against 24,340 on s4.
    - Falls were 27% (s2) and 21% (s4) of decisions, on top of the down-steps. So the walk-out made no headway.
    - Pulls: walk-out 22-30%, soil way out 20-18%, not scored about half.
  - **s2's starvers were often holding soil at the end:** 37% of their last 2k decisions held a pellet. They were empty on 58%, and carried food on 5%. Deep trace found 0% on s1, so this differs by seed.
  - Inferred: a hungry ant with a pellet and no free cell to drop it (`needs_no_site`) is stuck on the soil way out. That is a second load on the climb, beside the falls.
- **Released ants followed to an outcome (06:15 UTC, Deep trace's releasefunnel.py).** Earlier window = control on the same run. Raw tables are in /home/claude/runs (Nest race disk); the numbers are here.
  - **b30smell s2:**
    - Released at 90-104k: 282 ants, 95% reached the top, 1 starved.
    - Released at 104-106k: 89 ants, 86 of them at 0.3-0.5 of the grant, 26 rows deep. 55% reached the top, at a median of 1,045 frames against 358. **38 starved without reaching the top**, and 5 after.
    - Same link as Deep trace's s1: let go late, low, together, and they cannot climb.
  - **b30smell s4, a different link:**
    - Released at 23-28k: 159 ants, 149 at 0.7 or more, 33 rows deep. 92% reached the top, and only 11 starved without reaching it.
    - **60 starved after reaching the top**, 32 of them after being fed once.
    - "Top" includes the shaft (rows down to 10). These are the ants that died in and under the packed shaft. The climb worked; food at the top did not reach them.
  - **b90smell s4, the hold:**
    - Released at 118-136k: 336 ants, 170 of them under 0.3 of the grant, 33 rows deep. Of those, 18% reached the top, and **124 starved without reaching it**.
    - Released at 0.7 or more (125 ants, 2 rows deep): 100% reached the top.
    - At 100-118k it was 104 released, and 24 starved without reaching the top, all under 0.3.
    - So the store held them until they were too weak to climb.
  - **Reading:** on s1, s2 and heap-90 s4, the deaths are ants let go by the colony-wide count at low energy, deep. On s1 and s2 the release is late; on heap 90 the ants are held long. Ants let go at 0.7 or more nearly all get out. Seed 4 at heap 30 is famine at the top: a packed shaft with no food reaching it. A per-ant release would not fix it.
- **Deep trace's review of the give-up proposal, and what it changed (06:35 UTC).** The review is `nest-race/store-give-up-review-deep-trace-2026-10-07.md`.
  - **s1 and s2 were not "held, then let go".**
    - On s2, before their last store pull (`releasefunnel --before 5000`), the 43 starvers spent 7% of decisions on the store pull, 30% on the soil way out, and held soil 55% of the time. Their walk-out had started a median 2,585 frames earlier, at 0.85 of the grant.
    - The 46 who lived look the same.
    - So the "release" at 104.6k is the store pull flickering over ants that were already failing to climb out holding soil. **s1 and s2 drop out as evidence for a give-up rule.**
  - **Heap 90 s4, storewait.py.** Hunger episodes first drawn by the eat pull at 100-118k:
    - Fed deep: 339, on the pull p50/p90 115/855 frames.
    - Starved: 31, on the pull p50 1,275 frames.
    - At 118-136k: starved 116, on the pull p50 672 frames; fed deep p90 800.
    - A 1,000-frame clock on pull time would cut about 10% of the fed-deep waits, and would catch fewer than half of the 118-136k starvers. **Weak.**
  - **The heap-90 access problem: the store is being buried in spoil.**
    - The map at 129k shows the west store lobe (x 235-252, rows 185-210) filled with spoil, with food cells inside it. A crowd stands on top.
    - Spoil drops into that lobe per 5k frames at 100-135k, switch on against off (g90smell s4): 330/132/213/147/39/91/20 against 27/79/87/46/30/63/12.
    - **Who dropped it (cells.csv joined to the hungry log):** 823 of 851 drops at 100-125k were by hungry ants. Their energy quartiles were 0.488/0.498/0.500 of the grant, which is the LEAN_FORAGE line where NEEDS_FIRST makes a soil holder put its load down where it stands.
    - **Deep food cells with an open or ant-filled neighbour, against all:** 7/15, 15/20, 12/19, 8/17, 14/22, 18/36 at 110-130k. Roughly half are sealed in spoil.
    - **Reading (inferred):** soil carriers that dip under half a grant drop their pellets on the spot. At the store lobe that buries the store and packs the crowd onto it, so ants on the eat pull walk toward food they cannot reach.
  - **Proposal status:** the give-up rule is on hold. The heap-90 cause looks like the need-drop burying the store. Next: confirm need-drop as the drop path at the store (a decision trace with spoil_why at the lobe), then propose against it.
  - **Baseline for "sealed in spoil" (06:45 UTC). It is weaker than I wrote.** Deep food cells with an open or ant neighbour, as open of total:
    - Switch on, before the drops rise (90-105k): 16/18, 22/33, 20/27, 14/18, so 67-89% open.
    - Switch on, at 110-130k: 47-75% open.
    - Switch off (g90smell s4), 80-130k: 33-100% open, out of only 2-9 cells.
    - So with the switch the store is bigger, and the open share falls somewhat as drops rise. **"Half the store sealed" is not distinct from the switch-off run's share.** The burial reading stays a hypothesis.
    - Next, with Deep trace's lobedrops.py on a rerun with the decision log kept:
      - what each drop laid (soil or food);
      - spoil_why ("need" expected);
      - where the pellets were dug (in the lobe, or carried down);
      - what the droppers were doing before.
- **What fills the heap-90 store lobe (07:00 UTC, measured on rerun dl90-s4).** The rerun is b90smell s4 with the decision log kept, filtered to the lobe box (x 233-254, rows 183-212), 100-125k. Its stats are identical to b90smell s4.
  - **Drops into the lobe at 100-125k: 1,006.**
    - 861 spoil, 144 soil, 1 packed soil. No food was laid.
    - 1,004 were `spoil_why` = **need**: NEEDS_FIRST's hungry put-down. 1 was a store load.
    - That is 56% of the colony's 1,794 need-drops in the window.
  - **Where the pellets came from:** 834 were dug in the lobe itself, 165 elsewhere. The drop came a median 120 frames after the dropper's own cut (quartiles 20-315). The dropper's energy was a median 0.499 of the grant (quartiles 0.491-0.506), and its pull was store 32%, walk-out 27%, none or not scored 39%.
  - **Who cut in the lobe:** 1,058 cuts.
    - 1,005 by nest workers, at 0.52-0.66 of the grant (quartiles). 971 of 1,058 were under the grant.
    - Material: wall (packed soil) 601, soil 263, spoil 91, store crumbs 68, corpse 35.
    - Pull: soil way out 43%, not scored 51%.
  - **The chain (measured link by link):**
    - Nest workers just above half a grant keep digging at the store.
    - About 120 frames later they cross the 0.5 line, and NEEDS_FIRST makes them put the pellet down on the spot, in the lobe.
    - The lobe turns over in place: walls cut, the spoil dropped back inside, and food crumbs cut as soil.
  - **Inferred, not measured:**
    - that this churn is what keeps the starvers from the food (the open-food baseline above is weak);
    - that the store pull walks ants into the churn.
  - **Two candidate levers, neither proposed yet:**
    - A hungry nest worker does not dig, or a nest worker on the eat pull does not dig.
    - A need-drop does not take a cell beside food.
  - Which one matters needs a count: how many starvers were themselves diggers or droppers there, against ants walking in.
  - **Were the starvers the churners? Mostly yes, for those the log covers (07:05 UTC).**
    - Of the 26 heap-90 s4 starvers that died by 125k (the rerun's end), 20 had cut in the lobe and 17 had dropped there: 122 cuts and 143 drops between them.
    - Over all 125 starvers at 110-140k, 47 had cut in the lobe by 125k. The rest died after the log ends, so it cannot see them.
    - The lobe's churn was 206 cutters making 896 cuts.
    - Reading: many of the ants that starve beside the store spent their last stretch digging it and dropping the soil, under their grant, rather than eating. The lever this points at is the digging, not only where the pellet goes.
    - To confirm on the 99 later starvers, the rerun has to go to 140k.
  - **Rerun to 140k: the later starvers were churners too (dl90b-s4, frames 118-140k, identical to b90smell s4).**
    - 73 of the 125 starvers cut or dropped in the lobe in their last 10k frames: 120 cuts and 157 drops between them, against 923 cuts and 921 drops in the lobe overall in that time.
    - Deep trace's `lobedrops.py --digrows`, pellets put down within 3 cells of deep food, by 5k window:

      | Window | Put down by food | No decision row | `need` (with row) | Dug within 5 cells | Food cells open / total |
      |---|---|---|---|---|---|
      | 118-123k | 21 | 15 | 6 (energy 0.40-0.61) | 16 | 12 / 19 |
      | 123-128k | 20 | 16 | 4 | 18 | 8 / 17 |
      | 128-133k | 126 | 123 | 3 | 102 | 18 / 36 |
      | 133-138k | 26 | 15 | 11, all under the lean line | 26 | 13 / 39 |

    - **Blind spot:** "no decision row" is the filter box, not the event. The decision log was cut to the box x233-254, y183-212, and these droppers were logged with their head outside it. So the `need` column is a floor.
    - **Positive control (the colony counter `needs_down`):** 125, 213, 399 and 217 hungry put-downs world-wide in the four windows. The counter spikes at 128-133k exactly where the drops by food spike (126), so the instrument and the counter agree on when.
    - Reading: the churn goes on to the end. Most pellets by the food are dug within 5 cells, and only about half the store's cells are open. **Still inferred:** that this churn, rather than crowding, is what keeps the starvers from eating.
  - **Correction after Deep trace's reply (06:55 UTC): the churn is a small part of the starvers' time. Their time goes on falling in the room's open middle, the same as seed 1.**
    - Deep trace pointed out that the starvers made only 120 of the 923 lobe cuts (13%) and 157 of the 921 drops (17%). That is about 2 each in 10k frames, so they were not mainly churners. My "mostly the churners" reading above is withdrawn.
    - **Time budget of the 127 heap-90 s4 starvers in their last 10k frames** (b90smell s4, 142,684 hungry decisions):
      - Empty-handed 90% of the time; holding spoil 7%; laden 3%.
      - Pull: not scored 43%, walk-out 30%, store 20%, soil way out 4%.
      - Outcome: stepped 57%, idle (move roll failed) 30%, fell 13%.
      - Net climb: 17,525 cells up by stepping, 18,562 cells down by falling. **Every climb undone.** Rows sit at y 187-195, 27-35 rows below the old ground line.
    - **Where they fall** (Deep trace's `fallwhere.py`, deep hungry ants, 110-140k):
      - **82% of the 62,956 falls are in a chamber's open middle** (more than 2 cells from soil or brood). There 35% of decisions are a fall, against 4-5% at a wall or beside brood and 0% in the door shaft.
      - The starvers spent 40% of their deep hungry time in the open middle. Each fell a median 124 times (p90 481); every one fell at least once.
      - The last pull before a fall in the open middle: walk-out 58%, soil way out 18%, store 13%.
      - 113 of the 125 starvers had reached the top 10 rows or the open while hungry at some point.
    - Reading (measured): this is the same failure Deep trace found on b30smell s1 (`body-s1-trace-deep-trace-2026-10-07.md`): hungry ants trying to cross the open middle of a big room fall back.
    - **Inferred, not measured:** what they climb on there. Deep trace's s1 note infers they climb on other ants, since the footing map does not count ants.
    - **Shut-in check (Deep trace's two asks):**
      - Weak digs (`needs_weak_digs`, a lean ant reading shut in and allowed to dig) per 5k: 118-123k 224, 123-128k 492, 128-133k 276, 133-138k 104. **They do not climb with the 128-133k drop spike.**
      - Need drops at 0.5 of the grant or more, which only a shut-in reading can fire: 485 of 1,255 (39%) in the lobe at 100-125k, and 106 of 491 (22%) at 118-140k. So shut-in readings are common there; whether they are wrong is for the probe.
  - **Shut-in probe on heap-90 s4 (07:20 UTC; Deep trace's patch v1 on 26752d47; run pr90-s4 to 140k).**
    - **It changes nothing:** stats.csv and events.txt are byte-identical to dl90b-s4.
    - The decision log keeps the lobe box from 100k, plus every need or shut_in=1 row anywhere.
    - **Positive control holds, once the store job is split out.** Need drops at 0.505 of the grant or more:
      - 843 read shut_in=1;
      - the other 710 are all by ants at or over their grant, and 666 of them have food beside the head. That is the storeroom `job` drop (`store_job_waits`: a fed picker puts its soil down to take up food), not a hunger drop.
    - **The shut-in drops were stale.** Of the shut-in need drops at 0.5 or more, only 4 still read shut in on a way built after the step (`shut_now`). That holds in the lobe and elsewhere, in every 10k window, the same as seed 1.
    - **It is not what kills the starvers.** The 127 starvers read shut_in=1 on 1,452 of their 142,684 decisions in their last 10k frames (1.0%).
    - **Real sealing exists, mostly not in the nest.** 11,148 rows read sealed (shut now and cut off from the air, `air_then`=0):
      - 9,330 are above the old ground line, in the spoil mound, at x 232-287, y 140-159;
      - 1,818 are below it, by 31 ants from 100.5k to 140k, 10 of whom starved. One cluster sits at x 232-235, y 212-215, at the foot of the lobe.
      - The mound pockets are Nest building's encased-carrier topic (`pack_behind`). They are not traced here.
    - Reading (measured): the heap-90 s4 starvers die from falling back in the open middle, not from a shut-in misread. The stale shut-in read is real and common (it makes the hungry soil drops), but it is a small share of the starvers' time.
  - **Probe v2 on heap-90 s4: the way out runs over the crowd, as on seed 1 (07:45 UTC; run pr2-s4; stats byte-identical to dl90b-s4; Deep trace's `routefoot.py`, nest zone, 105-140k).**
    - **What fallers stand on:** of 75,384 falls, 89.0% had only ants round the head and 0.2% had ground.
    - **Where pulls aim:** the target's footing, when the way was built, was held up only by ants for 41.4% of walk-out pulls, 16.1% of store pulls and 42.4% of soil-way-out pulls. At 110-120k it was 66% for the soil way out.
    - **A step toward an ant-footed target is followed by a fall on the next decision:**

      | Pull | Target held up by ants | Target with ground beside |
      |---|---|---|
      | walk-out | 18.0% | 1.1% |
      | store | 14.6% | 1.8% |
      | soil way out | 26.1% | 2.5% |

    - **This holds within the ant's own footing.** Already on the crowd, the walk-out falls next 23.3% of the time toward an ant-footed target against 1.9% toward a ground one. At a wall it is 1.1% against 0.1%.
    - **Reading (measured link by link):** the ways that ants walk out and to the store are built every 30 frames, counting ants as footing. So they lead over the crowd in the room's middle, and ants following them fall back.
    - **Code (inferred):** `way_cell` takes an animal beside a cell as footing.
