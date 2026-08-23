# Phase 3: caves carved by dissolution instead of a Voronoi lattice

*The record of `Reports/world-scale-handoff.md`'s Phase 3. Written to be
picked up cold. Branch: `claude/world-scale-phase-2-review-vmn56t`.
**Status: built and measured; the owner's verdict is outstanding** (card
`20260823T095830007Z-122452`, blind).*

## What Phase 3 was for, in the owner's words

The handoff frames Phase 3 as "cave shape" with two candidates. It does not
quote the specification, which already existed. Card
`20260821T104146851Z-e2ef3e`, 2026-08-21:

> *"It hard to tell the scale of the caves zoomed in and without other
> context. But the large looks better and small looks too small. I think you
> could go bigger or more even better **longer** or have **chains of caves**
> for the bigger. The stalagmite and stalactites look way better. The
> **voroni patter is too much**. I like a little of it, but there is too
> much."*

Three of those map onto mechanisms. The fourth — the speleothems — is the one
thing he liked, and this phase deliberately does not touch it.

**None of them maps onto the two census numbers §8 of the Phase 2 report
nominates as Phase 3's target** (`walk_regions` p90, chamber-to-passage
contrast). Those are proxies. They moved, and that is reported below, but they
were not what the work aimed at.

## 1. Why the old carve could not be tuned into what he asked for

`carve_cave_void` thresholded a Worley field: void where `f2 - f1 <
CAVE_THRESHOLD`, in a frame sheared onto the bedding and squashed by
`CAVE_SQUASH`. `F2 - F1` is **zero along Worley cell boundaries**, so the
carve *is* the boundary network — and Worley boundaries are perpendicular
bisectors. Straight segments meeting at 120-degree junctions, by construction,
at every setting of every constant.

That is *"the voroni patter is too much"*, and it is not a tuning complaint:
no setting removes a straight edge from a Voronoi diagram. Rounds 3, 5 and 6
each retuned the trio and each produced the same web at a different scale;
round 6's A1 retune was measured and reverted. `Reports/dead-ends.md` carries
the full settings history, because it constrains any successor.

Two further properties of that field mattered:

- **`CaveEnv::cell` scaled the lattice with the envelope**, so every system
  held the same ~31 lattice cells. A big cave was a literal zoom of a small
  one. That was deliberate (round 6's A2 measured the alternative and
  rejected it), and it is why "bigger" never produced "different".
- **Every cave in the game was exactly 2.5:1.** `CaveEnv::draw` took width
  and height from one `u`, and 220/88, 580/232 and 800/320 are all 2.5, so
  the ratio cancels for every value of it. *"Longer"* was not reachable by a
  size draw that only made bigger copies of one shape.

## 2. A guard that has never fired, and what will wake it

Phase 3 opened with a hypothesis: that `MAX_CEILING_SPAN = 36` is what
fragments the caves. It drops a 3-row stone tooth into any roof run past 36
columns; the player is 7x14 with crouch unimplemented, so a tooth blocks him
in any passage under 17 rows tall while leaving the void *connected* — one
system to the census, several caves to the player.

**Falsified, and the falsification is the useful part.** With the guard off
entirely (span 100000), canyon 6 seeds, paired against the shipped 36:
`walk_regions` 1/29/92 against 1/29/90, `largest walkable` 30/73/92 both
sides, every other column identical. A delta of *nothing* means suspect the
condition, and a counter says why: **zero teeth, ever.**

A roof run needs void with stone **directly above it** for 36 consecutive
columns — a *flat* ceiling. Neither a Worley boundary web nor a rasterised
ellipse produces one: on a curved roof each row contributes only the few
columns where the curve sits at that exact height.

`cave_probe`'s "widest ceiling span" column could not have caught this,
because it is not that quantity — it measures the widest void run in *any*
row, so it reads 143 against a bound of 36 with no violation anywhere. Its doc
claimed it was "the ceiling-span bound's own quantity". Two different
quantities wearing one name.

