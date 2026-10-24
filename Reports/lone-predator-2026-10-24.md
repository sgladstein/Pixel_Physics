# The lone predator: a spider trained by in-run selection against the ant, and the colony's answer

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

## Stage two: the colony with a spider on its bed (2026-10-24)

At the Teacher lane's ask, on its bed (`examples/inrun.rs` + `scratch/inrun/colsel.py`, founder `hcs`, the `bp1pile`
args: breeding on), the prey's side, with the spider held fixed (pool `score-1`, not evolving).

**What was built**, all off by default:

- `inrun spiders=POOL:N` keeps N spiders on the bed (`spider_start`, `spider_gap`, `spider_far`, `spider_grants`),
  logs `spider`, `spdeath` and `wound` (spider cells bitten off) events, and keeps the spiders' meals out of the ants'
  food rows. It turns the bite on, and the colony's mind is then offered Attack and the alarm emit
  (`colony_bed::verbs`, `emits`); a founder with the bite banned takes it as a blank verb (`mind::enter_bite`,
  `ant_bite=K` its job constant, -100 the "cannot bite" control).
- colsel cases `+guard` (fewer ants killed) and `+slay` (damage done to spiders: cells bitten off, six for a kill),
  columns `killed`, `slay`, `spider_at`, and `spiderramp=START:STEP:K[:FROM[:FLOOR]]`, the spiders' arrival ramped
  like `pilecap` and carried in the resume file. `LEXSEED` for a replicate arm.

**What it did.** Single runs, 200,000 frames:

| Setting | Colonies with ants left at the end | Ants killed a run | Spiders the ants killed a run |
|---|---|---|---|
| 4 spiders from frame 0, 100,000 frames, bite / no bite / bite wired on | 0 of 12 | 24-237 | 1-13 / 0 / 1-12 |
| 1 spider from 20,000, ants can bite | 1 of 4 | 34-88 | 0-3 |
| 1 spider from 20,000, ants cannot | 1 of 4 | 39-93 | 0 |
| 1 spider from 100,000, 50,000 refill gap, can bite | 4 of 6 | 0-18 | 1 in 5 of 6 runs |
| same, cannot | 2 of 6 | 9-30 | 0 |

**The ants fight from the start.** With the bite entered as a blank verb, the colony kills about one spider a round.
Held off, it kills none. The switch with no spider on the bed costs the colony nothing distinguishable from a different
mutant draw (mean alive over the run 49 against 56 and 50 for two draws with it off, 12 seeds).

Selection, five arms of 16 colonies x 10 rounds, one spider from round 2 at frame 100,000, 50,000-frame gap (the ramp
never stepped: median killed stayed between K=10 and 3K). In-run medians over rounds 5-9, and the common garden of the
round-9 carry (fresh world, selection off, the same spider):

| Arm | In-run food home | In-run ants killed | Garden alive | Garden food home | Garden killed per 1,000 deliveries |
|---|---|---|---|---|---|
| plain cases | 6,347 | 27 | 70 | 7,141 | 3.2 |
| plain cases, replicate | 5,973 | 21 | 74 | 2,656 | 3.8 |
| + guard | 5,009 | 16.5 | 16 | 1,694 | 4.7 |
| + slay | 4,532 | 14.5 | 4 | 1,559 | 5.8 |
| + slay, replicate | 3,253 | 27 | 95 | 3,369 | 6.6 |
| plain cases, **no spider** while selected | 2,328 | -- | 10 | 1,406 | 4.6 |

- **Neither defence case beat plain selection on defence, and both cost food.** +guard cut ants killed by paying
  colonies that forage less: killed per delivery did not fall. +slay did not raise the damage done to spiders (median
  9-10 against 11 plain). Both took in-run food from about 6,000 to 3,300-5,000.
- **Plain selection with a spider on the bed learned something.** Ants killed per 1,000 deliveries fell from about 17 in
  round 2 to 3-5 by round 9 in every spider arm, and damage to spiders rose from about 7 to 11-17. But a line selected
  with no spider, judged against the same spider, loses ants at the same rate per delivery (4.6). So what fell is the
  colony getting better at everything, not a defence the line without a spider lacks.
