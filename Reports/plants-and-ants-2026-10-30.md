# Plants and ants in the lab bed: who needs whom, the garden the colony already plants, and the tree (2026-10-30)

*Plant lane, goal G4: a plant community that lives with the colony. Status:
measured on `main` 678a4fb4 (collar resprout on; before PR #990's fungus
garden). Pip food value lowered 40 -> 14 and shipped (§3); the tree's loss
traced to shrub competition and handed to the invasion test (§4-5);
stage 0 of that test ran on the VM 2026-10-31 (§6): its gates pass, and no
role grows from four seeds on more than one bed in four. New player-facing interactions (leaf garden, guarded
plants, a planting verb) belong to the plants-and-ants interaction lane and
are not designed here. Pictures: <https://claude.ai/artifact/KSsVbrwiPbC4vDqwMbXprw>.*

## 1. How much they need each other

`played_bed`, beds (seeds) 1-12, 450,000 frames, `latecensus mister=1`, the
shipped ant, mutation off; colony as written against `no_colony=1`, and the
collar-resprout arm of `Reports/shrub-grazing-2026-10-28.md`.

**Ants on plants: completely.** The colony's intake has three sources --
`harvested_plant` (leaf, seed, fruit, litter, nectar) and `harvested_corpse`,
both fields of `EnergyLedger` (`src/sim/world.rs`), and a 200 J grant per
founder (`ant.ron`), about 10 kJ for a 52-ant landing. The bed
has no predators (`attacks=0`, `al_eat_animal=0` on every bed).

| | median of 12 | range |
|---|---|---|
| plant eaten, whole run | 7,941 kJ (before resprout) / 7,793 kJ (resprout) | 2,641-14,234 kJ |
| corpse eaten | 228 J / 272 J | 47-686 J |
| plant share of intake | **99.9%** | 99.7-99.9% |

With no plants at all (`played_bed`, `placements: []`, seed 1, mister on)
all 52 founders starved by frame 12,000; nothing was born. With plants the
colony ends anywhere from 0 to 354 ants, so its dependence is already a
spread, not a switch.

**Plants on ants: the colony re-sorts the stand rather than shrinking it.**
Median count at 450k (beds with more / fewer under the colony):

| kind | colony, before resprout | colony, resprout | no colony |
|---|---|---|---|
| grass | 52 (3/9) | 73 (2/10) | 110 |
| **herb** | **33 (10/1)** | **17.5 (9/2)** | **3** |
| shrub | 0 (0/11) | 20.5 (3/9) | 32 |
| scrambler | 0 (1/10) | 4.5 (2/9) | 3.5 |
| tree | 9 (8/1) | 0 (4/3) | 1 |
| seed bank | 238 | 302 | 445 |
| plant cells | 11,314 | 11,150 | 12,146 |

Ants graze every low plant at about one rate per shoot cell (the shrub
report), which costs the slow growers; what they give back is the seed
loop below. The herb is the plant that gains, on 9-10 of 12 beds.

## 2. The garden the colony already plants

**The loop** (checked against the source on 678a4fb4): a bitten seed whose
coat survives the mouth (`plant::seed_survives_bite`) is left as a `pip`;
an ant whose crop carries no seed yet (one per crop) lifts it as a
passenger (`plant::take_seed_passenger`); it comes out either when the crop
is digested empty, as a bare pip at the eater's head
(`plant::deliver_seed_passenger`), or when the ant puts the crop down or
dies carrying it, back inside its fruit as windfall -- or as a plain seed
for a kind with no fruit (`deliver_seed_passenger_uneaten`); a seed
set down on ground too dry to sprout is redirected up to
`MIDDEN_SEARCH_COLUMNS` (6) either side to a wetter spot
(`plant::find_midden_site`). Germination needs light, soil water and a
shallow enough burial; soil does not block light (only solids and plants
do), so near the door, where the colony keeps the ground clear, water is
what binds. Under a shrub layer light binds too.