**This is a live constraint on Phase 3 rather than a curiosity.** Dissolution
carves *along* bedding, and a passage lying along a bedding plane is exactly a
long horizontal run at constant height with stone above it. A guard dormant
against Worley blobs begins firing the moment the carve produces flat roofs,
and would saw a trunk into segments — the one thing a chain cannot survive.
That is why the trunk's radius carries an `fbm` whose wavelength is
deliberately well under 36. `ceiling teeth N` now rides in `vaults detail`;
read it beside any cave-shape change.

## 3. What a cave is now

`carve_cave_void`'s contract is unchanged — it returns a `Vec<bool>` over the
envelope grid — and **everything downstream consumes only that**: the seal
assertion, gravel floors, breakdown mounds, speleothems, the waterline, the
report. `Purpose::Cave` was referenced in exactly one place in the tree. So
this replaced a threshold loop, not the pass, which is why the speleothems
survived untouched.

**A trunk.** One conduit stamped as a swept elliptical section whose centre
follows a bed and whose radius varies continuously.

- The bed is picked by sampling `HardnessField` across the envelope's middle
  and taking the **softest** band — dissolution follows the rock that gives.
  Sampled rather than solved, so it stays a pure function of position.
- The centreline is `strata_offset` (the same locus the shade pass bands, the
  benches snap to and the lenses lie in — fifth consumer of that one function)
  plus a slow meander.
- The radius is a narrow passage that swells into rooms where a slow `fbm`
  crosses a threshold, times a faster `fbm` for roughness. Rooms are beads on
  a conduit: **that is the chain**.

**Feeders.** 4 to 9 branches leave the trunk and run down the dip, tapering to
nothing. They are most of the void, and the arithmetic is why: a conduit is a
one-dimensional structure through a two-dimensional envelope, so it fills far
less of it than a space-filling field. Trunk-only measured **0.121%** of the
deep massif against the honeycomb's 0.563%.

**Aspect draws separately** (`Purpose::CaveVariety`, reserved and until now
unused for exactly this), clamped to 1.5:1 through 8:1.

### Four things that were wrong first

- **Feeders seeded off the trunk were silently deleted.**
  `keep_seed_component` keeps only the component containing the envelope
  centre, so a branch anchored on the *bed* while the trunk had meandered off
  it is a disconnected satellite and goes back to being stone. The signature
  is distinctive and worth knowing: adding feeders moved void 0.121% ->
  0.138% and *raised* `walk_regions` max from 3 to 10 — fragments being culled
  rather than branches being joined.
- **Feeders came out as lightning bolts.** The meander was several
  wavelengths over a hundred-cell branch, swinging tens of cells between steps
  one cell apart. Under one wavelength per branch now. A feeder meanders; it
  does not corrugate.
- **Stamping at sampled centres leaves gaps.** Any path that turns puts
  consecutive centres further apart than a section is wide, and it comes out
  as a string of beads. `stamp_run` sweeps the segment at half-cell steps, so
  continuity is by construction rather than by the sampling happening to be
  fine enough.
- **Half of the edge fade went missing — and the failure that seemed to prove
  it did not.** The old field faded its threshold on *both* axes; this carve
  kept only the horizontal one, so a room could swell until it touched the
  envelope's top, which sits `VAULT_RIND` (2 cells) below the depth band's
  ceiling. `VERT_MARGIN` restores the missing axis.

  **The diagnosis that led there was wrong, and that is the part worth
  keeping.** `a_forced_vault_world_is_sealed_and_arrives_at_rest` came back
  red — *"wetland seed 3: 8 cells left their position"*, at y 132-133, in a
  test built at `vault_min_depth: 40` where caves sit within 42 rows of the
  surface. That is an extremely good fit for the missing fade, and it was
  wrong: run on a worktree build of the previous commit, **the same test fails
  with the same 8 cells at the same coordinates**, differing only in the order
  a `HashSet` iterated them. The red is pre-existing and this carve does not
  touch it — its shape matches open bug 0h, loose cells moving near the
  *surface* in a forced-vault world with no cave anywhere near them.

  The fade is kept, on the field's own argument rather than on that evidence,
  and its doc says so. Two lessons, both already in `CLAUDE.md` and both paid
  for again here: a plausible cause that fits the symptom is not a measured
  one, and **the control has to be run even when the story is convincing.**

