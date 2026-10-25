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
  sight 24 (spider 64). A bitten-off segment is a shorter centipede, a bitten head a dead one. Held off the lab's
  animal chip until it has a trained pool.
- **It shoves through stored food it cannot eat** (`CreatureDef::digs_inedible_food`, the centipede only): food its
  gut will not take is loose ground to a mind's Dig. Walled in with the cut in its jaws, the mind's pack move puts
  the cell back where its tail was (`options::pack_on`), the whole `Cell` riding in the pellet, so a crumb keeps its
  worth and the store is moved, never lost. Its verbs are the spider's and Dig and Put Down
  (`train::predator::RAIDER_VERBS`). Owner's question, asked 2026-10-25 (see *Why it shoves*).
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

*(in progress)*

## Where it goes next

*(to be written)*

## Reproduce

```
cargo run --release --example predtrain -- out=DIR species=centipede founder=blank sel=score seed=1 rounds=8 frames=100000 warm=60000
cargo run --release --example predtrain -- out=DIR species=centipede pool=DIR/r7/pool sel=off preds=4 rounds=1 frames=30000 warm=60000 seed=101
cargo run --release --example predtrain -- ... far=0 gif=0:0:20000:100 crop=96,40,80,52 scale=6   # at the door; r0/round.gif
PIXEL_PHYSICS_LAB_SCENARIOS=scratch/inrun/labscen cargo run --release --example spiderlab -- scenario=lab_mid_fruit \
    species=centipede pool=blank spiders=2 frames=300000 seed=2 delay=30000 [gif=100000:110000:50 crop=136,120,240,90 out=DIR]
```
