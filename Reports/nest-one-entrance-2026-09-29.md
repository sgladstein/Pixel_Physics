# One entrance round the door (2026-09-29)

**Status: measurement and switches, all off; the owner picked the walked
cycle (§7), traced and fixed in §10; what holds it at 200 ants traced in
§11.** The nest lane's live question after the granary shipped
(#513): with a five-column door instead of the old 53-column strip of nest
paint, a colony in `digbox` opens 6-7 entrances where it used to open 2.
This report finds why, builds three ways to fix it behind switches, and
measures each at 40 and 200 ants. None of the three gives one entrance
*and* a nest that grows with the colony; the choice between them is a
trade-off the owner has to see.

**Later the same day** the owner picked "walk it out, then back to the
dig". §10 traces that cycle and fixes three faults in it; **its one claim
here that did not survive is corrected in §0 and §4**: what made the first
build's nest grow with the colony was a carrier giving up the walk and
posting its pellet up through the roof, not the trip back.

**§11 corrects §10's reading of the 200-ant limit**: the walked carriers are
not queueing in the shaft, they are shut in by their own spoil on the mouth.
**§12 carries the pellet away from the mouth** (`PIXEL_PHYSICS_SPOIL_RING`,
off): at 40 ants the walked nest doubles with its door clear; at 200 ants the
colony's own idle ants, homed over the mouth, still stand on the way out.

Lane note: [lanes/nest-mouth.md](lanes/nest-mouth.md). Predictions 168-206
and 215-223 were written before their runs; they are in §8 with their
scores.

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
  (§3). Walking them back to the face keeps 2 entrances at 40 ants, and the
  first build's nest grew with the colony (51 cells at 40 ants, 304 at
  200) with 7 entrances at 200 -- **but that growth, and those entrances,
  were a carrier giving up the walk and posting its pellet up through the
  roof** (§10). Without that, the walked cycle keeps 1.5 entrances at 40
  ants and 3 at 200, and its nest barely grows with the colony: 43 cells
  and 70.
- **At 200 ants the walked colony buries its own door** (§11). A walked
  carrier puts its pellet down on the first ground outside, the mouth's
  rim, and a mound grows over the mouth; at the shaft's top cell 53% of a
  carrier's decisions have no way up. §10 read the carriers' standing time
  as a queue in the shaft. It is not one: they step on 78% of their
  decisions and wander the storeroom and galleries, as often down as up.
  So a relay through nest workers would bring the same pellets to the same
  rim.
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

**Corrected (§10): the trip back is not what made this nest grow.** When a
carrier's patience ran out inside the nest, `keep` let it go as it always
had, and with no cell beside it the pellet went up the column: at 200 ants
81% of all pellets left that way, from all over the nest. Taken out, the
200-ant nest falls from 296 cells to 62 on the merged tree.

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
- Later (§10): `keep` never lifts from inside the nest
  (`CreatureStats::spoil_kept_no_lift`); the haul's target
  (`spoil_haul_target`, the door a row above the mouth under `SPOIL_OUT`);
  the laden pace's bearing (`spoil_pace_target`). `digbox` gains `LIFTS`,
  `TRIPS`, `tripcsv=`, `decisions=` and the site line.
- `examples/digbox.rs`: founders homed at the door; the `OPENINGS` ledger;
  the `TIME` budget; pellets carried out booked apart; which caste made each
  cut.

Unset, each build reproduced the run before it byte for byte in `digbox`
(seeds 1-3, against the binary before the change), and the finished branch
reproduces the colony bed (`trailfollow`, 20 founders at 90 cells, seeds 1-2)
byte for byte against the pre-change binary; with `SPOIL_OUT=on` the same
bed run differs, so the identical one is evidence. The lab was not run. The
full suite passes on the branch (library 1,930, 0 failed; `tests/
determinism.rs` 4; `tests/worldgen.rs` 44); no test covers the new switches
yet: they are candidates, not ship arms.

## 7. The choice