- **Garden numbers swing more between replicates than between arms** (plain 7,141 against 2,656 food home), so 16
  colonies cannot rank these arms. A defence case needs the 64-colony scale.
- The spider-selected lines forage much more than the no-spider line (in-run 6,000 against 2,300). A corpse delivered
  is not it (corpse pick-up off moves garden food 1-13%), and one no-spider replicate cannot say whether it is the
  predator or the draw.

Picture: `/mnt/project-files/predator/ants-kill-spider.gif` (ants yellow, spider red, food green): the colony
kills a trained spider at its food pile.

## Stage three: spider sight, and the switches on by default (2026-10-24)

**Spiders see.** Owner (15:07Z): "It seems realistic that spiders should use sight." A mind's senses were all
touch-range plus scent; the spider's species file had a 64-cell eye that only its hand-built fallback brain used.
`PIXEL_PHYSICS_MIND_SIGHT` gives a mind with an eye the game's own cast (`creature::sighted`: 16 rays all round,
stopped by soil and rock), and reads the nearest prey on the **home senses** rather than new inputs: a new input
would move every weight after it in every genome on disk, and a spider's home was only the cell it landed on. No
prey in sight reads as no home. The eye is billed as every eye is. Off, byte-identical (inrun `events.csv` and
`nest.csv`, seed 3, 40k frames, 2 spiders).

| Trained 6 rounds x 100k (`predtrain`, 4 seeds), judged on beds 101-106 (4 spiders, 30k, sel=off) | mean ants killed | median | paired |
|---|---|---|---|
| from the hunter, sight on | **80.6** | 39 | wins 19 of 24 |
| from the hunter, sight off | 40.9 | 33 | wins 4 |
| from pool `score-1`, sight on | 63.2 | 36 | |
| from pool `score-1`, sight off | 87.2 | 60 | |