## 4. Measured, canyon, 3 seeds, paired on one build

| | Worley honeycomb | dissolution |
|---|---|---|
| **separate walkable regions** med/p90/max | 1 / 7 / **29** | 1 / 3 / **3** |
| **largest walkable share** % | 30 / 73 / 92 | **90** / 98 / 99 |
| reachable by player % | 50 / 83 / 93 | **92** / 98 / 99 |
| contrast p95/med | 3.68x | 2.64x med, **21.3x p90** |
| **void % of the deep massif** | 0.563 | **0.187** |
| ceiling teeth | 0 | 0 |
| speleothem cells (seed 5) | 5364 | **14703** |

**The chartered numbers moved, and one of them a long way.** A system used to
break into as many as 29 separate walkable regions — twenty-nine caves to the
player walking it — and now breaks into at most 3. The share of a system
reachable from one place went from 30% to 90%.

**Contrast is the honest complication.** Its median fell (3.68x -> 2.64x)
while its p90 rose enormously (5.4x -> 21.3x). Both are real: rooms are far
bigger relative to passages than the honeycomb's were, and the passages are
wide enough to walk, which puts a floor under the median that a honeycomb of
unwalkable fringe did not have. Chasing the median back up means narrowing
passages below the 14 rows the player needs, which is the trade the old carve
made and lost. **Do not tune for this number.**

**The void fraction is the open question, and it is the one on the card.**
0.187% against 0.563% is about a third of the cave there used to be. It buys
rooms walkable end to end instead of pockets nobody can get between, and the
owner never asked for more void — but "a world with essentially no caves in
it" is exactly what §4 of the Phase 2 report measured as the failure mode, and
this moves in that direction. If the verdict is "too empty", the levers are
`ROOM_GAIN`, `TRIB_MIN`/`TRIB_MAX` and the envelope size draw, in that order.

## 5. Open, and what a successor should know

- **The monumental chamber is still a rasterised ellipse.**
  `grow_monumental_chamber` runs unchanged after the trunk, so the largest
  room in most systems is still `(dx/rh)² + (dy/rv)² > 1.0`. With the trunk
  now making rooms of its own it is arguably redundant, and it is the loudest
  drawn primitive left in the carve. Not removed here because the card in
  front of the owner shows what the code actually does, and his verdict on
  whether the rooms read right is the evidence that should decide it.
- **A room can fill its whole envelope, and nothing caps it.** The radius is
  `TRUNK_RADIUS + open^2 * half_h * ROOM_GAIN`, so with `ROOM_GAIN` at 1.6 an
  `open` of only 0.5 gives a radius of ~200 in an envelope whose half-height
  may be 320 — most of it. Measured, tallest open column reaches 156 in
  envelopes as small as 176 tall, i.e. 89% of the envelope. Found by reading
  the diff, not by a metric.

  Deliberately **not** capped before the owner has seen it. It is a
  judge-by-eye question and the card in front of him shows exactly this; a cap
  (`r.min(half_h * something)`) is a one-line change if the verdict is "one
  big blob", and guessing now would be tuning against my own taste. Worth
  knowing that round 5 collected a related complaint — *"it looks like a
  single room instead of a cave system"* — though that was a system that was
  *only* a lens, where this one has a trunk and feeders around the room.
- **`near-pairs` is 0** and has been since Phase 2. Not investigated here.
- **`geode vugs` fell to 0 over 3 seeds.** The vug branch was not touched, so
  this is either seed variation at n=3 or the census's `sh.w < 50` cut
  interacting with the new shapes. Worth one measurement before it is quoted.
- **A space-filling field remains the answer to "not enough cave"**, and
  `dead-ends.md` keeps the Worley trio's settings history for exactly that
  case — with the note that its straight boundaries would have to be broken by
  domain-warping the sample position, which was never tried.
