# The nest raider: a centipede that goes into the nest

*Nest raider thread, 2026-10-25. Owner's ask (2026-10-24/25): new predator types with different behaviours, "the
nest raider first" -- a predator whose body fits the ants' two-wide shaft, so it raids inside the nest rather than
picking off foragers on the surface, and so makes nest defence worth learning. Brief:
`/mnt/project-files/predator/nest-raider-handoff.md` from the lone predator lane
([lone-predator-2026-10-24.md](lone-predator-2026-10-24.md)).*

## What was built

- **The centipede** (`assets/species/centipede.ron`, `assets/materials/centipede.ron`): `Chain(5)`, one cell wide
  and five long, a following chain, so it fits any tunnel an ant fits and follows its head round corners (the chain
  is blocked on 2-12% of its moves against 25-43% for a rigid body, `creature.rs`). Its jaw (0.5) cannot cut soil
  (0.8) or a packed lining (0.95), so it comes in by the door. The spider's plate (2.0: four ant bites a cell), its
  gut, its prices per cell, its 3,200 J breeding bar and its handling time; ticks every 5 frames (ant 6, spider 4);
  sight 24 (spider 64). A bitten-off segment is a shorter centipede, a bitten head a dead one. On the lab's COLONY
  chip, and every centipede in a box gets a mind from `assets/minds/centipede_pool.txt` (`Lab::give_predator_minds`,
  the spider's hookup made per species).
- **It shoves through stored food it cannot eat** (`CreatureDef::digs_inedible_food`, the centipede only): food its
  gut will not take is loose ground to a mind's Dig. Walled in with the cut in its jaws, the mind's pack move puts
  the cell back where its tail was (`options::pack_on`), the whole `Cell` riding in the pellet, so a crumb keeps its
  worth and the store is moved, never lost. Its verbs are the spider's and Dig and Put Down
  (`train::predator::RAIDER_VERBS`). **Owner, 2026-10-25: yes, it shoves through crumbs** (see *Why it shoves*).
- **The harnesses take the species**: `predtrain species= warm= far= credit=`, `spiderlab species= far= gif=
  pool=<preset>`, `inrun spider_species=`; `train::predator::place_predator`, `verbs_for`, `mind_for`,
  `mutate_breeding_of`, `picture`, `save_gif`. Defaults are the spider's, unchanged.

## Why it shoves: the trained colony's nest has no tunnels

The first look, on the training bed after a head start for the colony (`predtrain warm=`, std_pool ants): **the
nest is a bowl of stored crumbs and the ants live inside it.** At 40,000 and 80,000 frames the chambers were solid
`crumbs` (a 0.2 powder, half-eaten food put down) inside a `packedsoil` lining, with ants in one-cell pockets and
no open cell a walker could enter. The brood lane found the same packed larder on the Teacher bed
([brood-in-training-2026-10-24.md](brood-in-training-2026-10-24.md)). The lab's own colony (the hand-built ant)
does keep open tunnels, but its two-wide shaft was jammed solid with ants.

So a raider that could not cut a cell of food it would never eat had no way in, and against the colony headed for
the lab it would have been a second spider at the door. With `digs_inedible_food`, three blank-minded centipedes
set at the trained colony's door (`far=0`, warm 80,000, seed 102) burrowed into the larder and emptied it: 30 ants
to 8 in 20,000 frames, refills included. Picture: `/mnt/project-files/predator/nest-raider/two-nests.png`.

## Two things fixed on the way

**A predator paid for its children out of the living crowd.** `try_bud`'s second mouth (`provision_at`) counted
every living ant cell round the head as food in reach, and its shortfall loop took them whole: no bite, no crop,
none of `handling`'s pace. One centipede set down at a lab_mid_fruit door (seed 1, frame 36,000, 1,600 J against a
3,200 J bar) **budded three times in its first 23 frames**, each child 800 J. With `handling` a cell of a living
animal is no longer a provision (corpses, bitten-off cells and fruit still are); the same landing made 0 births.
Gated on `handling`, so every species without it is bit-identical; `PIXEL_PHYSICS_HANDLING=off` restores it with the
rest. **It changes the spider too** (handling is on for it), so spider lab results judged on breeding move. It is
the third time this mouth has needed the first mouth's rule (the nectar-only flitter was the second,
`dead-ends.md`). Guard: `mind::tests::a_predator_with_handling_cannot_pay_a_birth_with_living_prey`, with the
beetle (a carnivore without handling) as the positive control; red with the gate removed.

