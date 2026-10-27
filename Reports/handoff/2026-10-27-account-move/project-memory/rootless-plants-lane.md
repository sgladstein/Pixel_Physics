---
name: rootless-plants-lane
description: Rootless surviving plants thread (2026-10-23): why lab plants live with no roots, the World::roots_required switch, probe plant_m0 rootcensus=1
metadata:
  type: project
  modified: 2026-10-23T20:13:42.779Z
---

Thread "Rootless surviving plants" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBWGCyL4S5SWCXgBUrp41b1m), branch `claude/project-thread-wefr10`. Owner 2026-10-23: "roots seem more just a visual but they should actually be functional. This is a simulation."

- Owner's marked plant (four-role bed, seed 5, 450k, TODAY) = tree with 1,332 leaves, 0 root cells, drinking through trunk cells on the soil.
- Count (plant_m0 rootcensus=1): 150k seed 5: 7/27 leafy plants with no contact root; seed 3: 14/44; mostly herbs, many at water status 1.0.
- Causes: (1) die-back can't take foliage or cells beside it -> sheds bare roots first (171/173 small-plant root losses = Starved, at ws 1.0); (2) Absorb on every MatureBody, so stems drink; (3) nutrient_status = 1.0 when root_cells == 0; (4) break_root_tips only branches from existing rootwood; (5) germinate gives no root if the cell below isn't growable (seed on litter/wood).
- Fix: `World::roots_required` / `PIXEL_PHYSICS_ROOTS_REQUIRED=1` (off; off bit-identical): only root tissue absorbs, die-back keeps last 2 roots (DIEBACK_ROOT_FLOOR), rootless plant reads 0 nutrient, thirsty rootless plant sprouts an adventitious root from a stem cell on moist soil. 50k seed 5: rootless leafy 4 -> 0.
- Probe: plant_m0 `rootcensus=1` prints ROOTS/ROOTLOG/ROOTLOSS rows and an `_owners.png` (each plant one colour, roots darker). Use `below=150` to see deep roots.
- 8 beds paired 450k: rootless leafy 78/236 -> 9/239, shrub leaf 1,925 -> 5,353, roles 2.62 -> 2.88, light under cover 0.155 -> 0.170, tree leaf ~same, cost noise. Seeds 3, 6 went the other way.
- PR #837 MERGED to main as 8da0b01 (2026-10-23 ~21:00Z), switch off.
- OWNER CHOSE 2026-10-23 22:22Z: turn it ON everywhere. PR #841 (head 9d210ba0) flips the default; with merge desk. PIXEL_PHYSICS_ROOTS_REQUIRED=0 = old engine. ascii: organisms 855->706, forage trips 68->46. Test shedding_every_leaf_does_not_disconnect_the_stem now pinned roots_required=false (pre-existing tip-on-leaf defect, 5/16 off, 7/16 on).
- #841 MERGED 2026-10-24 00:02Z (roots_required ON on main).
- Owner said test nutrient-by-root: `World::nutrient_by_root` / PIXEL_PHYSICS_NUTRIENT_BY_ROOT=1 (off): status = min(mean, sum/(0.5*foliage)). 8 beds 450k: root/foliage 0.81->0.86, tree leaf -11%, understory flat, roles 2.88 both. Not visible in pictures. PR #846 MERGED 2026-10-24 01:16Z as 4fb531d (switch off); owner chose ON 2026-10-24 01:23Z; PR #847 MERGED 2026-10-24 02:08Z as a4af86c (nutrient_by_root ON on main). Asked owner whether to build pale hungry leaves next (no answer yet). Realism review + 6 ranked ideas landed in the report via PR #860 (MERGED 2026-10-24 15:27Z as 1e4e845); Plant community plan thread told of both switches. Research doc on realism + ideas (pale leaves, roots toward rich soil, store+resorption): https://claude.ai/artifact/HELUJxoRzov7XNEvGSBcaE
- Data: /mnt/project-files/rootless-plants/, Reports/rootless-plants-2026-10-23.md.

**Why:** later plant sessions must know rootless plants were a real engine hole, not a render issue. **How to apply:** check the PR state before citing; related [[plant-community-lane]].
