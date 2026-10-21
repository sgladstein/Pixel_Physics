# Re-pricing shade tolerance (plan step 3, 2026-10-21)

`plant_m0 mode=plastic axis=shade species=shrub genes=shrub:tolerate:0.7 seed=16
frames=60000`, with `PIXEL_PHYSICS_SHADE_LEAF_PRICE` unset (0.35) and 0.9;
`shade_p0.6.txt` is the 8-seed 0.6 arm (means only). One `SEED` line per seed
per arm, so tolerant and not are paired on the same dice.

| price | bright: tolerant larger | bright: median ratio | dim: alive, genes ignored / read |
|---|---|---|---|
| 0.35 (old) | 12 of 16 | 1.05 | 13 / 16 of 16 |
| **0.9 (new)** | **8 of 16** | 1.07 (sign split) | 13 / 16 of 16 |

**Why no price makes tolerance a loss in bright light.** `trace=10000` on seed
0: the lone shrub under the lamp earns ~7x its upkeep (income 0.45, bill 0.06
at 40,000 frames) and its carbon store only grows (246 -> 890). Its size is set
by development -- its last growing tip ends near 40,000 -- not by carbon, so a
cut to income changes nothing it builds. The cost has to show where light is
contested (a neighbour overtopping it): rung 4's arena.

**What this means beyond shade.** A lone plant in bright light is not carbon
limited at all, so a rung that scores a lone plant under the lamp cannot see
any leaf-economics trait. Rungs 1-3 need their dim and dry stations to carry
that signal (§6.2's station sets), and A2's development keying is what sets a
bright plant's size.