**It fires, and it works.** Over 12 beds: about 1,000-1,350 seeds carried a
run, 97-99% put back down, 300-350 germinations from a pip (about 11% of all
germinations). The bed sows nothing between columns 180 and 330 and the nest
is at 256, so a herb or tree by the door is one an ant brought. Within 32
columns of the door, beds 1/2/3/6 germinated 159/101/20/42 herbs with the
colony and 0/0/0/7 without; herbs set down within 64 columns of the door
established at 17-24%, against 5-8% anywhere in a bed with no colony.
Carried pips reach ground wet enough to sprout 50% of the time; pips left
where they were bitten, 8%.

**Nobody can see it, and the colony eats it.** One `labforage` run
(seed 1, 150k): of the seeds that survived a bite, 235 sprouted (104 within
32 columns of the door), 375 rotted, **469 were eaten again -- 220 of them
by the door**. `pip.ron` argued the shipped ant could not see a pip
(40 J x quality 0.25 = 10 J, under `EAT_YIELD_THRESHOLD` 12). The lab's
founder gut is -0.8 (`labforage` prints it), so its quality is 0.81 and a
pip is worth 32 J: the premise no longer holds in the lab.

## 3. Pips the lab ant does not eat

**Change:** `pip.ron` `food_energy` 40 -> 14. At gut -0.8 that is
14 x 0.81 = 11.3 J, under the threshold; a -1.0 seed specialist still sees
14 J, so the seed-predator niche the material was built to leave open is
still open (`a_neutral_gut_cannot_see_a_pip_but_a_seed_specialist_can`
reads the compiled `pip.ron` and still passes). The margin is thin: a
colony whose gut drifts to -0.86 or beyond sees a 14 J pip again
((1 - 0.07)^2 x 14 = 12.1), so a seed-eating lineage can still evolve in the
lab -- it has to move toward it.

**Deciding measures, named before launch.** Mechanism, `labforage
scenario=played_bed frames=150000`, beds 1-12, paired: pips eaten,
plants from pips, plants from pips within 32 columns of a nest. Outcome,
`latecensus mister=1` 450k, beds 1-12, paired: kinds alive (4+), herb alive,
ants at 250k and 450k. Sign tests; the mechanism family (3) and the outcome
family (4) each Holm-corrected.

**Mechanism (150k), 12 paired beds:**

| | pip 40 | pip 14 | paired |
|---|---|---|---|
| pips eaten again | 470 | **0** | 0-12 (p 0.0005) |
| plants from pips | 225 | **333** | 12-0 (p 0.0005) |
| ... within 32 columns of the door | 91 | **144** | 10-2 (p 0.039) |
| pips rotted | 444 | 651 | 10-2 |
| ants alive at 150k | 232 | 320 | 8-4 (n.s.) |
| colony intake | 3,586 kJ | 3,969 kJ | 8-4 (n.s.) |
| plants standing | 375 | 268 | 4-8 (n.s.) |

All three mechanism measures pass Holm. The colony loses nothing it can
measure: pips were about 0.4% of its food.

**Outcome (450k), 12 paired beds** (the pip-40 arm reproduces the shrub
report's resprout arm bed for bed, which is the control that both builds
measure the same bed):

| at 450k | pip 40 | pip 14 | paired |
|---|---|---|---|
| beds with 4+ kinds | 8 | 9 | 3-2 (n.s.) |
| herb alive | 9 | 10 | count 5 more / 6 fewer (n.s.) |
| tree alive | 5 | 7 | count 5 / 3 (n.s.) |
| scrambler, shrub, grass alive | 9, 12, 11 | 8, 12, 12 | n.s. |
| ants, median at 250k / 450k | 132 / 107 | 185 / 143 | 7-5, 6-6 (n.s.) |
| dead colonies at 450k | 2 | 1 | |

