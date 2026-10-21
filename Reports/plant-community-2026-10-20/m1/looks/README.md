# The bed has to look like plants (M1 step 7a), 2026-10-21

Data behind `Reports/plant-community-plan-2026-10-20.md` §8 step 7a.
All runs: `examples/plant_m0 mode=community loadfail=0 rain=steady
spells=20000,40000 ground=patchy sample=40000`, one thread each;
`mix=roles` is the forest bed (tree, shrub, herb, creeper), `mix=roles4`
the grass bed (shrub, creeper, herb, grass). Each log's `SCREEN` line is
what the picture is made of and its `WATER` line what each standing drop
rests on.

- `runs/before-*` -- the bed before the water fix (`roots=0`, drip reach 16).
- `runs/after2-*` -- as shipped: soil water through roots (two-ended), drip reach 32.
- `runs/yield0-*` -- as shipped plus `DECAY_YIELD=0`: heaps go, holes open.
- `runs/closed-*` -- as shipped plus the closed soil budget (`rotsoil=0`): held for the owner.
- `water-arms/` -- the first four-arm comparison (240k frames): today,
  `level=1` (sideways levelling), `roots=1` (one-ended, before review), both.
- `1-water-before-after.png`, `2-heaps-tradeoff.png`, `3-colour-*.png`
  (`altpal=2`: A as is, B brighter foliage with seed and litter toned
  toward soil, C also roots) -- the pictures shown to the owner.
