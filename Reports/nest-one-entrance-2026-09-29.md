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

