# A3, laterals by node instead of by coin: built, measured, parked off (2026-10-21)

## The trace that motivated it
16 lone lab shrubs, bright lamp, 60,000 frames, each founder traced every 500
frames (`plant_m0 mode=plastic axis=shade species=shrub seed=16 arm=bright-off
trace=500 traceall=1`, `runs/shrub_trace_bn0.txt`). Final size 127-1,211.
What separated them was the shoot-tip count by frame 1,500 (1 on the four
smallest, 4-9 on most large ones), while every plant held carbon in store
(the smallest 100-600 while standing at one tip): `break_buds` licenses buds
on income, so early laterals -> leaf -> income -> more buds.

## The lever
`World::branch_by_node` (`plant::branch_due`, `PIXEL_PHYSICS_BRANCH_NODE=1`,
phase `PIXEL_PHYSICS_BRANCH_PHASE`): a lateral on the step where
`(step + phase) * branch_chance` crosses a whole number -- the coin's mean
rate with none of its spread.

## Result: the spread does not shrink

Same 16 world seeds, bright arm, final shoot cells:

| species | coin | by node, phase 0 | phase 0.5 | phase 0.9 |
|---|---|---|---|---|
| shrub | CV 0.42, 127-1,211 | CV **0.75**, 5 stalled at 20-84 | 0.56 | 0.52 |
| herb | 0.33 | 0.37 | | |
| grass | 0.37 (9 of 16 alive) | 0.54 | 0.53 (8) | 0.64 (7) |

Worse in every arm. **The second lever after A2 to fail the same way**, which
`CLAUDE.md` reads as the approach being wrong rather than the tuning. The
likely reason, from the code rather than a run: the lab uses
`DevelopmentalKey::World`, whose growth draws are keyed on organism id,
position and frame -- **not on the world seed** -- so with the founder at the
same column and id in every arm, these 16 "seeds" differ only in the
environment's dice (where the mister's drops land, soil shading, litter). The
spread here is environmental noise amplified by the plant's sensitivity, not
the plant's own dice. The next step is the decomposition (plan §3.2's
instrument, in the lab): environment dice and plant dice varied separately.
