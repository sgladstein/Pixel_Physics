---
name: colsel-batches-1023c
description: 2026-10-23 afternoon: colsel20 gardens (food-packed bowl), colsel21 rooms+light, store=S switch, brood lane, eat trace
metadata:
  type: project
---

- **colsel21-1023 (a8ee2b9f, 15:10Z):** colsel20 light arm + shape case: cs21shape (25), cs21shapemix (moving), cs21shape45. Waiter csvm21.sh.
- **colsel20 gardens (rules off, gardens-colsel20.png):** one roofed bowl per colony packed with stored food; light arm nest_food 389 vs ctl 186, tank 87 vs 235, alive 24 vs 22.5; 2/8 light colonies start a 2nd room.
- **16:24-16:45Z:** owner disliked food-packed nest; Brood and breeding lane started (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBLmYKPvphE4XS19VEKs68gU), briefed. store=S switch (89453ec9): delivery/progress pay x (1 - nest_food/(S*ants)). Eat trace (scratchpad eattrace/, trace=200, 100k): 75-83% of digestion in nest (light), 95-97% (ctl); home bites ~= in-nest re-drops (larder churn). Offered owner 2 parallel threads (inheritance soil drain; land compute work on main), awaiting answer. colsel21 preempted 15:32Z, rerun as colsel21b-1023 (start21b.sh + csvm21b.sh -> scratchpad/colsel21).

**Why:** next session picks up colsel21 and store=S. **How to apply:** judge rules-off; see [[colsel-batches-1023b]].
- **colsel21 result (17:05Z, rules off at judge, built a8ee2b9f):** CG r47 nest/crater/starved/shape: shape@25 208/4/0.5/0 (peak 419 r35); shapemix 76/14/1/9 (26/54 >=10); shape45 386/10/0/0. Rooms only with moving food, small nests. Gardens at DIST=45 (cg15.sh DIST= env) running.
- **colsel22-1023 (b112ff91, ~17:10Z):** colsel21 arms + store=4. start22.sh && csvm22.sh -> scratchpad/colsel22.
- colsel21 gardens (DIST=45, gardens-colsel21.png): food-packed bowls; 45 arm nest_food 452, tank 226; moving nest_food 137, small nests; room scores fooled by food shapes. Plant-food lane (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBHNgTg91PZKDrDAAo8cErcs) 17:09Z: PP_MIND_GRAZE=1 needed (mind couldn't bite plants), hcs grazes in place and dies; nestmap writes plants '%' (ground); proposing weaning bed (shrinking pile + per-joule pay), owner card.
Child lanes: [[teacher-children-1023]].
- **colsel22 result (~18:45Z, store=4, rules off at judge):** CG r47 nest/crater/starved: @25 316/16/1 (crater 127-138 at r23-35); moving 114/34/0 (rooms 0); @45 172/310/0. store=4 brings the crater BACK vs colsel21 (4-14) and kills the rooms signal. Gardens running.
- colsel22 gardens (gardens-colsel22.png): store=4 @45 empties larder (nest_food 36 vs 452) but nests become sky-open pits/shafts (crater 310); @25 mixed (some open bowls). Verdict: drop store pay; fix larder via brood/plants.
- **19:15Z:** #834 on main (c08e7f1f); merged main into compute -> 49a1109c (main brought only off-by-default/inherit code). colsel23 jobs (ecd0de23): cs23inh / cs23inhmix (+inherit, inherit_nest=1 inherit_link=20) / cs23ctl. Queue: brood1b -> plantwean2 -> colsel23 (bllsn6441 waits for plantwean2/DONE; times out ~21:10Z, requeue).
