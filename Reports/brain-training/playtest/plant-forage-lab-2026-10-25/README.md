# The owner's plant-foraging lab playtest — 2026-10-25

**What was played.** Build `f367374f` (main, 2026-10-24), the lab's trained ant
(`ANT BRAIN: NEW`), a hand-built 1024x512 box, 668,388 frames at up to 1024x.
The owner planted a tree (X 970), a shrub (865), three herbs (790-826) and two
creepers (706, 727), let them grow for ~115k frames, then placed colonies at
X 422 (F116999, 52 ants), 522 (F179739, 49), 631 (F237225, 36), 602 (F286752,
37) and 579 (F377629, 32). Food was painted once, at 466,236 and 464,228
(F368174-368978, ~1,040 cells). The full action list is
`data/*.actions.csv`.

**The owner's note:** *"This playtest was mostly trying to get ants to forage
from plants, but they basically immediately abandon the nest and live on the
plants for a bit, which moves the strong part of the pheromone trail away from
the nest. They live on the plants for a bit and then die."*

**The answer, in one paragraph.** The trained ant does not eat because it is
hungry. Next to a plant it takes a bite in about 5% of 25-frame windows whether
it is starving or carrying twenty tanks, so a founder that lands on one tank
dies beside the plants (median life ~1,100 frames, the length of one tank),
and an ant that gets lucky early keeps eating, grows fat, stays on the plants
and dies of old age there. Neither kind carries anything home. Its home trail
is laid where it stands, so with the colony living on the plants, the plants
become "home" on the trail. The hand-built ant, replayed into the same box,
keeps its founders alive 4-13 times longer, carries food home, and ends the
run with 165 adults against the trained ant's 11.

## How it was measured

