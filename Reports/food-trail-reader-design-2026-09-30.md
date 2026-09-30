# The food trail's reader and give-up: the design to build (Stages 2-3)

*Design, 2026-09-30. `engine`. Not built. Stage 2 (read) and Stage 3 (give up)
of [`food-trail-plan-2026-09-29.md`](food-trail-plan-2026-09-29.md), designed
on top of Stage 1 (only a trip load lays trail B), which shipped on that day
([`ant-scenes-2026-09-23.md`](ant-scenes-2026-09-23.md) §23c).*

**How this was made.** A panel of agents worked from the Stage 1 traces
(`s1lay`, 16 `dwide` CSVs, 1.16M rows, the BTRAIL planes):
- one gathered facts;
- three designed independently;
- a judge synthesised the three;
- three refuters attacked the synthesis.

**None of the refuters refuted it. All three found defects, and the must-fix
list below is the part a builder cannot skip.** The raw result, with every
design and every defect's evidence, is
`Reports/data/food-trail-reader-panel-2026-09-30.json.gz`. The panel's
scratch scripts lived in `/tmp` and are gone; every number below was measured
by them on the Stage 1 traces (`food-trail-lay-2026-09-29.tar.gz`), and any
you build on should be re-measured with committed code first.

## 1. What the traces say about where a reader can act

- **The door is where direction is decided, and trail B does not decide it
  today.** At the door, an empty hungry or driven ant's east/west pick is its
  arrival heading. Facing E it picks E 77% (90 cells) and W 5%; facing W it
  picks W 74% and E 6%. Trail B does not steer it at reach 2 or reach 6. The
  live reach-2 read is saturated on both sides.
- **The gated plane at the door knows the food side.** Stage 1's plane at
  reach 6 reads food side (g > 0.1) on 75% / 53% of door decisions (90 / 140
  cells) and the wrong side on 1% / 2%. Both sides are dark on 17% / 38%.
- **A door visit is short.** The median dwell is 42 / 36 frames, about 3
  chooser decisions. 46% of visits start by coming up the shaft, and 36% end
  going back down it.
- **The road has nothing to compare.** On about 60% of road decisions the only
  level alternative to going on is the reverse, so a forward-only comparison
  along the road almost never has two candidates.
- **`AtNest`, and anchor == head, is not a door detector.** It fires in the
  lined shaft about as often as on the door paint, and never on the spoil
  mound above the door, which holds 39% of door-zone decisions.
- **The road east of the door is lit in pieces** under the gated plane. The
  gaps sit next to the door, where a departing follower needs the trail.
- Only 2-3% of hungry or driven decisions happen at the door. 40% / 25%
  happen west of the door or up the walls.

## 2. The rule

**Stage 2, `read`: a door-only pull toward the food side.** An empty ant
that is hungry or driven, standing in the door box, reads trail B six cells
east and six cells west on its own row. Every level heading on the stronger
side gets an additive bonus. Nothing else in the world is steered by B, and
nothing is ever subtracted.

- **Gate.** All of these must hold:
  - `ft.read`;
  - the ant reads the trail and is not laden (so a packed-lunch carrier
    counts);
  - `pull.is_none()`;
  - the ant carries no spoil;
  - it is not nest-bound (see M7 below);
  - it is not a scout that has given up.
- **Door box.** `door_site(world, hx, hy)`:
  - it iterates `world.nest_sites`;
  - columns within `nest_door_of + 1` of the site;
  - rows from `surface - 1` up to `DOOR_READ_RISE = 4` above it, which is the
    paint row plus the spoil mound;
  - it never includes a shaft cell;
  - it is inert under `NEST_DOOR=off`.
- **Want** is `max(hunger, drive)`, where
  `hunger = 1 - clamp(energy / start_energy)`. When want is 0 there is no
  term.
- **Read.** `bE`, `bW` are the B values at `(hx ± sensor_offset, hy)`, the
  ant's own reach-6 sensor, never the 1- or 2-cell ring (the door's own
  delivery smear). Then `g = (bE - bW) / (bE + bW + TRAIL_HALF)`. When
  `g == 0` there is no term.
