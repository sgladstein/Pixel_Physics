# Go out when hungry: the lesson, the founders, what's left (hungry lane, 2026-10-22)

## Why in-run colonies die
- Copy v2's job choice does not respond to its own energy. At every energy level it picks walk ~60% and feed ~37%.
- In the last 4k frames before a die-off, 94% of hungry decisions (bank < 5 grants) are ants **holding a soil pellet at the bottom of the nest chamber**, 4-9 rows below the datum.
  - Jaws full means no PickUp.
  - An upward step was offered on all but 38 of 15,052 of these decisions; a Drop on only 184.
- Feeding is billed like a cut, and they feed constantly. So tanks drain in lockstep and everyone starves within ~1k frames.

## The teacher (scratch, `PP_HUNGRY=lo:hi` in `mind.rs`; patch in `scratch/`)
The rule acts for a bank between 1.05 and 2 grants (bank/start_energy), with an empty crop:
- **Laden:** drop the pellet off the hole if offered. Otherwise carry it up, then away.
- **Empty:** take any PickUp. Below ground, step up. On the surface, step further from home, up to 40 columns.
- **Return** (any energy): beyond 40 columns with an empty crop and empty jaws, step nearer home.

Switches: `PP_HUNGRY=1.99:2 PP_HUNGRY_FLOOR=1.05 PP_HUNGRY_AWAY=1 PP_HUNGRY_SOFT=1 PP_HUNGRY_RETURN=1`. `PP_HUNGRY_ACT=q` sets compliance.

**Ceiling** (teacher acting, clone colonies of copy v2, 30 requested, fresh config, 200k frames, seeds 11-18):

| Teacher obeyed | Survivors of 24 (mean) |
|---|---|
| Never (copy v2 alone) | 1.5 |
| 25% of the time | 7.4 |
| 50% of the time | 5.1 |
| Always | 15.1 |

What did not work:
- Firing at 3+ grants: founders start at 3 and run out before the heaps fill.
- Heading for the food the teacher can see: ants chased provisions buried in mound walls and piled into towers.
- No search radius: ants ran to the bed's walls.

**Why the window starts at 1.05 grants.** The training arenas read energy against their own budget, not start_energy:
- fetch and trail sit at exactly 1 grant for the whole run;
- the soil arena's ant falls from 1 to 0.25 grants as it digs.

So "below 2 grants" alone would fire on every soil-arena decision.

## Founders (copier `scratch/copier_h.py`, hungry + return groups, v2 anchors)

| Genome | Live links | Clone survivors, seeds 11-18 | Soil full loads (v2 10.71) | Fetch worse side (floor 17.61) |
|---|---|---|---|---|
| copy v2 | 614 | 2 0 1 9 0 0 0 0 (1.5) | 10.71 | 19.83 |
| **hungry-h1** | 881 | 7 9 0 23 3 22 4 2 (8.8) | 10.49 (-0.22 +- 0.36) | 18.16 |
| **hungry-h6** | 1,135 | 18 6 2 0 23 22 14 0 (10.6) | 9.59 (-1.11 +- 0.37) | 18.33 |
| h8 (L1 on new links) | 907 | 0 0 1 0 0 1 0 11 (1.6) | 10.58 | 18.33 |

- h6's dig loss is behaviour, not the link bill. v2 with the same 521 extra slots set to 1e-5 scores 10.70.
- Copies obey the teacher 30-50% of the time. The ceiling needs nearly all of it.

## The real fix: PR #781 (owner approved 2026-10-22, with the merge desk)
Make the training arenas show energy in real grants. The lesson then becomes a plain threshold, and it no longer fights the soil arena's low readings.

**Price of the fix, measured:** copy v2 shown real grants in the arenas (scratch `PP_ENERGY_REAL=200`), with no retraining:
- soil full loads 10.12 vs 10.71 (-0.58 +- 0.32);
- fetch worse side 19.76 vs 19.83.

**As built (PR #781, Brain lane's patch, `CreatureDef::sensed_tank`):** every arena reads the species' tank. Re-priced against main on the same build, it matches the scratch switch world for world: soil 10.71 -> 10.12, fetch 19.83 -> 19.76. Side effect: fed arenas (1e9 J) now feel full (~1.0), so the forager preset's egg bar fires there (Teacher lane chose this, option A).

## Copy 9 (failed, not delivered)
A plain "below 2 tanks" lesson, with v2's kept skills re-taught on honest readings. Lesson overlap rose only from 0.24 to 0.30, and clone survivors on seeds 11-18 were 24 3 0 0 0 0 0 0 (3.4).

## In-colony ranking (Teacher lane, 64 seeds x 6 rounds)
**h1 beats h6:** it lost 20 colonies of 64 against h6's 28, and brought in 6,906 food per seed against 5,291. For the colony line, h1 is the founder. Any next copy should be aimed at in-colony survival, not clone survival.
