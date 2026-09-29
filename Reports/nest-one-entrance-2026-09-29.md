# One entrance round the door (2026-09-29)

**Status: measurement and switches, all off; the direction is the owner's
call (§7).** The nest lane's live question after the granary shipped
(#513): with a five-column door instead of the old 53-column strip of nest
paint, a colony in `digbox` opens 6-7 entrances where it used to open 2.
This report finds why, builds three ways to fix it behind switches, and
measures each at 40 and 200 ants. None of the three gives one entrance
*and* a nest that grows with the colony; the choice between them is a
trade-off the owner has to see.

Lane note: [lanes/nest-mouth.md](lanes/nest-mouth.md). Predictions 168-191
were written before their runs; they are in §8 with their scores.

## 0. What was found, in world terms

- **The storeroom adds no entrances; the door is all of it.** Once founders
  are homed at the door, as founding homes them, the door alone opens 6
  entrances, the whole granary 6.5, and each storeroom part alone 5-6 (§1).
- **The roof breaks under the colony's own spoil heaps.** A pellet dug in a
  gallery is posted straight up its carrier's column (the lift), so every
  gallery grows a heap on the ground right over it. The heap cue, which lets
  ground be opened only beside a heap, reads ground above a cell as a roof,
  so it never judges a cut into the crust *under* a heap. Of 444 times the
  crust over a door colony's nest broke, **314 were cuts into ground with a
  heap on it** and only 48 were cuts into open sky, the one kind the cue
  sees (§2). Under the old strip the heaps sat on paint nobody could dig.
- **Carrying the soil out through the door, as the ants' own cycle does,
  gives one entrance** -- on 24 of 24 seeds fewer than today, the dug space
  roofed, and 19 of 24 colonies more nest-like than random digging against 0
  today -- **but a nest a fifth the size**, because a digger that
  walks its pellet out leaves its face and, traced, most never come back
  (§3). Walking them back to the face makes the nest scale with the colony
  (51 cells at 40 ants, 304 at 200) and keeps 2 entrances at 40 ants, but
  at 200 the entrances return: 7, against today's 9.5 (§4).
- **Sending the lift out through the passages instead of up the column**
  keeps the digger at its face and digs more than today, but puts each
  pellet out of whichever hole is nearest, so every hole grows a heap: 5
  entrances at 40 ants, 12 at 200 (§5).

| digbox, 24 seeds, frame 24,000 (medians) | shipped | walk out | walk out and back | lift out |
|---|---:|---:|---:|---:|
| 40 ants: entrances | 6.5 | **1** | **2** | 5 |
| 40 ants: cells dug | 204 | 41 | 51 | 286 |
| 40 ants: dug space roofed | 85% | 100% | 93% | 90% |
| 40 ants: more nest-like than random walkers | 0 of 24 | 19 of 24 | 12 of 24 | 0 of 24 |
| 200 ants: entrances | 9.5 | **3** | 7 | 12 |
| 200 ants: cells dug | 618 | 74 | 304 | 764 |

"Walk out" is the haul, the laden pace and no drop under cover
(`SPOIL_HAUL=1.0`, the first build's pace switch and `SPOIL_DROP_COVER=0`;
`hpc0` in the logs); its consolidated form, `PIXEL_PHYSICS_SPOIL_OUT=haul,
pace,keep`, also counts the founding cut as inside and was not re-run. "Walk
out and back" is `SPOIL_OUT=on`; "lift out" is `SPOIL_LIFT=out`. Every arm
is the shipped default otherwise, and every switch unset reproduced the run
before it byte for byte (§6).

## 1. The storeroom split: the door is the whole of it

`digbox` now founds as the game does (`0415980a`): founders are homed at the
door's anchor, and a new per-cut ledger books whether a nest worker made
each cut. 40 ants, `energy=1000`, 24 seeds, frame 24,000:

| arm | entrances | cells dug | nest-like (of 24) |
|---|---:|---:|---:|
| old strip, no storeroom | 2 | 57 | 21 |
| door alone | 6 | 206 | 0 |
| door + nest workers only (`caste=4,workerhome`) | 6 | 222 | 0 |
| door + side room only (`side`) | 5 | 193 | 0 |
| door + storeroom without the side room | 6 | 222 | 0 |
| new default (door + whole storeroom) | 6.5 | 204 | 0 |
| storeroom alone, strip kept | 6 | 114 | 12 |

Paired against the door alone no part clears a sign test (nest workers:
more entrances on 12 seeds, fewer on 6). Nest workers open their share of
the new holes from the surface and no more (17-23% of them, a quarter of the
ants), and make 33-39% of the cuts below the old surface, since the cut is
their home. The two effects do not add: under the old strip the storeroom
alone still takes entrances from 2 to 6 (how is not traced), but on top of
the door it adds none (6 -> 6.5). #513's body split the extra entrances
half and half between door and storeroom on the assumption that they add;
they open the crust together no more than the door does alone.

## 2. The opening ledger: heaps over the galleries

`digbox`'s new `OPENINGS` lines book every cell of the old surface row that
goes from ground to open, by cause, with the cause of each column's *first*
opening kept apart from the grains that later fall through an open hole. Door
alone, 24 seeds, frame 24,000, summed:

| what broke the crust | first breaks | holes standing at the end, by first break |
|---|---:|---:|
| a cut from above into ground with a heap on it | 206 | 121 |
| a cut from level or below into ground with a heap on it | 108 | 87 |
| a cut from above into open sky | 47 | 21 |
| a cut from level or below into open sky | 1 | 1 |
| no cut: unlined soil fell away | 82 | 33 |

The heap cue ([`spoil_cue_factor`] in `creature.rs`) judges a cut only when
the cell cut is the top of the ground in its column; a heap on it makes it
not the top, and an enclosed digger cutting a covered cell is left alone by
design (a tunnel's digger must not meet the cue). Heaps also make the ground
between them curve in, and the dig wiring reads curving-in ground as
enclosed, so ants standing between heaps dig. The old strip hid all of it:
its heaps sat on nest paint, and paint cannot be cut.

## 3. Walk it out

The excavation reference's cycle is grab, climb out, carry out, drop; the
shipped ant walks the grab and abstracts the rest into the lift. Walked
(`SPOIL_HAUL=1.0` pulls a carrier to its nest's door; `SPOIL_DROP_COVER=0`
takes no drop under cover; the new pace part walks it at the laden pace),
40 ants: 1 entrance, fewer than shipped on 24 of 24 seeds, the dug space
roofed (median 100%, more of it roofed than shipped on 23 of 24); exactly one
entrance on 16 of 24. At 200 ants: 3 entrances, the
two extra 3-4 columns from the door beside its paint; holes more than 8
columns out, 7 summed over 24 seeds against 141 shipped.

**It digs a fifth as much and does not scale** (41 cells at 40 ants, 74 at
200). Removing the lift instead (`SPOIL_LIFT=none`, with or without the
haul) digs as little and puts 56-70% of pellets back below the old surface
wherever headroom appears, the "first empty neighbour" failure `dead-ends.md`
already records.

**Traced (the new `TIME` lines, seed 3):** the colony cuts at the same rate
per ant-frame underground either way (3.3 and 3.8 per thousand); the walked
arm spends 33,000 ant-frames underground against 136,000, and 19 of the 40
ants that put a pellet down in the open never went back under. The lift had
hidden the trip: a pellet posted up the column leaves its digger at the
face. Hunger's scouting is not the cause: with it off (`SCOUT=0`) the walked
arm digs 33 cells over 8 seeds (41 with it on, 24 seeds).

## 4. Walk it out and back

`PIXEL_PHYSICS_SPOIL_OUT` gathers the cycle into one switch with parts:
`haul` (the pull to the door), `pace` (the laden pace), `keep` (nothing put
down inside the nest until the haul's patience runs out), `back` (a digger
that is not hungry walks back to the cell it cut: to the door while outside,
then to the face). "Inside" is ground overhead **or the founding cut**: the
shaft is a hole open to the sky, and a first build keyed on cover alone let
carriers drop pellets down the shaft and pulled returning ants back up out
of it. The first build also had a "fed" gate of energy at or above
`start_energy`, which never holds in the food-free box, so its run was byte
for byte the run without it; the gate is half of `start_energy` now
(`DIG_RETURN_FED`).

`SPOIL_OUT=on`, 24 seeds: at 40 ants 2 entrances (fewer than shipped on 23
of 24; 24 of its 36 extra holes are 3-4 columns from the door), 51 cells
dug, roofed 93%, 12 of 24 nest-like; **at 200 ants 304 cells dug** -- six
times the 40-ant nest, where walking out alone reached 74 -- **but 7
entrances** (fewer than shipped on 19 of 24). The ant-time underground is
still well under the lift's (seed 3: 51,000 against 136,000), and most of
the colony's time in every arm is spent on the surface beyond the door.

## 5. Lift it out through the passages

`PIXEL_PHYSICS_SPOIL_LIFT=out` keeps the lift but sends it out the way the
ant came in: breadth first through open passage from the carrier's head to
the nearest cell in the open that would hold the pellet, the column lift
only if none lies within 4,096 cells ([`lift_out`]). The digger stays at its
face: 286 cells dug at 40 ants against 204, and on seed 3 twice the time
underground. But pellets leave by whichever hole is nearest, so every early
hole grows its own heap and the heap cue widens it: crust breaks under a
heap 386 against 322, 5 entrances at 40 ants and **12 at 200** (more than
shipped on 17 of 24). Pictured, the nest is large and galleried under a long
ridge of spoil.

## 6. What was built, and its proofs

All in `creature.rs` unless named, all off unless set:
- `PIXEL_PHYSICS_SPOIL_OUT` = `SpoilOut { haul, pace, keep, back }`,
  `inside_nest`, `dig_return_target`, `OrganismState::dig_return`,
  `CreatureStats::spoil_kept_inside`. `spoil_haul` reads the `haul` part as
  `1.0` when `PIXEL_PHYSICS_SPOIL_HAUL` is unset.
- `PIXEL_PHYSICS_SPOIL_LIFT=out` = `SpoilLift::Out`, `lift_out`,
  `CreatureStats::spoil_lifted_out`.
- `examples/digbox.rs`: founders homed at the door; the `OPENINGS` ledger;
  the `TIME` budget; pellets carried out booked apart; which caste made each
  cut.

Unset, each build reproduced the run before it byte for byte in `digbox`
(seeds 1-3, against the binary before the change); the colony bed and the
lab were not run. No test covers the new switches yet; they are candidates,
not ship arms.

## 7. The choice

Every form trades entrances against size:
- **Walk it out** reads most like a nest with one way in, and stays small.
- **Walk it out and back** grows with the colony, and at 200 ants opens 7
  holes.
- **Lift it out** grows most, with a spoil ridge and as many holes as today.

A fourth form was not built: the lift carrying the pellet out *to the door*
rather than to the nearest hole. That is a rule about where the colony's
spoil lands, which the owner ruled against on 2026-08-31 ("is this a problem
for you to solve or for the ants to solve"; `dead-ends.md`, the two
placement rules), so it waits on the owner.

## 8. Predictions (written before each run) and scores

| # | arm | prediction | score |
|---|---|---|---|
| 168 | digbox 40, new default, founders homed at the door | entrances within ±1 of the unhomed 7 | holds (6.5) |
| 169 | door alone, homed | entrances at least 4 | holds (6) |
| 170 | new default, homed | nest workers make at least 40% of new-mouth cuts | fails (17% from the surface, 35% from below) |
| 171 | door + nest workers only | entrances at least 1 above the door alone's 6 | fails (6) |
| 172 | door + side room only | entrances within ±1 of 6 | holds (5) |
| 173 | `SPOIL_HAUL=1.0 SPOIL_LIFT=none` | entrances at most 4 | holds (2) |
| 174 | the same | cells dug below the base's and at least 100 | fails (40 new, 26 door) |
| 175 | the same | crust breaks under a heap at most half the base's | holds, trivially (322 -> 16) |
| 176 | `SPOIL_LIFT=none` alone | cells dug below 100 | holds (40 / 28) |
| 177 | `SPOIL_HAUL=1.0` alone | entrances within ±1 of the base's | split (new 5.5: holds; door 4: fails) |
| 178 | `SPOIL_DROP_COVER=0` | entrances at most 3; dug below the default's | holds (1; 37) |
| 179 | + haul | entrances at most 3; dug at least 178's | holds (1; 40) |
| 180 | both | pellets below the old surface at most a quarter of the default's | fails (51% / 33% against 31%) |
| 181 | haul + pace + no drop underground, 200 ants | entrances at most 3 | holds (3) |
| 182 | the same | cells dug at least 123 | fails (74) |
| 183 | no drop underground alone, 200 ants | dug below the haul arm's | holds, barely (70 against 74) |
| 184 | `SPOIL_LIFT=out`, 40 ants | entrances at most 3 | fails (5) |
| 185 | the same | cells dug at least 60% of 204 | holds (286) |
| 186 | the same | crust breaks under a heap at most half of 322 | fails (386) |
| 187 | the same, 200 ants | entrances at most 4; dug at least 60% of 618 | fails (12; 764) |
| 188 | walk out, scouting off | cells dug at least twice 41 | fails (33) |
| 189 | walk out and back, scouting off | dug at least the walk-out's; entrances at most 2 | holds, trivially (34; 1) |
| 190 | `SPOIL_OUT=on`, 40 ants | entrances at most 2; dug at least 60 | split (2; 51) |
| 191 | the same, 200 ants | entrances at most 3; dug at least twice its 40-ant median | split (7; 304) |

188-189 were first written for `energy=20000`, digbox's own default; eight
runs had started when it was noticed that at that energy every ant buds past
800 (against the owner's not-too-many-ants ruling). They were stopped before
any finished or was read, deleted, and the arm changed to scouting off.

## 9. Instruments

- `digbox`'s `OPENINGS` and `TIME` lines (every stop, with the funnel).
- The tallies are scratch scripts (`openings.py`, `caste.py`); `nestscore.py`
  reads the scoreboard and funnel as before, and its pellet line is
  unchanged (pellets carried out count as lifted, with their own line).
