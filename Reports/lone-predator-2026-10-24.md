# The lone predator, stage one: a spider trained by in-run selection against the ant

*Lone predator thread, 2026-10-23/24. Owner's ask (2026-10-23 23:13Z): "train a new lone predator creature using
the same in colony training scheme the ants use. Eventually we should competitively train the ant colony to survive
and defend while training the predator to eat the ants." This is stage one: the predator trains, the ant does not.*

## What was built

- **The spider** (`assets/species/spider.ron`, `assets/materials/spider.ron`): a six-cell rigid body, three wide and
  two tall, ticking every 4 frames against the ant's 6. Its jaw (0.5) cannot cut soil (0.8), so it cannot dig, and
  takes a cell of ant (0.25) in one bite; its plate (2.0) takes four ant bites a cell. No nest. Every number is set
  against the ant and its derivation is in the species file. Hidden from the lab's animal chip until a trained mind
  ships with it (`lab::ui::NOT_STOCKABLE_IN_LAB`).
- **The bite, for a mind** (`creature::mind::attack`, switch `PIXEL_PHYSICS_MIND_ATTACK=on`, **off by default**):
  the Attack verb, listed since stage 2 and never picked, now acts. It is the game's bite (`contest::bite_progress`,
  jaw against plate) on a stranger in reach; a cell worn through comes off, into the crop if the gut takes it;
  `reconcile_chain` makes a bitten head a death; alarm is cried where it lands. With the switch on a mind may also
  pick up a corpse its gut takes. Off, nothing is read or written: Attack is never picked and no corpse is offered.
  Guard: `mind::tests::the_bite_takes_a_strangers_cell_and_the_mouth_keeps_it`.
- **The `hunter` preset** (`mind::preset`): bites any stranger in reach, picks flesh up, never rests, walks where the
  ants' scent A grows. The `follower` preset on the home trail, with the bite on top.
- **The training loop** (`examples/predtrain.rs`, helpers in `train::predator`): the colony training bed with a colony
  of the lab's ant mind (`h1`), not evolving, held at 30 by ants hatched in the nest; 12 spiders scattered at least 30
  columns from the door. Every 1,000 frames the best adults (flesh eaten per 1,000 frames alive) have children, each
  a size-neutral mutation of its parent, placed at a random column; the worst are culled above 16. A starved spider
  stays a candidate parent for 20,000 frames at half its score. Each round's 8 best found the next. `sel=random` is
  the same births and culls chosen blind. The spiders share one colony label, so they do not bite each other.

## What it did

Training: 8 seeds x 10 rounds x 100,000 frames, local (a 100,000-frame round takes about 15 s on 4 cores).
Judging: each line's last pool on 6 fresh beds (seeds 101-106), 4 spiders, no births, 30,000 frames; ants killed.

| Start | Arm | Ants killed (mean of 48 or 6 runs) | Median |
|---|---|---|---|
| blank mind | by score | 79 | -- |
| blank mind | random parents | 71 | -- |
| blank mind | untrained | 74 | 81 |
| hunter | by score | **145** | 160 |
| hunter | random parents | 57 | 59 |
| hunter | untrained | 116 | 158 |

- **From a blank mind, selection held the hunt but did not improve it** (79 against 74 untrained). The early gain
  available to it was switching rest off; finding the colony by scent never appeared in 780 births a line.
- **From the hunter, 6 of 8 trained lines beat the hunter** on the 6-bed total, and the best two (190 each) did so on
  every bed. **Random parents lost the hunt**: two of eight lines fell to 0 and 15 kills, one by switching its bite
  off. The trained spiders are steadier than the hunter, which fails on two of the six beds (34 and 43 kills).
- **What selection kept**: the bite and the trail-climb; some lines added the food trail (scent B, where foragers
  walk) and turned rest off. In the pictures the trained spiders wait at the nest door, where every forager passes.
- **The crater nest is no refuge**: a blank spider walks into the training bed's open nest mouth.

Pictures: `/mnt/project-files/predator/spider-hunt-paired.gif` (hunter above, a trained pool below, same bed).
Pools: `/mnt/project-files/predator/pools/`.

## Where it goes next

Stage two is the owner's competition: the ant trains to survive and defend while the spider trains to eat it. The
ant already has what it needs in the engine. Its Attack verb acts under the same switch, four bites take a spider
cell, the alarm scent is laid where a bite lands, and its options carry a stranger flag on every cell in reach.
What stage two needs from the Teacher lane is Attack and the alarm emit in the ant's arena verbs, and spiders on its
bed. Until then a trained spider pool is the fixed predator to train the colony against, as the h1 ant was the
fixed prey here.

## Reproduce

```
cargo run --release --example predtrain -- out=DIR founder=hunter sel=score seed=1 rounds=10 frames=100000
cargo run --release --example predtrain -- out=DIR pool=DIR/r9/pool sel=off preds=4 rounds=1 frames=30000 seed=101
cargo run --release --example predtrain -- ... gif=0:3000:15000:30 crop=40,34,176,40 scale=4   # rN/round.gif
```
