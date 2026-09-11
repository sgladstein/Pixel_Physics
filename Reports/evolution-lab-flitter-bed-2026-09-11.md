# A bed the flitter can work — and why it still cannot live on one

*Round 29, the bed lane, after B1 (`evolution-lab-flight-design-2026-09-11.md`).
Every figure is `examples/labforage`, `creature=flitter`, `RAYON_NUM_THREADS=4`,
one binary per comparison, 120,000 frames, seeds 1/2/3. Against `main` at
`52309af1`.*

## The answer

**The bed is a real lever and it is not the binding one.** Four flowering clumps
placed just outside the nest band multiply flowers drunk from by about nine, and
the colony still empties by frame 30,000 with the flowers still standing. What
the measurement found instead is a wiring fault the poorer bed was hiding: **the
float's ON condition is "a flower is somewhere in sight", so on a bed worth
flying over it never switches off** — 96% of airborne frames powered, 55% of
everything the colony eats spent on lift, 29 of 30 deaths in mid-air
(`open-bugs-handoff.md` §Z10). Gate it on *proximity* instead and aloft deaths
fall 59 → 8 across three seeds and births go 5 → 10 — and nobody persists
anyway. At the best arm measured the visit rate is **0.22 per 1,000 frames per
animal against the 1.6 the economy needs**: seven times short, with the bed as
close as the founding rules allow and the verb correctly gated.

## 1. Three beds

`played_bed` (the control), `played_bed_scrambler` (the thicket, scramblers at
195 and 315 — **inside** the nest band), and `played_bed_understory` (this
lane's file: `played_bed` plus four scramblers at 140/170/340/372, ten columns
outside the band at each end).

| bed | seed | founders of 52 | flowers at 6,100 | flowers at 120,000 | `flower_visits` | `born` | alive at 120,000 | plants |
|---|---|---|---|---|---|---|---|---|
| played | 1 | 36 | 81 | 13 | 76 | 3 | 0 | 659 |
| played | 2 | 24 | 21 | 22 | 8 | 0 | 0 | 1561 |
| played | 3 | 35 | 57 | 9 | 5 | 0 | 0 | 1066 |
| thicket | 1 | **15** | 182 | 20 | 61 | 1 | 0 | 995 |
| thicket | 2 | 25 | 43 | 31 | 108 | 1 | 0 | 1266 |
| thicket | 3 | 28 | 46 | 12 | 69 | 4 | 0 | 1659 |
| understory | 1 | 28 | 154 | 108 | 115 | 2 | 0 | 817 |
| understory | 2 | 28 | 67 | 17 | 32 | 0 | 0 | 912 |
| understory | 3 | 32 | 67 | 16 | 74 | 3 | 0 | 1246 |

**Medians: `flower_visits` 8 (played) → 69 (thicket) → 74 (understory)** — about
nine times. `born` totals 3 → 6 → 5. **Alive at 120,000: zero everywhere.**

**The founder gate decides between the two rich beds.** `played_bed` seats
36/24/35 flitters of the 52 asked for; the understory holds at 28/28/32, a 9%
loss at the median; the thicket seats **15** on seed 1, which is
`played_bed.ron`'s own recorded warning about scramblers inside columns 180–330
reproducing exactly. So the understory is the only rich bed inside the founding
rules, and it is the one carried forward.

**The ship condition is not met**, and not narrowly: it asked for flitters alive
at 120,000 on a majority of seeds, and the answer is zero on nine of nine
bed-seed pairs. `played_bed_understory.ron` therefore ships as a **named
scenario, not as `played_bed`'s new default.**

### What actually separates the beds, and it is not the flower count

Across the nine pairs the standing-flower count at 6,100 does not predict
`flower_visits`: the thicket's best seed (108 visits) holds **43** flowers and
`played_bed`'s worst (5 visits) holds **57**. What does line up is **how far the
nearest flowering clump stands from the nest**, against a 32-cell eye:

| bed | nearest scrambler to x=256 | median `flower_visits` |
|---|---|---|
| played | 138 columns | 8 |
| understory | 84 | 74 |
| thicket | 59 | 69 |

Bringing the nearest bloom inside about ninety columns is worth the whole
ninefold; bringing it closer than that buys nothing more and starts costing
founders. **That is the design's §6 condition answered, and it answers it with a
distance rather than a density** — "more food where nobody goes is not more
food" one level down.

## 2. What kills them, and the census that says so

Every visit on the understory bed happens in the first 6,000 frames of the
colony's life. Seed 1, sampled every 6,000:

| frame | alive | cumulative visits | standing flowers |
|---|---|---|---|
| 6,000 | 28 | 0 | 146 |
| 12,000 | 13 | 112 | 145 |
| 18,000 | 6 | 115 | 137 |
| 24,000 | 1 | 115 | 125 |
| 30,000 | **0** | 115 | 116 |
| 120,000 | 0 | 115 | 108 |