**A raider could farm its own score.** Its Put Down (for pellets) also puts a crop cell down, and picking the same
cell of ant up again wrote a new swallow row each time: one 100,000-frame `predtrain` round logged 7,300 swallows
against 37 cells bitten, each paid at face value. `predtrain credit=digest` (the default for a species that can put
food down) and `inrun spider_species=centipede` now pay what the crop digests (`digest_log`). The spider, which has
no Put Down, keeps the swallow credit stage one used.

## Does a raider population live in a lab box?

`spiderlab species=centipede`, lab_mid_fruit (the lab's hand-built ant, its default), 300,000 frames, landed 30,000
frames after the first ant, at least 30 columns from the door, nothing refilled:

| minds | raiders | seeds | outcome |
|---|---|---|---|
| blank | 1 | 1-3 | killed by ants within the first thousands of frames, 0-3 births |
| blank | 2 | 1, 3 | the same |
| blank | 2 | 2 | **lived the whole run**: 2 -> 34 (110k) -> 1 (210k) -> 23 (290k), the colony 165-228 throughout, `coexist` 1.00 |
| hunter | 1-2 | 1-3 | killed early in all 6 |

The one population that took hold cycled like the textbook predator and prey: it rose on the colony, fell to one
(77 old age, 24 starved, 11 killed by ants over the run) and rose again, while the colony held. **It lived on the
surface, not in the nest**: in the pictures the raiders held the west side of the mound and the ants foraged east
(`/mnt/project-files/predator/nest-raider/`). A blank mind does not raid on purpose.

## Training the raider

`predtrain species=centipede`, four lines (founders blank and hunter, two seeds each), 8 rounds of 100,000 frames,
the std_pool colony given 60,000 frames alone first (`warm=60000`), paid on digestion. Each line's last pool (r7)
and the two untrained founders were then judged on six fresh beds (seeds 101-106, 4 raiders, 30,000 frames):

| mind | ants killed | of them in the nest |
|---|---|---|
| hunter (untrained) | 78 | 16 |
| blank-1 (trained) | 70 | 11 |
| blank (untrained) | 63 | 17 |
| hunter-2 (trained) | 53 | 3 |
| hunter-1 (trained) | 46 | 5 |
| blank-2 (trained) | 25 | 0 |

**No line beat its founder on the bed.** The round scores showed no upward trend either. This is the spider's
co-evolution result again (one noisy life a placement, no elitism; `lone-predator-2026-10-24.md`), and the bed's
kills are mostly at the door: the six arms together killed 335 ants, 52 of them in the nest.

## Which mind the lab gets: chosen in the lab, not on the bed

The bed's ranking says nothing about whether a raider population lives in a lab box, so the four candidates were
run where the pool will be used: `spiderlab`, lab_mid_fruit, 2 raiders, 300,000 frames, seeds 1-6 (landing as above). Measured
before `main`'s last-food memory and trunk crossing (PRs #883, #882) reached this branch; the memory enters this
layout-4 pool at weight 0, and lab_mid_fruit's trees are the only place the crossing can differ.

| mind | took hold (peak > 6) | lived the whole run | colony eaten out | raiders' peak | ants killed (mean) |
|---|---|---|---|---|---|
| hunter (untrained) | 0 of 6 | 0 | 0 | 2-6 | 264 |
| blank (untrained) | 2 of 6 | 1 | 1 | 2-38 | 289 |
| blank-1, r7 | 0 of 6 | 0 | 0 | 2-6 | 227 |
| **hunter-2, r7** | **4 of 6** | **1** | **2** | **2-241** | **722** |

**hunter-2 ships** (`assets/minds/centipede_pool.txt`, 8 minds). It was fourth of six on the bed and is
the only pool that makes a population in most lab boxes, which is the thing a stocked predator has to do to be
seen. It is also the only one that ate colonies out, so the lab now has a predator that can lose you the box.
The four outcomes are a spread rather than a switch: died out early (seeds 1, 6), a boom the ants beat back (seed 3:
33 raiders at 120,000 frames, 112 of them killed by ants in the next 30,000), a cycle for the whole run beside the
colony (seed 4: 10 raiders and 205 ants at the end), and a boom that ate the colony and then starved (seeds 2, 5).

**And it still does not raid.** The seed-3 boom filmed over the whole box, 100,000 to 126,000 frames
(`/mnt/project-files/predator/nest-raider/lab-boom-seed3-wholebox.gif`): the raiders live on the surface against
the east wall, at columns 428-507, 170-250 columns from the door at 256, and eat the foragers that range out that
far. None is in the shaft or a chamber in any frame. The trained pool makes a population by keeping its distance
from the nest and picking off the far edge of the foraging range, which is a predator the colony has to answer, but
not the one the owner asked for.

*Correction, 2026-10-25.* The first version of this paragraph said the raiders sat in two heaps 75 and 85 columns
either side of the door. It was read from a crop of the box (columns 136-376) that held no raider at all: the two
heaps are lab_mid_fruit's fruit bushes, drawn in their own many-coloured palette. A count of predator-red pixels in
that GIF reads 0 in every frame; over the whole box it reads 83-140 cells' worth, all east of column 427. **Count
the predator's colour in a crop before describing where it is**: a picture with none of it in shows nothing.

## Where it goes next

- **Train it to go in.** Nothing in training pays for the nest: credit is food digested, and the door is where the
  food walks past. The bed already counts kills in the nest (`predtrain`'s round line); paying a kill there more than
  one on the surface, or landing raiders inside the nest (`far=0` puts them at the door, not in it), is the next
  batch. The bed's best in-nest killers were the untrained minds (16-17 of their kills), so it is a selection
  problem, not a body one.
- **Keep elitism.** No line beat its founder, as with the spider; the next batch should carry the best minds
  through unchanged.
- **The ants' side** is the Teacher lane's: a colony that can lose its larder to something inside the nest is a
  defence case it has not had. `inrun spider_species=centipede` puts the raider on the colony bed.

## Reproduce

```
cargo run --release --example predtrain -- out=DIR species=centipede founder=blank sel=score seed=1 rounds=8 frames=100000 warm=60000
cargo run --release --example predtrain -- out=DIR species=centipede pool=DIR/r7/pool sel=off preds=4 rounds=1 frames=30000 warm=60000 seed=101
cargo run --release --example predtrain -- ... far=0 gif=0:0:20000:100 crop=96,40,80,52 scale=6   # at the door; r0/round.gif
PIXEL_PHYSICS_LAB_SCENARIOS=scratch/inrun/labscen cargo run --release --example spiderlab -- scenario=lab_mid_fruit \
    species=centipede pool=blank spiders=2 frames=300000 seed=2 delay=30000 [gif=100000:110000:50 crop=136,120,240,90 out=DIR]
PIXEL_PHYSICS_LAB_SCENARIOS=scratch/inrun/labscen cargo run --release --example spiderlab -- scenario=lab_mid_fruit \
    species=centipede pool=lab spiders=2 frames=136000 seed=3 delay=30000 gif=112000:136000:150 crop=136,110,240,110 out=DIR
python3 scratch/inrun/mkpredpool.py POOL_DIR assets/minds/centipede_pool.txt "header" ...   # a pool dir as the lab pool
```
