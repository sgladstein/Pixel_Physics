---
name: colsel-batches-1023b
description: 2026-10-23 morning: inherited-nest crust collapse finding; colsel16/17 queue
metadata:
  type: project
---

- **Inheritance opens the crust (08:10Z):** in an inherited round (inherit_nest=1) a band of soil under the surface beside the shaft empties within 10-50 frames with no ant cutting (nomind=1, 1 ant too): cs15roof c5 r0 map, top-6-row open cells near door 16 -> 192 by frame 400. Copying all packed soil too did not stop it (reverted, not committed). Mechanism not found (soil drains between packed crust and lining?). Explains the flat scraped band in inherited selection maps (nests-colsel15.png). All +inherit batches since colsel3 started each round with this.
- **colsel17 (b of 08:15Z commit):** roof rule with NO inheritance: cs17noinh (food 25), cs17noinhmix (moving), cs17noinhshape (+shape). start17.sh retries until colsel16 frees quota; then run csvm17.sh -> scratchpad/colsel17.

**Why:** next session needs to know inheritance is suspect. **How to apply:** compare cs17noinh with cs15roof (same but inherit). See [[colsel-batches-1023]].
- **colsel16 result (09:10Z):** CG r47 food/starved/nest/crater: roof+shape 3,936/0/133/1 (shape 2 vs 0: rooms case barely moves); roof moving 2,752/14/152/2 (52 colonies left); roof food 45 3,399/2/127/1 (colsel10 at 45: 4,418/0/3/61). Roof rule keeps the nest at far food.
- **colsel17 result (10:40Z): NO INHERITANCE WINS.** CG r47: roof no-inherit nest 794 (vs 143 inherit, 58/4 paired), crater 1, starved 0; moving 604/0 (vs 152/14); +shape 514. Local gardens (cg15.sh SET=7, 8 col): tank 205, stored nest food 514, all 24 alive. One huge roofed bowl, no rooms (gardens-colsel17.png). home8 inflated by re-drops (events 'churn' ~11k of 13k deliveries): judge feeding by tank_mean, nest_food, starved.
- **colsel18-1023 (19f6063b, 10:55Z):** no inherit: both rules food 25 (cs18both), both moving (cs18bothmix), roof food 45 (cs18d45). Waiter csvm18.sh.
- **colsel18 result (12:05Z):** no inherit, CG r47 nest/crater/starved/shape: both (roof+room) 437/1/0/8.5 (flat room + side galleries, gardens-colsel18.png; tank 974 = gorging); both moving 400/3/1 (47 colonies left); roof at 45 292/1/0.
- **colsel19-1023 (b8875c18, 12:15Z):** both + shape case at 25 / moving; both at 45. Waiter csvm19.sh.
- **Scaffold plan (owner 12:22Z: "Scaffold is fine", doubts it sticks):** 221be486 adds PP_BODYRULES_P (per-cell hash) + colsel `wean=R0:R1` (CG always p=0 = judged rules-off) + evolvable dig head (PP_DIGHEAD_EVOLVE=1). jobs colsel20: headwean (both, head), wean (both, no head = revert control), headroof; wean=8:32; not yet started (start20.sh/csvm20.sh). Owner 12:25Z: use a general light sense instead of the exact soil count (ants have depth and up/down already: option f::UP, f::DEPTH, f::ROOFED). Built PP_DIGLIGHT=1 (dbd: light halves per non-open cell above; slots 3/4/8), added to colsel20 head arms; start20.sh queued behind colsel19.
- **colsel19 result (13:27Z), rules ON, no inherit:** CG r47 nest/crater/starved/shape: shape case food 25 413/6/0/4; shape+moving 433/4/0/12 (old-ant-like plan: flat room + galleries both sides, gardens-colsel19.png); both at 45 432/1/0/7.5. colsel20 (weaning, light head) queued via start20.sh.
- **colsel20 result (14:57Z), judged RULES OFF (weaned 8:32):** CG r47 nest/crater/starved: light head (hcs+dh7 evolve+PP_DIGLIGHT) 322/5/0, 63/64 nests>=100; moving 216/6/0; no-head control 71/6/1.5. Light beats control 59/5 paired. Crater stayed gone after weaning in both arms. Shape 0 (no rooms case). Gardens running locally (cg15.sh now takes SET=20 full number). Scratchpad disk filled once: deleted colsel10-12,15-17 dirs (data in bucket).
Later: [[colsel-batches-1023c]].