Every box-changing action is now in the chronicle (PRs #867, #869), so the box
can be rebuilt headlessly. `examples/replay.rs` gained `actions=FILE` (replays
the PLANTED / PLACED COLONY / PAINTED FOOD lines at their logged frames),
`newant=1` (trained brain) and `trace=N` (one row per ant every N frames:
position, height to surface, energy, crop, bites, both trails under it,
distance to the nearest plant). Paused speed changes and the light rain dial
are not in the action file; the replay used the box defaults.

**The replay is the owner's run, not a lookalike**: its census matched the
uploaded census exactly to frame 40,000 and stayed close after (colonies are
chaotic, so later frames drift). Painted food is rebuilt as a disc of the
logged cell count, since a stroke logs only its start and size.

```
cargo run --release --example replay -- frames=480000 seed=1 ants=0 \
  actions=data/chronicle-2026-10-25-034139-herb_ant-s1.actions.csv newant=1 \
  out=OUT probe=10000 snap=999999999 trace=25
python3 -I ants.py OUT/trace.csv          # one row per drop site
```

Three arms, same box, same actions, 480k frames: **trained** (`newant=1`),
**pile** (`newant=1` with `PIXEL_PHYSICS_NEST_DOOR_FOUNDERS=pile`, the founders
heaped at the door as `inrun` does), **old** (`newant=0`, the hand-built ant).

## 1. Founders starve on their first tank, even next to the plants

Per drop site (trained arm; "near plant" = within 3 cells of plant tissue):

| site | ants | median life | reached a plant | median bites | home / near plant (time) |
|---|---|---|---|---|---|
| 422 | 52 | 1,075 | 2 | 0 | 0.02 / 0.03 |
| 522 | 48 | 1,300 | 22 | 0 | 0.07 / 0.39 |
| 631 | 36 | 2,938 | 33 | 9.5 | 0.11 / 0.78 |
| 602 | 27 | 1,225 | 19 | 2 | 0.04 / 0.51 |
| 579 (+ all brood) | 367 | 8,600 | 235 | 16 | 0.04 / 0.36 |

One tank (200 energy) at ~0.2 per frame is ~1,000 frames. The 422 colony was
~280 columns from the nearest plant and never found it. The 631 colony landed
among the creepers and 33 of 36 reached plant tissue at once, yet half took
fewer than ten bites in their whole lives.

## 2. Eating does not follow hunger

A bite is worth ~11.7 energy (median over 11,422 bites), about 58 frames of
upkeep. Bite rate while near a plant, per 25-frame window, by the ant's energy:

| energy | trained: windows | bit | rate | old ant: rate |
|---|---|---|---|---|
| under 100 (starving) | 3,857 | 169 | 4.4% | 0.8% |
| 100-500 | 24,331 | 1,230 | 5.1% | 4.7% |
| 500-2,000 | 48,423 | 2,250 | 4.6% | 7.4% |
| over 2,000 (10+ tanks) | 55,029 | 2,657 | 4.8% | 3.4% |

The trained rate is flat: the brain's grazing does not read the gut. A
starving ant beside a herb bites no more often than one carrying twenty tanks,
and at ~5% per window it takes in ~0.6 energy per 25 frames against 5 burned,
so a hungry ant on a plant still starves. The fat ones are the other half of
the owner's picture. **The trained ants spend 55,029 windows above 2,000
energy; the old ant spends 3,078.** The old ant's energy sits at 100-2,000,
because it takes food home instead of gorging.

## 3. "They live on the plants for a bit and then die"

Long-lived trained ants (over 5k frames, mostly near plants): 49 died of old
age with median energy 4,479 (22 tanks in the body), 11 were alive at the end,
9 were killed and 62 starved. They spent 0.4-2% of their time within 8 columns
of their door, and were almost never at home with food in the crop. The old
ant is at home laden in 10.6% of its at-home samples. The plant-dwellers are
not lost: they are fed, stay where the food is, and die there without breeding
the colony a replacement.

## 4. The trail moves to the plants because the ants do

Median trail under ants, at the plants against at home:

| | at plants | at home |
|---|---|---|
| trail A (home) | 41.6k | 35.5k |
| trail B | 3,527 | 129 |

The home channel is laid by ants as they go, so it tracks where the colony
actually lives. With 80-95% of ant-time away from the door, the plants hold
the strongest "home" scent in the box, and anything that follows trail A to
get home walks to the plants. This is a consequence of §2-3, not a separate
bug. A fix to eating would move the trail back without touching trails.

## 5. Heaping the founders at the door does not save them

Lab founding spreads a colony over ~200 columns (`COLONY_ANT_SPACING` 4);
`inrun` heaps them at the door. The pile arm tested whether the spread is why
founders never come home:

| site | trained: life / home | pile: life / home |
|---|---|---|
| 422 | 1,075 / 0.02 | 1,025 / 0.23 |
| 522 | 1,300 / 0.07 | 1,100 / 0.25 |
| 631 | 2,938 / 0.11 | 1,400 / 0.30 |
| 602 | 1,225 / 0.04 | 1,175 / 0.20 |

Heaped founders spend 3-10x more time at home and die just as fast. The spread
changes where they die, not whether.

## 6. The hand-built ant in the same box

| site | trained median life | old median life | old: reached plant | old: home / under / near plant |
|---|---|---|---|---|
| 422 | 1,075 | 3,900 | 22 of 53 | 0.06 / 0.02 / 0.05 |
| 522 | 1,300 | 13,600 | 38 of 58 | 0.15 / 0.11 / 0.08 |
| 631 | 2,938 | 14,000 | 49 of 69 | 0.30 / 0.24 / 0.24 |
| 602 (+ brood) | 1,225 | 20,650 | 1,210 of 1,401 | 0.12 / 0.31 / 0.36 |

Adults alive, every 40k frames from F120000 (trained / old): 2/51, 1/0, 3/21,
14/38, 3/21, 7/85, 4/320, 129/123, 13/70, 11/165 at F480000. The trained
ant's one boom (129 at F400000) is the painted food, and it crashes within 40k
frames. The old ant
ranges further (median farthest 136-394 columns against 106-162), spends a
third of its time under ground, and its 602 colony grew to 1,401 ants on the
plants and the painted food. So the box is survivable and plants can feed a
colony; the trained ant is what fails in it.

## What this means for training

- **Inrun has never asked the trained ant to eat by hunger.** Training's food
  is a pile at the door that it carries; grazing (`PP_MIND_GRAZE`, default on
  since 2026-10-24) was taught only on the weaning bed, where the pile still
  fed the colony. Nothing in training punishes a fat ant that never goes
  home, or a hungry ant beside a plant that does not bite.
- **A case to add:** plants and no pile, founders landing on 1 tank, scored on
  colony survival and food carried home. This box is a ready example.
- The trained ant could not cross tree trunks on this build (fixed by #883,
  default on). The tree here was at X 970, far from every colony, so it is
  not part of this result.

Files: `data/` holds the chronicle, census and actions exactly as uploaded;
`ants.py` is the per-drop summary of a replay trace.
