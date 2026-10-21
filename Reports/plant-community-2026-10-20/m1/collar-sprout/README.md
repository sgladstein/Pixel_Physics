# Why lone shrubs end at different sizes, and a basal sprout that does not fix it (2026-10-21)

Twelve lone shrubs under the lamp, one per world seed (`plant_m0
mode=plastic axis=shade species=shrub seed=1 frames=60000 arm=bright-off
vary=world base=500000{1..12} trace=500 whys=1`, run one seed per process).
`whys=1` prints, under each trace row, why the founder's shoot tips did or
did not grow since the last row (`GrowWhy`). Shoot sizes reproduce
`../clone-spread/base5000001/shrub_world_keyworld.txt` exactly.

## What the traces show (`runs/w*.txt.gz`)

- **No early state predicts the end.** Cells at frame 6,000 against the
  final shoot: 336 -> 1,211, 591 -> 698, 108 -> 450. The ordering is still
  being made after 10,000 frames.
- **7 of 12 end the run frozen: no growing tip and no bud left**, with
  ~500-1,800 carbon in store. They froze at frames 11,500 (world 5,000,011,
  278 cells), 39,000, 40,500, 54,000, 57,000 and 58,000. Buds are laid down
  only by an extending tip, and thickening wood buries them, so once the
  last tip retires nothing can restart growth.
- **Late growth is a crawl with 1-3 tips.** `break_buds` licenses
  `floor(surplus / step_cost)` tips; a shrub earning 0.4 a tick against a
  0.2 step gets one. Most late visits are `Boxed` (no open cell beside the
  tip), `TaperRoll` (near the height bound) or `TooPoor`.

## The basal sprout (`World::collar_sprout`, off; `PIXEL_PHYSICS_COLLAR_SPROUT=1`)

A plant with no tip and no bud, holding `BUD_RESERVE_STEPS` steps of carbon,
turns its lowest stem cell with open air beside or above it into a tip --
`break_tillers`' answer for grass, given to plants with buds. Off is
byte-identical (`runs/off10.txt.gz`: 127 cells, as before).

| | off | on |
|---|--:|--:|
| shoot cells, mean | 725 | 755 |
| CV | 0.41 | 0.40 |
| larger with it on | | 8 of 12 |

**It does not narrow the spread, and the trace says why: a frozen shrub
has nowhere to sprout.** The stunted one (`frozen-shrub-*.png`) is a tight
clump whose stems are wrapped in its own leaves and flowers; no stem cell
touches open air, so the rule finds no site. It sat frozen from 11,500 to
~55,000 with the switch on and gained 32 cells. Parked off.

`collar_sprouts` in the `SEED` line counts the whole bed (the founder's
offspring too), so it is not the founder's count.
