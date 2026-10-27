---
name: link-price-farfood
description: 2026-10-25 old-ant thread — the far-food gap (old ant vs trained) is the brain bill (~1,900 links billed per tick), not old-ant rules; PIXEL_PHYSICS_LINK_PRICE switch PR #881; owner chose both arms
metadata:
  type: project
  modified: 2026-10-25T02:45:51.837Z
---

Thread "Open the old ant to evolution" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB5DjcuMiZLTCCiZvPMh8Dch), 2026-10-25.

- **Cause of the fruit-200 gap:** std_pool (laygene2 c8) bills ~1,922 live links (482 = dig head) at synapse_fraction x start_energy each per tick = 0.85 J vs idle 0.1. One tank lasts trained ~1,000 frames, old ant ~3,700 (lab, no food). Fruit 200 alive by price factor: 1 -> 28, .75 -> 50, .5 -> 89, .25 -> 114, 0 -> 150; old ant 129. Bed garden at .25: starved 1-4 vs 18-38, nest bigger 3/4.
- **Old ant's rules ablated (21 arms, labsurvive founders=old):** only the chooser's trailaway (walk out along trail B) matters; scout, drive, store, carry rules ~neutral. Old ant brain ~50 weights: nothing to transfer.
- Switch `PIXEL_PHYSICS_LINK_PRICE` (default 1.0) merged PR #881 as f367374 (2026-10-25 03:06Z); labsurvive `probe=`, `tax=`; scenarios scratch/oldant/scen, scratch/inrun/labscen/lab_far200_fruit.ron.
- **Owner decision 02:40Z:** test both arms (lower price, and full price trained on far food). Teacher lane batch: 32x24 from std_pool, scalegate, prices 1 / .25 / 0, lab case with lab_far200_fruit.

- **Hard-codes still on the trained ant** (report Reports/brain-training/teacher-training/old-ant-rules-2026-10-25.md, merged #881; handed to Teacher lane as proposals): H1 Storeroom caste=4 = 1-in-4 nest-bound caste by id incl. minds (contradicts no-castes; NURSE=any first), H2 larva touch-feeding, H3 egg site/suppression, H4 door-column put-down ban + spoil roll on in lab but off in training arenas, H5 dig-head hand gate, H6 founding cut, H7 body forbids (keep), H8 harness scaffolds, H9 link price. Trunk crossing is the plant-feeding thread's.
- Don't commit labsurvive runs' scratch/oldant/assets/chronicles (1,299 files slipped into #881 once).

- **Teacher batch linkprice1 result (05:08Z):** the price itself does the work; training at low price adds little. std_pool fruit 200 alive (labsurvive, seeds 1/2): price 0 -> 163/146, price 1 -> 20/39. Garden starved 39 -> 2 at price 0, nest 280 -> 249. At low price the food ramp reached 120-125 cols and 17/32 died there at 0.25. Teacher lane recommended a lower default in every world; level (0 or 0.25) is the owner's call.

- **Opened 06:00-07:00Z (owner asked "is removing rules + training old brain worth it": no for old brain, yes for trained ant):** `PP_MIND_CASTELESS` (H1) and `PP_MIND_DOOR_OPEN=drop|clear|1` (H4), both default off, merged #895 (5eb050b) 07:10Z. Bed (std_pool, 8 seeds): casteless = all touch-feeding effect, food home up 7/8 but crater bigger/nest smaller 6/8; door column's per-frame soil DELETION over the door is load-bearing (stop it: food home -60%, starvation x2) -- biggest hidden cheat. Spoil roll is NOT a train/lab mismatch on inrun's bed (only ladder arenas pin it off).

**Why:** later sessions should not hunt old-ant rules for far food. **How to apply:** price brain size before blaming behaviour; judge lab numbers at the arm's own price. See [[far-food-1024]], [[teacher-lane]], [[no-oldant-body-rules]].
