# Teaching "dig at the foot" into copy v2: what it takes, and where the rest of the gap is

*2026-10-21, teacher lane. All local and offline: the copier and Rust scratch tools rebased onto `main`, run on the
96 evaluation worlds (8-row shaft, 12,000 frames). Standard errors are paired against the start and clustered by
mirrored seed pair. The tools reproduce s171 = 9.47 and copy v2 = 10.71 to the digit.*

## The answer

- **Copying "dig at the foot" into copy v2 needs a new sense.** Without one, every copy collapsed. With a
  body-level "I can cut the bottom of my hole" sense, the copy holds its level and adds a little (+0.3, within
  noise).
- **Most of the remaining gain is not in digging.** I let the hand-written teacher act on copy v2 for each
  lesson alone. Walking home empty is worth **+6.0** full loads a world, digging at the foot **+3.5**, and going
  down +1.2. All three together are worth +11.0 (copy v2 alone: 10.7). Copy v2 already spends half of its
  decisions empty on the surface.

## 1. Plain dig copies collapse

Four arms copied the dig lesson into copy v2: dig alone, dig + go-down, and two with depth links free and wall
cuts weighted ×10 or ×30. Every one scored **5.2 to 7.5 full loads, against v2's 10.7**.

The copies dig the foot on 95% of visits, but cuts at spots with only walls in reach triple (3.4 to 10.5 a world).
Each trip down ends at its first cut, so wall cuts eat the trips. This is the same pattern step 0.5 found on s161.

**Why:** the information is in the ant's senses (a linear probe separates the two at AUC 0.998), but not in a
form the brain can use. Depth is read as `squash(rows / 16)`, so row 7 and row 8 differ by only 0.03, and the dig
choice sees the cut options only through one saturated "best score". A "bottom of the hole" mark on each cut
option did not help either (5.3–5.7), because it reaches the dig choice through the same saturated channel.

## 2. A body-level sense the dig choice reads directly

Scratch only, on a borrowed slot (`c::CROWD`, which reads 0 in solo arenas), with its links zeroed in the start
genome. The sense is **"a cut at the bottom of the hole I'm in, below the ground as built, is in reach"**: the
cavity is flooded through open cells within 3 cells of the head, and nothing open lies lower than the target. It
is on at 82% of spots with the foot in reach and 1.8% of spots with only walls.

| Copy (from v2 with the sense, links at 0) | Full loads | vs start | Foot / wall cuts a world | P(dig) at the foot / walls only (held out) |
|---|---|---|---|---|
| start (sense on, no links) | 10.81 | | 13.4 / 3.4 | 0.14 / 0.0070 |
| walls ×10 | 6.56 | -4.25 ± 0.35 | 7.5 / 9.1 | 0.70 / 0.0146 |
| walls ×30 | 9.48 | -1.33 ± 0.37 | 11.9 / 5.3 | 0.66 / 0.0091 |
| **walls ×100** | **11.14** | **+0.32 ± 0.43** | 12.8 / 3.2 | 0.47 / 0.0057 |
| walls ×300 | 10.95 | +0.14 ± 0.34 | 13.4 / 3.4 | 0.37 / 0.0068 |

Controls: with the sense on and its links at 0, every world scores identically to the sense off.

So this is the first dig copy that doesn't collapse. But it still cuts the walls about as often as before, and
the acting teacher's +3.5 comes mostly from refusing those cuts.

**A correction to my earlier message:** I said copy v2 "walks back up empty on 6.5 visits a world". That count
treated every step out of reach of the foot as a visit ending, so it counted short steps around the bottom of
the hole as abandoned trips. The copy that digs on 93% of its visits got no more foot cuts (12.8 against 13.4),
so those visits were not wasted trips.

## 3. Where the gap is (the teacher acting on copy v2)

| Teacher acting | Full loads | vs v2 |
|---|---|---|
| none (v2) | 10.71 | |
| go down | 11.85 | +1.15 ± 0.41 |
| dig, floor-limited (only as sure as a saturated brain can be) | 14.23 | +3.52 ± 0.32 |
| dig | 15.69 | +4.98 ± 0.28 |
| walk home, floor-limited | 16.74 | +6.03 ± 0.42 |
| walk home | 16.96 | +6.25 ± 0.41 |
| all three | 21.66 | +10.95 ± 0.35 |

## What it means for the plan

- **Walk home is the largest piece left** (+6), and it is the lesson we agreed to leave to selection, because
  copying it leaks into foraging. That makes the copy-versus-selection run's answer on the empty surface trip the
  number to watch. It also argues for building the food-and-dig world sooner rather than later.
- **The dig sense is real but not yet worth a change to the brain.** It makes the copy safe, not better. If the
  brain lane ever adds it, the scratch code is ready.
- **The search teacher** was aimed at dig-or-not, which is now the smaller prize. If it is built, it should
  target the empty surface trip.

Scratch code (never for `main`): `/home/user/tp`, branch `scratch/teacher-probes` (rebased onto `main` 7ff76eda),
switches `PP_BOTTOM=<reach>` and `PP_BOTTOM_CTX=1`, and the copy log's 15th field.
