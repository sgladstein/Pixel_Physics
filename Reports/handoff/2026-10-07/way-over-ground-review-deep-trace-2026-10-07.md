# Review: "the nest's ways prefer ground to the backs of other ants" (Deep trace, 2026-10-07 08:25 UTC)

Second key on `nest-race/way-over-ground-proposal-2026-10-07.md` (PIXEL_PHYSICS_WAY_FOOT).

- **Code read on:** 95d65cd66, the b-arm build. File references are to `src/sim/creature.rs` there.
- **Labels:** measured or read from the code unless a line says inferred.

## Verdict

**Yes, build it behind the off switch, with four changes.** One of them is a design change that removes the
question you most needed answered.

1. **Do not weight `NestWay::dist` itself.** Five readers in the engine take it as a step count. Two of them walk *up*
   it, and on a weighted field walking up usually leads onto the crowd. Instead, add a second field built in the same rebuild over the same
   cells, read only by the walk-out's aim. See Q4.
2. **Drop `out`.** It cannot change anything. The soil way out does not walk it: it walks the nest's way, so `way`
   already covers it. See Q2.
3. **`way` does not cover the store pull.** The store field is its own breadth-first count. If you want the store
   pull's 14.6% vs 1.8% to move, it needs its own part. See "The store pull".
4. **Leave `mound` out of `on`.** It has the same footing, but no trace has looked at falls in the mound.

**K is not set by the length ratio.** See Q3. I am running a seed-1 check now: how far each K moves the walk-out's aim
off the crowd, with the engine's own aim as the control. It reports before your arms need it.

## Q1. Does the rule address what both traces show? Cost K or ground only?

**It acts on the measured mechanism.**

- On both seeds, a step toward a target held up only by ants is followed by a fall 14-26% of the time, against 1-2.5%
  toward a target with ground beside it.
- 89-94% of falls in the nest have only ants round the head.
- The rule changes exactly that: where the walk-out aims.

**Cost K over ground only: agreed.**

- Weighting changes the cost of a step, not which cells are on the way. The weighted search runs over the same cells
  and reaches the same set, so every ant that has a direction today keeps one.
- Ground only would leave 3-11% of the starvers' crowd rows with no target. That is the no-pull state HUNGRY_OUT and
  then WAY_GAPS were built to end: the WAY_GAPS doc records hungry empty ants in the nest with no pull on 57-68% of
  their steps before it, and 0.1-0.5% after.

**Inferred, not shown:** that an ant re-aimed at a ground-footed cell falls at the ground-target rate.

- The 1-2.5% was measured on ants that aimed at ground because they were already near a wall.
- My split by the ant's own footing (s1: on the crowd, 25.4% toward an ant-held target vs 2.0% toward ground) is the
  best evidence the rate holds for ants on the crowd. It is still observational.
- An ant in the open middle still crosses crowd cells until it reaches the wall. Only the arms can show whether that
  first crossing is where the falls are.

**A second effect, inferred.** Ground footing is still there 30 frames later; ant footing usually is not. So the
weighting also aims the walk at cells whose footing the 30-frame-old picture still gets right.

## Q2. `out` and `mound`

**`out` (build_out_way) changes nothing.**

- Its only reader is `shut_in` (`out.covers(x, y) && out.at(x, y).is_none() && under_cover(..)`). That asks only
  whether a cell was reached.
- A weighted search reaches exactly the same cells, so an `out` arm is byte-identical to off: an exact-zero arm.
- At most, run it once for 20k frames as a check that the weighted search reaches the same cells.

**The soil way out walks the nest's way.** `soil_way_pull` calls `way_out_from`, which calls `step_down_way` on
`world.nest_ways` (creature.rs ~15044-15053 and ~12968-12972). So the soil way out's 26.1% vs 2.5% on s4 is already in
`way`, alongside the walk-out.

**`mound` (build_mound_way) has the same footing** (`way_cell`). Its readers:

