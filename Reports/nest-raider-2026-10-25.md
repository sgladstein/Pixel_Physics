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

**It does not raid early.** *(Overturned for long runs the next day: see* Round two*. The paragraph stands for
the window it describes.)* The seed-3 boom filmed over the whole box, 100,000 to 126,000 frames
(`/mnt/project-files/predator/nest-raider/lab-boom-seed3-wholebox.gif`): the raiders live on the surface against
the east wall, at columns 428-507, 170-250 columns from the door at 256, and eat the foragers that range out that
far. None is in the shaft or a chamber in any frame. The trained pool makes a population by keeping its distance
from the nest and picking off the far edge of the foraging range, which is a predator the colony has to answer, but
not the one the owner asked for.

*Correction, 2026-10-25.* The first version of this paragraph said the raiders sat in two heaps 75 and 85 columns
either side of the door. It was read from a crop of the box (columns 136-376) that held no raider at all: the two
heaps are lab_mid_fruit's fruit bushes, drawn in their own many-coloured palette. A count of predator-red pixels in
that GIF reads 0 in every frame; over the whole box it reads 74-140 raider cells, all east of column 427. **Count
the predator's colour in a crop before describing where it is**: a picture with none of it in shows nothing.

## Round two, 2026-10-26: training it to go in, a nest smell, and the lab overturning both

Owner, asked for next steps: the recommended one was to train it to go in.

**Paying for nest kills** (`predtrain nestpay=K`): an ant killed under the ground pays the nearest raider head
(within 6 cells) K ant cells' worth on top of digestion. **The first cut paid on bites and was farmed**:
`LifeCounters::bites` counts every pickup into the crop, so a raider that puts a crop cell down underground and picks
it up again "bites in the nest" -- 42,000 bites a round, 6,900 of them in the nest, against about 350 ants killed.
Thrown out; a death is the one thing a raider cannot repeat. Batch 2 (shipped pool as founders, 4 arms x 2 seeds x
16 rounds of 100,000, warm 60,000), judged on 6 fresh beds, 4 raiders, 30,000 frames, ants killed in the nest:

| landed | shipped | no bonus | nestpay 3 | nestpay 10 |
|---|---|---|---|---|
| at the door (`far=0`) | 160 | 46, 87 | 149, 177 | 36, 137 |
| 30 columns out (`far=30`) | 6 | 3, 4 | 6, 8 | 8, 5 |

Nothing beats the founder, and from 30 columns out no mind finds the nest: its eye (24 cells) stops at soil, and
nothing else tells it where the nest is.

**A nest smell** (`CreatureDef::smells_nest`, the centipede; `PIXEL_PHYSICS_NEST_SMELL=on`): with no ant in its eye,
its home senses point at the nearest nest door, from anywhere, with no cap (through the home senses, as sight is, so
no genome moves). On the bed it does what it was built for. The shipped pool with it, landed 30 columns out: 102 ants
killed over 6 beds (30 without), 13 in the nest (6). Batch 3, all with the smell, landed 30 out unless marked:

| pool | 6 judging beds, nest kills | 12 fresh beds, nest kills | wins against shipped+smell, 12 beds |
|---|---|---|---|
| shipped + smell | 13 | 35 | -- |
| no bonus, r8 | 23 | 33 | 4 won, 4 lost |
| nestpay 10, line 1, r8 | 181 | **191** | **11 won, 1 lost** |
| nestpay 10, line 2, r15 | 141 | **81** | **12 won, 0 lost** |

Two pools picked on the first six beds held on twelve they were not picked on. On the bed, smell plus nest pay
works.

**In the lab it does the opposite.** `spiderlab`, lab_mid_fruit (the hand-built ant), 2 raiders landed 30 columns
out, 300,000 frames, the same 6 boxes, every kill's place logged (`PP_KILLLOG`). "Deep" is more than two cells under
its column's surface: the first two rows are the mound's flanks and the shaft's mouth, where most "in the nest"
kills on the bed happened.

| arm | ants killed | at the mouth | deep | deep, per box | raiders' peak | colonies eaten |
|---|---|---|---|---|---|---|
| **shipped, smell off** | **3,698** | 571 | **538** | 5, 76, 266, 183, 0, 8 | 2-204 | 2 |
| shipped, smell on | 1,630 | 199 | 78 | 5, 26, 7, 14, 0, 26 | 2-46 | 3 |
| nestpay 10 line 1, smell on | 1,505 | 97 | 64 | 3, 6, 32, 4, 18, 1 | 2-10 | 0 |
| nestpay 10 line 2, smell on | 2,071 | 312 | 75 | 0, 13, 31, 15, 0, 16 | 2-59 | 4 |

**Without the smell the shipped raider raids.** Its population grows out at the far edge of the box (59-204 at the
peak in the three boxes where it took hold), and that crowd comes into the chambers 50,000-150,000 frames after
landing: seed 3, 208,000-232,000 frames, raiders fill the chambers and the shaft, raider cells under the ground in the
picture rising 1 -> 164 (`/mnt/project-files/predator/nest-raider/lab-deep-raid-seed3.gif`). With the smell, the raiders
head for the door from the start: where they are few they are killed there (shipped pool, peaks 2-46), and where they
breed they crowd the mound and kill at the mouth without going down (`lab-smell-door-seed2.gif`, the nestpay-10
line-2 pool, 59 raiders at the peak: 0 raider cells under the ground in any of 141 frames from 40,000 to 82,000, the
colony gone by 90,000). The
bed could not show this: it lands 4 raiders for 30,000 frames on a 256-wide box, so it can only reward the few at the
door. **The smell ships off** (`options::nest_smell_on` records the harm), the trained pools are kept
(`/mnt/project-files/predator/nest-raider/pool-smell-*`) and not shipped, and the lab keeps the hunter-2 pool.

The earlier *it does not raid* was true of the window it was read from (100,000-126,000 frames, seed 3, on the
build before #887) and is wrong as a description of the raider. A boom at the far wall is how its raids start.

## Where it goes next

- **The ants' side** is the Teacher lane's: the lab raider now empties chambers once its numbers build, a defence
  case the colony has not had. `inrun spider_species=centipede` puts the raider on the colony bed.
- **Train predators where they will live.** A training bed that lands a few raiders for a short round rewards the
  few at the door; the raid is a crowd's. Selecting raider pools on lab boxes (`spiderlab`, a long run, deep kills
  from `PP_KILLLOG`) is the honest test, and costs about an hour of four cores for 24 boxes.
- **The trained ant's nest** (std_pool since c87fd03c digs open rooms) has not met the raider in a lab box; every lab
  number here is the hand-built ant's.

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
cargo run --release --example predtrain -- out=DIR species=centipede pool=POOL sel=score rounds=16 frames=100000 warm=60000 far=30 nestpay=10 seed=1
PP_KILLLOG=1 [PIXEL_PHYSICS_NEST_SMELL=on] PIXEL_PHYSICS_LAB_SCENARIOS=scratch/inrun/labscen cargo run --release --example spiderlab -- \
    scenario=lab_mid_fruit pool=POOL species=centipede spiders=2 frames=300000 seed=3 delay=30000 2> kills.txt   # nest_kills= in RESULT
```

The raw logs, the three raider pools and the lab grids behind every number above are in
[`nest-raider-2026-10-25/`](nest-raider-2026-10-25/README.md).
