# How the ant works

**A living reference: what the shipped ant (`assets/species/ant.ron`) does on
every tick, how each mechanism is implemented, and what it reads.** It is
written from the source and describes the code as it is now, not as it was or
will be.

- **Verified against:** `main` at `bb65d507`, 2026-09-22. §2, §6c, §13 and
  §15 re-checked the same day against the decision-trace change
  (`usable_headings`, `home_weighted_pick_why`); §5, §6b, §6c, §13, §14 and
  §15 re-checked 2026-09-23 against the drop, cone and homeward counters;
  §5, §12 and §15 again the same day for the drop through bodies, and §9
  for what a part-eaten cell is worth once put down.
  Update this line whenever a section is re-checked against the code.
- **Edit it in place. Never append history.** When you change a mechanism
  described here, update the section in the same commit. When you find this
  document was wrong, fix the text and say so in the commit message. It
  should not grow as time passes; it grows only if the ant does.
- **What does not belong here:** measurements, experiment results, open
  questions, plans, or "we used to think". Those go in a dated report,
  `open-bugs-handoff.md` or `dead-ends.md`. The one exception is §14, a
  short list of source comments that currently contradict the code, deleted
  line by line as they are fixed.
- **Sections name functions, not line numbers.** Line numbers go stale within
  a day in `creature.rs`.
- **The weights below are the founder genome as authored.** Every ant carries
  its own copy of the genome, and descendants drift from it by mutation
  (`mutation_rate: 0.0033758` per slot per birth). An evolved colony can
  differ from this page, and a measurement that matters should read the
  genome it ran.

---

## 1. The shape of one tick

`creature::creature_tick`, once per scheduled tick, in this order. Every step
can end the tick early.

1. **Housekeeping.** `reconcile_chain`. A burning ant only reschedules.
   **Old age:** `life_half_life: 40000` frames, rolled every tick. An ant
   mid-`Crossing` (inside a trunk) or mid-`flight` runs that branch instead
   of everything below.
2. **Sense.** `sense` fills the input vector (§3). If `AtNest`, the colony
   scent blends (`blend_with_nest`).
3. **Brain.** `brain::eval_brain` turns inputs into outputs (§4).
4. **Standing costs** are charged: idle, synapse, force and armour taxes.
5. **Act.** `act`: attack, share, feed, drop, dig, at most one of each and in
   that order, with early returns (§5).
6. **Move or tumble** (§6). One draw against `P(move)`: on success, step;
   on failure, maybe tumble.
7. **Lay trail**, only if the step succeeded (§7).
8. **Memory upkeep.** `phero_a_mem`, `since_nest += 1`, `still_ticks` (reset
   by a move, otherwise incremented).
9. **Digest** the crop (§9), apply the energy change, starve if energy ≤ 0,
   bud if eligible (`try_bud`).

**How often.** `organism_tick_interval` is `tick_interval: 6`, scaled by the
ant's expressed `TRAIT_PACE` and by its body's leg fraction. A founder decides
**once every 6 frames**, so the ceiling on any per-frame movement rate is 1 in
6. Pace mutates (width 0.15 per birth), so descendants differ.

## 2. The body and the world it can stand in

- **Body:** `Chain(2)`, a head and one segment. Headings are the 8 `DIRS`.
- **Support** (`step_chain`): the body is held up if **any** of its cells has
  a `Solid`, `Powder` or `Plant` cell among its 8 neighbours. Otherwise the
  ant falls one cell. **The fall happens only inside `step_chain`, so only
  after a successful move roll**: an unsupported ant with `P(move) = 0`
  hangs in the air. A fall counts as a move and lays trail.
- **Enterable cell** (`cell_is_enterable`): empty, its own body, or living
  plant tissue (`is_partable`: leaf, wood, grass, reed, moss and fruit are
  walk-through while alive; `TISSUE_PARTING` on). A nestmate is **not**
  enterable at the default `PIXEL_PHYSICS_STACK_DEPTH=1`.
- **Foothold** (`head_has_foothold`): the **head's** 8 neighbours include
  `Solid`, `Powder` or `Plant`, or a nestmate (`climbs_over_kin: true`).
  Ants walk on walls and ceilings.