**The owner picked walk it out and back** ("I lean to 'walk it out, then
back to the dig'", 2026-09-29), the form real colonies use. §10 is what
that turned up. The rest of this section is the choice as it was put.

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
| 192 | give-up fix, 200 ants | lifts from inside at most 2% of the first build's | holds (21 against 17,110) |
| 193 | the same | entrances at most 4, fewer on 16 of 24 | holds (3; 23 of 24) |
| 194 | the same | cells dug at least half the first build's | fails (62.5 against 296) |
| 195 | the same, 40 ants | entrances at most 2; dug within 25% | holds (2; 41.5 against 50.5) |
| 196 | walk out (`haul,pace,keep`), 200 ants | entrances at most 3 | holds (3) |
| 197 | shipped, merged tree | entrances within 1 of 6.5 / 9.5 | holds (7 / 9.5) |
| 198 | door target alone, 200 ants | carrying frames in the cut at most 0.8x | fails (43.2% against 43.3%) |
| 199 | door + pace, 40 ants | trip at most 0.7x; dug at least the fix's | split (0.703; 43 against 41.5) |
| 200 | the same, 200 ants | dug 1.0-1.5x the fix's; entrances at most 4 | holds (1.11x; 3) |
| 201 | the same, 40 ants | entrances at most 2 | holds (1.5) |
| 202 | bed, branch unset against main, 20 founders | byte for byte | holds |
| 203 | bed, walked cycle, 20 founders | starved at most +15%; born at least 85% | split (37 against 73; 160 against 223) |
| 204 | the same, 80 founders | starved at most +25% | holds (1,097 against 1,102) |
| 205 | lab, walked cycle against off | births within 25% per seed | holds, barely (0.756) |
| 206 | the same, labshot's census at frame 119,700 | the mouth open on at least as many seeds | fails (1 of 12 against 6) |
| 215 | walked, 200 ants, frames 0-6,000, 6 seeds | a carrier standing facing an animal faces another carrier at least half the time | holds (59%, 44-65 by seed) |
| 216 | the same | nest workers with nothing held at least a quarter of the heads in the shaft | fails (8-17%) |
| 217 | walked, 40 ants | the shaft holds under 2 heads on average | holds (1.29) |
| 218 | walked + carry 2,2, 40 ants, 24 seeds | entrances at most 2 | holds (2) |
| 219 | the same, frame 6,000 | dug space at least 1.3x the walked cycle's | holds (53 against 35; per seed 1.45) |
| 220 | the same, 200 ants, frame 6,000 | dug space at least 2x the walked cycle's | fails (34 against 39; 0.79) |
| 221 | the same, 200 ants, seed 1 | no way up at the shaft's top on at most half the walked cycle's 53% | fails (56%) |
| 222 | the same, both sizes | spoil within 2 columns of the door at most half the walked cycle's | holds (0.03 against 0.77 a column; 0.02 against 0.78) |
| 223 | carry 3,2 against 2,2 | its heap peaks at least 2 columns further out | fails (both peak 6-9 out; 3,2 has more past 10) |

188-189 were first written for `energy=20000`, digbox's own default; eight
runs had started when it was noticed that at that energy every ant buds past
800 (against the owner's not-too-many-ants ruling). They were stopped before
any finished or was read, deleted, and the arm changed to scouting off.

## 9. Instruments

- `digbox`'s `OPENINGS` and `TIME` lines (every stop, with the funnel).
- `digbox`'s `LIFTS` line: where each pellet posted up the column left
  from (the founding cut, depth under cover, or the open), and the drop
  rolls `keep` held.
- `digbox`'s `TRIPS` line (§10): every pellet followed from the cut to where
  it went down -- frames a trip, frames to leave the nest, whether the head
  moved each frame and what stood in front of it, patience under the
  give-up line, and where the carrying frames were spent. It counts
  frames; an ant decides once in six, so its standing shares are mostly the
  frames between decisions. `tripcsv=PATH` writes it per carrier per frame.
- `digbox decisions=PATH`: the engine's own `DecisionRow` for every decision
  of an animal holding a pellet -- usable headings, `P(move)` and the roll,
  the outcome, patience, `HomeAligned`, `AtNest`. This is what found both
  walk faults in §10; the `TRIPS` shares alone did not.
- `digbox`'s site line: the nest site, its cut, and whether the haul's
  target lies inside the cut.
- `digbox`'s `JAM` and `JAMAT` lines (§11): when a carrier stood facing an
  animal, what that animal was doing (carrying a pellet, a store load, a
  nest worker with nothing held, anything else) and whether it stood too;
  the mean heads in the shaft and the rest of the cut by the same roles; and
  every standing frame by where the carrier was (at the mouth, lower in the
  shaft, in the chamber or side room, outside the cut) and what it faced.
  Frames, like `TRIPS`, so read it for where, not for how often. The `JAM`
  line also counts the heads on the two rows over the mouth, by role (§12).
- `digbox`'s `CRATER` line (§12, every stop): ground standing above the old
  surface in bins of columns from the nest's centre (0-2 is the door), what
  stands over the mouth's own columns, and `SPOIL_RING`'s counters (carry
  distances drawn, drop rolls held short of them).
- The tallies are scratch scripts (`openings.py`, `caste.py`, `arms.py`);
  `nestscore.py` reads the scoreboard and funnel as before, and its pellet
  line is unchanged (pellets carried out count as lifted, with their own
  line).

## 10. Walk it out and back, traced (later on 2026-09-29)

The owner picked the walked cycle. Its first build had three faults, all
in `SPOIL_OUT`'s own parts, each found by following individual carriers.
All numbers are `digbox`, 24 seeds, on the tree with main's #515 merged in
(the shipped arm there: 7 entrances and 207.5 cells at 40 ants, 9.5 and
613.5 at 200; the first build 2 / 50.5 and 6.5 / 296).

**1. A carrier that gave up the walk sent its pellet up through the roof.**
`keep` held the pellet inside the nest while the haul's patience lasted,
then let the drop roll through as before, and with no cell beside the
carrier that meant the column lift. The `LIFTS` line put the starting
points all over the nest at 200 ants (per seed: 150 in the founding cut,
165-293 one to four rows down, 59-244 at nine to sixteen), 81% of all
pellets. Now `keep` never lifts from inside the nest: a carrier that ran
out of patience there lays its pellet beside itself where a cell holds it,
or keeps carrying. At 200 ants pellets lifted from inside fell from 17,110
to 21 and entrances from 6.5 to 3 (fewer on 23 of 24 seeds), and the nest
from 296 cells to 62.5 (per seed 0.22). At 40 ants: 2 entrances, 50.5 ->
41.5 cells.

**2. The haul aimed inside the nest.** It pulled a carrier to
`(site.x, site.surface)`, which over a founding cut is the mouth's own row
-- inside the cut, where `keep` holds the pellet. A carrier climbed the
shaft in a handful of steps, reached the target, lost its pull (standing on
a target is not a direction) and milled in the mouth. It now aims at the
door every ant is homed to, a row above the mouth (`spoil_haul_target`).

**3. At the mouth the carrier lost its hurry.** The laden pace reads
`HomeAligned` against the forage anchor, which every contact with the nest
re-sets to where the ant stands, so in the mouth it read 0: `P(move)` 0.36-
0.46 in the mouth's two rows and at the door, against 0.78 a row deeper,
and a step on 30-45% of decisions against 75-86% (40 ants, seed 1). One
carrier: six steps up the shaft, then forty decisions at `P(move)` 0.15 in
the mouth. The pace now reads the bearing to where the carrier is going,
the door for a pellet and the face for a digger walking back
(`spoil_pace_target`); in the mouth `P(move)` is 0.78.

| digbox, 24 seeds, medians | shipped | first build | + 1 | + 1, 2, 3 |
|---|---:|---:|---:|---:|
| 40 ants, frame 24,000: entrances | 7 | 2 | 2 | **1.5** |
| 40 ants, frame 24,000: cells dug | 207.5 | 50.5 | 41.5 | 43 |
| 40 ants, frame 6,000 (fed): cells dug | 73.5 | 32 | 26.5 | 35 |
| 40 ants: frames a trip, cut to put-down | -- | -- | 111 | 78 |
| 200 ants, frame 24,000: entrances | 9.5 | 6.5 | 3 | **3** |
| 200 ants, frame 24,000: cells dug | 613.5 | 296 | 62.5 | 69.5 |
| 200 ants: frames a trip | -- | -- | 474 | 384 |

**What is left is the shaft.** At 200 ants carriers still spend 40-72% of
their carrying frames in the founding cut, standing behind one another in
a shaft two cells wide, and the cut rate per ant-frame under cover falls to
a third of the shipped ant's (0.9 against 2.4 per thousand at frame 6,000).
At 40 ants it is the shipped rate (3.4 against 3.8); the walked colony is
simply under cover less. So the walk gives one mouth, and a nest that
does not grow with the colony through one shaft. The lining does not fix
the width: tamped soil resists at 0.95 against the ant's dig force of 1.0,
so a crowded shaft can be cut wider, and is not.

**The box starves late.** `digbox` has no food: by frame 9,000 the median
ant is under the half of `start_energy` the trip back asks for, so late in
the run a walked digger that reaches the surface stays there (two thirds
of the colony's ant-frames are on the surface beyond the door). Frame 6,000
is the fair comparison of the walk against the lift.

**On the colony bed** (`trailfollow`, gap 90 at 20 founders and 135 at 80,
24 seeds; the branch unset reproduces main byte for byte): at 20 founders
the walked cycle starved 37 against 73 (fewer on 14 seeds, more on 3) and
bore 160 against 223 (per seed median 6 against 8; more on 11, fewer on 12;
one main seed bore 40). At 80 founders it starved 1,097 against 1,102 and
bore 40 against 70 (more on 7, fewer on 12). Fewer births, no more
starvation. The owner's ruling that colony numbers do not veto a nest step
stands; the cost is recorded.

**In the lab box** (`labforage` and `labshot`, `scenario=played_bed`, 12
seeds, 120,000 frames, the tree with main's #516 merged in; seeds 6-12
were re-run after a container restart killed them, same binaries): births
per seed 0.756 of today's (median 439 against 619; more on 5 seeds, fewer
on 7), deaths 1.04, food eaten 0.96 -- inside the spread of twelve seeds.
The nest is not: the walked colony digs a shallow layer under ground it
leaves whole, where today's opens a wide pit with spoil spires over it,
and **the founding mouth is under the colony's own loose soil**. Open to
the surface at the last stop on 1 of 12 walked seeds against 6 of 12
today, and the cut holds 115 cells of loose soil against 44 (more on 9
seeds, fewer on 2). A walked carrier puts its pellet down on the first
ground outside the cut, which is the rim of the mouth, and the pellet
turns loose and runs back in. Today's "open" is often part of the pit,
not a mouth.

All three are off unless `PIXEL_PHYSICS_SPOIL_OUT` is set;
`PIXEL_PHYSICS_SPOIL_HAUL` alone keeps its old target. Unset, every build
reproduced the run before it line for line in `digbox` (seed 1, 40 and 200
ants) and byte for byte on the bed.

## 11. What holds the walked carriers at 200 ants: their own spoil on the mouth

*Later on 2026-09-29, before building either answer on card `…2d6747`
(nest workers relay the soil up, or more mouths beside the door).
`digbox` with the `JAM` and `JAMAT` lines (§9), `SPOIL_OUT=on`, 200 ants,
energy 1,000, frames 0-6,000 while the box is fed, seeds 1-6; the lift
beside it; seed 1's carriers traced decision by decision with
`decisions=`.*

**Not a queue in the shaft.** §10 read the carriers' standing time in the
founding cut as a queue in its two-wide shaft, and the card asked on that
reading. Traced, the reading was wrong:

- **Where they stand.** Of a carrier's standing frames, 12-16% are at the
  mouth (its rim and top rows), 5-9% lower in the shaft, 37-47% in the
  chamber and the side room, and 30-41% in the galleries. The ants a stood
  carrier faces are other carriers (median 59%), ants with nothing held
  (34%) and nest workers (6%); the shaft holds 3.5 heads on average.
- **They are not stuck, they are lost.** Seed 1, 10,410 carrier decisions:
  they step on 78% of them. 77% of the decisions are in the side room and
  the galleries off it (58% in the side room's three rows alone), where
  they go up 2,362 times and down 2,262, and the step taken points at the
  door no better than chance (mean cosine 0.08; 0.12 lower in the shaft,
  -0.03 at the mouth).
- **What shuts them in is the mouth.** At the shaft's top cell, 53% of a
  carrier's decisions have no way up at all (north, north-east and
  north-west all blocked); one row down, 62%. Ant 20 took its pellet in the
  chamber at frame 847, was at the top of the shaft by 1,033, shuttled
  between the top rows until 1,123 because the only way on was back,
  wandered back down past the passage, fell into the chamber at 1,249, and
  was still holding at 1,261.
- **What covers the mouth is the colony's own spoil.** A walked carrier
  puts its pellet down on the first ground outside the cut, which is the
  mouth's rim (§3; §10's buried mouth in the lab box is the same thing
  over 120,000 frames). At 200 ants a mound of spoil and ants covers the
  mouth by frame 2,000; at 40 ants a heap sits on it by frame 4,000
  (card `…04be5b`, seed 1).

**So a relay would not help.** It brings the same pellets up the same shaft
to the same rim, and the nest workers who would carry them are 6% of what
stands in a carrier's way. More mouths beside the door would each grow the
same mound. **The lever is where a pellet goes down.** Harvester ants carry
soil some way from the entrance and leave a crater ring round an open mouth
(the drop-distance `p(r)` in `nest-entrance-dimensions-2026-09-19.md` §1).
Here that would be the ant's own walk: a carrier outside the nest keeps
walking away from the mouth for a distance it draws before it may put the
pellet down, as it carries food home before it may drop it. That is the ant
carrying, not a rule placing soil, but it is close to the owner's ruling of
2026-08-31 on where spoil goes, so it is put to the owner on card
`…04be5b` before it is built. Dead end: the relay, sized by this trace and
not built.

## 12. The pellet carried away from the mouth (`PIXEL_PHYSICS_SPOIL_RING`, off)

*Built the same evening, while card `…04be5b` asks the owner whether it may
be; off unless set. `digbox`, 24 seeds, 40 and 200 ants, energy 1,000,
frames 6,000 and 24,000, four arms on one binary: today's lift, the walked
cycle (`SPOIL_OUT=on`), and the walked cycle with the carry in each of its
two shapes.*

**What it is.** The first time a carrier stands outside the nest with its
pellet, it draws how far to take it: the door's half-width plus one (so
nothing is set on the door) plus a Gamma(`shape`, `scale`) draw, on the side
it came out of. Until its head is that many columns from the nest's centre,
its drop roll is held, as `keep` holds it inside; past it, the roll and the
cell predicate are the ant's own, as before. It changes when the ant lets go,
not where a pellet may lie, which is the line the owner's 2026-08-31 ruling
draws (the spoil drop's comment in `act`). Shape 2 is the section through a
mound, `p(r)/r` for a shape-3 walk, and shape 3 the flat form
(`nest-entrance-dimensions-2026-09-19.md` §3); scale 2 cells.

| `digbox`, 24 seeds, medians | lift | walked | walked + carry 2,2 | + carry 3,2 |
|---|---:|---:|---:|---:|
| 40 ants, frame 6,000: dug space | 75 | 35 | 53 (more on 22) | 53.5 |
| 40 ants, frame 24,000: dug space | 260 | 42.5 | **84** (more on 24) | 87 |
| 40 ants: entrances | 6 | 2 | 2 | 2 |
| 40 ants: ground on the door, a column | 0.13 | 0.77 | **0.03** | 0.07 |
| 200 ants, frame 6,000: dug space | 203.5 | 39 | 34 (less on 18) | 35 |
| 200 ants, frame 24,000: dug space | 583.5 | 69.5 | 91 (more on 23) | 95 |
| 200 ants: entrances | 9.5 | 3 | 2 | 2 |

"More on" and "less on" are seeds against the walked cycle.

**At 40 ants it does what it was built for.** The door stays clear and the
spoil stands in a ring: 0.03 cells a column on the door, then 2.1, 3.5 and
2.0 at 3-5, 6-9 and 10-14 columns out, against the walked cycle's heap on the
mouth (0.77 and 0.95 on the door and just beside it). The nest doubles, and
not by moving more soil: the carry lets go of fewer pellets (147 against 169
by frame 24,000), but a pellet set on the rim slides back into the tunnels
it came from, and one carried away stays out. In pictures, the door is clear
at every stop and an open room lies under it (card `…04be5b`).

**At 200 ants the door is clear of spoil and still shut.** Seed 1: the
shaft's top cell has no way up on 56% of carrier decisions with the carry,
53% without. What stands there is the colony: about three heads on the two
rows over the mouth at any moment (0.5-1.3 nest workers and 1.8-2.7 other
ants with nothing held, seeds 1-3, with the carry or without), against about
one at 40 ants. Every founder's
home is the cell over the mouth's middle (`World::door_anchor`), and in a
box with no food an ant with nothing to do goes home. The carry makes it
worse early (a carrier now walks out through that crowd and along the ground
beyond it: 216 frames to leave the nest against 156) and better late.

**So the next lever is where the colony stands, not where the soil goes.**
Real workers rest inside the nest, not on its entrance. Two switches already
move home off the mouth, and both cost the colony bed when they were built:
the mouth cut beside the door (`PIXEL_PHYSICS_NEST_SHAFT_OFFSET`; births
150 -> 105, `nest-granary-2026-09-28.md` §8h, not traced) and part of the cut
as home (`PIXEL_PHYSICS_NEST_HOME`; `nest-mouth-2026-09-26.md`). Home is the
foraging lane's walk as much as this lane's founding, so it is put to the
owner and that lane before it is built.

**Shipped on, 2026-09-29** (owner: "Q1 - Yes", §14): the carry defaults to
`2,2` and acts **only while the walked cycle is on** -- `spoil_ring_of`
reads it as absent with `PIXEL_PHYSICS_SPOIL_OUT` unset, because the drop's
hold reads the carry for any carrier outside the nest and the shipped lift
must not change under it. Guarded by
`the_shipped_carry_is_inert_without_the_walked_cycle` (watched red with the
gate removed).

## 13. Crowding: stacking, and tunnels two cells wide (owner, 2026-09-29)

*The owner, mid-session: "For crowding issues, have you looked at the
stacking feature (PIXEL_PHYSICS_STACK_DEPTH). Also tunnels should be wider
than 1 pixel, for crowding and aesthetics." `digbox`, 24 seeds, 40 and 200
ants, frame 24,000 unless stated.*

**Stacking** (`PIXEL_PHYSICS_STACK_DEPTH=4`, as the colony bed runs it; the
game ships 1, where a nestmate is as solid as rock). No `digbox` run in this
report had it on.

| dug space (entrances), 24 seeds | depth 1 | depth 4 |
|---|---:|---:|
| 200 ants, walked | 69.5 (3) | **181** (5), more on 24 |
| 200 ants, walked + carry | 91 (2) | **181.5** (4), more on 24 |
| 200 ants, today's lift | 583.5 (9.5) | 1,265 (16), more on 24 |
| 40 ants, walked | 42.5 (2) | 69.5 (2), more on 18 |
| 40 ants, walked + carry | 84 (2) | 91 (2), more on 20 |

At 200 ants by frame 6,000, 164 pellets are out against 24 walked, and 90
against 10 with the carry: stacked ants no longer shut the mouth. Entrances
rise with the digging. Predictions 224 holds, 225 and 226 fail (the heads
over the mouth rose, 4.2 at frame 6,000, because a stacked ant no longer
blocks; and it helps at 40 ants too).

**Tunnels two cells wide** (`PIXEL_PHYSICS_DIG_WIDEN=on`, off; see
`how-the-ant-works.md` §12 and `dig_widen_of`). A digger walking a one-cell
passage cuts its wall, and at a face cuts a shoulder beside the cell ahead
on half its rolls; a passage two wide is left alone. The first form widened
at the face on every roll and lost the galleries (seed 1: dug 179 -> 106,
depth 13 -> 7); the second, walls only, barely moved the width.

| 24 seeds, frame 24,000 | today | widened |
|---|---:|---:|
| 40 ants: open cells in one-cell passages | 25% | 20% |
| 40 ants: dug / depth90 / entrances | 260 / 15 / 6 | 263.5 / 14 / 7 |
| 200 ants: dug / depth90 / entrances | 583.5 / 18 / 9.5 | 647 / 17 / 11 |
| 200 ants, walked + carry + stack: dug / depth90 / entrances | 181.5 / 17 / 4 | 172 / 14 / 5.5 |

The census moves little because most open cells are the founding cut and
chambers, already wide; the galleries are what reads as thin, and the
pictures (card `…ca2baa`, seeds by rule) are the judgement. Predictions 227
holds, 228 and 229 fail (§8's list continues in the lane's scratch).


## 14. The owner's answers, and the 200-ant pile traced (2026-09-29, evening)

*Asked in chat with the pictures redrawn in the lab's colours (`digbox`
`look=lab`, render-only, logs identical). The owner: "Q1 - Yes. This also
exposes a huge problem with the 200-ant colony. It is just a huge pile of
ants at the entrance and they totally fill the nest. Q2 - Looks slightly
better with the 40 ants (although might be within normal variability).
Everything again is broken at 200-ants so no impact on Q2 but needs more
thinking. Q3 - this seems much better for the 200 ant tests. What is the
darker brown in the nest the borders/edges the tunnels? In Q3 bottom right
image, it looks like the tunnels have filled in with the darker brown
material."* So the carry is approved, stacking is approved, and widening
waits for 200 ants to work.

