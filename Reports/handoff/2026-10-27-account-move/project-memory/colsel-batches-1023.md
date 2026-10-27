---
name: colsel-batches-1023
description: Colony-selection batches of 2026-10-23 night (after colsel12): crater review finding, body rules PP_BODYRULES, meter fix, colsel13/14 state
metadata:
  type: project
---

- **Crater review (independent agent, 04:20Z):** /mnt/project-files/teacher-lane/review-crater-2026-10-23.md (+ .png, folder with patch/scripts). Cause: the shipped ant's BODY refuses roof cuts (PIXEL_PHYSICS_DIG_ROOF=6, outside door columns) and room-breaking cuts (SHAFT_ROOMS); mind ant's dig() -> cut_cell skips both. Selected brains with the refusals on, no retraining: crater 0 in 8/8, nest 240-490 (old ant 356), old-ant-like room plans. Pay rules can't change which cell is cut (colsel11/12 confirm).
- **c5e87f68:** PP_BODYRULES=roof|rooms|both (options.rs, offers only allowed Dig cells; no escape waiver). Meter fix: nestcount/roomshape/read_nestmap flood passes anything not ground (stored food cut rooms off); old ant nest 564.5 -> 610.5 median (mixed distances), cs10std r47 177 -> 186, crater unchanged.
- **colsel13 (crutch) preempted 04:17Z, 0 jobs done**: deprioritised (review says no behaviour source needed). Rerun only if body rules disappoint.
- **colsel14-1023 (started 04:20Z, c5e87f68):** cs14ctl / cs14roof / cs14both / cs14head (dh7), 64x48, P=24 each. Waiter csvm14.sh -> scratchpad/colsel14. Decision card to owner 04:25Z: body rules permanent (rec) / training aid / must be learned.

**Why:** next session must not rerun pay-rule fixes for the crater. **How to apply:** judge digging with body rules on unless the owner rules otherwise. See [[colsel-batches-1022b]], [[nest-cuts-rule]], [[dig-lesson-oldant]].
- **Dig thread round 4 (05:07Z, 32 seeds sel-off, c5e87f68, BODYRULES=both):** hcs alone nest 171.5 / crater 8.5 / food 569.5 / starved 1; +dh7 nest 191.5, food 518 (11/21); +dh8 (re-recorded with rules) nest 190.5, food 474 -> head not worth stacking; at most dh7 evolvable (PP_DIGHEAD_EVOLVE=1, patch dighead-evolve-vs-1a98de26, not yet applied). README Round 4.
- **colsel14 VOID (06:05Z):** my meter fix let read_nestmap keep larder cells and inheritance COPIED their material -> stored food carried round to round; colonies learned to re-drop it as deliveries (CG home8 ctl 11,143, roof 13,189 vs colsel10 2,016; ctl crater 7). Arm contrast anyway (r47 nest/crater): ctl 41/7, roof 342/2, both 129/0, head 84/14. Fixed cf261f61: nest-only inheritance sets kept cells EMPTY (check: 131 food cells -> 0). **colsel15-1023 (06:15Z):** ctl/roof/both rerun, 64x48, waiter csvm15.sh -> scratchpad/colsel15.
- Lesson: any change to what read_nestmap keeps changes what is inherited; check the new world's start, not only the meter.
- **colsel15 result (07:30Z, cf261f61):** CG r47 food/starved/nest/crater: ctl 1,461/9/19/13.5 (meter+carve change made the control worse than colsel10's 2,016/0/111/207); **roof 4,465/1/143/1** (nest peaks r15 285, erodes); both 2,875/12/189/0. Fresh-world gardens (scratchpad cg15.sh, 12 colonies/arm, gardens-colsel15.png): roof = roofed chamber under a shaft, one room, smaller than old ant. Inherited selection worlds show a flat scraped band under a spoil/food crust counted as nest (nests-colsel15.png): suspect inheritance ratchet + spoil counting as roof; not yet investigated. Disk filled at 07:30Z (deleted colsel3-9, cst*, review raw runs).
- **colsel16-1023 (7e3b5ae9, 07:50Z):** roof rule + shape case (cs16shape), roof at moving food (cs16mix), roof at 45 (cs16d45), 64x48. Waiter csvm16.sh -> scratchpad/colsel16.
