# Eat seed (chambers lane, 2026-10-29)

A hand-set brain seed so a hungry ant at home eats the food it picks up, instead of putting 95% of it straight back
(the chambers trace, `../chambers/`). Selection may keep or drop it; no rule in `act()` changes.

    python3 scratch/nest/eatseed/make.py POOLDIR OUT nest=60      # arm B, the one to keep
    bash scratch/nest/eatseed/go.sh SCRATCH; bash scratch/nest/eatseed/go2.sh SCRATCH h3n stdn   # the screen below

What it writes is in `make.py`'s header: Eat's switch on, Eat's job = 60 x (0.36 - energy) + 60 x (at_nest - 1),
its drifted job row, hidden-unit and memory links zeroed (23 fewer live links, ~1.5%).

## Screen: 4 worlds (68 73 61 64) x 4 rounds, sel=score, poolrounds.sh, same commit as the trace (6b99b892)

The H3 and std_pool arms are the trace's own runs: this build reproduces H3 w68 round 0 exactly (nest.csv identical).

| alive at end of round 3 | w68 | w73 | w61 | w64 | higher |
|---|---|---|---|---|---|
| H3 | 7 | 18 | 25 | 47 | |
| H3 + B (eat at home) | 58 | 56 | 38 | 64 | 4/4, p 0.125 |
| H3 + A (eat anywhere) | 0 | 0 | 66 | 74 | 2/4 |
| std_pool | 75 | 30 | 53 | 60 | |
| std_pool + B | 65 | 3 | 86 | 77 | 2/4 |
| std_pool + A | 84 | extinct r0 | 65 | 76 | 3/4 |

Starved underground, 4 rounds summed: H3 196/224/207/256 against H3+B 50/96/85/71 (lower 4/4). Passage share falls
(round 3: 44/65/44/51% -> 64/21/17/45%) and the pictures show why: the seed's nests are bigger, crumb-filled cones and
blobs rather than H3's thin branching tunnels. Arm A failed: a carrier a little under half a tank on a long trip eats its
load where it stands, loses the load that draws it home, and 3 of 8 lines drifted to the end walls and died.

Seed survival: every top-30 founder in every round still carries the seed (switch on, -60, constant intact), but at
kpm 0.003 four rounds barely mutate it, so this says the seed was not lost, not that selection chose it.

Readers: `table.py` (per round), `eatloop.py` (how a hungry home pick-up ends, from the mind log), `survive.py`.
