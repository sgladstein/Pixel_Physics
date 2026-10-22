# M2: the mixed community bed as the judge (2026-10-22)

The plan's rung 7: the four roles sown together in the 512-wide lab bed
and left to run, the bed every trained species has to live in. `plant_m0
mode=community` gained `swap=ROLE:FILE.ron,...`, which registers a trained
file under the shipped role's name right after the bed is built (founders
are still seeds then), so the census, mix and pictures read it as that
role. Positive control: swapping in the rung-2 shrub re-fit alone takes the
shrub's leaf cells at frame 4,000 from 127 to 339 on the same dice.

Command (the plan's training world, cut to 200,000 frames -- about five
minutes a run):

    plant_m0 mode=community mix=roles loadfail=0 rain=steady spells=20000,40000
      ground=patchy seed=S frames=200000 sample=40000 shots=200000
      [swap=tree:../refit/tree/score_best.ron,shrub:...,herb:...,creeper:...]

## Shipped against the rung-2 re-fits

Frame 200,000, plants alive and leaf cells per role (`<arm>_s<seed>.txt`):

| seed | arm | tree | shrub | herb | creeper |
|---|---|---|---|---|---|
| 1 | shipped | 18 / 5,530 | 9 / 541 | 57 / 891 | 5 / 33 |
| 1 | re-fits | 7 / 7,270 | 6 / 325 | 23 / 1,897 | 1 / 76 |
| 2 | shipped | 19 / 3,899 | 17 / 258 | 59 / 884 | **0** |
| 2 | re-fits | 14 / 10,202 | 11 / 423 | 6 / 481 | **0** |
| 3 | shipped | 14 / 5,214 | 6 / 332 | 65 / 799 | 2 / 19 |
| 3 | re-fits | 18 / 9,316 | 13 / 360 | **0** | **0** |
| 4 | shipped | 21 / 4,405 | 4 / 770 | 54 / 880 | 2 / 49 |
| 4 | re-fits | 10 / 6,910 | 12 / 778 | 28 / 2,005 | **0** |

**The re-fits fail the community.** The tree's canopy roughly doubles
(leaf 4,760 -> 8,425 mean) and closes over the bed
(`shipped-vs-refits-seed2.png`), the herb goes extinct on one seed of four
and falls from 59 plants to 6 on another, and the creeper is gone on three
of four (shipped: one). This is rung 4's finding at the scale of the whole
bed: rung 2 bred plants that win by taking light from everything else.

The shipped creeper is failing here too -- one seed extinct and two or three
plants left on the others -- which is the creeper's own problem (its
lifespan and ramets, still open with the owner), not the trainer's.

## The rung-4 champions trained with the neighbour floor

Same bed, same seeds. `floor`: all four floor champions (`../hold/floor/`);
`floor3`: shrub, herb and creeper champions with the shipped tree;
`herbshrub` and `creeperonly`: the champions of those roles alone, the rest
shipped. Frame 200,000, plants / leaf cells:

| arm | tree | shrub | herb | creeper |
|---|---|---|---|---|
| shipped (seeds 1-4) | 18 19 14 21 / 4,762 mean | 9 17 6 4 / 475 | 57 59 65 54 / 864 | 5 0 2 2 / 25 |
| re-fits | 7 14 18 10 / 8,425 | 6 11 13 12 / 472 | 23 6 **0** 28 / 1,096 | 1 **0 0 0** / 19 |
| floor | 14 11 20 11 / 4,635 | 7 2 15 25 / 447 | 16 43 3 25 / 1,391 | **0 0 0** 1 / 19 |
| floor3 | 5 13 18 19 / 4,151 | 8 23 13 16 / 496 | 27 31 23 24 / 1,520 | **0 0 0 0** / 0 |
| herbshrub | 8 10 28 14 / 4,759 | 7 34 1 32 / 722 | 19 22 41 24 / 1,424 | 1 3 2 0 / 19 |
| creeperonly | 18 20 7 32 / 5,544 | 26 6 9 5 / 444 | 28 44 29 29 / 518 | 2 2 0 3 / 68 |

