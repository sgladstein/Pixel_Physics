# Lane note — the flitter, rounds 29–30

*Current as of 2026-09-12. Rounds 29's three builds (#332 the float, #334 the
bed, #339 the float's gate) and their numbers are in
[`evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md) and the
two flight reports; **the two findings below are the ones a later lane cannot
reconstruct**, and they are followed by round 30's, which overturns the
framing of all three.*

## The two round-29 findings that still stand

**A bias on the `Fly` row is a distance threshold, not a taste setting.**
`BloomNear` is `1 - dist/reach`, so the gate opens inside `4 x (12 - B)` cells
at full energy and the energy term slides that range shut as the tank empties.
Any gate on a `*Near` sense has this shape, and it saturates once the world is
dense.

**What separates a bed the flitter can work from one it cannot is a distance,
not a density** — the nearest flowering clump's distance from the nest
predicts visits (138/84/59 columns against medians 8/74/69) where the
standing-flower count predicts nothing. Both are derived in full in
`evolution-lab-flitter-bed-2026-09-11.md`.

## Round 30: the owner judged four cards, all negative

Verbatim, on #332/#334/#339 and then on main:

1. *"Both look very much like hopping and not flying"*
2. *"I cannot tell from these images. I see a creature move a little within a
   plant. That is it."*
3. *"No. It looks like it is hopping and mostly stuck in this video. It is
   stuck next to a plant, then does one long hop (clearly not fly) and then
   gets stuck next to another plant"*
4. (follow-camera GIF, 240 frames) *"This creature seems stuck in the plant or
   just decided not to move much in the time."*

The first three were still-strips; the fourth was the right instrument.

## **The bed is a cage, and that is what all four verdicts were looking at** (§Z15)

**`translated_if_free` requires every target cell to be empty, and `Plant` is
not empty — but `Plant` *does* count as support.** So an animal inside a
canopy can still launch and cannot step in any direction, in the air or on the
ground, however good its wings or its brain are. It chatters between airborne
and grounded on the spot, which is exactly what reads as hopping.

Measured on `played_bed_understory` seed 1, the followed flitter at frame
9,000: **zero empty neighbours on 88% of frames**, median **7 of its 8
neighbours `Plant`**. Colony-wide, the share of live flitters that cannot step
anywhere at a given instant:

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| main | 42% | 12% | 18% |
| round-30 build | 54% | 25% | 17% |

**It is worse on the new build because the new build breeds more animals into
the same canopy**, not because anything regressed. `labgif follow=` takes the
lowest live id, which is disproportionately a long-settled — i.e. caged —
animal, so **every card this round was aimed at the failure**.

**Filed as §Z15, and it took three tries to land on a free letter.** The
section was written as §Z12, renumbered to §Z14 when lane I's `#353` turned out
to hold §Z12 and §Z13, and renumbered again to §Z15 when lane O's `#357` took
§Z14 — plus §Z11 had already gone to the zoom-out lane the same way. **Four
collisions in one day, and not one of them was a careless pick**: the letter is
chosen by the author at filing time and `scripts/bugindex.py --check` can only
see `main`, so every concurrent filing is a guess that goes stale the moment
another lane pushes.

**The check that works is a sweep of the remote branches, run immediately
before pushing rather than when the section is written**: fetch all of
`refs/heads/*`, then `git show $b:Reports/open-bugs-handoff.md | grep -oE
'^### Z[0-9]+\.'` over `git branch -r`. It caught all three collisions and
nothing else did. The durable fix is allocating the letter at merge time, or a
`bugindex.py` mode that sweeps the branches itself — a command rather than a
discipline.

**This is not fixable inside the flight code and was deliberately not
attempted.** Letting a body move into a plant cell means `relocate_chain`
overwriting it, which is `dead-ends.md`'s already-paid disaster (the flitter
eating its own bed: 960 plant cells -> 566, 34 standing flowers -> 4). A
displacement or swap mechanism is a design question for the movement layer,
not a lane fix. **Escalated rather than cut blind.**