- **Term.** `f[d] = gain × want × |g|` for each level heading on g's side
  whose landing is at or above `surface - 1`; every other heading gets 0.
  - Added as a branch (`base + f`), never as `+ 0.0`.
  - `FOOD_TRAIL_GAIN` goes 3 → **6**: on the door a west-facing ant scores
    about 1.9 for going on, and even odds need `gain × want × g ≈ 1.9`.
- **Static effect on Stage 1's traces:**
  - a west-facing reader's P(east) goes 6.5% → 51.8% (90 cells) and
    6.7% → 44.8% (140);
  - all facings go 46% → 74%;
  - steps into the ground go 12.7% → 5.4%;
  - with both reads dark it is exactly `lay`.

**Stage 3, `giveup`: once given up, the trail lets go.** It keeps
`spent = ft.giveup && scout_w > 0 && scout_home` as its only clock, which is
scout patience, not B. When spent:
- the away term is 0;
- the pull home is `scout_w × cos(home)` without `(1 - presence)`;
- the trail no longer holds the heading (`hold = 1`).

On Stage 1's traces, given-up ants east of the door on lit B step outward on
28.6% / 41.9% of decisions today. That falls to 3.4% / 5.2% with all three.

**Rejected, with the reason recorded in the JSON:**
- an off-door fork rule (it cannot be measured on the bed, where the road
  has one forward option);