- **Usable heading** (`usable_headings`): enterable **and** footed. This one
  predicate decides what the ant can do in each setting, and both the tumble
  and the decision trace's setting class read it:

| Setting | Usable headings |
|---|---|
| Flat ground, 1-wide tunnel, flat wall face | 2: forward and back. The upper diagonal has no foothold; the lower one is ground |
| Corners, ledges, steps, the base of a stem, tunnel junctions | 3–5 |
| Inside a canopy (tissue is enterable and footed) | up to 8 |
| Burrow pocket, crowd | 0–1 |

## 3. What the ant senses

`sense`, evaluated at the ant's own cell `(x, y)` with its current heading.
Only the inputs `ant.ron` wires are listed. Everything else in
`brain::BrainInput` is computed and read by nothing in the ant.

| Input | How it is computed |
|---|---|
| `Bias` | 1.0 |
| `Energy` | `energy / start_energy`, clamped to 0..1. **`start_energy` is 200 while budding needs 1,100, so any fed ant reads 1.0**; only a hungry one reads less |
| `FoodAdjacent` | 1 if an edible cell is in reach of the mouth (`adjacent_food_counted`), else 0 |
| `KinNeed` | the energy deficit of the neediest adjacent nestmate, from the same scan |
| `AtNest` | 1 if nest material is among the 8 neighbours (`nest_within_reach`; radius 1 by default) |
| `Crowding` | **At the nest:** nest-room occupancy at the nearest nest site, `2 / (2 + roofed empty cells per ant)` (`room_target` 2.0), so 0.5 at two roofed cells per ant and rising as the nest fills. **Elsewhere:** creature cells within radius 2, divided by 8, capped at 1 |
| `Stillness` | `(still_ticks / 192)²`, capped at 1. `still_ticks` counts consecutive ticks without a move |
| `CarryingFood` | 1 if the crop holds any food, else 0 |
| `Carrying` | crop fill (`crop.worth() / crop_capacity`), or 1 while holding a spoil pellet |
| `HomeAligned` | laden only: the cosine between the heading and the vector from the head to `home_target`. 0 when the crop is empty or within 1 cell of the target. `home_target` is `forage_anchor` (§8) |
| `PheroAAlong`, `PheroBAlong` | `(front − here) / (front + here + 256)` on that trail plane. `here` is the scent at the ant's own cell. `front` is **6 cells** along the heading (`sensor_offset: 6`); for a diagonal heading on the ground it is projected onto the ant's own row (`trail_sample_point`). Forced to 0 when both planes read 0 at the front point and that point is inside solid, powder or plant (the "honesty" gate) |
| `Alarm` | the alarm plane at the ant's own cell |
| `TempAboveAmb` | (noon-equivalent temperature − ambient) / scale, clamped to −1..1 |
| `MoistureGrad`, `SurfaceCurvature` | used only by digging and spoil dropping |

**Computed and read by nothing:** `PheroAFront` and `PheroBFront`, the trail
strength 6 ahead, so **no ant responds to how strong a trail is**.
`PheroALateral` and `PheroBLateral`, right minus left at 6 cells along
heading ± 45°. On a surface these points are sky and rock, and the plane
holds scent there because it ignores terrain (§7). Also `PheroARise`
(laden only: trail A under the head now against `phero_a_mem`, a 0.995
running average), light, moisture-ahead, and every sight input. Founders
are blind (`sight_range` 0), but `TRAIT_SIGHT_RANGE` is heritable, so a
lineage can evolve eyes.

## 4. The brain

`brain::eval_brain`: one hidden layer of 8 units, each with optional
self-recurrence, and 16 outputs. `squash(x) = x / (1 + |x|)`.

```
hidden_h = squash(rec_h * hidden_h(previous tick) + Σ w * input)
output_o = squash(Σ w * input + Σ w * hidden_h)
```

**What the founder genome's hidden units do.**

