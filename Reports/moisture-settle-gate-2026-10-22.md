# Looser field moisture settle threshold: gate results (2026-10-22)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*): copied verbatim from the project shared folder `lab-perf/`. Shipped as PR #812 (`SETTLE_EPSILON_MOISTURE` 0.005 -> 0.05, merged 2026-10-22 as 41cda8a). The two gate tables and the moisture-threshold runner's census are in [`moisture-settle-gate-2026-10-22/`](moisture-settle-gate-2026-10-22/); the 1 MB A/B picture stays in the shared folder (`lab-perf/moisture-settle-ab.png`).

The change: `SETTLE_EPSILON_MOISTURE` in `src/sim/field.rs`, 0.005 -> 0.05. This is the per-frame moisture change below which a field tile counts as settled and stops being re-solved. Tested through a scratch env switch on main e7f294c (branch claude/project-thread-pppbyd, d22010fc4). With the switch unset, all hashes match main.

## Cost
- Lab box, callgrind, 300 frames: 1.739B -> 1.588B instructions (-8.7%).
- Plant community bed: -6.0%.
- Ant bench wall time, 120k frames: mean 185 s -> 162 s; faster on 9 of 12 seeds. The machine was loaded, so treat this as rough.

## Ants (scripts/labbench.py, 12 seeds, played bed, 120k frames, paired)
Every gated row is inside noise (sign test p >= 0.77). Births 602 -> 695, intake 2.21M -> 2.45M J, ant-frames lived 20.3M -> 23.1M, starved per M ant-frames 0.7 -> 1.1 (6/6 split, p 1.0). No box died out. Full table: moisture-settle-ants-labpair.txt.

## Plants: training rungs (4 species x 3 rungs x 2 lights, two independent sets of 12 layouts)
- Germination is identical in every row.
- Establishment is identical, except in dim 10k-frame beds, where 3 of 4 rows lost 1 of 12.
- Most end sizes move less than the gap between the two layout sets.
- Herbs end 10-20% bigger in long bright beds in both sets (191 -> 210, 150 -> 179).
- Raw output: moisture-settle-plant-rungs.txt.

## Plants: community bed (plant_m0 mix=roles, 4 seeds, frames 60k-120k means; runner thread)
| seed | arm | tree leaf | shrub leaf | herb leaf | creeper leaf | plants | leaf all | top water |
|---|---|---|---|---|---|---|---|---|
| 1 | off | 3980 | 609 | 1321 | 132 | 79.8 | 6042 | 0.571 |
| 1 | on | 4774 | 530 | 992 | 120 | 64.0 | 6417 | 0.554 |
| 2 | off | 4488 | 1837 | 832 | 121 | 57.0 | 7278 | 0.560 |
| 2 | on | 3732 | 1862 | 739 | 159 | 71.0 | 6492 | 0.567 |
| 3 | off | 6054 | 873 | 960 | 70 | 69.5 | 7958 | 0.562 |
| 3 | on | 5888 | 1060 | 1188 | 88 | 82.8 | 8224 | 0.564 |
| 4 | off | 5812 | 621 | 874 | 140 | 47.2 | 7448 | 0.561 |
| 4 | on | 7002 | 484 | 1107 | 156 | 55.0 | 8749 | 0.549 |

Moves go both ways by seed, about as large as the noise arm in the fast-drainage check (plant-perf/fast-drainage-growth-check-2026-10-22.md). Soil water and vapour are unchanged.

## Picture
moisture-settle-ab.png: the air humidity overlay through the real lab tick (mister on) at 20k, 60k and 120k frames. Mean air moisture: main 1.43 / 1.34 / 1.51, looser 1.31 / 1.40 / 1.40.

## Caveat
The near-zero control (0.0051) barely re-rolled anything, so the noise floor here comes from the second layout set and from the earlier check's control arm, not from a control run in this test.