- `spoil_haul_target` under NEST_STORE `sky`, which is on in the b-arm: a pellet carrier under cover in the mound
  walks `step_out_way`, a down-walk, so it would bend to the tunnel walls;
- `mound_out_pull`, MOUND_OUT's `way` part, which is off;
- three that ask only reached or not: `shut_in_mound`, `beside_mound_hole` and `on_mound_way_covered`.

No trace has measured falls on the mound's way. Keep it a named part, out of `on`, until one does.

## Q4. What reads `dist` as a step count (the one you most needed)

**These break if `dist` itself is weighted:**

| Reader | What it does with `dist` | What weighting would do |
|---|---|---|
| `fill_nest_store` | Store food = loose food beside a way cell with `dist >= depth` (STORE_DEPTH 20) | A cell 8 steps in, reached across 4 crowd cells, reads 20 at K = 4. Shallower food becomes store, so **the storeroom moves** |
| `is_store_cell` | The same test at the moment of a bite (`keep`, store bites) | The same shift, so `keep` and store bites change |
| `store_inward`, arrival | `d >= depth && way_roomy`: where a Carry with no store food and a `home` walker (on under `on`) stop | They stop shallower |
| `store_inward`, the deeper walk | `field_walk(.., way.at, false)` walks **up**, to the largest neighbour | Beside a crowd the largest neighbour is usually the crowd cell, because entering it costs K. An ant on the wall at d sees the next wall cell down at d + 1 and the crowd cell beside it at about d - 1 + K or more. **Inward walkers are steered onto the crowd**, the opposite of the intent. On the b-arm that is the `home` pull and Carry with no store food |
| `rest_pull` | Walks up the same way | The same; NEST_REST is off on this stack |
| `NestWay::depth()` | Deepest step | Read only by `examples/digbox.rs`, a readout |
| The probe (deeptrace) | digrows `way_d`; `wallway.csv`'s depth and ratio columns | They would read cost, not steps, on the arm. (`match` compares only reached or not, so it stays 0 either way) |

**These are fine on a weighted field:**

- The down-walks: `step_down_way` (walk-out, soil way out, `mound_out_pull`), `step_out_way`, and `field_walk` down
  the store field. They go to the smallest neighbour, and on a cheapest-cost field the smallest neighbour is always on
  a cheapest route.
- Everything that asks only `at(..).is_some()`: `store_feeds_here`, `store_carry_arrived`, `fetch_target`,
  `way_roomy`, `shut_in`, `shut_in_mound`, the mound-hole tests.

**So: do not change `dist`.**

- Add a second field to `NestWay`, say `foot`: the cost from the door, 1 into a cell with ground or a plant beside it
  and K into a cell held up only by an animal.
- Build it in the same rebuild, over the same cells (inside and `way_cell`).
- Read it **only** in `way_out_from`, so in the walk-out and the soil way out.
- Everything else keeps steps. The store, the inward walks, the rest pull and the probe are then the same on both
  arms, and the arm changes only the aim it is meant to change.
- Cost: one more `u16` per cell in a 129 x 65 box, and one more search per nest per 30 frames.

**Determinism.** Cheapest costs are unique whatever order the queue pops in, so the field is deterministic. The walk's
tie order is unchanged.

**Guards, each able to fail:**

- At K = 1, `foot` equals `dist` on every cell, in a unit test over a built scene.
- With the switch off, the build is byte-identical to the b-arm (`stats.csv`, `events.txt`, ledger).
- A scene where the shortest way crosses a crowd and a slightly longer one runs up a wall. Off, the aim is on the
  crowd; on, it is on the wall. Put the fault back by setting K = 1 and watch it go red.

## The store pull is not covered by `way`

- The store field (`NestWay::store`) is its own plain breadth-first count, built in `fill_nest_store` from the way
  cells beside store food, over the way's cells.
- The Eat and Carry pulls walk it down: `field_walk(.., store_at, true)`.
- Weighting `build_nest_way` leaves it as it is, so the store pull's fall-next (14.6% vs 1.8% on s4) will not move
  under `way`.

