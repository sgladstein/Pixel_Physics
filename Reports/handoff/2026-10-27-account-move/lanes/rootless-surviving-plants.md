# Rootless surviving plants — lane hand-off (2026-10-27)

Thread: "Rootless surviving plants". Record of everything: `Reports/rootless-plants-2026-10-23.md` (on main).

## What landed (all on main)
- PR #837: `World::roots_required` switch (off). #841 turned it **on** everywhere (owner, 2026-10-23). Only root tissue drinks. Die-back keeps a plant's last 2 roots. A rootless plant reads 0 nutrient. A thirsty rootless plant sprouts an adventitious root from a stem cell on moist soil. `PIXEL_PHYSICS_ROOTS_REQUIRED=0` = old engine.
- PR #846: `World::nutrient_by_root` switch (off). #847 turned it **on** (owner, 2026-10-24). Nutrient status = min(mean root-face richness, total / (0.5 x foliage cells)). `PIXEL_PHYSICS_NUTRIENT_BY_ROOT=0` = old engine.
- PR #860: the realism review (nutrient model against real plants) and six ranked ideas, as a section of the report.
- Probe: `examples/plant_m0 rootcensus=1` (ROOTS / ROOTLOG / ROOTLOSS rows plus an `_owners.png`).

## Open
- **PR #945** (head 1e7f08c9): docs only, with the merge desk. It puts the four pictures shown to the owner into `Reports/rootless-plants-2026-10-23/img/`; until then they live only in `/mnt/project-files/rootless-plants/`. Branch `claude/project-thread-wefr10` holds only this PR.
- **Owner decision pending:** "Build pale hungry leaves next?" Leaf colour would move toward yellow-green as a plant's nutrient falls. It is render only, so no simulation number changes. Recommendation: yes, first, because the nutrient system works but cannot be seen; the 8-bed pictures looked the same. Quantise the colour to a few steps so it does not keep the dirty-rect skip busy. The next two ideas in the report, in order: roots lean toward rich soil, then an internal nutrient store refilled by leaf resorption.
- **Known gaps, not fixed:**
  - grass is barely taxed by nutrient_by_root, because its foliage counts tips only;
  - 9 of 239 leafy plants still read rootless at 450k, not traced;
  - a growing tip can hang off a leaf. `shedding_every_leaf_does_not_disconnect_the_stem` is pinned to roots_required=false: the defect showed up in 5/16 runs with roots_required off and 7/16 with it on.

## Nothing else
No jobs are running on the VM or elsewhere. The scratchpad holds only superseded binaries and logs; the run data is already in `Reports/rootless-plants-2026-10-23/`. The working doc (https://claude.ai/artifact/HELUJxoRzov7XNEvGSBcaE) is fully copied into the report, so nothing is lost if it becomes unreachable.
