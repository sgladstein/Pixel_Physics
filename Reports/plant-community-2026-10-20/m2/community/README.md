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