## What round 30 did change, and what it is worth

Five things, all in the flight path, all gated on a species pricing flight, so
`ascii` is byte-identical over 1,099 non-timing lines and the ant, long ant,
hopper and beetle are byte-identical on `played_bed`.

- **The hover.** `BrainOutput::Fly` is `squash(sum)`, strictly below 1, and the
  lift was `GRAVITY * (1 - fly)` — so **gravity could never be cancelled and
  every "flight" was an arc by construction**. This is the arithmetic reason
  three builds all read as hopping. `HOVER_GAIN` 2.5 closes it.
- **The price follows the lift.** The per-frame charge was binary: a
  quarter-lift glide cost exactly a full hover. Graded outcome, switch-shaped
  bill. **On its own this doubled the economy** — `flower_visits` 274 -> 506,
  `born` 12 -> 25 on seed 1 at 20,000 frames — and it is what makes any cheap
  traverse affordable at all.
- **The stall-out.** A flier hovering two cells short of a bloom it could see
  hung there **371 consecutive frames at a rock-steady `fly` of 0.648** and
  never arrived: the row that ends a bout is `(FoodAdjacent, Fly, -12.0)`, so
  **a verb whose OFF condition is "arrival" hangs for ever on journeys that do
  not arrive.** A blocked bout now gives up its lift. `blocked` is load-bearing
  — keyed on "went nowhere" alone it cut the wings of a freely hovering body
  and dropped it in the water §Z9 exists to keep it out of.
- **The cruise** (`CreatureDef::cruise_lift`, 0.7). Every previous build gated
  lift on a bloom already within ~9.6 cells, so **every journey longer than
  nine cells was unpowered** — all travel ballistic, all flight a final
  approach. The cruise powers a launch on spec and ramps to nothing, so it
  lands by construction. Not `dead-ends.md`'s rejected always-on float: that
  had no OFF condition.
- **Genome**: `(FoodAdjacent, Impulse, -2.0)` -> `-1.75`. At exactly -2.0 it
  cancelled `(Bias, Impulse, 2.0)` to zero.

**The cruise is not an economic win and should not be sold as one.** Seeds 1-3
at 20,000 frames, `flower_visits`: no cruise 510/23/131 (median 131), 0.70
264/96/142 (median **142**). It trades the rich seed's take for the poor
seeds'. What it buys unambiguously is `fly_share` **26-69% -> 85-86%** —
powered flight instead of arcs. Three seeds is not a sweep.

At 120,000 frames, seeds 1-3, `RAYON_NUM_THREADS=1`: visits 142/39/85 ->
676/23/131, `born` 1/0/0 -> 34/0/4, aloft-death share 8/5/21% -> ~11%.
**`alive` at 120,000 is still 0 on every seed, both arms** — the standing gap
is untouched.

## Environment notes that cost time here

- **`PIXEL_PHYSICS_FLIGHT29=0` reverts the whole round-30 flight model** and is verified to
  reproduce the `origin/main` binary's summary line on seeds 1-3. Use it for
  the A/B arm rather than an older binary: two binaries also means two
  harnesses.
- **`labgif` now has `track=1`** (per-frame position, `aloft`, `fly`, energy
  and the 8-neighbourhood census) and a `CAGE` line. A follow camera holds the
  animal dead centre, so the one thing a still cannot show is whether it is
  travelling. Read the track, not the picture.
- **`labgif follow=` picks the lowest live id**, usually a caged animal;
  `follow_air=N` picks one that is up and holds it.
- **`labgif` now has `wire=`** (a genome sweep is one binary) and defaults
  `rain=steady` over the scenario's own rate — pass `rain=off`.
- **A `carried <= 0.0` test can never fire**: air has a density too. Four
  sweep arms including the OFF arm came back byte-identical before it showed.
- Divide a visit rate by the window the animals were **alive**.
