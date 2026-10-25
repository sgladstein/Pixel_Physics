# Opening the old ant to evolution: what its rules do, and what is still hard-coded (2026-10-25)

**Asked (owner, 2026-10-25 02:13Z):** explore running the hand-built ("old") ant through in-colony training, or
transferring its weights into the trained ("new") brain; go through all the old ant's rules, get rid of the
hard-coded behaviours and open everything else to evolution and selection.

**Short answer.**

1. **There are almost no old-ant weights to transfer.** The hand-built brain is about 50 live weights (29 direct wires
   in `assets/species/ant.ron`, ~20 hidden-unit links, 4 of its 8 units doing work). Its behaviour lives in about 110
   hand-written rules (`how-the-ant-works.md` §12) plus an always-on core in `creature.rs`. Copying that behaviour
   into the mind by imitation has been tried at least six times and failed (`foraging-loop`, `crater-refusal-copy`,
   `dig-lesson`, `far-home-sense` trail rule 2).
2. **Running the old ant through colony selection would tune only those ~50 weights and its ten body genes.** Its
   rules would stay hand-written, which is the opposite of the ask, and it would start a second ant line beside the
   Teacher lane's. The old ant already went through selection once in the goal box: `scene::LAB_ANT_TRAITS` is the
   body it evolved. Not recommended.