**With the floor the bed is back to four roles' worth of tree, not
twice it.** The floor champions' tree canopy is the shipped tree's
(4,635 leaf cells against 4,762; the re-fits' 8,425), and the herb no longer
goes extinct: it lives as fewer, larger plants (16-43 against shipped's
54-65, with half again the leaf), and stays near 25 on every seed with the
shipped tree. `shipped-refits-floor-seed2.png` shows the three beds.

**The creeper is failing in every arm, shipped included.** Its count falls
on every seed from the first census (shipped: 8-14 at frame 40,000, 0-5 at
200,000), and swapping it alone or leaving it shipped beside the trained
herb and shrub changes nothing outside that spread. Four seeds of two or
three plants cannot tell the arms apart. This is the open creeper question
(lifespan, or ramets that root as their own plants), not a trainer result.

## At full length, 450,000 frames: the floor champions do not hold the bed (2026-10-22)

Shipped against all four floor champions, six seeds, the plan's full
450,000 frames (`450k/`, about eight minutes a run). Frame 450,000, plants
per role:

| seed | shipped tree / shrub / herb / creeper | floor champions |
|---|---|---|
| 1 | 28 / 2 / 22 / 0 | 15 / 7 / **1** / 0 |
| 2 | 21 / 2 / 41 / 0 | 17 / **0 / 0** / 0 |
| 3 | 6 / 0 / 77 / 0 | 15 / 0 / **0** / 0 |
| 4 | 19 / 0 / 67 / 0 | 10 / 25 / 19 / 0 |
| 5 | 25 / 0 / **0** / 0 | 37 / 0 / **0** / 0 |
| 6 | 10 / 0 / 16 / 0 | 10 / 0 / **0** / 0 |

**The 200,000-frame result does not survive.** The trained herb goes
extinct on four seeds of six (shipped: one), and on four of six the bed
ends as trees alone (`450k/shipped-vs-floor-450k.png`). The cause is in
the seed bank, which each row of `450k/*.txt` prints: the trained herb's
waiting seed sits near 25 through the run against the shipped herb's
~100. It grows as fewer, larger plants, and fewer plants make fewer seeds,
so each bad spell takes it a step down that it does not climb back. Rung 4
is one life and pays a flowering plant the same however many seeds it
sets; this is the plan's rung 5 (recruitment) and the reason it comes
before anything ships. **Not shipped, and not to be: no floor champion
replaces a shipped species.**

**The shipped bed does not keep four roles at this length either.** The
creeper is gone on all six seeds by 450,000 and the shrub on four; what is
left is tree and herb. The plan already records this for the four-role
bed (its §M1 step 6 table: creeper gone by 120,000-180,000, shrub by
720,000, nothing under the tree's canopy -- the niche A5 exists to open),
and it is unchanged here. A trainer cannot fix a role the bed cannot carry
at all, so the creeper's lifespan or ramets (open with the owner) and an
understory under the canopy come before training those two.

## The seed-trained roles at 450,000 frames (2026-10-22)

`seeds`: all four rung-4 champions trained with seed set paid
(`../hold/seeds/`); `seeds3`: the same shrub, herb and creeper with the
shipped tree. Six seeds, frame 450,000, plants per role
(`450k/seeds*_s*.txt`):

| seed | shipped T/S/H/C | floor | seeds | seeds3 |
|---|---|---|---|---|
| 1 | 28/2/22/0 | 15/7/1/0 | 20/0/9/0 | 18/1/14/0 |
| 2 | 21/2/41/0 | 17/0/0/0 | 17/3/0/0 | 21/23/9/0 |
| 3 | 6/0/77/0 | 15/0/0/0 | 7/21/36/0 | 25/9/0/0 |
| 4 | 19/0/67/0 | 10/25/19/0 | 17/11/20/0 | 9/5/72/0 |
| 5 | 25/0/0/0 | 37/0/0/0 | 32/0/6/0 | 24/2/40/3 |
| 6 | 10/0/16/0 | 10/0/0/0 | 9/1/0/1 | 9/31/39/2 |
| **roles alive, mean** | **2.2** | 1.7 | 2.5 | **3.2** |

**With the shipped tree, the seed-trained understory holds the bed.** The
shrub is alive on all six seeds (shipped: two, at 2 plants), the herb on
five of six (shipped: five), and all four roles last the run on two seeds
-- no shipped arm has managed that at this length. The creeper is still
nearly gone. `450k/shipped-vs-seeds-450k.png`, seed 6: the shipped bed is
trees with one herb gap; the seeds3 bed is two trees and a shrub-herb
thicket in flower.

**The seed-trained tree costs the others.** With it the shrub drops to
four seeds of six and the herb to four: it sets three to four times the
shipped tree's seed, and the bed has no term that charges a tree for
filling every gap with its own seedlings. That is the plan's rung 5/6
(recruit and invade against a resident stand), not something rung 4 can
see. **Candidates to ship, pending the owner: the seed-trained shrub and
herb.** Not the tree, not the creeper.

## Eighteen seeds: the shrub holds, the herb pays for it (2026-10-22)

Seeds 7-18 added to `shipped` and `seeds3` (three runner threads,
`/mnt/project-files/plant-community/runners/mixed450.sh`; logs in `450k/`).
Beds out of 18 in which each role is alive at frame 450,000:

| arm | tree | shrub | herb | creeper | roles alive, mean | herb plants, median |
|---|---|---|---|---|---|---|
| shipped | 18 | 3 | 17 | 4 | 2.33 | 39 |
| seeds3 (shipped tree) | 18 | **16** | **13** | 7 | **3.00** | **14** |

The six-seed result holds for the shrub and the bed as a whole, and not
for the herb: the trained herb is lost on five beds of eighteen against
the shipped herb's one, and where it lives it is a third as many plants.
On the beds where it dies the trained shrub is large (s11: 43 shrubs, s13:
28), so this may be the shrub's thicket rather than the herb itself. Each
trained species alone in an otherwise shipped bed (`split450.sh`, arms
`shrub1` and `herb1`) is the run that separates the two.

### Each trained species alone (seeds 1-12)

`shrub1`: the seed-trained shrub alone in an otherwise shipped bed;
`herb1`: the seed-trained herb alone (`/mnt/project-files/plant-community/runners/split450.sh`,
logs in `450k/`). Beds out of 12 with each role alive at 450,000:

| arm | tree | shrub | herb | creeper | roles alive, mean | herb plants, median |
|---|---|---|---|---|---|---|
| shipped | 12 | 2 | 11 | 1 | 2.17 | 35 |
| seeds3 | 12 | 11 | 9 | 4 | 3.00 | 21 |
| **shrub1** | 12 | **11** | **12** | 0 | **2.92** | 26 |
| herb1 | 12 | 2 | 11 | 3 | 2.33 | 24 |

**The trained shrub is the gain, and it does not cost the shipped herb**:
alone it lives on 11 beds of 12 and the shipped herb beside it on all 12.
The trained herb alone does no better than shipped (alive on 11 of 12,
fewer plants), and with the trained shrub it is lost on three. The
recommendation is to ship the trained shrub and keep the shipped herb.

**Shipped, 2026-10-22 (owner's card, 14:29Z): the trained shrub only.**
`assets/species/shrub.ron` now exports byte-identical to
`../hold/seeds/shrub/score_best.ron`, with the hand-written notes kept and
each selected value marked. The lab's PLANTS switch (PR #811) is retired:
its shrub is now the game's, and its herb was not wanted.