**The darker brown** (`FILL` and `tintout=`'s two new classes, corpses and
soil back in a dug cell). The borders are **tamped lining**: every tunnel
wall an ant presses turns to `packedsoil`, whose palette is greyer and
darker than `soil`'s. The filled tunnels are **the nest refilling
itself**. In the Q3 run (200 ants, walked + carry + stacking, seed 6), of
477 cells dug below the old ground line by frame 24,000, 147 are still
open: 146 lining, 129 loose soil, 54 pellets. Half of it had refilled while
the colony was alive (191 of 388 by frame 12,000). Carriers that cannot get
out let go inside (306 pellets inside against 221 outside by 18,000, and
138 died holding one), and a pellet crumbles to soil and is then tamped
into the wall. At 40 ants the same nest stays open: 72 of 105. And that
bottom row was a dead colony: the box has no food, and 6 of the 200 were
alive at frame 24,000.

**Who is in the pile** (`PILE`, `antscsv=`: every live animal at each stop
booked by where its head is and by its own last decision). Today's nest,
200 ants, seed 21, frame 6,000: 149 of 200 above the old ground line, 98 of
them 5 or more rows up (standing on one another), all within 15 columns of
the mouth. **None of the pile is queuing to get in, and none is at home**
(`AtNest` 0, the home test replayed: 0 of 88 on the mound); none has stood
still for 60 frames. They are **scouts**: the chooser's scouting weight
reads 0.5 at frame 6,000 and 1.3 by 12,000. **38 of the 50 nest workers are
up there too.** The box sets `start_energy` to `energy=` and holds no food,
so every ant is below its start energy from its first tick and the engine
reads the whole colony as hungry for the whole run; a hungry empty ant
scouts, and a nest worker is pulled home only while fed (`home_pull`, and
`chooser_step`'s way out). At 40 ants the same scouts spread across the box
(22 of 29 above ground stood 40+ columns from the mouth); at 200 their
loops overlap on the mouth.

**Fed** (`fed`: every ant topped up to `start_energy` each frame, booked as
granted; still no food on the ground). The heap flattens to a carpet (5+
rows up: 98 -> 25, and 7 with stacking) but does not go: **fed ants rest
where they stop**, and they stop on the surface round the mouth (79 of 126
had stood still 60+ frames at frame 6,000, 112 of the 126 above ground).
Nest workers are still out (24 of 31). And **a fed colony digs far
slower**:

| seed 21 (40 ants: seed 2) | hungry | fed |
|---|---:|---:|
| 200 ants + stacking: digs by frame 12,000 | 3,001 | 273 |
| cells dug below ground by 12,000 / 24,000 | 1,574 / 2,347 | 163 / 1,130 |
| alive at 24,000 (of 200) | 4 | 169 |
| underground at 24,000 | 2 | 97 |
| 40 ants: digs by 12,000 / 24,000 | 488 / 680 | 289 / 708 |

So **every nest judged in this box was dug by a starving, restless
colony**. Two gaps come out of it, and neither is the mouth. A resting ant
rests where it stops, and nothing takes it inside: three ants in four
(the foragers) have the door on the surface as home, and the nest is a
place they dig, not a place they live. And digging follows restlessness,
not the crowd: the hungry colony dug the whole box and died, the fed one
barely dug while it rested on the surface. A real colony keeps most of its
workers inside, the idle ones resting in chambers, and digs in proportion
to its numbers, stopping when there is room for everyone (Rasse and
Deneubourg 2001, *Lasius niger*: excavation slows as the room per ant
grows; Buhl et al. 2004, *Messor sancta*: volume dug grows with group
size). The fed box's 200-ant nest at frame 24,000 (97 of
169 underground, 1,130 cells dug) is the nearest picture of that so far.

