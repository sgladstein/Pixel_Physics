---
name: nest-priorities
description: Owner's ranking of nest problems (2026-10-25 15:05Z): too small first, crater instead of a mouth second, single room third; current state of the nest line
metadata:
  type: feedback
  modified: 2026-10-25T21:02:01.829Z
---

Owner (Scotttt, 2026-10-25 15:05Z, Teacher lane thread): the nest is "one of the biggest issues". Fix all three, in this order:
1. **Too small** for the colony (biggest).
2. **A crater instead of a mouth** opening into the nest (second).
3. **No multiple rooms** (third).
Also: food distance random/moving, never one learnable location. Owner 19:09Z: test food far and always moving; replays after most runs ([[replays-and-far-food]]).

**Why:** the owner's own playtest complaints, ranked by the owner.
**How to apply:** judge every nest change on OPEN room first (nestcount `nest_open`; `nest` counts stored food as nest), then crater, then rooms (roomshape OPEN_ONLY / colsel `shapeopen=1`; the plain rooms score is fooled by food piles). Lead reports with size. Never trust a nest number without looking at the picture.

**State 2026-10-25 21:30Z:** trained nests are pits packed with stored food (~100 open cells vs hand-built ~1,000). ROOT found: trained ants never eat stored food, so on moving food they starve beside a full larder and the store only grows. Batch history and numbers: [[nest-batches-1025]]. nest5 (tramp move ramp, home-eat crutch) FAILED: ramp never started (median starved 99-285 with the heap never moving), no garden gain. Hunger probe: hungry trained ants die pressed against the bed walls (338/438 sightings in the outer 16 cols); at home touching food they Walk 32/39. Replays https://claude.ai/artifact/HYDeKaY32fzLUdudCkFG3i. nest7 DONE 00:24Z 10-26 (page https://claude.ai/artifact/Q7w3bV8voybduVpourMBLF): weaned teacher did NOT stick (garden r47 starved 218 vs ctl 241, 19 vs 12 colonies dead). hweanrules grew nests (garden nest 199->617, 4 dead) but crater 138, starved 289; its c13 (r47, carry scratchpad rp7/hweanrules13.carry) world 13099: 85 alive, 162 starved, open 386, crater 17, rooms 29 (hand-built 1,085 open). Card withdrawn 00:12Z (owner asked; I'd moved on). c13 8-world test PASSED (open 398 vs 16.5, alive 71.5 vs 10, rooms 8/8) -> made std_pool (commit c87fd03c). nest8 live (SHA c87fd03c, ~01:20Z, waiter bmgos47md): gctl / grules / grulesrooms from c13 pool. Copy blocked: copier_c.py assumes GENOME_LEN 2797, pool is 3285 layout 5.

**Wall trace 2026-10-25:** hungry trained ants walk out fat and random-walk at the walls (brain ignores home sense; scores saturated); logged in colony-selection log (PR #923).

**Ceiling 2026-10-25 ~22:45Z (page https://claude.ai/artifact/Y2ERy6mXkeZYxZ7sg5KDYY):** teacher acting (PP_CARRY=1 PP_CARRY_HUNGRY=2 PP_CARRY_HUNGRYONLY=1 PP_CARRY_OUT=0 PP_CARRY_STORE=1 PP_HOMEFAR=1 PP_HOMEFAR_HUNGRY=1 PP_HOMEFAR_FUEL=0.012) on c10 pool, patches=20000:40:120, seeds 21099-24099: starved 14/24/31/25 vs no teacher 217/381/476/145; alive 71-76 vs 0-37. Fixes that mattered: fuel-scaled turn (flat 2 tanks too late), hungry pellet holders drop then go home (old hungry rule walked them AWAY), climb/cut out of heap pits, TurnAround instead of a step into own tail (that last took 50-129 -> 14-31). Gotchas: H>=3 grants makes founders always hungry (all 24 die); fuel 0.025 too timid (founders never reach food). Code: mind.rs HOMEFAR block, in /home/user/nb uncommitted + scratchpad homefar_hungry.patch. Nest still tiny (open 2-14). Next: make it a lesson (copy onto pool, train only home-sense weights per far-home-sense recipe, then colony selection), judged teacher-off.

Lesson fit (weights) 2026-10-26: [[home-lesson-fit]] (failed, pinned scorer). nest8 DONE 02:31Z: nothing beats c13 (best grulesrooms c19 open 525 vs 398 but starved 362 vs 240, 8/8 worse); page https://claude.ai/artifact/6R8cj4dx6dizbYCydqPmqi. nest9 LIVE (SHA ad74c3c, 02:43Z, waiter bzj94uigj): sctl / sharp2 / sharp1 (PP_SHARP_MAX). PR #925 to merge desk (head d8b5dd7). See [[far-food-1024]], [[room-shape-measure]], [[nest-score]], [[no-oldant-body-rules]].