If you want it, give the store field its own weighted twin for that down-walk, as its own part (say `store`). Keep the
step count for `smell`: `store_smelt` reads `store_at <= smell` as steps. Weighted, an ant 3 crowd cells from the
store would read 12 at K = 4 and stop smelling it at smell 10.

## Q3. K

**K is not compared to the whole route's length ratio.**

- The weighted way leaves the crowd wherever the detour costs fewer extra steps than (K - 1) x the crowd cells it
  avoids.
- Example: a route that crosses 2 crowd cells switches to the wall only if the wall route is fewer than 6 steps
  longer at K = 4. That limit is 2 steps at K = 2 and 14 at K = 8.
- So the p90 of 1.16-1.55 does not pick K. My per-rebuild p90 ratios over all cells on both ways were 3.05-3.52 at
  55-65k on s1: some rooms' wall routes are much longer than the crowd's.

**The 2 / 4 / 8 sweep is a sound dose-response.**

**What I am running now (seed 1, measuring only).** Each rebuild, the probe builds the same way at K = 1, 2, 4 and 8
from the same world. For every walk-out and soil-way row, it records where `step_down_way` would aim at each K and what
holds that cell up.

- **Control:** at K = 1 the probe's aim must hit the engine's own aim on every row, and the K = 1 field must equal the
  engine's cache cell for cell.
- **What it answers:** which K moves the aim off the crowd, for which ants (the starvers), and whether K = 2 already
  does most of it. If it does, two of your arms can go.

## Notes for the test

**Arms.**

- Drop `out`.
- If you build the store part: off; `way`; `store`; `way,store` at K = 4; then K = 2 and K = 8 on the winner.

**Judge the traced problem per ant, not per event.**

- Use the funnel for hungry ants in the deep room: hungry deep, then reached the shaft, then reached the door, then
  reached food, then ate. Book each ant at its high-water mark, with counts and both percentages.
- Fall-next by target class will change its mix under the arm. Quote fall-next per walk-out step overall, and falls
  per hungry deep ant.
- Pair within seed.

**Watch the brood pile.**

- Brood is a powder cell (`is_partable`: "Brood is a powder cell owned by its brood organism"), so a cell beside brood
  counts as ground.
- Under WAY_GAPS `brood` the way runs through the pile, and every cell in it is ground-footed, so it costs 1. The new
  route may prefer the brood column to the crowd.
- Earlier traces tied ants walking through that column to brood falls and to where eggs go. The WAY_GAPS doc has eggs
  tracking falls at r 0.81.
- Count brood falls and eggs per arm. The store's food pile is the same case: food beside a cell is ground.

**Instruments.** With the separate field, probe v3 reads both arms unchanged, and `way_d` stays steps. Add the new
field's value as its own column if you want the cost.

**Walk-out and soil way out share `way_out_from`.** To tell their effects apart, a part per caller is a small split.

**Frame cost.** As you said: quote ascii's worst frame before and after.

## Addendum, 08:25: Q3 measured on seed 1

Full write-up: the v4 section of `store-arms/sky-meal/shut-in-probe-s1-deep-trace.md`.

**Checks.** The probe's walk at K = 1 aims where the engine's did on all 475,571 walk-out and soil-way rows. The
weighted way reaches the same cells as today's at every K.

**Walk-out aims on a cell held up only by ants, ants standing on the crowd:**

| | K = 1 (today) | K = 2 | K = 4 | K = 8 |
|---|---|---|---|---|
| All rows | 47.5% | 19.1% | 6.7% | 4.8% |
| The 53 starvers' last 5k frames | 89.7% | 43.9% | 11.9% | 8.3% |

**Inferred:** at today's fall rates per footing and aim, the walk-out's next-fall from the crowd would go from 13.1% to
3.5% at K = 4 (starvers 23.0% to 4.8%).

**So for K:** 2 does about half and 4 most; 8 adds little over 4. Sweep 2 and 4, and keep 8 for a residue.