Predictions 230-234 (written before the fed runs) scored: 230, 231 and 234
fail, 232 fails narrowly, 233 holds; the fed colony's slow digging was not
predicted. One run per arm; the pile census is per animal, so it is
the trace, not a split, and the fed/hungry digging gap is large enough to
read from one seed a size, but it has not been swept.

**Instruments added** (`digbox`, all read-only; runs are identical with
them on): `FILL` (what stands in every cell dug since frame 0, and the
lining on undug walls), `PILE` and `antscsv=PATH` (every live animal: place,
caste, pellet, home replayed, frames still, and its last decision row's
`AtNest`, crowding, energy, outcome, scouting), `tintout=`'s corpse (green)
and refilled-soil (violet) classes, and `fed`.

## 15. Tamped blocks and sealed chambers: the nest refilling itself (2026-09-29, night)

*The owner: "Yes and Yes, but the tamped tunnel walls and the nest
refilling itself are both big issues? the tamped tunnel is not just walls
around a tunnel or chamber. You have chambers fully enclosed by tamped soil
and big blocks of tamped soil." Then: "Make sure you look at the image to
see the actual issue for yourself." §14 had called the borders intended and
stopped there; zoomed in, the sheet says otherwise.*

**What the picture shows.** At 40 ants (hungry, walked + carry) the nest is
black tunnels with a one-cell cyan wall, as designed. At 200 ants it is a
sponge: tamped cells, ants and small holes mixed through the whole dug
region; by frame 24,000 a solid tamped mass pocked with holes, most of them
cut off from one another. Fed, even 40 ants in today's nest turn half their
nest into a block by 24,000.

