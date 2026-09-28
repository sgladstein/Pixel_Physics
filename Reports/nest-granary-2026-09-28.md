# Food in a chamber: a prototype, and the trade it runs into

*2026-09-28. The nest lane (`lanes/nest-mouth.md`). A measurement of a
scratch prototype. Nothing here is on a branch or in the game.*

## 0. The answer

**Food can be carried into a chamber, and when it is the doorstep stays
clear. But in this engine carrying is eating, so every version tried either
stores little or costs the colony its breeding.**

- **Everyone carries food in** (the founding room made bigger): on the colony
  bed a row of food lies on the room's floor by frame 20,000 and the doorstep
  is clear. Food taken from the pile rises by half and fewer ants starve. But
  young born fall from 54 to 11, and the colony digs a third as much, because
  an ant carrying food never digs.
- **Only food picked up at home goes in** (foragers still drop at the door):
  breeding survives and even rises (30 -> 58 with today's room), but with
  today's room almost none of it reaches the room. The carriers eat it on the
  way.
- **In the lab box**, looked at before the owner set the lab aside for this
  work, carrying food in kept the entrance open at 8 of 9 checks against 2 of
  9.

The next step is a design decision (§6), put to the owner and the foraging
lane.

## 1. Where this sits

The nest lane is after one mouth every ant uses and a nest that reads as a
nest. In the lab box the founding entrance is buried by frame 30,600 on 12 of
12 seeds, under delivered food and plants (`nest-heap-cue-2026-09-28.md` §7).
Asked whether the colony should keep its door clear or take its food inside,
the owner answered **"Food in chamber"**. The food drop and a laden ant's
homing are the foraging lane's; the lane was told the plan before anything was
built, and the prototype was kept to a scratch worktree.

Midway the owner ruled: *"I don't think we should worry about plants or the
standard lab bed during this development. Just use a simpler test environment
for now."* So the colony bed (`trailfollow`: soil, one food pile 90 cells out,
20 founders, no plants or weather) is the test environment, and the lab
results in §4 are the only lab ones.

## 2. What was built (scratch only)

One switch, `PIXEL_PHYSICS_GRANARY`, on `main` at `34e07b86`:

1. **The founding chamber is home**: a cell within one of the chamber's
   rectangle counts as next to the nest (`adjacent_nest`).
2. **A laden ant's home target is its nest's chamber floor** (`home_target`),
   so it walks down the entrance instead of to the last strip cell it touched.
3. **At home, the load goes down only inside the chamber**, on an empty
   chamber cell, the floor row first. On the strip or in the shaft it is held.
4. **`=split` sends only a store load in**: a crop every cell of which was
   picked up at home. A load from the field is dropped at the door as today.
5. Dials for the room: `PIXEL_PHYSICS_NEST_SHAFT_WIDTH` (shipped) and two
   scratch ones for the chamber's height and half-width. "The bigger room" is
   a shaft 4 wide over a chamber 13 x 3 (63 cells, against today's 2-wide
   shaft over 7 x 2, 26 cells).

A census in `trailfollow` counted food cells in the chamber, the shaft, on the
surface and elsewhere under the nest, every 3,000 frames. `=off` on the scratch
binary reproduced `main`'s colony bed exactly (24 of 24).

## 3. The colony bed (24 seeds, gap 90)