**The flowers are not what ran out.** The standing count never falls below 103
for the rest of the run. `intake` is 13,800 J against a founding grant of 28 ×
200 = 5,600 J — these animals ate nearly three times what they were given and
died anyway. `deaths_by` says where it went: **`STARVED:1 / STARVED_ALOFT:29`**,
with `fly_j` 9,636 J of a 17,482 J total burn.

That is §Z10, filed. The gate is *visibility* (`BloomNear > 0`, i.e. a flower
inside the 32-cell eye) where it should be *proximity*; on a bed this rich the
term is almost always positive and the float becomes a hover.

**The one-weight control, same binary, `wire=Bias:Fly:-9.6`** (the float fires
within ~9 cells, the design's own approach distance):

| | shipped gate | near gate |
|---|---|---|
| powered share of airborne frames | 96 / 83 / 77% | 81 / 58 / **23%** |
| `fly_j` as a share of burn | 55 / 39 / 30% | 36 / 22 / **5%** |
| `STARVED ALOFT` of total deaths | 29/30, 17/28, 13/35 | **3/36, 3/30, 2/32** |
| `flower_visits` | 115 / 32 / 74 | 258 / 71 / **31** |
| `born` | 2 / 0 / 3 | 8 / 2 / 0 |
| last frame with an animal alive | 24,000 / 12,000 / 18,000 | **48,000** / 18,000 / 18,000 |

Aloft deaths collapse and births double; median visits are flat (74 → 71) with
seed 3 going the other way, so it is not shipped on three seeds. **It is the
first thing to sweep.**

## 3. The threshold, which is the finding the brief asked for

The economy needs **~1.6 flower visits per 1,000 frames per animal**
(`flight-design` §3). Read over the window the animals are actually alive —
which is the correction that matters, since dividing by 120,000 frames divides
by a bed that was empty for three quarters of them:

| arm | visits | animals | frames alive | visits / 1,000 frames / animal |
|---|---|---|---|---|
| `played_bed`, shipped | 76 | 36 | ~24,000 | 0.088 |
| understory, shipped | 115 | 28 | 18,000 | 0.228 |
| understory, near gate | 258 | 28 | 42,000 | **0.219** |
| required | | | | **1.6** |

**Seven times short at the best arm measured.** Scaled linearly off the
understory's 154 standing flowers that is ~1,100 blooms standing on a 512-wide
bed — but the relationship is demonstrably *not* linear in the flower count (the
table in §1 shows it is barely a relationship at all), so that number is an
upper bound on a quantity the bed cannot supply rather than a target to build
for. **The honest statement is the one the distance table supports: the bed has
already given what a bed can give — about ninefold — and seven times more is not
in it.**

Where the remaining seven might be, in the order this measurement ranks them:
the float's gate (§Z10, measured above, worth ~2x on births); `start_energy`,
held at 200 through B1 so the arm stayed attributable and never swept; and the
per-visit yield, since `nectar_yield` 120 J against a 200 J grant means a
flitter must drink thirteen times in a life to bud.

## 4. B2, measured and not shipped

The design's §7 B2: move the take-off drive off `Bias` onto the sense —
`(Bias, Impulse, 0.5)`, `(BloomNear, Impulse, 2.0)` — so the animal leaves the
ground *toward* something. Measured on the understory bed, one binary, `wire=`:

| | shipped | sense-driven take-off |
|---|---|---|
| `moves` per real launch | 0.42 / 0.51 / 0.54 | **0.90 / 1.47 / 1.40** |
| `flower_visits` | 115 / 32 / 74 | 165 / 34 / **62** |
| `born` | 2 / 0 / 3 (5) | 7 / 0 / 2 (**9**) |
| alive at 120,000 | 0 / 0 / 0 | 0 / 0 / 0 |

**It does exactly what it says** — the animal walks two to three times further
between hops, which was the whole point — and it does not clear the bar it was
given: visits per animal are 4.71 / 1.21 / 1.82 against 3.83 / 1.14 / 2.11, a
median that falls. Recorded in `dead-ends.md` with its numbers and its re-test
condition rather than shipped. `(Energy, Impulse, …)` was **not** tried:
`hopper.ron` carries that as a recorded do-not-retry.

## What this contradicts

- **The brief's own ship condition**, which assumed a bed could be found that
  lets flitters persist. None can, inside the founding rules — and the reason is
  not the bed.
- **"The payoff tracks the flower supply"**, B1's own closing line and this
  lane's premise. It tracks the *distance to the nearest flowering clump*; the
  standing-flower count predicts nothing across nine bed-seed pairs.
- **B1's dead-end entry on the float's OFF condition**, which recorded the fault
  as fixed by wiring the gate to a sense. The gate is wired to a sense and the
  fault returns whenever the sense saturates — see §Z10.