The judged kills are bimodal (about 30 or 100-190 a bed); sight moves judgings into the high mode (10 of 24 against 2
of 24). **A blind-trained pool loses from sight**: its home weights were learnt on the landing cell. Spiders saw an
ant on 1-15% of ticks; ants are mostly underground. The sighted pool is `predator/pools/sight-h2` in the project
files (seed 2's last pool). `/mnt/project-files/predator/spider-sight-paired.gif`: the same bed, sight above, none
below.

**On by default** (owner, 17:34Z: "Everything should be on by default"): `PIXEL_PHYSICS_MIND_ATTACK` (the bite, the
corpse pickup and the colony bed's alarm emit) and `PIXEL_PHYSICS_MIND_SIGHT`, each with `=off`. The Teacher lane
asked first whether the bite or alarm fire between nestmates on a bed with no spider. Four seeds, std_pool founders,
200k frames, sel=score, on against off:

| seed | bites | alarm laid | food home (on/off) | starved (on/off) | alive (on/off) |
|---|---|---|---|---|---|
| 1 | 0 | 2,198 | 11,681 / 9,971 | 24 / 18 | 80 / 75 |
| 2 | 0 | 13,054 | 14,046 / 9,439 | 24 / 7 | 88 / 86 |
| 3 | 0 | 4,658 | 8,247 / 9,085 | 20 / 19 | 75 / 72 |
| 4 | 0 | 7,473 | 10,868 / 9,079 | 8 / 23 | 77 / 75 |

No bite fires (no stranger lives on the bed); mutants lay some alarm (against 290-350k of scent A). Food and starving
show no harm on four seeds. A genome that holds Attack banned (every trained pool, the lab's new ant) still never
bites outside `inrun`, the only caller of `mind::enter_bite`.

**Co-evolution**: inrun `spider_mut=PM:SIGMA:EVENTS spider_out=DIR` makes each placed
spider a mutant of its pool parent and writes its credit (flesh swallowed, by face worth / 100) over its life
(floored at 20,000 frames: a spider that gorged and died in 1,330 frames otherwise scored 124.8); colsel
`spiderevo=KEEP:PM:SIGMA:EVENTS` takes the best KEEP placements across all colonies as the next round's pool
(`OUT/spool/rN`, `OUT/spool.csv`; carried in `state.json`). Gardens face the current pool unmutated.

**The first co-evolution batch: the spiders evolved backwards.** Three arms, local, each 16 colonies x 12 rounds x
200k frames from `std_pool` (`lesson=c1:0.1`), one spider at a time from frame 100k every 50k (`spiderramp=
100000:20000:10:2`), `CG_EVERY=4`: **coevo** (blind spiders from `score-1`, `spiderevo=8:0.03:1.0:4`), **fixed**
(the same spiders, not evolving) and **sight** (sighted spiders from `sight-h2`, evolving). Judged after round 11
on each colony's garden world (seed c x 1000 + 99, sel=off, 200k frames, spiders from 60k every 50k), every
colony line against the starting spider pool and against its arm's last evolved pool:

| colonies | spiders | ants killed (median / mean) | spiders killed by ants (mean) | food home (median) | starved (median) |
|---|---|---|---|---|---|
| founder | score-1 | 57 / 48.8 | 1.88 | 10,601 | 9 |
| founder | coevo's last | 32 / 38.6 | 0.88 | 10,906 | 8 |
| coevo | score-1 | 42 / 36.9 | 1.62 | 15,351 | 1 |
| coevo | coevo's last | 34 / 57.2 | 0.56 | 15,037 | 0 |
| fixed | score-1 | 49 / 53.2 | 1.75 | 16,061 | 1 |
| fixed | coevo's last | 28 / 34.1 | 0.81 | 14,058 | 1 |
| founder (sighted) | sight-h2 | 65 / 73.8 | 2.25 | 10,464 | 10 |
| founder (sighted) | sight's last | 40 / 29.2 | 0.88 | 11,090 | 10 |
| sight | sight-h2 | 41 / 53.6 | 2.12 | 12,705 | 5 |
| sight | sight's last | 21 / 39.1 | 0.94 | 12,545 | 5 |

- **The evolved spider pools kill less than the pools they started from**: fewer ants in 49 of 77 paired judgings,
  more in 28. They are also killed by ants half as often. Ten rounds of picking the 8 best of about 30 placements,
  each placement scored once, selected spiders that hunt less and avoid ants. Each placement is one noisy sample;
  sigma 1.0 over 4 events moves a tuned genome a long way; and nothing keeps a parent that was already good. **Before
  another run:** keep the parents (elitism), judge each candidate on more than one life, and mutate more gently.
- **Colonies: no clear defence learning.** Coevo colonies lost fewer ants to `score-1` than fixed colonies in 12 of 16
  paired colonies (591 vs 852). But the fixed line was the one trained against `score-1`, and neither trained line
  beat the untrained founder clearly (coevo 8 of 16, fixed 7 of 16). The sighted line lost fewer ants than the founder
  to the sighted spider in 11 of 16 (858 vs 1,180): suggestive, at this size.
- **Every trained line got better at living** (starved 9 -> 0-5, food home 10.6k -> 12.5-16k): that is the training
  bed itself, with or without spiders, not defence.

Where it goes next: fix the spider side's selection (above) before spending a 64-colony VM batch on the arms race.
The judge is `scratch/inrun/predjudge.py` (`S=` the folder holding the arms as `ce/coevo`, `ce/fixed`, `ce/sight`;
`SIGHT=on COLS=founder,sight EVO=sight ORIG=.../sight-h2` for the sighted grid). The judged rows are in
`/mnt/project-files/predator/coevo1/`.

## Reproduce

```
cargo run --release --example predtrain -- out=DIR founder=hunter sel=score seed=1 rounds=10 frames=100000
cargo run --release --example predtrain -- out=DIR pool=DIR/r9/pool sel=off preds=4 rounds=1 frames=30000 seed=101
cargo run --release --example predtrain -- ... gif=0:3000:15000:30 crop=40,34,176,40 scale=4   # rN/round.gif
# stage two: the batch's arms (ARGS = line 2 of scratch/inrun/jobs/broodplant1.txt from its 9th field; LEXSEED=2 for a replicate)
CG_EVERY=5 python3 scratch/inrun/colsel.py OUT lex+spoil+crater+shape[+guard|+slay] 16 10 200000 scratch/inrun/genomes/hcs.raw.txt $ARGS \
    spiders=/mnt/project-files/predator/pools/score-1:1 spider_gap=50000 spiderramp=100000:20000:10:2
```