**Measured** (`gridout=` and a script over it, `blocks.py`; positive
control a hand-made grid read back exactly). Thickness of tamped soil is
its distance from the nearest open cell (1 = a wall, 2+ = inside a block);
a pocket is sealed when none of its open cells reaches the open air above
the old ground line. The Q3 run (200 ants, hungry, walked + carry +
stacking, seed 6) at frame 24,000: 112 of its 182 open cells in 18 sealed
pockets (the largest 47), and 252 of its 563 tamped cells inside blocks; at
12,000, while alive, 14 ants were shut in sealed pockets.

**How they form** (a per-cell trace of every packing, and the refill
ledger). A cell is tamped only as the neighbour of a cut, so it starts as a
wall beside open space; every block cell in the Q3 run was tamped as a wall
(162) or as fill in a dug cell (87), and lies deep because **the tunnel
beside it filled**. What fills them: today's dig puts about **half its
pellets down inside the nest** -- beside the digger's head in the tunnel it
has just cut, or posted up the shaft onto whatever stands there -- and a
pellet with nothing solid under it crumbles to loose soil and pours on
down. The colony re-cuts the fill (about half of all its cuts) and every
cut tamps its eight neighbours, so fill and old walls cement into blocks
and the pockets beyond them are cut off, ants and all.

Today's nest, fed, stacking 4, 8 seeds, frame 24,000 (medians; "8/8" is
the count of seeds that moved that way):

| | 40 ants | 200 ants |
|---|---:|---:|
| pellets put down / inside the nest / into a dug hole | 968 / 507 / 399.5 | 2,653.5 / 1,036.5 / 899 |
| cuts re-cutting fill, of all cuts | 528.5 | 1,014 |
| share of the dug cells refilled | 47% | 39% |
| sealed-off cells (pockets) | 36.5 (6.5) | 123.5 (13) |
| ants shut in sealed pockets | 7.5 | 27 |
| tamped cells inside blocks | 103 | 226 |

**Pellets only on real ground** (`PIXEL_PHYSICS_SPOIL_FOOTING=ground`, the
2026-09-27 switch, off): a pellet is set down only with ground straight
beneath it. Same 8 seeds:

| today's nest + footing, frame 24,000 | 40 ants | 200 ants |
|---|---:|---:|
| sealed-off cells | 36.5 -> 26.5 (lower on 5, higher on 3) | 123.5 -> 7 (8/8) |
| ants shut in | 7.5 -> 5 | 27 -> 2 |
| tamped cells inside blocks | 103 -> 67.5 (6/8) | 226 -> 94 (8/8) |
| pellets put inside the nest | 507 -> 315.5 | 1,036.5 -> 181.5 |
| new ground dug | 373.5 -> 269 | 1,309.5 -> 382.5 (8/8) |
| new ground by frame 12,000 | 157.5 -> 94 | 275.5 -> 35.5 (8/8) |

At 200 ants it clears the sealed pockets but the colony nearly stops
digging while the mouth is covered in ants -- there is no ground there to
set a pellet on. At 40 ants it is mixed, because it does not stop the
largest source: pellets set down inside the nest on ground that is there.

**Walking the soil out** (the owner's pick for one mouth, `SPOIL_OUT=on`,
with the carry `SPOIL_RING=2,2`; off by default) never puts a pellet down
inside the nest by design, and it answers most of this. Same 8 seeds, fed
and stacked, frame 24,000, against today's nest:

| | 40 ants | 200 ants |
|---|---:|---:|
| pellets put inside the nest | 507 -> 56.5 (8/8) | 1,036.5 -> 108 (8/8) |
| sealed-off cells | 36.5 -> 1.5 (8/8) | 123.5 -> 9 (8/8) |
| ants shut in | 7.5 -> 0 | 27 -> 7 (8/8) |
| tamped cells inside blocks | 103 -> 51.5 (8/8) | 226 -> 65.5 (8/8) |
| cuts re-cutting fill | 528.5 -> 55.5 | 1,014 -> 155 |
| open cells | 235.5 -> 138 | 851 -> 193 |
| new ground dug | 373.5 -> 129 | 1,309.5 -> 232 |

And **walked out + only on real ground**, against walked out alone: blocks
51.5 -> 26.5 at 40 ants and 65.5 -> 28.5 at 200 (8/8 both), sealed-off
cells 1.5 -> 2 and 9 -> 1.5 (7/8), the share of dug cells refilled 36% ->
26% and 41% -> 17% (8/8), for new ground 129 -> 92.5 and 232 -> 138.5.

