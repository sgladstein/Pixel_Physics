# Drier soil grows a smaller plant, gradually (soil water and plant growth, 2026-10-21)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*). The code shipped as PR #777
> (merged 2026-10-21, dc79ccd5) with its explanation in `src/sim/plant.rs` (`soil_water_factor`) and
> `wiki/plants.md`; the numbers and the traps lived only in project memory. **Not re-measured.** The two before/after
> pictures are in [`soil-water-stress-2026-10-21/`](soil-water-stress-2026-10-21/).

**What changed.** Before #777 a plant grew the same in wet and dry soil (trainer dry station, herb p50 flat at about
106 whatever the water). Now `plant::soil_water_factor` closes the stomata linearly once the root zone's available
water falls below 0.5 (the FAO-56 rule, p = 0.5). On by default; `PIXEL_PHYSICS_SOIL_STRESS=0` restores the old
engine exactly.

**Numbers (herb p50 at the trainer's dry station).**

| soil water | 1.0 | 0.5 | 0.35 | 0.25 | 0.18 | 0.15 |
|---|---|---|---|---|---|---|
| herb p50 | 108 | 100 | 84 | 67 | 59 | 29 |

The lab box with the mister moves about -5% (noise). Outdoor stands drop about 17%, mostly trees.

**Tried and removed.** Dry herbs build fewer leaves but their stems keep the wet height, so they read as bare
stalks. Scaling the turgor height bound as well collapsed establishment at soil 0.25 (tree 4 of 12, creeper 4 of 12,
herb 8 of 12, against 12 of 12), so it was removed (filed in `dead-ends.md`, plants).

**Traps for the next plant or water measurement.**

- **`examples/labshot` never runs the lab mister**: it calls `frame::step`, not `lab::tick_world`. Any lab plant or
  water measurement through it is a box with no rain.
- The root zone reads wetter than the bed, because `displace_soil_water` fills the first neighbour, so established
  plants rarely close.