Starved of about 480; food taken from the pile in cells; food standing at the
nest in joules (the harness's `FOOD STORE`); room food from seed 1's census.

| | food taken | starved | born | food at the nest | room holds (seed 1) |
|---|---:|---:|---:|---:|---:|
| today's ant | 3,057 | 152 | 30 | 160,063 | -- |
| everyone carries in, today's room | 4,755 | 90 | 0 | 36,973 | 2-7 cells |
| the bigger room, no granary | 2,672 | 242 | 54 | 151,443 | -- |
| everyone carries in, the bigger room | 4,061 | 161 | 11 | 92,742 | up to 12 |
| store loads only, today's room | 3,231 | 180 | 58 | 83,368 | 0-1 |
| store loads only, the bigger room | 2,518 | 295 | 40 | 94,489 | 7-12 |

Paired, against the arm without a granary on the same room: everyone carrying
in raises food taken on 21-22 of 24 seeds and lowers starvation on 16-17, and
lowers births on 20. Store loads only, today's room, against today's ant: food
taken higher on 15, starved 12 / 9, born higher on 15, food at the nest lower
on 23.

- **Today's room jams.** On seed 1 with everyone carrying in, deliveries fell
  from 6,177 to 274. Laden ants reached the entrance, where 63% of their
  decisions at home were on the surface row, and the 2-wide shaft let few
  through; those that reached the room found no space for 203 of their drops.
  The bigger room clears most of it (ticks spent holding a load at home
  10,166 -> 4,217).
- **Carrying replaces digging.** With everyone carrying in (bigger room) the
  colony's time laden goes 48.5% -> 62.9% of decisions, and its time carrying
  spoil 20.8% -> 7.1%.
- **The bigger room alone costs the bed starvation** (152 -> 242, though
  births rise, 30 -> 54), which is why its rows are compared with each other
  and not with today's ant.

## 4. The lab box, before it was set aside

Three seeds, the bigger room, everyone carrying in against the same room
without a granary, `labshot`'s census of the founding cut at frames 12,600,
30,600 and 60,300:

- **The entrance stays open**: open to the surface at 8 of 9 checks, against
  2 of 9. The cut keeps a median 18 of its 63 cells open, against 3, with
  crumbs and up to 24 ants in it.
- **Without a granary the cut fills with plants** (up to 49 cells), loose
  soil, crumbs and seeds, and water pools over it.
- In pictures (seed 1) the granary colony keeps an open pit over a room full
  of food: a larder more than a roofed chamber.

The lab's colony numbers were not taken: the run was stopped by the ruling.

## 5. What limits it

- **Carrying is eating.** A crop digests while it is carried, so every tick of
  the trip down is food eaten rather than stored. That is why everyone
  carrying in starves fewer ants and breeds fewer young, and why store loads
  carried by ants at home rarely arrive: the carriers eat them.
- **Births are paid from food within reach** (`try_bud`). A store in the room
  is within reach only of an ant in the room, and there is little in it.
- **A laden ant never digs**, so a colony that spends its time carrying stops
  building.
- **The founding cut is a one-lane shaft into a small room**, which jams when
  many ants use it at once.

## 6. What is next

A decision, before building further:

- **A. Everyone carries in, made cheap**: a room right under the door so the
  trip is short, and a birth that can be paid from the room's store. The
  clearest picture of a granary; it asks the foraging lane's crop and the
  birth rule to change.
- **B. Foragers drop at the door, ants at home carry it in, without eating
  it on the way**: a store load held in the mandibles like a pellet of spoil,
  not in the crop. Closest to real harvester ants. The crop and the drop are
  the foraging lane's; the carry is shared with the spoil machinery, which is
  this lane's.

Either way the founding room needs to be big enough for the traffic, and the
bigger room's own cost on the bed (§3) comes first.

## 7. Predictions, written before each batch

| # | arm | prediction | result | right? |
|---|---|---|---|---|
| 115 | everyone in, today's room, frame 12,000 | half the nest's food in the room; the shaft at most 2 | 7 of 9 cells in the room (seed 1) | right |
| 116 | same | starved within ±30, food taken within ±15% | 90 against 152; +56% | wrong |
| 117 | same | the room full by 6,000; holding climbs | never full; holding climbs | half |
| 118 | everyone in, bigger room | deliveries at least half the off arm's; the room holds most food | 19%; 7 of 9 cells | half |
| 119 | same | births at least half the off arm's | 11 against 54 | wrong |
| 120 | lab | the mouth open at 30,600 on at least 2 of 3 seeds | 3 of 3 | right |
| 121 | lab, 12 seeds | births lower on at least 8 | not run (stopped) | -- |
| 122 | store loads only, today's room | food taken within ±10%; starved within ±25 | +6%; 180 against 152 | half |
| 123 | same | food in the room at 12,000 on most seeds | 0 cells (seed 1) | wrong |
| 124 | store loads only | births at least 20 | 58 and 40 | right |
