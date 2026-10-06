# Garden harmony: why lab gardens crash, and grass that grows back

*2026-10-05. Lab played bed, main 3cabe97d for the trace and f4cc3972 for the
fix, mutation off (`PIXEL_PHYSICS_MUTATION=off`), seeds 1-4. The owner's ask:
"we don't want to have to decide between an ant colony that cannot eat enough
to survive and an ant that overgrazes ... then all plants die and the ants
crash too." Result page with charts:
https://claude.ai/artifact/RaeBydhExNV8Fe18eUWr3d. Data:
`/mnt/project-files/garden-harmony/` (project share).*

**Status: grass regrowth shipped on (`PIXEL_PHYSICS_GRASS_REGROW`, default on).
Laying keyed to food income is the open next step.**

## 1. The ants were not overgrazing (traced)

Recorded with `deeptrace garden=1` (every bite, every plant), 200k frames,
today's ant vs the evolved founder, and the garden alone (`colony=0`).

- After 20k frames, living plant tissue is **2-15%** of what colonies eat;
  seed, litter and crumbs are the rest. A seed is worth 480 J, a leaf cell 40 J.
- The median plant that died lost **0-10%** of its peak cells to bites.
  Plants were still standing at the end of every arm, 4-9k cells.
- Both colonies grew on **one burst** of seed and litter from the first
  planting (peak 21-42k). The evolved ant overshot it: on seed 4 the median
  bite moved from 20 to 75 cells from the nest between 10k and 50k, food per
  ant per 2k frames fell 978 -> 83 while ants grew 100 -> 210, and laying ran
  on about 20k frames after food per ant fell. Evolved runs starved 57-422
  adults and 128-216 larvae. Today's ant mostly died of old age.
- `PIXEL_PHYSICS_FOOD_BRAKE=on` was mixed: mean ants over 100-200k 39/65/51/81
  against 8/54/105/106 off.

## 2. The garden ran itself down with no ants (traced)

No-ant garden, 600k frames: edible food fell from 441-553 kJ at 100k to
72-114 kJ, loose seed from 400-750 to 8-32. Every plant was traced to 400k:

- **Grass made the burst** (2,400-4,800 seeds per 100k at first) **and then
  stopped** (0-153 by 300-400k). Herbs, scramblers and trees kept a lower rate.
- **A grass plant grew only in its first ~1,500 frames.** Every `GrowingTip`
  retires at `ORGANISM_STALE_LIMIT` and grass has no buds to start another
  (no `SecondaryThicken`, `plastochron: 0`, so `break_buds` returns at its
  first line). Founders froze at 6-38 cells; a seedling that started in a poor
  spot froze at one blade.
- The frozen plants shed blades to stubs of one, which **never died** (grass
  has no `life_half_life`) and **never seeded**: a one-blade stub's income
  does not clear its maintenance, so its `reproductive_budget` stays empty.
  The surviving grass at 300k was the first cohort, 270-290k frames old.
- One-blade grass almost always had something on top: a neighbour's blade
  (~50%), seed, soil or a puddle. Light at the stubs was fine (~2.1).

## 3. Five fixes that failed the same way (traced)

Each raised early seed 1.5-3x and each still lost the grass by 300-400k:
regrowth from the crown as first written, smothered grass dies, grass old
age (`life_half_life` 20k/40k), seeds wait for open air, `seed_maturity` 3.
Entries in `dead-ends.md`. Five fixes failing the same way meant the cause was
somewhere none of them touched.

## 4. The cause: a regrown blade inherited "full height" (traced)

Counting grass `Grow` exits over a 300k bed with regrowth on: the turgor gate
refused **9,885 of ~18,000** attempts. A crown cell turned back into a tip kept
the `path_len` it had as the top of a blade whose lower cells were shed, so
the turgor bound read it as already at full height. Resetting the tiller's path
to the crown's cut those refusals by about 60%.

## 5. Result, regrowth with the path fix (`break_tillers`)

| Seed | Grass seed per 100k at 400k, no ants (off -> on) | Edible kJ at 400k (off -> on) | Evolved colony, mean ants 100-200k | Ants at 200k | Starved |
|---|---|---|---|---|---|
| 1 | 486 -> 17,224 | 102 -> 1,982 | 8 -> 187 | 1 -> 262 | 57 -> 12 |
| 2 | 440 -> 11,298 | 60 -> 1,407 | 54 -> 146 | 2 -> 75 | 286 -> 350 |
| 3 | 327 -> 5,657 | 103 -> 679 | 105 -> 183 | 29 -> 127 | 71 -> 565 |
| 4 | 1,277 -> 9,171 | 133 -> 1,101 | 106 -> 229 | 17 -> 160 | 422 -> 630 |

With ants, the regrowing garden still made 15-18k seeds per 100k at 200k,
and ants ate about 1% living tissue. The bed looks like a meadow from wall to
wall. **Cost:** a no-ant 300k bed took 553 s against 260 s (single thread),
because there are far more plants and seeds; the PR quotes `ascii` worst-frame.

## 6. What is inferred, and next

- Colonies still swing (seed 3: 89 -> 341 -> 119) and starvation is high on
  three seeds. The garden is no longer the limit; **laying keyed to food
  coming in at the nest** (Prabhakar, Dektar & Gordon 2012,
  doi 10.1371/journal.pcbi.1002670; Gordon 2013, doi 10.1038/nature12137) is
  the next piece.
- Real grass regrows from basal meristems, which is why it tolerates grazing
  (McNaughton 1983, *Oikos* 40:329, cited from memory).