| Unit | Wiring | Job |
|---|---|---|
| 0, 1 | `Bias −45, CarryingFood +45.5, PheroAAlong ±6`; out to `Move ±2.5` | **Laden only:** trail A's forward difference as a throttle on stepping. Shut when empty; the mirror pair cancels, so a shut pair adds ≈0 |
| 2, 3 | `Bias +0.5, CarryingFood −45.5, PheroBAlong ±6`; out to `Move ±2.5` | **Empty only:** trail B's forward difference as a throttle on stepping |
| 4 | `AtNest +0.05`, recurrence `0.99995`; out to `EmitA +32` | **Nest odometer.** Charges slowly while touching the nest and decays after leaving. It sets how much trail A the ant lays; the source's own fit quotes 0.819 falling to 0.010 over 3,000 ticks |
| 5, 6 | `Bias −30, AtNest +30, Crowding ±6`; out to `Dig ±2.5` | **At the nest only:** the less room per ant, the stronger the urge to dig |
| 7 | nothing | free |

The trail throttle's size, from those weights: `2.5 × (squash(0.5 + 6a) −
squash(0.5 − 6a))` for a forward difference `a`. That is ±1.5 at a = ±0.1,
±2.6 at ±0.2, and at most ±4.3.

**Outputs, their direct wires, and what they come to.**

| Output | Direct wires (plus the hidden units above) | Read by |
|---|---|---|
| `Move` | `Bias +2.0, Energy −1.75, HomeAligned +3.0, Stillness +1.5, KinNeed +1.25, FoodAdjacent −1.16, Alarm −1.0, Crowding −0.3` | the step roll (§6) |
| `Turn` | `TempAboveAmb −0.8`, so it lies within ±0.44, is exactly 0 at ambient (no bias), and stays near 0 wherever the temperature does | the forward cone (§6) |
| `Persist`, `Caution`, `Tumble` | **none**: `squash(0) = 0`, which `unit_scale` maps to the midpoint. **Persist 1.0, footing 0.6, tumble chance 0.5** | cone and tumble (§6) |
| `EmitA` | unit 4 only | trail A (§7) |
| `EmitB` | `CarryingFood +2.5`, so **0.714 while laden, 0 while empty** | trail B (§7) |
| `Feed` | `Bias +0.4, FoodAdjacent +0.8`: 0.29, or 0.55 with food in reach | §5 |
| `Drop` | `Bias −0.2, AtNest +1.0889, Carrying +0.2`: **0 away from the nest, ~0.5 at it** | §5 |
| `DropSpoil` | `AtNest +0.9, Carrying +0.2, SurfaceCurvature +0.169` | §5 |
| `Dig` | `Bias +0.15, FoodAdjacent +0.8, MoistureGrad −0.55` | §5 |
| `Share` | `Energy +2.5, KinNeed +1.9, Bias −2.5` | §5 |
| `Attack` | `Alarm +2.0` | §5 |
| `Impulse`, `Provision`, `Fly` | none (0): no hops, no birth provisioning, no flight | — |

**`P(move)` in common states**, founder genome, no trail reading, not still,
nothing adjacent:

| State | Sum | `P(move)` per decision |
|---|---|---|
| Empty, fed | 2.0 − 1.75 = 0.25 | **0.20** |
| Empty, hungry (`Energy` 0.5) | 1.125 | 0.53 |
| Laden, facing home | 3.25 | 0.76 |
| Laden, 45° off home | 2.37 | 0.70 |
| Laden, perpendicular to home | 0.25 | 0.20 |
| Laden, facing away | −2.75 | **0, exactly** (the clamp) |
| Any, food in reach | 0.25 − 1.16 | 0 |

The trail throttle adds to these sums and can outweigh any of them. Stillness
adds up to +1.5 after 192 still ticks, which is what eventually frees a stuck
ant.

## 5. Acting: feed, drop, dig, share, attack

`act`, before movement, in this order. **Later steps are skipped by an early
return**, so the order is part of the behaviour. A return ends `act`, not
the tick: the ant still gets its move roll (§6) afterwards.

1. **Attack** (at `Attack` probability): bite the nearest foe. It displays
   instead of committing when `contest` odds say so.
2. **Share (trophallaxis)**, on by default: give a quarter (`SHARE_FRACTION`)
   of the energy difference to the neediest adjacent nestmate that has less.
3. **Feed = pick up.** With food in the crop, `choose_weighted` between
   `Feed` and `Drop` first decides whether to try feeding at all. Feeding
   requires room in the crop and the same material as what is already held.
   A successful feed roll **removes one adjacent food cell from the world
   into the crop**, and `act` returns. **The crop is both the cargo and the
   stomach** (§9). `crop_capacity: 2880` worth units is three fruit cells
   (960 each) at the neutral gut.
4. **Drop food**, whenever the crop holds anything, then **return**. The roll
   against `Drop` is taken **first**. Only on a win does it look for a place
   (`food_drop_site`), and put one food cell there:
   - the first empty cell among the 8 neighbours, in fixed order;
   - if there is none, **the nearest empty cell reachable by handing the food
     through bodies**, the ant's own and any other creature's, never through
     ground, nest material or food already put down. So a blocked ant passes
     its food back along its body or through the crowd;
   - if even that finds nothing, nothing happens: the roll is spent.

   `PIXEL_PHYSICS_DROP_REACH=adjacent` turns the second rule off.

   The drop is
   not gated on being at the nest, but `Drop` is 0 elsewhere. `deliveries`
   counts any drop made while `AtNest`; `drop_census` counts every roll by
   outcome (`DropWhy`: lost, placed, delivered, no room), and
   `drops_passed_on` the drops that went past the neighbours.
5. **Drop spoil**, if holding a dig pellet, then **return**. The target must
   be empty, sit on at least two filled cells of the three below it, and have
   clear headroom above. It may lift up the shaft (`lift_reach`).
6. **Dig**, only if both crop and spoil are empty. **So a laden ant never
   digs.** It removes the cell straight ahead if it is diggable
   (`penetration_resistance ≤ dig_force: 1.0`, not a creature, plant or
   live seed), keeps it as a spoil pellet, and **lines the burrow**
   (`line_burrow`): every soil cell among the 8 neighbours of the dug cell
   becomes `packedsoil`.

## 6. Moving

### 6a. Step or not

```
p_move = clamp(Move, 0, 1)
if draw < p_move:      hop (Impulse; 0 for the ant), else step_chain
else if draw < 0.5:    tumble                      (0.5 = unwired Tumble)
```

**Stepping and tumbling are the two arms of one `if`**, so an ant turns only
on a tick when it did not step. A tick where the roll fails and the tumble
roll fails does nothing.

### 6b. Where a step goes (`step_chain`)

1. The support check and possible fall (§2).
2. **Three candidates:** ahead-left, ahead, ahead-right (heading + 1, heading,
   heading + 7). **An ant can turn at most 45° per step.**
3. Each candidate's landing is the whole body after the step
   (`body_after_step`). It scores 0 if not enterable, otherwise
   `[max(Turn, 0), Persist, max(−Turn, 0)]` for its position, **plus 0.6 if
   the head would have a foothold**. **No pheromone or home term appears
   anywhere in this score.**
4. Any candidate scoring ≤ 0.3 (footing × 0.5) is zeroed. On flat ground that
   removes both diagonals, so the ant can only go straight: open bug §R4,
   *"it can be scattered, not steered"*.
5. The pick is `choose_weighted(scores, k = 0.1)`: probability ∝ (0.1 + s)².
   With three footed candidates at `Turn` 0, that is 12.7% / 74.6% / 12.7%.
   Its randomness is intended; never replace it with an argmax.
   `cone_picks` counts which candidate each step took. `turn_requests`
   counts steps chosen with a nonzero `Turn`, and `turn_discarded` those
   whose requested side had been zeroed, so the turn could not happen.
6. **The blocked path**, when nothing survives: try a trunk crossing (a woody
   cell ahead puts the ant in a `Crossing` for thickness × interval frames).
   Then a **reversal**, if the ant is **boxed**: refused in all eight
   headings, laden or not (`is_boxed`). The default `ReverseRule::Flip`
   reverses the body in place and turns the heading 180°. A laden ant boxed
   only by other creatures waits instead, for a bounded number of ticks
   (`boxed_by_traffic`, `traffic_deferred`). Then a swap with a
   nestmate (only for `passes_through_kin` species; the ant is not one).
   Otherwise **tumble**. A blocked ant always re-rolls, whatever `Tumble`
   says.
7. **After a successful step:** heading = the chosen candidate. If the new
   head cell is next to nest material, `forage_anchor` = that cell and
   `since_nest = 0` (§8).

### 6c. Tumble (`tumble`)

Re-roll the heading **uniformly among the usable headings** (§2's predicate,
over all 8). **The one exception is the homeward re-roll**
(`home_weighted_pick_why`), which replaces the uniform pick with **the usable
heading whose cosine to `forage_anchor` is highest**: a hard argmax, ties to
the last. It fires only if all of these hold:

- `home_bias > 0`: `ant.ron` authors **1.0**;
- the crop holds food, with fill > 0;
- the ant is at least 1 cell from `forage_anchor`;
- a draw < `home_bias × fill`, taken last, so a gate that fails costs no
  draw.

It aims at `forage_anchor` **even when `PIXEL_PHYSICS_HOME_TARGET=nest`**
moves `HomeAligned`'s target, so under that switch the throttle and the
re-roll aim at different places. `creature_stats.homeward_why` counts every
call by the gate that decided it (its two firing slots sum to
`tumbles_homeward`), and `homeward_aim` counts every firing by where the
chosen heading pointed: toward the anchor, across, or away (cosine above
0.01, between, below −0.01). "Away" is possible because the pick is the best
*usable* heading.

**This re-roll is the only place in the engine where anything aims a walking
ant.** `HomeAligned` only grants or withholds permission to step.

## 7. The trail planes

`pheromone.rs`: two world-sized `u16` planes, A and B, plus an alarm plane.
The channels carry no meaning in the engine; the meaning is in the wiring.

- **Laying** (end of `creature_tick`, **only if the ant moved**): add
  `emit × DEPOSIT` to the head cell. `DEPOSIT` is 40 × 256.
  `PIXEL_PHYSICS_DEPOSIT_AT=vacated` uses the cell just left instead.
  Adding saturates. Laying costs energy.
  - **Trail A:** every ant, laden or empty, at the unit-4 odometer's
    strength: strong just after leaving the nest, fading with time away.
    It works as nest scent.
  - **Trail B:** only laden ants, at a constant 0.714. It works as the food
    trail.
- **Spreading and fading**, every `PHEROMONE_INTERVAL = 12` frames, every
  awake tile: each cell becomes `here + 0.25 × (mean of its 3×3 − here)`,
  then fades by `× (1 − rho)` with a forced minimum drop of 1 raw unit.
  - **A:** `rho = TRAIL_A_RHO = 0.0`, so it fades only by that minimum.
  - **B:** `rho = DECAY_RHO = 0.03`.
  - **Terrain is ignored.** Scent spreads into rock and sky exactly as into
    open air, and leaks between parallel tunnels.
- **Alarm:** spreads by distance falloff (`Spread::ActiveSpace`),
  `ALARM_RHO = 0.35`. It is laid by being bitten, and by displays.

**Who reads what.** Laden ants read A (units 0 and 1); empty ants read B
(units 2 and 3). **Only the forward difference, and only into `Move`.**
Laden ants lay B, and everyone lays A. No other wire or code path acts on
either plane: the other trail inputs are computed and wired to nothing (§3).

## 8. Home: the nest, the anchor, and "at nest"

- **Nest material** is the species' `nest: "nest"` material. Being next to it
  drives `AtNest`, which is the only way the ant knows it is home.
- **`forage_anchor`** is a world coordinate. It is set to the spawn cell at
  birth, and **reset to the new head cell on every step that lands next to
  nest material** (in `step_chain`, not after falls, swaps or reversals).
  So the anchor is the last cell the ant stood on **beside** the nest, and
  leaving a wide nest anchors it at the edge it left from, not at the
  nest's centre.
- It is the homing target for both `HomeAligned` and the homeward re-roll.
  The re-anchoring makes "home" mean *the last spot beside the nest I
  stood on*.
- `since_nest` counts ticks since the last such step. `forage_max` records
  excursion depth, for measurement only.

## 9. The crop, digestion and energy

- **Crop** (`Crop`): one food material, `cells`, a per-cell `unit` worth, and
  `digesting` progress. `worth()` is net of what has already been digested.
  Fill = worth / capacity.
- **Digestion runs every tick the crop holds food, wherever the ant is.** At
  `digest_rate: 3.3` (trait-scaled), times diet quality, minus overhead, it
  pays into energy continuously. **A laden ant eats its cargo while carrying
  it**, so fill falls on the way home and with it the homeward re-roll's
  chance. `digest_hunger_weight: 0.0`: digestion does not wait for hunger.
- **A part-eaten fruit put down comes back whole.** The drop hands over the
  worth left (`unit - digesting`), but only `corpse` stores a worth in the
  cell; any plant food goes down as a plain cell of its material and is
  priced at its full worth when picked up. So putting food down and picking
  it up again creates food (open bug §Z33).
- **Energy costs:**
  - per tick: idle 0.05 × body cells, plus the synapse, sight, curvature,
    force and armour taxes;
  - per step: `move_cost_per_cell` 0.125 × (body + carried cells), **so a
    laden ant pays more per step; it does not step less often**;
  - per dig: 6 × a step;
  - per laying tick: 0.0625 × a step × (emit A + emit B).
- **Death:** starved at energy ≤ 0; old age by half-life. **Budding:**
  `try_bud` above `reproduce_threshold: 1100`.

## 10. Laden versus empty, every difference in one place

| | Laden (crop holds food) | Empty |
|---|---|---|
| Throttle on stepping | trail A's forward difference (units 0–1) | trail B's forward difference (units 2–3) |
| `HomeAligned` → `Move +3.0` | yes | 0 |
| Homeward re-roll when tumbling | yes, at chance `fill` | no, uniform only |
| Reversal when boxed in | yes, but a jam of creatures is waited out first | yes, at once |
| Lays trail B | 0.714 on every step | no |
| Lays trail A | at the odometer's (by then faded) level | at the odometer's level, strongest just out of the nest |
| Digs | never (`act` returns first) | when the dig roll wins |
| Drops | food, ~0.5 at the nest | spoil, if holding it |
| Cost per step | higher (the load) | base |
| Digests | continuously, the cargo | nothing (the crop is empty) |
| **Anything that aims it** | the homeward re-roll | **nothing** |

## 11. Other species sharing this machinery

`ancestor.ron` (the lab ancestor), `hopper`, `longant`, `ant_block`,
`ant_block_shaded`, `ant_long`, `ant_wide` and `chitin_pale` carry the same
hidden-unit layout, with two differences that matter:

- **Their units 2–3 are saturated** (`Bias +45`, `Carrying −75`), so the
  empty ant's trail-B throttle moves `Move` by about 0.003, effectively
  nothing.
- **They lay B on `Carrying`, which includes spoil**, so their diggers lay
  "food trail".

Everything in §2 and §6 is shared engine code, so it applies to every
walking creature.

## 12. Switches that change ant behaviour

Read once per process from the environment. The default is what ships.

| Variable | Default | Effect when set |
|---|---|---|
| `PIXEL_PHYSICS_HOME_TARGET` | anchor | `nest`: `HomeAligned` aims at the nearest nest site's surface; **the re-roll does not follow** |
| `PIXEL_PHYSICS_REVERSE` | flip | `off` / `back` |
| `PIXEL_PHYSICS_STACK_DEPTH` | 1 | >1: nestmates become enterable, up to the cap |
| `PIXEL_PHYSICS_DEPOSIT_AT` | head | `vacated` |
| `PIXEL_PHYSICS_TRAIL_READ` | here-vs-front | `fwd`: trail A read as far-minus-near at 6 and 12 cells |
| `PIXEL_PHYSICS_SENSOR_PROJECT` | on | `off`: no row projection; `none`: also no honesty gate |
| `PIXEL_PHYSICS_A_RHO` | 0.0 | trail A's fade rate |
| `PIXEL_PHYSICS_NEST_REACH` | r1 | `rN`: nest contact within radius N; `body`: any body cell |
| `PIXEL_PHYSICS_LAB_ROOM` | on | `off`: at-nest `Crowding` falls back to local density |
| `PIXEL_PHYSICS_TROPHALLAXIS` | on | `off` |
| `PIXEL_PHYSICS_DROP_REACH` | through bodies | `adjacent`: a food drop looks only at the 8 neighbours |
| `SPOIL_IS_CARGO` | on | `0`: spoil no longer counts toward `Carrying` |
| `PIXEL_PHYSICS_DIG_SPOIL` | kept | `destroy`: dug cells vanish |
| `PIXEL_PHYSICS_BURROW_LINING` | on | `off`: no `packedsoil` lining |
| `CROSS_TRUNK`, `TISSUE_PARTING` | on | `0` |
| `PIXEL_PHYSICS_DIGEST` | continuous | `lump`: pays out per whole cell |
| `PIXEL_PHYSICS_SPOIL_HAUL`, `_DIG_DOWN`, `_SPOIL_DROP_COVER`, `_TRAFFIC_DEFER`, `_COLONY_SPACING` | unset | haulage re-roll to the nest door, downward dig bias, spoil held under cover, jam deferral length, founder spacing |

## 13. Where the implementation lives

- `creature.rs`: `creature_tick`, `sense`, `act`, `step_chain`, `tumble`,
  `usable_headings`, `home_weighted_pick_why`, `trail_sample_point`,
  `adjacent_nest`, `line_burrow`, `food_drop_site`, `choose_weighted`, and the decision trace's
  types (`DecisionRow`, `DecisionOutcome`, `HomewardWhy`, `DropWhy`).
- `brain.rs`: `eval_brain`, the `BrainInput` / `BrainOutput` enums.
- `pheromone.rs`: the planes and the constants in §7.
- `organism.rs`: `OrganismState` (`forage_anchor`, `crop`, `still_ticks`,
  `brain_state`) and `Crop`.
- `assets/species/ant.ron`: every weight and constant quoted here.

## 14. Source comments that currently contradict the code

Delete each line when the comment is fixed. None are known.

## 15. The decision trace, built into the ant

Off unless a harness sets `World::decision_log = Some(Vec::new())`. While it
is on, every walking decision, the move stage of `creature_tick`, pushes one
`DecisionRow`:
- the head position and heading before and after;
- the usable-heading mask, which gives the setting class (0–1 pocket,
  2 corridor, 3–5 junction, 6–8 open);
- the leg as the brain saw it, the crop fill and the anchor;
- the wired inputs, the raw `Move`, `p_move` and `Turn`;
- both rolls;
- the branch taken (`DecisionOutcome`);
- the homeward re-roll's gate (`HomewardWhy`) and, if it fired, the true
  cosine of the chosen heading;
- the drop in `act` that same tick (`DropWhy`), its roll and probability,
  and the head's eight neighbours at the roll: how many were empty
  (`free8`), their materials, and which were the ant's own body or another
  organism; and how far the food went (`drop_reach`: 1 for a neighbour,
  more when handed on through bodies);
- the cone's three scores after the zeroing and the candidate taken.

`CreatureStats::decision_census` counts the same decisions by leg × setting
× outcome, and only while the trace is on, because the setting needs all
eight headings tested. The drop, cone and homeward counters (§5, §6b, §6c)
are always on.

- **It takes no RNG draw and changes no branch.** `act`, `step_chain` and
  `tumble` write a scratch value (`World::decision_scratch`) only while it is
  on. The scratch is reset before `act`, so a row's drop and move are the
  same tick.
- **Guards:**
  - `the_decision_trace_changes_nothing_it_watches` compares a whole bed
    with it on and off;
  - `the_setting_class_reads_the_ground_the_ant_stands_on` checks the
    classifier on three known terrains;
  - `every_traced_decision_agrees_with_the_counters_and_the_positions`
    checks rows against the census, against every counter above and the
    per-verb counters (`moves`, `drops`, `deliveries`, `tumbles_homeward`),
    and against where the head went;
  - four scenes with known answers: `the_homeward_re_roll_aims_along_a_known_floor_at_the_rate_its_fill_sets`,
    `a_drop_with_nowhere_to_go_is_counted_and_one_with_room_is_delivered`,
    `a_blocked_drop_passes_the_food_through_bodies_to_the_nearest_empty_cell`,
    and `the_cone_discards_a_turn_with_nowhere_to_go_and_follows_one_with_somewhere`,
    which feeds `Turn` directly because the shipped ant's is near 0 at
    ordinary temperatures.
- **In the harness:** `trailfollow decisioncsv` writes the rows and repeats
  those reconciliations at the end of every run; `decisionnorows` keeps only
  the census. `scripts/decisioncensus.py` reads the rows, including the drop
  and cone columns.