- the max-of-three door read (it reads the door's own smear);
- a per-excursion commitment (it is memory for empty ants and needs an owner
  ruling; it is the registered next lever if the west-departure bar fails);
- trail-paid patience, which keys give-up on dark road; the 140-cell road has
  dark runs longer than its budget. It is the fallback if the pulsed or
  two-pile bed shows a stale leash.

**Code sites** are listed in the JSON's `synthesis.code_sites`. Line numbers
there are for `96a9e81d`: run `bash scripts/branchcheck.sh --who-touched
src/sim/creature.rs` and re-find them.
- `FoodTrail`/`parse_food_trail` beside `trip_reach_of`.
- `door_site` beside `nest_door_of`.
- `door_side_read`/`door_follow` beside `trail_presence`.
- `chooser_step`'s score closure and trace block.
- `DecisionScratch`/`DecisionRow`.
- Four `CreatureStats` counters.
- A `dread` flag in `trailfollow`.
- A §5 READER and §6 GIVE-UPS in `scripts/trailclimb.py`.

## 3. Must fix before building (the refuters' major defects)

1. **A given-up ant is re-armed at the door and sent straight back out.**
   Nest contact on the paint resets the scout, and the paint is inside the
   door box. Fix one of two ways:
   - under `giveup`, re-arm only on a contact below the surface, or when the
     ant has eaten;
   - or keep one bool set until it eats.

   Test it on the door, not only on a corridor.
2. **A stale leash lasts 800-1,400 frames.** After returns stop, the door
   still reads "food side". Gate the door term on the colony's own news:
   `world.frame - last return at this site <= RETURN_WINDOW`. That is the
   forage drive's own clock, an integer compare with no new constant, and it
   drops about 1-3% of door decisions on the unlimited pile.
3. **Stage 3's guards 15 and 16 are blind as specified.** Make test 16 a
   per-decision probe:
   - facing **out** (east), with `scout_home` set;
   - `scout_w` about 1 (energy 0.5 of grant, no nest, drive 0);
   - 300 decisions, put back each frame.

   The design gives 0.8% outward. Each fault gives 87-97%. The bar is
   <= 0.05.
4. **Give-up still needs a wall.** Patience decays only on steps that get no
   further out, so on open ground a follower past a dead trail walks on. The
   bed and the lab are walled, and the target world is not. Before building:
   - register a readout of cells walked past the trail's last lit cell
     before give-up;
   - add a no-wall corridor test (300+ dark cells) that records the distance.

   If the distance is large, build the bounded form.
5. **The ship rule dropped the plan's owner-approved gates.** `read` ships
   only on all of these:
   - F against L, paired, on all five beds (unlimited 90 and 140, pulsed 90
     and 140, 80 founders): taken not lower and starved not higher, sign test
     p < 0.1 each way;
   - **B5, the two-pile bed**;
   - **B6, the lab pair**: no gate worse at p < 0.05;
   - §23c's readouts: the lab's starved per ant-frame, and the terrain seeds'
     hungry-at-home share.

   Until B5 exists, `lay,giveup` may ship at most.
6. **The pulsed bar cannot fail in the harm's direction.** Pulsed taken sits
   at its cap, and a leash makes refills go faster. Replace it with:
   - pulsed starved against L and against M;
   - walking J per ant-frame;
   - stale visits per refill cycle, from a traced pulsed subset.
7. **Stage 3's instruments are not in the repo.** `giveup.py` and
   `departures.py` were only ever in `runs/s1/v22/`, and are archived in
   `food-trail-lay-2026-09-29.tar.gz`. Port them into `trailclimb.py` §6 and
   reproduce 0.295 / 0.300 and 1,253 / 1,086 frames on the `s1lay` traces as
   the positive control before registering. Define "give-ups after a stray
   reversal" and "east legs that load" in code, or drop them.
8. **The negative control D is unmeasurable as registered.** Trace D on seeds
   1-8 with `dread` and its own `dtag`, or withdraw that half of P2.10. Make
   bare `read` mean `lay,read` and spell D as `read,nolay`.

**Minor, fix while building:**
- **No hungry ant is gated out.** `HUNGRY_HOME` is off, so `pull.is_none()`
  protects nobody. Either `want = drive` whenever hunger > drive, or add a
  reserve floor. Register a hungry-class readout: energy at departure, and
  eaten within N frames after a door visit.
- **The `!is_nest_bound` gate excludes exactly the hungry workers**, since
  the drive already zeroes fed ones. Drop it, or state the reason.
- **On the mound, the anchor terms oppose the read.** The anchor sits 1-6
  rows below the ant, so `scout_cos` gives ±1 there. Split every door readout
  into paint and mound; in test 7, cover an anchor one column off.
- **Byte identity breaks on the header echo.** The `gain` in `FoodTrail`'s
  Debug goes 3 → 6. Compare logs with header lines filtered, and CSVs in full.
  Rebuild the `lay` reference from the parent in the same session.
- **Placement and reach:**
  - count `trail_door_reads` where the term is computed, because a
    trunk-crossing pick returns before `commit_step`;
  - reject a negative `gain=` in the parser;
  - make the box half-width `scaled_cells(half) + scaled_cells(1)`;
  - read reach 2 and reach 6 the way `sense` does.
- **Test scenes:**
  - test 6 must set `foraged = false`, or the drive makes want 1;
  - test 8's scene must be asymmetric, or its fault stays green;
  - test 12's drive-0 lunch carrier needs energy >= `start_energy`.
- **Traced arms and the record:**
  - give each traced arm its own `dtag`, because the CSV name carries no
    food-trail part;
  - log `base` beside `fo`, since `sc - fo` is not exact in f32;
  - add births on 80 founders to the harm bars (the one lean against `lay`);
  - add F against LR on taken at 80 founders (the hold drop at density).
- **Frame cost:** measure it as `ascii`'s ant-scene mean and `trailfollow`
  wall time at `RAYON_NUM_THREADS=1`, paired and alternating, never a
  worst-frame bar. Compute `drive` once per decision; it calls `exp`.

## 4. Order of work

1. Build **B5, the two-pile alternating bed** first. The plan requires it, the
   stale leash (M2) and lock-in only show there, and it needs no reader.
2. Port the Stage 3 instruments (M7) and register §23d with the amended bars
   (M3, M5, M6, M8) **before** any Stage 2 run.
3. Build `read` and `giveup` behind the existing switch, fixing M1 and M2 in
   the rule itself. Watch every guard red.
4. Measure L / LR / LR3 / LG / F / M on the five beds, plus B5, plus B6. Ship
   what passes. `lay,giveup` alone is a legitimate subset.
