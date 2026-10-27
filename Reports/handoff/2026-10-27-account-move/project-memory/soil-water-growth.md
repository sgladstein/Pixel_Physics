---
name: soil-water-growth
description: Soil-water stomatal closure (PR 777, 2026-10-21): drier soil grades plant growth; numbers, what was tried, labshot has no mister
metadata:
  type: project
  modified: 2026-10-21T23:27:31.228Z
---

Soil water and plant growth thread (branch claude/project-thread-jpvof2, PR #777, with the merge desk 2026-10-21).

- Shipped on by default: `plant::soil_water_factor`, which closes stomata linearly below root-zone available 0.5 (FAO-56 p=0.5). `PIXEL_PHYSICS_SOIL_STRESS=0` turns it off.
- Trainer dry station, herb p50: 108/100/84/67/59/29 at soil 1.0/0.5/0.35/0.25/0.18/0.15 (was flat ~106). The lab box with the mister changes ~-5% (noise). Outdoor stands drop ~17%, mostly trees.
- Dry herbs build fewer leaves, but their stems keep the wet height, so they read as bare stalks. Scaling the turgor height bound too was tried: establishment collapsed at soil 0.25. Removed (dead-ends plants:174).
- **Trap: `examples/labshot` never runs the lab mister** (it calls frame::step, not lab::tick_world). Any lab plant/water measurement through it is a box with no rain.
- The root zone reads wetter than the bed, because displace_soil_water fills the first neighbour, so established plants rarely close.
- Plant-community thread wants to hear when #777 lands, so it can turn on the dry station and retrain (photosynthesize_rate was a free lever).

**Why:** later plant/water work should start from these numbers and avoid labshot for water questions.
**How to apply:** check PR 777's state before citing. Related: [[plant-community-lane]], [[soil-physics-and-roof-sense]].
