# A pellet that rolls down the mound before it settles (`PIXEL_PHYSICS_SPOIL_ROLL`)

*2026-10-21, by the teacher-approach session, for the owner's question "should a freshly placed pellet settle a cell
or two before it hardens?" -- answered "go with your suggestion". Built in `src/sim/creature.rs` (`spoil_roll_of`,
`spoil_rest`, `place_spoil`), **off by default**, per world through `World::spoil_roll`. Measured on the shipped
colony; not yet on the trained brain's arena.*

## What it does

When an ant puts a dug pellet down **on the mound** -- above the ground its nest was founded on -- the pellet takes up
to two 1:1 diagonal steps **down the flank away from the nearest door**, each onto a cell the drop itself would accept,
and is tamped where it stops. Nothing loose is ever created. Pellets put down inside the nest, at or below the founding
ground, and pellets on a flank that falls toward the door, do not move.

**Why not the obvious version.** Real loads that miss the crest roll down the outer slope (Dorymyrmex: 18-27% roll,
almost never back to the hole; Tofilski & Ratnieks 2005). Here placed spoil is `self_supporting` and stands at any
slope, so nothing ever rolls. Making placed pellets loose is the record's worst nest result: loose tilth near a gallery
runs along it and closes it (roofed void 197/179 -> 2/1), and a loose crust costs a third of the nest
(`dead-ends.md`, "putting a dug pellet back loose" and "loose spoil as an end state"). Deciding the roll at the moment
of placement gets the rolling without the loose soil.

## Tested

`a_pellet_rolls_down_the_outer_flank_only` (creature.rs tests): on a 1:1 staircase beside a founding shaft, off ->
stays; on -> two cells down, away from the door, counted; the staircase mirrored to fall toward the door -> stays; the
same staircase cut below the founding ground -> stays. **Both guards were put back wrong and the test went red**: a
roll toward the door fails the "on" and "toward the door" arms; a roll below ground fails the "buried" arm.

## Measured: the shipped colony (`deeptrace`, goal box, 4 seeds, 100,000 frames, roll off against on)

| Seed | Pellets rolled / put down | Nest score (well-built cells) | Mound cells | Overhanging mound cells | Tallest mound column | Door open |
|---|---|---|---|---|---|---|
| 1 | 24 / 1,642 | 129 -> 156 | 173 -> 194 | 54 -> 52 | 18 -> 15 | yes / yes |
| 2 | 33 / 1,929 | 139 -> 133 | 205 -> 196 | 63 -> 48 | 16 -> 15 | yes / yes |
| 3 | 32 / 1,903 | 151 -> 135 | 209 -> 199 | 49 -> 53 | 16 -> 13 | yes / yes |
| 4 | 39 / 2,251 | 117 -> 143 | 185 -> 196 | 63 -> 50 | 12 -> 13 | yes / yes |

Off arms: 0 rolled on all four (the counter's negative control). Nest score is `measurements/blueprints/nestscore.py`
(teacher design §6.1); door open, mound and overhang by `rollcheck.py` beside it (checked on a hand-sealed door, which
reads shut). Full rows: `rollcheck.txt`. Picture: `off-on-s2-s4.png` (roll off left, on right, seeds 2 and 4).

**Reading.** It fires on about 2% of put-downs -- the shipped colony's mound is a lattice (a quarter of its cells
overhang open air), and a 1:1 roll onto a cell with ground under two of its three cells is rarely there. Nothing
measured got worse: the door stayed open in all eight runs; overhanging cells fell on 3 of 4 seeds and the tallest
column on 3 of 4; the nest score moved both ways (mean +8, within four seeds' spread). **Safe, and on this colony
barely visible** -- the pictures are hard to tell apart.

## Where it matters, and the default

The rule is aimed at the teacher's ring mound (`Reports/brain-training/teacher-training/teacher-design-2026-10-21.md`
§5): a planner that puts pellets on the outer flank, where a roll is possible, should see it fire far more often than
the shipped ants' scatter does. That is the next measurement, on the brain's soil arena.

**Default.** Measured neutral, so by the house rule it should ship on in the games. It stays **off** for now for one
reason: switching the process default on would also change the brain lane's training arenas mid-run, which build
worlds with `spoil_roll` unset. The brain lane has been asked to pin `spoil_roll: Some(0)` in its arenas (or to take
the change); once it has, the default goes on.
