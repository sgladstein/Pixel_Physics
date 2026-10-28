# Why the lab colony kills the shrubs, and the fix (2026-10-28)

*Plant lane, goal G4: a plant community that lives with the colony. Status:
collar resprout (`World::collar_sprout`) switched on by default; founding
defence measured and recorded as a dead end. Pictures:
<https://claude.ai/artifact/4Ec9oKqE2nEnCuKRCA2EDe> (the problem) and
<https://claude.ai/artifact/39tcejfrt3syRuQejUst76> (the fixes).*

## 1. The problem, measured

`latecensus scenario=played_bed mister=1` to 450,000 frames, beds (seeds)
1-12, the shipped ant, mutation off, with the colony and with
`no_colony=1`. Kinds counted are plants past their seed (two or more cells).

| at 450k | with colony | no colony | paired, bed by bed |
|---|---|---|---|
| beds with 4+ kinds | 4 | 10 | 6 better without, 0 worse (p 0.031) |
| shrub alive | 4 | 12 | 8-0 (p 0.008) |
| scrambler alive | 4 | 11 | 7-0 (p 0.016) |
| herb alive | 11 | 8 | 0-3 (n.s.) |
| grass, tree | 11, 8 | 12, 7 | n.s. |

Three tests on the same beds; all three pass Holm's correction. The colony
is what costs the shrub and the scrambler. Re-run on the later build used for
§3, the colony arm reproduced bed for bed.

The first reading, from pictures, was that the nest's digging undermined
them (the colony digs wide chambers under the bed). §2 overturned it.

## 2. The cause, traced per plant

`planttrace scenario=played_bed seed=S frames=450000 life=1`, colony as
written and `colony=0`, beds 1, 2, 3 and 6 (the trace steps
`Lab::tick_for_harness`, so the mister runs).

- **Every established shrub with the colony died (0 of 33 alive at 450k)**,
  almost all `STARVED`; without the colony 65 of 98 were alive.
- **They starved in full light, water 1.00 and soil food 1.00.** Income per
  1,000 shoot cells, median over a shrub's samples with 50+ shoot cells:
  0.02-0.12 with the colony against 1.07-1.26 without, on all four beds. A
  shrub with stems and no leaves.
- **Not digging.** Shoot loss after each shrub's peak was mostly gradual, in
  both arms (big single-sample drops 309-815 cells a bed, gradual
  3,155-5,795), so the shrubs were not being felled.
- **Eaten.** A new recorder, `planttrace bites=1` (`World::bite_log` per
  plant and material; `hash.txt` identical on and off; summed cells equal
  `creature_stats.eaten_plant_cells` exactly, 20,007 of 20,007 on bed 1),
  gives leaf cells eaten per 1,000 shoot-cell samples (every 500 frames):

  | species | grazing rate |
  |---|---|
  | herb | 3.74 |
  | shrub | 3.72 |
  | scrambler | 3.25 |
  | tree | 0.93 |
  | grass | 0.00 |

  The ants do not single out the shrub. They graze every low plant at one
  rate. A shrub loses a median 51% of its peak size in leaf (herb 20%,
  scrambler 16%).
- **Why the herb lives and the shrub does not**: the herb renews by seed
  (587 established herbs on the four beds, 87 alive at the end). A stripped
  shrub keeps its wood (upkeep stays), its tips keep laying stem while it
  earns nothing (bed 3, shrub 2097449: shoot 529 -> 785 at income 0.005) and
  then retire, its buds are buried in thickened wood, and the graze-regrow
  rule (`plant::graze_regrow`) only reaches a plant with no tip and a bud to
  break. It starves over 80,000-200,000 frames.

## 3. Two fixes, paired on 12 beds

One build, `latecensus mister=1`, beds 1-12, colony on, 450k, each arm
against `base` on the same beds.

| arm | 4+ kinds | shrub | scrambler | herb | tree | ants 250k / 450k (median) |
|---|---|---|---|---|---|---|
| base | 4 | 4 | 4 | 11 | 8 | 180 / 52 |
| collar resprout | 8 (5-1, p 0.22) | 12 (8-0, p 0.008) | 9 (5-0, p 0.06) | 9 (1-3) | 5 (0-3) | 132 / 107 (n.s., n.s.) |
| founding defence 0.8 | 9 (6-1, p 0.13) | 12 (8-0, p 0.008) | 11 (7-0, p 0.016) | 6 (1-6, p 0.13) | 5 (2-5) | 74 / 57 (fewer on 10 of 12 at 250k, p 0.039; n.s.) |
| founding defence 0.5 | 4 (3-3) | 7 (4-1) | 5 (3-2) | 8 | 7 | 180 / 53 (n.s.) |

Five measures per arm; only the shrub results survive a correction for
them. Neither arm's herb or tree loss is significant.

- **Collar resprout** (`PIXEL_PHYSICS_COLLAR_SPROUT=1`, built 2026-10-21 and
  parked off because it barely moved lone shrubs): a plant with no tip and no
  bud grows a stem from its lowest open trunk cell, paid from stores. The
  lone-shrub result stands; grazing is what opens a shrub's base and gives
  the rule its job. It fired 40,746 times in one 450k run of bed 7
  (`labshot`'s `collar sprouts`).
- **Founding defence** (scratch `PIXEL_PHYSICS_FOUNDING_DEFENCE=shrub:D,scrambler:D`,
  inherited by bred seed): ants pass a defended leaf with probability `D`,
  per ant, per cell, per window. At 0.5 an ant turned away from one leaf
  eats the next in the same bush (shrub leaf eaten in the first 30k only
  1,080 -> 964). At 0.8 bites fall to 71, the shrub takes over (grass and
  shrub only on 3 beds) and the colony loses a fifth of its plant food
  (1,692 -> 760 cells eaten in the first 30k) and its mid-game size. Filed in
  `Reports/dead-ends.md`.

**Shipped: collar resprout on by default.** Same shrub result as defence at
0.8, no measured colony cost, and it is a plant doing what shrubs do (Bond &
Midgley 2001 on resprouting woody plants, cited from memory). The Teacher
lane was told before landing: three of its six exam boxes have plants, so
exam numbers either side of the merge are not comparable.

## 4. What this leaves open

- The herb and the tree are each lost on a bed or two more with resprout on.
  Not significant at 12 beds; worth watching with more beds.
- 4 of 12 beds still end with fewer than four kinds. The scrambler is the
  weakest role (alive on 9).
- Rungs 5-6 of the plant community plan (recruitment and long-run
  coexistence) should be scored on this bed with the colony, now that a
  colony bed can hold the shrub.

## Reproduce

```text
cargo build --release --example latecensus --example planttrace --example labshot
latecensus scenario=played_bed mister=1 seed=S frames=450000 sample=50000          # [no_colony=1]
PIXEL_PHYSICS_COLLAR_SPROUT=0 latecensus ...                                         # the old rule
planttrace scenario=played_bed seed=S frames=450000 life=1 bites=1 out=DIR           # [colony=0]
labshot scenario=played_bed mister=1 seed=7 frames=150000,300000,450000 crop=0,90,512,190 out=x.png
```
