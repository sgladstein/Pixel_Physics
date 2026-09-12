# Lane note — the flitter, round 29 (B1, the bed, B3)

*Closed 2026-09-11. Three builds in one day; every number below is
`examples/labforage`, `creature=flitter`, `RAYON_NUM_THREADS=4`, one binary per
comparison. The full accounts are the two reports; this is what a later lane
needs and cannot reconstruct.*

## Where it got to

The flitter **flies, steers, lands on the flower, and switches off when there is
nothing in range**. It still **cannot make a living**: alive at 120,000 frames
is **0 on every bed, seed and setting measured** — 24 runs at B3 alone.

## The three things that were wrong, in the order they were found

1. **No brain in the air** (B1, PR #332). `creature_tick` returned into
   `step_flight` before `sense`, so `(BloomBearing, Turn, -2.5)` steered nothing
   for a third to a half of the animal's life. Fixed by `BrainOutput::Fly`.
2. **§Z9, and it was most of the loss.** A body that came down on water hung
   there and starved: 63/57/87% of all deaths on `main`. One predicate closed it
   — the ballistic arm goes to 3/0/6%. **The design blamed blindness aloft; the
   measurement says the water was the bigger half.**
3. **§Z10, the float's gate** (B3, this branch). The ON condition was
   `BloomNear > 0` — *a flower somewhere inside the 32-cell eye* — which on a
   bed worth flying over is nearly always true, so the verb never switched off:
   96% of airborne frames powered, 55% of the colony's whole burn on lift, 29 of
   30 deaths in mid-air. `(Bias, Fly, ..)` **-4.0 -> -9.6** makes the gate a
   *distance* (9.6 cells at full energy, 1.6 at half, nothing at a quarter).

## The two findings a later lane should not re-derive

**A bias on the `Fly` row is a distance threshold, not a taste setting.**
`BloomNear` is `1 - dist/reach`, so `-B + 4*Energy + 8*BloomNear > 0` opens the
float inside `4 x (12 - B)` cells at full energy and the energy term slides that
range shut as the tank empties. Any future gate on a `*Near` sense has this
shape — and it is why the first wiring failed: a bias tuned as "how eager" is
silently "how far", and it saturates as soon as the world is dense.

**What separates a bed the flitter can work from one it cannot is a distance,
not a density.** Across nine bed-seed pairs the standing-flower count predicts
nothing (a 43-flower bed takes 108 visits, a 57-flower bed takes 5); the nearest
flowering clump's distance from the nest does — **138 / 84 / 59 columns against
median visits 8 / 74 / 69**. Inside about ninety columns the whole ninefold
arrives; closer buys nothing and starts costing the colony its footing
(scramblers inside columns 180-330 seat 15 founders of 52).

## The standing gap, and where it is NOT

The economy needs **~1.6 flower visits per 1,000 frames per animal**
(`evolution-lab-flight-design-2026-09-11.md` §3). Measured over the window the
animals are actually alive, the best settings give **0.08-0.45**. Four to twenty
times short, with the bed as close as the founding rules allow and the float
correctly gated.

**It is not in the wings.** Mid-air deaths are down to 6-10% of the total and
lift to 5-36% of burn; `deaths_by` is now overwhelmingly `STARVED` on the
ground. What is left is the animal's life on the ground:

- **resting** — nothing in this genome says "sit still when there is nothing in
  sight"; `(Bias, Move, 2.0)` walks it about regardless, at 0.25 J a step
  against 0.025 J a frame standing still;
- **the walk** — bug **R4**, `Turn` nearly inert for a walker on level footing,
  so a grounded flitter cannot be steered, only scattered;
- **the eye** — `sight_range: 32` never re-derived after the brain began casting
  aloft (`flight-design` §3 asks for it and B1 did not do it);
- **`start_energy: 200`** — held through all three builds so the arms stayed
  attributable, and never swept.

## Environment notes that cost time here

- **`labgif` has no `wire=`**, so a card at a non-shipped wiring needs a `.ron`
  edit and a rebuild. `labforage` does have it, and it replaces the authored
  weight, which is what made a 24-run sweep one binary.
- **`labgif follow=` picks the lowest-id live animal**, which is often one
  sitting still at a flower. Two cards were re-rendered before this was noticed;
  a fixed `center=` on the flowering columns shows more.
- **`labgif` defaults `rain=steady`** and overrides the scenario's own rate —
  pass `rain=off` or the card is not the bed the measurement ran on.
- **`no_colony=1` on a scenario is a post-arrival sweep**, so the bare-bed
  control still reports `died=N`; read `flower_visits=0` as the tell instead.
- A visit rate divided by the run length is divided by a bed that was empty for
  three quarters of it. **Divide by the window the animals were alive** — the
  population curve (`sample=6000`) is the denominator.
