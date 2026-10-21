# What makes two identical plants grow to different sizes (2026-10-21)

Plan §3.2's decomposition, run before spending more on A3/A4. One lone plant,
bright lamp, no shade genes, 60,000 frames
(`plant_m0 mode=plastic axis=shade arm=bright-off species=S seed=12
frames=60000 vary=V base=B [devkey=0]`). Three arms:

- `vary=world`: same plant, same spot, a different world seed per run (the
  soil patches and everything else the world draws).
- `vary=column`: one world; the plant moves one column per run.
- `vary=none`: one plant repeated. The determinism control: CV 0.00 in every
  `base5000001` arm, so the instrument adds no noise of its own.

Each arm under the lab's `World` key (the plant's draws keyed on world
position and the clock) and under `devkey=0`, the per-plant key A4 would ship
(`DevelopmentalKey::Plant { coarseness: 0 }`: draws keyed on the plant's own
seed and its cell *relative to the collar*). Under `devkey=0` the column arm
therefore holds the plant's dice fixed and moves only the ground under it.

## Result (`base5000001`, 12 seeds, `summary.txt`)

| shoot cells, CV | world arm | column arm |
|---|--:|--:|
| shrub, World key | 0.41 | 0.32 |
| shrub, plant key | 0.36 | 0.33 |
| herb, World key | 0.35 | 0.39 |
| herb, plant key | 0.40 | 0.42 |

**The per-plant key does not narrow anything.** With the dice held fixed and
the plant moved one column at a time, the shrub still spans 442 -> 1,206
cells, and adjacent columns jump (876, 965, 895, 1,118, 512).

**For the shrub the site barely matters either.** The same column under the
two keys (same ground, different dice) correlates at r = 0.27: the site
explains under a tenth of the variance. A one-column shift is about the
smallest change the world can make, and it produces the full spread. The
shrub's size is a sensitive function of *any* early difference, which is the
seedling stall lottery seen in the A3 trace (`../a3-branch-by-node/`): final
size is fixed by how many tips survive the first ~1,500 frames, with carbon
in store throughout. Shrinking or re-keying the dice (A2, A3's laterals, A4)
cannot remove a sensitivity; they only change which plants lose.

**The herb does read its site.** Same column, two keys: r = 0.76. Its spread
is mostly the ground it stands on, which is the behaviour wanted.

One real environment effect for the shrub: world 5,000,011 stunts it under
both keys (137 and 127 cells against a median ~700). That one world carries
the shrub world-arm correlation (0.52 with it, 0.18 without).

## `base5000000`

The first pass, at world 5,000,000. **Seed 0 in every arm there is
contaminated**: the light probe stepped seed 0's own world 30 times (one
plant repeated read 206 cells on seed 0 and 513 on the other 15), fixed since
by probing a clone. The herb mostly dies in that world, so it carries little.
Excluding seed 0 the shrub agrees: world CV 0.37, column 0.41.
