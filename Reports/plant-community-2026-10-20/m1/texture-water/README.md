# Texture-aware plant water, and the re-baseline (2026-10-21)

Plan step "Next 1-2" (`Reports/plant-community-plan-2026-10-20.md` §8 handoff):
re-baseline the training world after through-roots water and the closed soil
budget, and test `World::texture_water` (a plant reads soil water on its
texture's own scale) in the same session, paired by seed.

## The training world, 8 beds x 720,000 frames

`plant_m0 mode=community mix=roles|roles4 loadfail=0 rain=steady spells=20000,40000
ground=patchy seed=1..4 frames=720000 sample=40000 shots=720000 twater=0|1`.
`runs/off_*` is the re-baseline (switch off, byte-identical census to the
build before the switch existed, checked over 6,000 frames); `runs/on_*` the
switch. `summary.txt` is `python3 summarize.py runs off on`.

- **Forest bed (`roles`), re-baseline:** the tree closes the canopy as before,
  but **herb now recruits and persists**: 3-37 plants at 720,000 frames and
  20-95 established per 240,000 frames, against 0-16 plants in the M1 long runs.
  Shrub survives on 1 of 4 seeds, creeper on none.
- **Grass bed (`roles4`), re-baseline:** grass 244-381 plants, and 2-4 kinds
  with 100+ leaf at 720,000 frames; creeper and herb hang on, shrub on 3 of 4.
- **Switch on, paired:** kinds with 100+ leaf at 720,000 frames forest
  `[1,3,2,2] -> [2,2,1,2]`, grass bed `[4,3,2,3] -> [2,3,3,3]`. Inside what
  seeds alone move; `pair-*.png` puts each seed's two arms side by side.

## A lone herb on each ground (the mechanism)

`plant_m0 mode=plastic axis=ground species=herb seed=8 frames=60000 twater=0|1`
(`runs/ground_herb_tw*.txt`, one `SEED` line per seed with the mean root-zone
water the founder read):

| ground | root water read, off | on | shoot, paired |
|---|---|---|---|
| sandy | 0.20-0.61 | **0.30-0.90, higher on 8 of 8** | unchanged (mean 100 -> 98) |
| loam | identical | identical | identical (the specificity control) |
| rich | 0.00-0.93 | 0.36-0.81 | 104 -> 144 mean; 4 of 6 survivors larger, 2 equal -- **not traced**, so not claimed |

Sand at its own field capacity now reads as wet as loam at its own. The herb
on sand was not short of water before, so its form does not move; the
switch's job is to let drought-tolerant and thirsty species sort by ground
once training can move their water traits.