**Reading it.** Today's nest is large because it is a sponge: half of what
it digs is its own fill, dug again, and at 200 ants a seventh of its open
space is sealed off with 27 ants inside. The walked-out nest is a third to
a quarter the size and reads as tunnels: the pictures (seeds nearest the
median of today's sealed cells, by rule: 3 at 40 ants, 1 at 200) show a
compact nest with a low crater and no towers of pellets over the mouth,
where today's nest stacks them into columns. At 200 ants the walked-out nest
is too small for the colony -- packed with ants, the rest lying on the
surface -- which is the owner's other yes, the colony living inside its
nest and digging as it crowds.

Predictions 235-238 (one seed an arm, fed and stacked): 235, 236, 237 hold;
238 fails -- the refill is not a 200-ant problem once the colony lives.

## 16. Walked out at the game's own cap: clean, and it hardly digs (2026-09-30)

*The owner, on the recommendation to make walked + carry the default nest
(§15's answer to the tamped blocks): "Sounds good." The recommendation
carried a caveat -- §15's sweep ran at stacking 4 -- so it was re-checked at
the game's cap first.*

`digbox` fed, stacking unset (1), 8 seeds a size, frame 24,000; today's
nest (`SPOIL_OUT` unset) against walked (`SPOIL_OUT=on`, the carry at its
shipped `2,2`). Medians; every row moved the same way on 8 of 8 seeds:

| | 40 ants | 200 ants |
|---|---:|---:|
| sealed-off cells | 38 -> 2 | 47.5 -> 2 |
| ants shut in sealed pockets | 9 -> 2 | 15 -> 2 |
| tamped cells inside blocks | 90 -> 28 | 80 -> 31.5 |
| pellets put down inside the nest | 506.5 -> 38.5 | 447.5 -> 67.5 |
| **new ground dug** | **342.5 -> 88** | **696 -> 51.5** |
| open cells | 234 -> 123.5 | 448 -> 73 |

**The nest comes out clean because it is hardly dug.** At 200 ants the door
jams: without stacking, carriers going up and ants coming down cannot pass
in the one-cell founding cut. Traced (`TRIPS`, the typical seed, 2): by
frame 24,000, 64 of 74 pellets were let go *inside* after patience ran out,
25 of 99 trips ever left the nest, carriers spent 57% of their carrying
frames in the founding cut (76% by frame 12,000), and a quarter of all of
them standing facing another ant. At 40 ants the
cycle works -- 80 of 107 pellets went outside, 87 trips left the nest --
but each is a long round trip (median 420 frames), so the colony cut 118
cells where today's nest cut about 900.

With stacking on (§15) the same change gave a working nest: open cells 138
at 40 ants and 193 at 200, against 123.5 and 73 here. **So the walked
cycle is not a default on its own; it ships with stacking**, and stacking
waits on its lab births cost (`creature-stacking-design-2026-09-17.md`
§12: newborns refused for want of a free cell beside a stacked parent).
Order from here: the births fix, then stacking and the walked cycle
together.

Predictions 239-242 (written before the sweep): 239, 240 and 241 hold --
240 by far more than predicted, new ground falling 93% at 200 ants; 242
fails, fewer ants shut in than at stacking 4, since a nest barely dug has
little to seal.

## 17. The package, and the carry's latch at the door (2026-09-30)

*The step §16 ordered: stacking at 4, the walked cycle with the carry, and
births on nestmates
([`creature-stacking-design-2026-09-17.md`](creature-stacking-design-2026-09-17.md)
§13), switched on together and measured as one default against today (the
game's cap of 1, the lift). Env: `PIXEL_PHYSICS_STACK_DEPTH=4
PIXEL_PHYSICS_SPOIL_OUT=on PIXEL_PHYSICS_BUD_STACK=on`, `RAYON_NUM_THREADS=1`.*

**The dig box** (`digbox fed nulls=0 energy=1000 w=200 soil=60
frames=24000 stops=0,12000,24000 gridout=`, 8 seeds a size; medians at
24,000, pairs from `gridout`'s blocks census):

| | today, 40 | package, 40 | today, 200 | package, 200 |
|---|---:|---:|---:|---:|
| sealed-off cells | 38 | 1 (lower on 8) | 47.5 | 8 (lower on 7) |
| tamped cells in blocks | 90 | 54 | 80 | 75 |
| open cells | 234 | 139.5 | 448 | 193.5 |
| new ground dug | 342.5 | 128.5 | 696 | 223 |
| pellets put down inside | 506.5 | 54.5 | 447.5 | 99.5 |

Looked at (seeds 4 and 7 at 40, 2 and 4 at 200, cropped and zoomed): today's
nest is a churned crater with heaps over it and ants through it; the
package's is one chamber with tunnels leaving it, a mound beside the door,
and at 200 ants a knot of stacked ants in the chamber. Clean, and a third of
the size.

**The lab** (`labforage scenario=played_bed frames=120000 bedenv`, 24 seeds,
against §12's cap-1 logs): births 418 -> 462.5 (lower on 13, higher on 11),
extinct 2 -> 3, starved 101.5 -> 31 (lower on 15), food eaten 1.09 M -> 1.14 M J,
peak colony 227.5 -> 263.5, alive at the end 203 -> 134 (12 / 11). No measured
harm. (Old carry; see below.)

**Why the package nest is small: the carry, traced.** A temporary line at
every carrier decision (seed 4, 40 ants; the run's stdout identical to the
untraced one), 151 pellets followed from the cut to the drop. 94 were walked
out and put down on the mound as designed; **40 went down below the old
surface after the haul's patience ran out inside, 15 were dropped short**. Of
all carrying frames, 21% came before the carrier first read as outside, 41%
outside, and **38% under cover again after that** (8% shallow, 30% deep).
Two faults in the carry's test of *outside*, which was cover (`inside_nest`)
read afresh at every step:

- **The mouth had widened to five columns over a chamber open to the sky**,
  so cells deep in the nest had nothing overhead. A carrier there read as
  out, drew its column there, and was pulled at it through the ground.
- **The column's target was the founding surface's row**, which the colony's
  own mound buries. A carrier one step from its column went under the
  mound's overhang, read as inside, was pulled back to the door and stepped
  out again. The target changed at every flip, and `home_pull` restarts the
  haul's patience on a new target, so it never gave up: one carrier held its
  pellet 3,636 frames between two cells.

**The fix** (`creature.rs`: `carry_stage`, `ring_target`; always on under
the walked cycle, so the shipped game is unchanged): the column is drawn when
the carrier comes out by the door (on or above the door's row, nothing
overhead), kept under overhangs, and let go only back in a tunnel (more than
two rows under the door's row with ground overhead, or in the founding cut:
`CreatureStats::spoil_ring_let_go`). The pull is to the top of the ground in
the column, and before the carrier is out the pellet is held wherever it
stands. Guards, each watched red against the cover test:
`a_carrier_draws_its_column_at_the_door_not_down_a_hole_open_to_the_sky`,
`a_carrier_out_keeps_its_column_under_the_mound_and_heads_for_its_top`,
`a_carrier_back_in_a_tunnel_lets_its_column_go`.

**Measured** (the same 8 seeds a size, the package env; old carry -> latched):

| | 40 ants | 200 ants |
|---|---:|---:|
| open cells | 139.5 -> 180 (higher on 8) | 193.5 -> 231.5 (7) |
| new ground dug | 128.5 -> 173 (7) | 223 -> 329.5 (8) |
| pellets put down | 171 -> 263.5 (8) | 307.5 -> 600.5 (8) |
| sealed-off cells | 1 -> 2 (4 / 3) | 8 -> 9.5 (5 / 3) |
| tamped cells in blocks | 54 -> 64.5 (4 / 4) | 75 -> 94.5 (7 / 1) |

Traced again (seed 4, 40 ants): 279 pellets against 151, the slowest tenth
of trips 1,740 -> 906 frames, the median unchanged (402 -> 414). **So the
latch removed the stuck carriers; it did not make a trip shorter.** 67 of
the 279 still went down below the old surface (24%, against 29%): carriers
whose patience runs out while they look for the door inside. That is the next
thing to trace. Against today the package with the latch keeps 77% of the
40-ant nest's open space (180 against 234) and 52% at 200 ants (231.5
against 448), with sealed-off space 38 -> 2 and 47.5 -> 9.5. Tamped blocks
at 200 ants now read above today's (94.5 against 80).

**Predictions** (written before each run): 248 half holds (sealed lower on
8 of 8; new ground at 37.5% of today's, not the 60% predicted); 249 half
holds (new ground 32%, at least 25% as predicted; sealed lower on 7, not 8);
250 holds (births lower on 13, no more than 16; extinct 3); 251 fails (below
the old surface 67, not 20 or fewer; median trip 414, not 300 or less); 252
and 253 hold.

**Not yet done, in order** (the handoff in
[lanes/nest-mouth.md](lanes/nest-mouth.md)): the lab pair re-run with the
latch (the table above ran the old carry); the colony bed pair for the
package; the pictures to the owner; then the default flip as its own PR,
with the foraging lane poked first.

## 18. The package on by default (2026-09-30)

*The owner, shown today's nest beside the package at 40 and 200 ants, plain
and colour-coded: "Neither look wrong, although the package only looks
slightly better"; then "You can turn all new feature on by default unless
there is a real trade off." So stacking at 4, the walked cycle with the
latched carry, and births on nestmates ship on together
(`SHIPPED_STACK_CAP`, `parse_spoil_out("")`, `parse_bud_stack("")`);
`PIXEL_PHYSICS_STACK_DEPTH=1 PIXEL_PHYSICS_SPOIL_OUT=off
PIXEL_PHYSICS_BUD_STACK=off` is the nest before it.*

**The dig box, re-run** (§17's command and seeds, `scripts/nestgrid.py
--pair`): §17's latched table reproduced to the digit. Two columns §17 did
not read, from the same runs (`SPEC mouths`, `SUMMARY p50x` at 24,000):

| median of 8 seeds | 40 ants: today -> package | 200 ants: today -> package |
|---|---:|---:|
| entrances | 6 -> 3.5 | 9 -> 5 |
| centre of the dug room, columns from the door | -7 -> -3.5 | -3 -> -3 |

So the walked cycle brings back part of the one-entrance nest the granary
took away (`week-review-2026-09-29.md` W3: 3 -> 6), and the nest still
leans west: the storeroom's fixed side (W2), which the dig box has no food
to confound.

**The lab gate on `main` at `f52bad55`** (#526 in; binary built at
`f1615277`, the flip merged with it; `labforage scenario=played_bed
frames=120000`, 24 seeds, `RAYON_NUM_THREADS=1`; base arm `STACK_DEPTH=1
SPOIL_OUT=off BUD_STACK=off` with `bedenv`, package arm unset;
`scripts/labpair.py`): **a measured harm, and the one place the package
does not pay.** Starved per million ant-frames 9.7 -> 12.0 (**higher on 18
of 24**, p 0.023; raw 88 -> 137.5). Births 441.5 -> 438.5 (16 / 8, p 0.15),
food eaten 1.20 M -> 1.16 M J (15 / 9), ant-frames 11.0 M -> 10.1 M (14 /
10), died of old age 133.5 -> 161 (12 / 12). Peak colony 202.5 -> 272 (16 /
8), alive at the end 144.5 -> 165, died out 1 -> 3 boxes. The box grazes
out, so the likely reading is a bigger colony starving harder in the crash
(inferred: the log carries deaths per row, not starvation, so the split
before and after the peak was not measured). Against the same gate on the
pre-#523 ant, below, the package's lab gain is gone: births 418 -> 536.5
then, flat now.

**The lab gate, with the latched carry, on the ant before #523** (`labforage scenario=played_bed
frames=120000`, 24 seeds, pre-flip binary, `bedenv` and the package env on
one arm; `scripts/labpair.py`): births 418 -> 536.5 (higher on 14), food
eaten 1.09 M -> 1.30 M J (13), ant-frames 10.4 M -> 11.1 M (12/12), starved
per million ant-frames 11.0 -> 9.7 (12/12), died out 2 -> 1, peak colony
227.5 -> 341.5. Died of old age 158 -> 142.5 (lower on 16, p 0.15): a
younger, larger colony, not a harm the gate names. No measured harm; the
sign test cannot see a 15-25% loss (`week-review` §4), and none of the
medians moved the wrong way by that much.

**What the flip exposed in the suite.** Six tests read cap 1 as the default
and now set it by hand (the cap-1 guards, the `SHARED` row, the armoured
duel, and `tests/determinism.rs`'s stack-index test, renamed `..._at_cap_one`).
Two were real:

- **A pellet sink.** `digging_moves_the_ground_rather_than_eating_it` lost
  16 of 133 pellets with their carriers against its bar of one in twenty. A
  temporary line at every loss: the colony dies packed into one chamber, no
  empty cell within five of the body and 43-54 within eight. The death-time
  release now searches rings out to eight (was two), only when the inner
  rings are full: 0 lost.
- **The armoured duel at reach 1** first breaches at a median 966 frames
  at cap 4 against its bar of 300. Not traced; the guard is about the plate,
  so it pins cap 1 as it already pins the walk.

**The colony bed pair on `main` at `f52bad55`** (with #526's even-sided
food drop and birth heading; same bed and seeds as below):

| medians by seed; totals where marked | 90 cells: off -> package | 140 cells: off -> package |
|---|---:|---:|
| food taken from the pile, total cells | 7,689 -> 9,751 (**higher on 23**, p < 0.001) | 5,461 -> 6,528 (16 / 7, p 0.09) |
| food standing at the nest, J | 14,369 -> 12,769 (8 / 16) | 10,213 -> 9,757 (13 / 11) |
| energy in the ants' bodies, J | 8,514 -> 7,059 (**lower on 18**, p 0.023) | 5,731 -> 4,647 (7 / 17, p 0.06) |
| born, total | 281 -> 529 (**higher on 22**, p < 0.001) | 119 -> 199 (**higher on 17**, p 0.017) |
| starved, total | 24 -> 4 (**lower on 11**, 1 higher, p 0.006) | 69 -> 29 (4 / 12, p 0.08) |

The same reading as on `bb11c1ed`, below, and stronger.

**The colony bed pair** (the foraging lane's bed, `STACK_DEPTH=4` both
arms, base `SPOIL_OUT=off BUD_STACK=off`, 24 seeds at 90 and 140 cells,
`scripts/antloop.py --vs`), **on `main` at `bb11c1ed`**, which carries the
food trail laid only by trip loads (#523). The baseline reproduces the
foraging lane's own (9,152 cells taken and 415 born at 90, against their
9,217 and 416):

| medians by seed; totals where marked | 90 cells: off -> package | 140 cells: off -> package |
|---|---:|---:|
| food taken from the pile, total cells | 9,152 -> 11,091 (**higher on 18**, p 0.023) | 6,452 -> 6,900 (14 / 10) |
| food standing at the nest, J | 15,221 -> 13,567 (8 / 16, p 0.15) | 10,823 -> 10,428 (9 / 15) |
| energy in the ants' bodies, J | 9,303 -> 7,627 (**lower on 20**, p 0.002) | 5,750 -> 4,652 (7 / 17, p 0.06) |
| born, total | 415 -> 765 (**higher on 18**, p 0.011) | 161 -> 241 (**higher on 17**, p 0.035) |
| starved, total | 14 -> 5 | 51 -> 34 |

**No real trade-off in the colony bed on the current ant** (the lab gate,
above, is the exception). The package takes a fifth more
food at 90 cells and breeds nearly twice the young, and half as many again
at 140, with fewer starved at both. The ants carry about a sixth less
energy in their bodies, most likely spent walking pellets to the door
(inferred, not traced), and the colony turns it into young rather than
holding it.

**The same pair on the ant before #523 read differently**, and is kept so
the difference is on record: on `8c36d45b` (the lift's food trail) food
taken was unchanged, energy in bodies a third lower at both distances
(lower on 21 and 19 of 24), food at the nest a quarter lower at 90 (lower
on 20, p 0.002), births 209 -> 183 at 90 and 62 -> 88 at 140. The lab gate
above ran on that ant too.

## 19. Pellets dropped under the door, traced; a carrier near it keeps its pellet (2026-10-01)

*The first item §17 left: why about a quarter of pellets still went down
inside. Branch head `8166697f` (the flip, on `main` at `f52bad55`).*

**The trace.** A temporary line at every carrier step (position, the
haul's target, the distance, patience) and every drop, over the dig box at
8 seeds a size (`digbox fed nulls=0 energy=1000 w=200 soil=60
frames=24000`, `RAYON_NUM_THREADS=1`; the runs' stdout identical to the
untraced ones). Pellets put down below the old ground line: 3-20% of drops
at 40 ants, 12-29% at 200 (median about 13%). **Nearly all were carriers
inside the nest whose patience had run out, and over 99% were within 12
cells of the haul's target, 5-7 rows down**: the room the door shaft opens
into. *Corrected the same day:* a first reading of the trace, from the
column latch at the moment of the drop, said these carriers had never come
out. Read from each carry's whole history, 354 of the 957 had been above the
old ground line during the carry and 195 had drawn a column, so about a
third came out by the door and walked back in (one followed: it stood on its
column's target, did not win its drop roll, lost its pull there and
wandered back down the shaft). The shaft enters that room at a corner of
its ceiling, so from the room's floor the straight-line pull points up
through open air. Followed one carrier (seed 4, 40 ants, frames
17,486-17,816): it stood at `(98, 30)`, two cells from the room's wall and
seven from the door, nothing over it for three rows, and swapped head and
tail in place for 27 decisions; its distance stayed 7.3-8.9, patience fell
by 0.9 a step to under 0.1, and it laid the pellet at `(99, 29)`. These are
the tamped blocks under the room in the colour-coded pictures, and the
colony digs them out again.

**The fix built** (`PIXEL_PHYSICS_SPOIL_HOLD`, on at 12 cells,
`creature::spoil_hold_of`): under `keep`, a carrier whose patience has run
out keeps its pellet while it is within 12 cells (Chebyshev) of the haul's
target, and comes out when its wandering takes it there. It does not touch
the cause, the straight-line pull; a carrier that followed the passages
out is the larger change, built and measured below. Guard:
`a_carrier_near_its_door_keeps_its_pellet_when_its_patience_runs_out`,
watched red with the hold removed from `kept_inside`.

**Measured** (the same 16 boxes, `scripts/nestgrid.py --pair` at 24,000;
`SPOIL_HOLD=off` -> shipped; identical to the trial build, which ran the
same rule under a scratch switch; `spoil_held_near_door` 20,228 rolls at
200 ants seed 1, 0 with it off):

| | 40 ants | 200 ants |
|---|---:|---:|
| pellets put down below the old ground line | 24 -> 2 (lower on 8) | 93.5 -> 12 (lower on 8) |
| ...of them into dug cells | 11 -> 2 (7) | 48.5 -> 11.5 (8) |
| cells the colony re-dug | 70 -> 30.5 (lower on 7) | 268 -> 146 (lower on 8) |
| new ground dug | 155.5 -> 140.5 (lower on 7) | 326.5 -> 299.5 (lower on 6) |
| open cells | 168 -> 171.5 (higher on 6) | 247.5 -> 230 (lower on 6) |
| tamped cells in blocks | 50 -> 41 (lower on 6) | 109 -> 90 (lower on 5) |
| sealed-off cells | 2 -> 2 (3 / 4) | 4 -> 8 (3 / 4) |
| pellets put down, all | 236 -> 181 | 563 -> 415.5 |

**What it costs**: about a tenth less new ground dug, because a carrier
holds its pellet longer, and at 200 ants 7% less open space. Looked at
(seed 4 at 40, seeds 2 and 7 at 200): the nests are alike; the held
pellets' nest has fewer packed blocks under the room.

**The other answer, built and compared** (owner: "build both and
compare"). `PIXEL_PHYSICS_SPOIL_ROUTE`, not landed: each nest kept a
breadth-first distance from its door through the cells an ant can stand in
(empty or an animal, with ground or an animal in the 8-neighbourhood),
rebuilt every 30 frames; a carrier inside the nest aimed its pull at the
cell three steps down that field and counted a step as progress when the
field fell. Its guard (a carrier on the floor of a room under a corner door
aimed along the floor to the wall, not up through the air) was watched red.
The first build carried a straight-line best into the field's units, so a
carrier at a field distance of 7-10 measured against a best of 4.47 lost
its patience walking to the door; the second kept the two apart and
counted ants as footing. Measured, second build, the same 16 boxes
(off -> route; the hold alone and both for comparison):

| put down below the old ground line | 40 ants | 200 ants |
|---|---:|---:|
| off | 24 | 93.5 |
| hold (shipped) | 2 (lower on 8) | 12 (lower on 8) |
| route | 52 (higher on 7) | 80 (5 / 3) |
| both | 5 (lower on 8) | 12.5 (lower on 8) |

New ground dug: hold 140.5 / 299.5, route 140.5 / 303, both 136.5 / 277.5,
against 155.5 / 326.5. **The route does not help, and the trace says why**:
carriers on it reach the foot of the door shaft, a field distance of 5-6,
with the waypoint up the shaft, and stand there; the shaft is full of ants
going both ways and the carrier waits behind them until its patience runs
out. So the limit under the door is **the one-lane shaft's traffic, not the
direction of the pull**, and the hold works because it is waiting. Recorded
in `dead-ends.md`. Picture: off, hold and route side by side at 40 ants seed
4 and 200 ants seed 2 (project file `nest/hold-vs-route-2026-10-01.png`).