**Level.** No outcome measure moves, in either direction, at 12 beds. The
change fixes a premise that had silently stopped holding, moves the
mechanism it was for strongly and on every bed, and costs nothing
measurable, so by the defaults-on rule it ships. What it changes on screen
is mid-game -- more seedlings by the door at 150k -- not the community that
is left at 450k; that is set by the shrub-tree trade in §4.

## 4. The tree: shrubs that survive the colony crowd it out

With resprout on, the tree is the role most often missing (alive on 5 of 12
beds against 8). Tree counts per bed across the run (`arms/base_s*` against
`arms/collar_s*`, the shrub report's arms):

| at | trees, before resprout (sorted) | trees, resprout | resprout more / fewer |
|---|---|---|---|
| 200k | 0 0 0 1 1 7 9 10 11 12 14 17 | 0 0 0 0 0 1 1 2 3 3 9 12 | 2 / 7 (p 0.18) |
| 450k | 0 0 0 0 2 2 16 17 17 18 20 26 | 0 0 0 0 0 0 0 2 5 6 8 9 | 1 / 7 (p 0.07) |

Not significant at 12 beds on its own, but the per-plant trace says what it
is. `planttrace life=1 bites=1`, beds 4, 5 and 6, resprout on and off:

- **The first tree dies early in all three resprout runs** and seeds little
  (29-187 seeds set, against 73-306 with resprout off). On bed 4 it went
  from 1,926 shoot cells (2,254 cells with roots) to 16 shoot cells (337
  with roots) between frames 45,500 and 46,000: the trunk
  came off its roots and the crown fell as one piece (picture strip in the
  pictures page; `SNAP_PROBE` reported no `break_under_load` snap, so it was
  the structural tick's detachment or span rule). Without resprout the same
  founder lived on beds 4 and 6.
- **Resprout itself never fired on the tree before it fell.** A scratch probe
  at the collar-sprout site (bed 4, to 46,500) logged 6,534 sprouts -- grass
  6,094, shrub 173, scrambler 145, herb 121 -- and one on the tree, at frame
  45,882, on the stump after the fall (345 cells, almost all root).
- **Then almost no tree seedling establishes**: tree lives 37 / 188 / 30
  with resprout against 1,204 / 337 / 1,067 without; established offspring
  0 / 0 / 0 against 6 / 3 / 7.

**The reading:** the bed without a colony is a shrub bed where trees stay
at 0-13 (no-colony tree counts at 450k: 1 0 0 3 0 13 1 2 0 0 6 1). Before
resprout the colony grazed the shrubs out and the trees moved into the
space; resprout keeps the shrubs, so the bed returns to its no-colony
state. That is grazer-mediated coexistence working in reverse, and it says
the tree's weakness is **recruiting into a standing shrub layer** -- exactly
what rungs 5-6 of `Reports/plant-community-plan-2026-10-20.md` (invasion
from rare into a grown stand) measure. The run recipe for that test on this
bed, with the colony, is in §5.

## 5. Rungs 5-6 on this bed, with the colony

`examples/plant_m0.rs` `mode=stand` gains `bed=played` (this commit):
`played_bed` as the game builds it, with every placement of the invading role
removed, the colony on its own timeline (`colony=0` clears it), sowing and
gaps kept out of the colony band (`columns`), a gap cut that spares animals,
an established count (20+ shoot cells) and an ant count per row, a `cripple`
arm (the role at photosynthesis x0.1, the known answer), a per-arm cell hash
and the collar sprouts made inside the window. Pilot, herb, bed 1, one core: 300k grow 407 s, arms
1.87 ms per arm-frame; two resident-only clones hash-identical.

The recipe (stage 0 = 4 roles x 4 beds, 5.6 core-hours; the deciding
measure is established invaders at 150k against the 4 sown, one-sided sign
test per role, Holm over 4 roles) and the plan gates it does not yet meet
(repeatability, heritability, role-specific scoring, a grass trend in the
resident-only control) were written up for this lane on 2026-10-30; the
arms, in order, are `none` (resident-only clone, the no-trend gate),
`shipped`, `cripple`, and on bed 1 a second `none` (determinism: the two
`HASH` lines must match). Grass is predicted to fail the no-trend gate (it
fell from 300k to 400k on 11 of 12 resprout beds, p 0.006); the fallback
decided in advance is `grow=450000`, then judging residents only against
their own clone. Stage 1 (12-24 beds per role, 17-33 core-hours) needs the
Google VM.

## 6. Stage 0 result: can a role get back in from four seeds?

Run on the Google VM (2026-10-31, built from `main` f245c3a9: pip 14,
fungus garden on, resprout on), `run.sh` in
`/mnt/project-files/plants/rungs56-stage0/`. For each role, beds 1-4: 300k grow,
then four seeds sown into 16-column gaps, 150k window. `E` = plants of
the role with 20+ shoot cells; four were sown.

| role | E at 150k, beds 1-4 | crippled arm | shipped above cripple |
|---|---|---|---|
| tree | 0, 1, **9**, 0 | 0, 0, 0, 0 | 2 beds, 2 ties |
| shrub | 0, 3, 1, 0 | 0, 0, 0, 0 | 2 beds, 2 ties |
| herb | 0, 2, 0, 0 | 0, 0, 0, 0 | 1 bed, 3 ties |
| scrambler | 3, 2, 2, 4 | 0, 0, 0, 0 | 4 beds |

**Gates.**
- **Determinism:** passes. Both resident-only clones on bed 1 hash the same, for all four roles.
- **Known answer:** passes. The crippled role never established (0 of 16). The shipped role beat it on 9 bed-roles and never lost (9-0, one-sided sign p 0.002, pooled across roles). The instrument can see invasion where it happens.
- **No trend among residents:** passes. Established residents, f=10k against f=150k, by kind (up / down / same): grass 7/5/4, herb 5/2/5, shrub 4/8/0, scrambler 4/4/4, tree 4/2/6. The grass trend predicted in §5 did not appear here.

**The reading.** No role grew from rare on more than one bed of four. The tree did so once, 4 seeds to 9 established on bed 3. The scrambler held its ground (3, 2, 2, 4). The herb and the shrub mostly failed. Four beds is a gate check, not a verdict: at this n no role's sign test can pass Holm.

**One pattern, found after the fact, so a hypothesis for the next run and not a result.** On the tree and shrub bed-roles, the role established (E >= 1) on four, where the colony numbered 258-392 ants at sowing. It established on none of the four where the colony numbered 582-643. That is perfect separation, 4 against 4 (exact Mann-Whitney p 0.029 two-sided, one test chosen after looking). On two of the failures, tree and shrub on bed 1, the four sown seeds were gone before 10k frames: no plant and no waiting seed. That fits seed predation. The pip change does not cover a sown `seed`, and a big colony finds four seeds in a cut gap quickly. **With the garden-sized colonies, recruiting from a handful of seeds may not be the tree's only problem: its seeds have to survive the colony first.**

**Next:**
- Stage 1, 12 beds per role, on the VM.
- Add a sown-seed fate count to the harness first: eaten, carried, rotted or germinated. Then the predation reading becomes a measurement.

## Reproduce

```text
latecensus scenario=played_bed mister=1 seed=S frames=450000 sample=50000      # [no_colony=1]
planttrace scenario=played_bed seed=S frames=450000 life=1 bites=1 out=DIR      # PIXEL_PHYSICS_COLLAR_SPROUT=0|1
labforage scenario=played_bed seed=S frames=150000                              # pip exits line
plant_m0 mode=stand bed=played role=R seed=S grow=300000 window=150000 sample=10000 sow=4 gap=16 arms=none,shipped,cripple
```