3. **The old ant's one real advantage, surviving fruit 200 columns out (114-132 alive against the trained pool's
   9-54), is not its rules. It is the trained brain's running cost** (below). The trained ant with its link bill cut
   to a quarter keeps 114 alive there. With no bill it keeps 150, more than the old ant at 100, 150 and 200 columns.
   Owner, 02:40Z: test a lower link price and training at the full price as two arms (PR #881's
   `PIXEL_PHYSICS_LINK_PRICE`, the Teacher lane's batch).
4. **The trained ant is already "the old ant with its policy opened"** (design rev3's *Physics vs policy*). What is
   still hard-coded on it is a short list, §4 below, each item with a proposed way to open it.

## 1. Which old-ant rules carry it on far food (21 arms, 8 seeds)

`examples/labsurvive.rs founders=old`, one rule switched off per arm by its own environment switch
(`scratch/oldant/arms.txt`, `ablate.sh`, `summ.py`). Lab box: 52 ants landing at column 256 on one tank, two fruit
heaps topped up every 500 frames (`scratch/oldant/scen/lab_far{100,150,200}_fruit.ron`), alive 15,000 frames after
landing. main 53f8c550.

| Arm (switch off) | Fruit 100 | Fruit 150 | Fruit 200 | Worse than base, at 200 |
|---|---|---|---|---|
| base (the old ant as shipped) | 178 | 156 | 129 | — |
| `CHOOSER=off` (brain.rs walk, no chooser) | 2 | 2 | 0 | 8/8 |
| `CHOOSER=on` (no trail terms) | 52 | 38 | 27 | 8/8 |
| `CHOOSER=trail` (trail hold, no walk outward along it, no scouting) | 73 | 60 | 52 | 8/8 |
| `FORAGE_DRIVE=off` | 99 | 86 | 81 | 8/8 |
| `TROPHALLAXIS=off` | 138 | 93 | 74 | 8/8 |
| `SCOUT=0` (walk outward along the trail kept) | 168 | 146 | 124 | 6/8 |
| `NEST_STORE`, `CARRY_HOME`, `CARRY_PATIENCE`, `PACKED_LUNCH`, `LEAN_FORAGE`, `STOREROOM`, `NEEDS_FIRST`, `HUNGRY_OUT`, `FORAGE_THROTTLE`, `FOOD_TRAIL` parts | — | — | 116-141 | 1-6/8 |
| **trained pool** (`std_pool`, laygene2 c8) | 122 | 90 | 28 | 8/8 |

Read: the chooser walk carries everything, and inside it **the walk outward along the food trail** (`trailaway`: an
empty ant on trail B scores headings by presence times the cosine away from home) carries most of it, 52 → 129 at
200 columns. Scouting off a route adds little (124 → 129). Without any trail term the old ant does as badly as the
trained ant at 200 (27 against 28). Positive control: base reproduces the Teacher lane's 114/132 at fruit 200.

## 2. Why the trained ant dies on far food: its brain bill

`labsurvive probe=` prints the food trail and how far out the colony is. At fruit 200, the trained colony *reaches*
the food (median column from the door 187 at 1,500 frames) and lays trail B, but only 9-13 of 52 ants are still
alive by then. The old ant at 1,500 frames still has all 52 alive, with median column 121-148.

With no food at all (`scen/lab_nofood.ron`), one tank lasts the hand-built ant about 3,700 frames (first death 3,500,
last 4,500) and the trained ant about 1,000 (first 750, last 1,750). The difference is the bill:
`mind::tick` charges `synapse_fraction × start_energy` per live link per tick, the same price `creature_tick`
charges the hand-built brain. `std_pool` carries about 1,922 live links (`scratch/oldant/pool.py`): scorer 919, job
head and its hidden units 364, memory 135, emits 15, **dig head 482**, lay gate 6. That comes to 0.85 J a tick
against an idle body's 0.1. The hand-built ant's ~50 links cost about 0.02 J.

**Dose response** (`labsurvive tax=K`, now `PIXEL_PHYSICS_LINK_PRICE`), alive, 8 paired seeds:

| Link price | Fruit 100 | Fruit 150 | Fruit 200 |
|---|---|---|---|
| 1 (as authored) | 122 | 90 | 28 |
| 0.75 | 146 | 124 | 50 |
| 0.5 | 182 | 132 | 89 |
| 0.25 | 224 | 178 | 114 |
| 0 | 252 | 198 | 150 |
| old ant at 1 / at 0 | 178 / 188 | 156 / 158 | 129 / 132 |

**On the training bed** (`scratch/oldant/bedcheck.py`: colsel's common garden, inrun sel=off, 200k frames, seeds
101-104), price 0.25 against 1: starved 1-4 against 18-38 (lower in 4 of 4), nest 631-768 against 535-721 (bigger in 3
of 4), crater 1-5 against 5-11, cuts the same. Price 0 starves as little, but its nests are smaller and more variable
(358-686). Four seeds is a hint, not a verdict.

**Why selection never shed the bill:** the bed puts food 25 columns out and gives every founder 3 tanks, so a
1,900-link brain eats a little more and loses nothing. The lab gives one tank and food 100 or more columns away,
where the bill decides who reaches the food. This is the same train-versus-play gap as food distance (colony-selection
log, 2026-10-24), on a different axis. The price was authored against the hand-built brain (`ant.ron`'s
`synapse_fraction` note: "a fully-dense brain (~168 connections)"). The mind's links are a different, much more
numerous object, so this is the *Fixing a bug exposes a constant* rule: a mechanism at an inherited constant.

**Decision (owner, 02:40Z): both arms.** The Teacher lane runs 32 colonies × 24 rounds from `std_pool`, scalegate
setup, the lab case with `lab_far200_fruit` added, at prices 1 (the control, and the "train at full price" arm), 0.25
and 0. Default stays 1.0 until that batch decides.

## 3. Every old-ant rule, sorted

Design rev3's test sorts each rule: *could a real ant have chosen otherwise?* No means physics, and it stays. Yes
means policy, and for the trained ant it must be the brain's. "Mind" says what the trained ant does instead.
Section numbers are `how-the-ant-works.md`'s.

**Physics: stays for every ant.** The support check and falls, kin footing, water footing, stack depth 4, push past
crumbs and brood, tissue parting (§2). Which cells the jaws cut (`jaw_can_cut`, `dig_force`). Crop capacity, one food
in the crop, continuous digestion, diet quality, starvation, old age, the energy costs of steps, digs, carried weight
and scent (§9). Scent spreading and fading (§7). Brood stages (§9). Burrow lining and the loose doorway (§5 step 6:
an ant tamps what it works) are borderline, and are kept as physics here.

**Policy the trained ant already decides (the rule is skipped for a mind).** All of §5's act ladder: stranger alarm,
attack, share, the feed/drop roll and its store, pick-up and door rules, every spoil hold, lift, ring, crest and side
rule, the walked excavation cycle, lean drop, soil way, face trip, the dig roll's target and its down, face and widen
turns, the six dig vetoes (heap cue, roof, shaft rooms, company, mound dig, face). All of §6: step-or-not, the
45-degree cone, tumble, the homeward re-roll, and the whole chooser (patience, home pull, trail hold, trailaway,
scouting, give-up, point of no return, door read, forage drive and throttle, carry home, packed lunch, hungry
home/out, nest rest, depth slow, the nest store's pulls, nurse seek, crop nurse/down, nurse stay, lay home and lay
in). The forage anchor and its re-anchoring (§8): the mind has its own home vector.

**Policy the trained ant cannot yet decide, because a move or a sense is missing:**

| Old-ant rule | What the mind lacks | Owner |
|---|---|---|
| Trunk crossing (`CROSS_TRUNK`, `organism::Crossing`) | the move itself: a mind has no option round a woody stem | Ants feeding on plants thread, approved 02:44Z |
| Patience: give up a carry or excursion that is not getting nearer | a body memory of progress (`home_best`, `scout_best`, `scout_e0`). Memory units can integrate `d_home`, but nothing hands it over | open |
| Hungry-out and the nest store's pulls (along `NestWay`) | a sense of the way through the nest: which neighbour is a step out or in | open |
| Door read, forage throttle, `met` drive | contact and colony senses: a returning forager met, food scent summed at the door | open |
| Nurse seek | larva scent (the mind sees a larva only beside it) | open |

**Trailaway**, the rule §1 found carries the old ant, needs no new sense for the trained ant at a lower price
(150 alive at price 0 without it). The far-home-sense report's "trail rule 2" blocker (on the trail, compared across
options) stays open for when a trail-following arm is wanted.

**Genes.** 28 `CREATURE_TRAITS`. Slots 0-9 (body) mutate at width 0.15. Slots 10-27 ship at width 0: scent and
tolerance, the six walk gains, the four trail-plane weights, the two door genes, return memory and appetite. For a
mind ant, 17 of the 27 older genes are inert (`body-genes-2026-10-22.md`): the walk, door and return-memory genes are
read only by the chooser and `act`, which a mind skips. Opening their widths would open nothing on the trained ant.

## 4. Hard-coded rules still acting on the trained ant

These run on the one ant line today, in training, the lab or both. Each has a proposed way to open it. Training
results change with any of them, so each goes through the Teacher lane before it lands (project rule).

| # | Rule | Where it acts | Proposed opening |
|---|---|---|---|
| H1 | **A nest-worker caste by id.** `Storeroom::SHIPPED`'s `caste=4` marks one ant in four, founders and born alike, nest-bound for life (`place_creature`), mind ants included. It decides which adults feed larvae by touch (`NurseMode::Workers`) and, through `worker_home`, where a put-down counts as delivered | bed and lab | Contradicts "no castes" (owner, 2026-10-21). For minds: `NURSE=any` (any touching adult) as the smallest change, or H2 |
| H2 | **Larvae feed themselves from a touching adult** (`brood::nurse`, `crop_feed` touch), beside the mind's own `Feed` verb | bed and lab | Design rev3 already lists it as policy ("becomes the adult's choice to feed that larva"). `challenges.rs`'s `clone_minds` notes it is deferred. An arm with touch-feeding off for minds needs `Feed`→larva to be learned first |
| H3 | **Where and when an egg goes**: only at the nest (`BUD_SITE`), the egg walk of reach 4 to the brood pile, a 24-frame retry, and graded breeding suppression (bar × 1.25 near a breeder) | bed and lab | The lay gate (`PP_LAYGENE`) opened *whether*; *where* is the body's. Laying stays body biology until workers carry eggs (design rev3). Suppression is a queen-like rule that could become a gate input |
| H4 | **Pellet put-down rules**: 2-of-3 footing and 3 rows of headroom (physics-ish), **never in the door column** (`DOOR_COLUMN`, labelled a stopgap, "not something an ant senses"), and the spoil roll (2 cells down the flank away from the nest) | footing and door column everywhere; **the roll is on in the lab and pinned off in every training arena** (`challenges.rs` `spoil_roll = Some(0)`) | Drop the door column for minds (a Drop option there is the mind's choice; the crater cases already score it). Run training with the lab's roll, or the lab without it: one physics |
| H5 | **The dig head's "when" gate**: only with empty jaws, an empty crop, below the datum, within 40 columns of the door (`dig_prepare`) | bed and lab | The head already reads door distance and depth. Ungated measured worse (food 318 → 248-275, dig thread 10-22) when the head was a fixed side file. It is evolvable now (`PP_DIGHEAD_EVOLVE`), so an ungated arm may come out differently |
| H6 | **The world cuts the founding nest**: a 6-row shaft, a chamber and a side room (`NEST_SHAFT`, `Storeroom` `side`) | bed and lab | A free starter nest. Leave it until colonies found nests from bare ground; it is world setup, not a per-tick rule |
| H7 | **Feed only downhill, share amount fixed, pick-up needs a bite worth taking** (`options` body forbids) | everywhere | Keep (design rev3's judgement calls) |
| H8 | **Training scaffolds**: `lay_cap` (harness only, so the lab has none: boom-bust), `PP_BODYRULES` and wean (off in the standard line) | bed only | Known (laygene2: no learned brake found). Listed for completeness |
| H9 | **The link price** (`synapse_fraction`), authored for the hand-built brain | everywhere | The owner's two-arm test (§2) |

**Recommended order:** H9 is running. H1 next: it is cheap, contradicts a standing owner rule, and the smallest
form (`NURSE=any` for minds) is one switch. Then H4's door column and the train/lab spoil-roll mismatch. H2 and H5
need an arm each.

## Tools and files

- `examples/labsurvive.rs`: `probe=N` (PROBE lines) and `tax=K` (this binary's per-link bill). PR #881.
- `creature::link_price()` / `PIXEL_PHYSICS_LINK_PRICE`, unset 1.0, bit-identical. PR #881.
- `scratch/oldant/`: `arms.txt`, `ablate.sh`/`one.sh` (one rule off per arm, `FOUNDERS=std` for the pool), `summ.py`,
  `pool.py` (a pool file's members and live links per block), `bedcheck.py` (the garden at several prices), and
  scenarios `scen/lab_far{100,150,200}_fruit.ron` and `scen/lab_nofood.ron`. `scratch/inrun/labscen/lab_far200_fruit.ron`
  is for colsel's `+lab` case.
- Each lab run takes about 4 s at 4 cores (21 arms × 8 seeds in 2.5 minutes). The bed garden takes about 60 s per run.
