# Rungs 5-6's arena: getting into a grown stand (2026-10-22)

`plant_m0 mode=stand` (see `Reports/instruments.md`): a stand of the
other three roles, 192 wide, grown 60,000 frames once per seed, then
cloned per arm. Each arm is sown with its role's seed and run on. The arm
`none` is sown with nothing: it is the resident-only control. Bed as the
long mixed bed: `loadfail=0 rain=steady spells=20000,40000 ground=patchy`,
heredity off.

## Without a gap, a tree never gets in (`tree_s*.txt`)

Three tree seeds were sown onto a stand of the seed-trained shrub, herb and
creeper, then run 80,000 frames, over 4 seeds. **No tree seed germinated in
any arm on any seed.** The shipped tree, the seed-trained tree and the floor
tree each ended cell-identical to one another, so the rung could not tell
them apart at all. `gap=G` cuts a G-column gap at each sowing column in
every arm (every plant cell above the ground in it is erased, and roots are
left to resprout), the way a fallen tree opens the canopy.

## With a 16-column gap, the tree's flooding does not show (`gap_s*.txt`)

The same stand, 6 seeds, 150,000 frames. Trees established on 3 seeds of
6. Where they did, the residents' outcome swung more between seeds than
between the shipped and seed-trained tree. That tree was set aside for
flooding the long bed, but it did not hold the residents lower here, and it
did not raise its own count. Its flooding at 450,000 frames (shrub alive on
4 of 6 against 6) is itself a six-seed result.

## Who can invade from rare (`inv_<role>_s*.txt`)

Each role was sown 4 seeds into 16-column gaps in a stand of the other three
shipped roles, 12 seeds, 150,000 frames. The table counts the beds where the
invader is alive at the end:

| role | shipped | seed-trained | residents' shoot, median: none / shipped / trained |
|---|---|---|---|
| tree | 7 / 12 | 7 / 12 | 3,363 / 2,226 / 2,659 |
| shrub | 7 / 12 | 8 / 12 | 6,549 / 5,264 / 5,368 |
| herb | 5 / 12 | 5 / 12 | 7,766 / 7,692 / 7,416 |
| creeper | **3 / 12** | 6 / 12 | 7,222 / 7,577 / 7,318 |

**No role gets in reliably, and the creeper is worst.** The order (creeper,
then herb, then tree and shrub) matches the long bed, where the creeper is
gone by 450,000 frames on nearly every seed. Two cautions:

- **The spread between seeds is large.** Every role scores 0 on several
  seeds and 8-19 plants on others, so a trainer on this rung needs many beds
  per candidate. Its repeatability and heritability (plan §12, M8) come
  before any training on it.
- **The tree pays its residents.** Where it gets in, the residents lose a
  third of their shoot. The other roles cost them little.

## Is the rung repeatable? Not at 12 seeds (`rep/herb_s*.txt`)

Plan §12 (M8) asks for this before any training on the rung. The herb was
sown into the same gapped stand (the residents now include the shipped
trained shrub, PR #814) on 24 seeds, 150,000 frames, in five arms: none,
shipped, and the three trained herbs (`hold/seeds`, `hold/floor`, `refit`).
The 24 seeds were split into two independent halves of 12:

| herb arm | alive, seeds 1-12 | alive, 13-24 | mean shoot, 1-12 | mean shoot, 13-24 | beats shipped on shoot (wins-losses), 1-12 | 13-24 |
|---|---|---|---|---|---|---|
| none (control) | 0 | 0 | 0 | 0 | | |
| shipped | 5 | 6 | 356 | 249 | | |
| seeds | 7 | 4 | 514 | 296 | 4-3 | 3-3 |
| floor | 3 | 2 | 281 | 242 | 2-3 | 2-4 |
| refit | 7 | 3 | 1,157 | 494 | **7-0** | **3-3** |

- **"Alive out of 12" cannot rank arms.** It reverses between halves
  (shipped 5 -> 6, refit 7 -> 3), a swing of three beds either way.
- **Mean shoot keeps its order** (refit > seeds > shipped > floor in both
  halves), but it is set by a few large beds: the median bed has no herb at
  all in most arms.
- **The paired comparison does not replicate.** Refit beat the shipped herb
  on 7 of 7 decided seeds in the first half and 3 of 6 in the second. A
  trainer scoring a candidate on 12 beds of this rung would be selecting on
  which seeds it drew.

So the rung needs either more beds per candidate (24 is the least that
showed any stable order) or a quantity with less spread than invasion
success, before it can be a training rung. The floor herb is the one
consistent result: last or level in every split.
