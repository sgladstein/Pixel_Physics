# Body genes: results (body-genes thread)

## Pilot, 2026-10-22 ~14:30Z: 8 seeds x 6 rounds x 200k, hA1, local (body=default then included pace)

Paired by seed (pairfood.py; home food = deliveries within 8 cols, per colony-round over rounds both reached).

| arm vs control | home food / colony-round | seeds higher | die-outs | starved / colony-round |
|---|---|---|---|---|
| bodyD (genes open, gate starts off) vs hA1c | 1,647 vs 1,542 | 3/8 | 3 vs 1 | 67 vs 50 |
| hg (gate fixed on) vs hA1c | 8,893 vs 1,476 | 8/8 (p=0.008) | 2 vs 1 | 51 vs 46 |
| hgbodyD (gate on + genes) vs hA1c | 6,704 vs 1,306 | 7/8 | 4 vs 1 | 53 vs 41 |
| hgbodyD vs hg | 7,049 vs 7,464 | 4/8 | 4 vs 2 | 54 vs 46 |

Where selection took the genes (mean allele of survivors, r0 -> r5; bodydrift.py):

| gene | bodyD | hgbodyD |
|---|---|---|
| appetite | 0.00 -> +0.14 | +0.13 -> +0.67 (above 0 does nothing when the species gate is already 1: only the down-steps are culled) |
| pace | +0.22 -> +0.55 | +0.28 -> +0.63..0.79 |
| crop_capacity | 0.00 -> +0.08 (peak +0.33) | 0.00 -> +0.76 |
| gut_bias | -0.78 -> -0.61 | -0.85 -> -0.59 |
| digest_rate | +0.16 -> +0.16 | +0.18 -> +0.16 |

Read: the gate is worth ~6x food home, but as a gene starting from off, selection only creeps it up (~0.15 in 6 rounds),
too slow to collect that inside a training run. Starting the gate on and letting the gene tune it costs nothing
measurable in 8 seeds. Pace climbs fastest and costs frame time: hgbodyD_1's r5 founders ran a 60k round at 8,961
frames/s against 13,443 with pace held at the founding body (on fewer live ants, 9.2 vs 11.9). So pace left
body=default (commit f8b00b7a); it still opens by name. 8 seeds is small: die-out differences are noise at this n.

## 64-seed batch body-1022 (VM pixel-train-body-1022, c4d-highcpu-64, started 14:36Z, ref f8b00b7a)

Arms as README.md, with body=default = appetite, digest_rate, crop_capacity, gut_bias (no pace). Pending.

### Results (64 seeds x 6 rounds, paired by seed; pairfood.py on evreduce rows)

| arm vs control | home food / colony-round | seeds higher | die-outs | starved / colony-round |
|---|---|---|---|---|
| hg (gate fixed on) vs hA1c | 8,269 vs 1,700 | 62/64 (p<0.001) | 16 vs 8 (p=0.12) | 47 vs 51 |
| bodyG (gate gene only, starts off) vs hA1c | 2,177 vs 1,651 | 38/64 (n.s.; diff CI -128..+1,192) | 7 vs 8 | 41 vs 49 |
| bodyD (gate, gut speed, crop, diet; gate starts off) vs hA1c | 2,430 vs 1,602 | 36/64 (n.s.; diff CI +297..+1,410) | 14 vs 8 (p=0.29) | 56 vs 52 |
| hgbodyD (gate on + those genes) vs hA1c | 5,644 vs 1,679 | 53/64 | 13 vs 8 | 64 vs 48 |
| **hgbodyD vs hg** | **5,434 vs 8,342** | **19/64 (p=0.002; diff CI -4,283..-1,581)** | 13 vs 16 | 66 vs 47 |

Where selection took the genes (survivors' mean allele, r0 -> r5):
- bodyG: appetite 0.01 -> 0.21 (steady, every round)
- bodyD: appetite 0.01 -> 0.09; crop_capacity 0.01 -> 0.44; gut_bias -0.74 -> -0.56; digest_rate flat 0.21
- hgbodyD: crop_capacity 0.03 -> 0.53; gut_bias -0.76 -> -0.68; digest_rate flat; appetite 0.12 -> 0.42 (inert above 0 on a species gate of 1: only the down-steps are culled)

Read:
1. The fixed gate is still the big lever (about 5x food home), with the die-out cost already known.
2. As a gene starting from off, selection does push the gate on, steadily and with no extra die-outs, but only to ~0.2 of
   a full gate in 6 rounds: too slow to collect the gain inside a training run.
3. Opening gut speed, crop size and diet on top of the gate costs a third of the food home and raises starvation.
   Selection grows the crop ~1.5x: the in-run score rewards something that food home does not. These genes should stay
   closed for training until the score is looked at; not pursued here (no new tasks, owner 15:02Z).

### Digging (owner asked 15:37Z: food is not the only goal). Paired by seed, last row of nest.csv per living colony-round

| arm vs control | cells dug | rooms (chambers) | pit (crater) | nest score |
|---|---|---|---|---|
| hg vs hA1c | 63 vs 93 (more in 10/59) | 0.2 vs 0.6 (more in 7/54) | 49 vs 92 | -57 vs -111 |
| bodyG vs hA1c | 89 vs 90 | 0.5 vs 0.5 | 82 vs 91 | -100 vs -110 |
| bodyD vs hA1c | 90 vs 92 | 0.5 vs 0.5 | 77 vs 91 | -93 vs -111 |
| hgbodyD vs hg | 83 vs 65 (more in 43/59) | 0.4 vs 0.2 (more in 37/50) | 62 vs 50 | -75 vs -58 |

The fixed gate digs a third less and makes a third as many rooms; its better nest score is mostly the smaller crater.
Opening the body genes on top of the gate buys back some digging for a third of the food home. A trade, not a verdict.
Patches of the whole branch (engine + bed switch): patches/ in this folder.
