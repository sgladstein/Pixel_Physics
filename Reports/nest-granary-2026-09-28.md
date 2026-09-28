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

The owner chose **B**: foragers drop food at the door and ants at home carry
it in. **Built as a switch (§8), it fills the room, and it costs the
colony**: with a door over the mouth, 3 or more food cells in the room at
mid-run on 14 of 24 seeds, and against the door alone 14% less food taken and
a third fewer young. This colony has no ants that stay home, so the carriers
are the fed ants that also breed. Found on the way: the door alone now helps
the bed a great deal (starved 93 -> 15).

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

Traced after this was written (`nest-heap-cue-2026-09-28.md` §16, packed lunch
on): when food is wanted, a fed forager inside the nest goes out first only
13-17% of the time, against 53-56% from the doorstep. Inside, it mostly picks
up food and empties it there (57-58%). So a store kept inside would hold the
ants who handle it unless handling it stops holding them. Under **B** the
ants carrying food in are inside by design. Packed lunch (#509, on since this
prototype) also takes store food from home back out as lunch, so under either
form the store's food is drawn two ways.

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
| 131 | B, `off` on the storeroom binary | reproduces the shipped bed exactly | 4,574 / 93 / 151 / 7,743 J, 24 of 24 | right |
| 132 | B, `home` (the room is home, nothing carried) | starved at least 110; born lower than off | 122; 137 | right |
| 133 | B, first form | at least 3 food cells in the room at 12,000 on at least 18 of 24 | 15 of 24 | wrong |
| 134 | B, first form | starved higher than off on at least 16 of 24 | 18 of 24 (206 against 93) | right |
| 135 | B, only ants the drive misses (`nestbound`) | store pick-ups under 10 a run | median 0 | right |
| 136 | B, first form | food on the surface at 12,000 lower than under `home` on at least 16 of 24 | medians 13 against 13.5 | wrong |
| 137 | the door alone, against off | starved lower on at least 14 of 24; food taken higher on at least 16 | 21 and 20 | right |
| 138 | door + `on,post,home`, against the door | room >= 3 cells at 12,000 on at least 14; born lower on at least 14; starved within ±15 | 14; 16; 41 against 15 | half |
| 139 | `on,post,home` without the door, against off | food taken lower on at least 16 of 24 | 24 of 24 | right |

## 8. B, built: foragers drop at the door, ants at home carry it in

*2026-09-28, after the owner chose B: "Foragers drop food at the door; ants
that stay home carry it into the storeroom." Built on the branch as a switch,
off by default. The colony bed with packed lunch and dig down on (`main`
after #509), gap 90.*

**The storeroom can be filled, and it is visible, but in this colony it
costs the loop and the breeding.** With a door over the mouth, the best form
found keeps 3 or more food cells in the room at mid-run on 14 of 24 seeds.
Against the door alone it costs 14% of the food taken from the pile, 26 more
ants starved and a third of the young (102 born against 150).

### 8a. What is built

`PIXEL_PHYSICS_STOREROOM`, comma-joined parts, off unless named:

- **`on`, the carry.** A fed ant at home, with an empty crop and empty
  mandibles, wins its `Feed` roll beside loose food lying more than a cell
  from the founding chamber. It takes the cell whole into its mandibles (the
  spoil slot), so the food is not eaten on the way. The load is pulled to the
  mouth, then to the chamber's floor, at the laden pace. It goes down on a
  `DropSpoil` roll in or beside the chamber. A carrier that has put a load
  down is pulled back up to the mouth. A carrier still for 48 decisions, or
  out of patience, lets the load go where it stands. A pick-up within a cell
  of the chamber reads as at home, the foraging lane's condition.
- **`home`**: the chamber is home to `AtNest` (breeding, the food drop, the
  dig gate).
- **`once`**: one load a trip, until the next pick-up away from home.
- **`post`**: the load is handed down the open shaft from the mouth, the walk
  down abstracted as the spoil lift abstracts the walk up.

Nothing is read or written with the switch off. On seeds 1-8 its `off` arm
reproduces `main`'s binary line for line, except the header line that names
the switches.

### 8b. How it got there (seeds 1-8 unless marked)

Each form fixed what the one before it showed, in pictures and counts:

1. **The first form** (24 seeds) sent loads to the chamber's floor and made
   the room home. 75 loads a run were picked up and 20 reached the room.
   Carriers pressed into the ground beside the mouth, or stood in the full
   room holding; starved 93 -> 206, born 151 -> 78.
2. **Only ants the drive misses** (`nestbound`) picked up nothing, median 0
   a run. Under the shipped drive every ant that has ever taken food away from
   home is sent out again, and nearly every ant has. **This colony has no
   ants that only stay home.**
3. **Mouth first, then the floor**: loads reached the room. But the carriers
   stayed underground, dug sideways and starved (seed 1: every ant below
   ground by frame 12,000). Nothing in the walk points up: the away pull is
   level on purpose. **The return trip** fixed that.
4. **One load a trip** (`once`) changed nothing: food taken 1,030 -> 1,000,
   born 7 -> 7. Carrying was not displacing foraging by its amount; about 60
   pick-ups a run is too few for that.
5. **Dug crumbs out**: the first forms counted any food in the mandibles,
   so crumbs a digger cut were carried too. Taking them out moved little
   (door and `once`: food taken 1,687 -> 1,709, born 9 -> 8).
6. **Handing the load down** (`post`): the 2-wide shaft jams, and crumbs
   fall into it (3-8 food cells lie in the shaft by frame 24,000). Posting
   from the mouth cost less (starved 62 -> 41, born 8 -> 14), and with the
   room as home it delivered the most (with the door: 55 loads a run
   against 10).

Where the colony spends its time (seeds 1-8, decision trace): away from home
44% of decisions with it off, 40% with the carry, 33% with the carry and the
room as home. Inside the nest 35%, 43% and 53%.

### 8c. The colony bed, 24 seeds

Food taken from the pile (cells), starved of about 480, born, food standing
at the nest (J), and the room at frame 12,000:

| | taken | starved | born | at the nest | room >= 3 cells |
|---|---:|---:|---:|---:|---:|
| off | 4,574 | 93 | 151 | 7,743 | -- |
| `home` only | 4,295 | 122 | 137 | 7,711 | 7 of 24 |
| first form (§8b item 1) | 3,504 | 206 | 78 | 6,122 | 15 of 24 |
| `on,post,home` | 2,994 | 151 | 35 | 5,058 | 7 of 24 |
| the door alone (`NEST_DOOR=2`) | **5,611** | **15** | 150 | 6,279 | -- |
| door + `on,post,home` | 4,831 | 41 | 102 | 6,454 | 14 of 24 |

Paired, door + storeroom against the door alone: food taken lower on 16 of
24, starved higher on 12 (lower on 5), born lower on 16, food standing at the
nest higher on 16. The room holds a median 3 cells at 12,000 and 4 at
24,000.

### 8d. What limits it

- **The carriers are the breeders.** A birth is paid from food within reach
  of an ant at home (`try_bud`). The fed ants that sit by the food at the
  door are the ones that breed, and they are the ones a won `Feed` roll turns
  into carriers. Food moved into the room is within reach only of an ant in
  the room, and only under `home` can it pay for a birth there (door and
  post: born 18 without `home`, 30 with it, seeds 1-8).
- **No ants stay home.** Under the shipped forage drive every forager is
  sent out, so there is no nest-worker caste to do the carrying.
- **The founding cut is small.** A 2-wide shaft jams, crumbs fall into it,
  and a 7 by 2 room fills.

### 8e. The door alone, on today's ant

Found on the way, and not this lane's lever: the painted door over the mouth
(`PIXEL_PHYSICS_NEST_DOOR=2`, the foraging lane's §19 switch) now helps the
bed a great deal. Against off, starved 93 -> 15 (lower on 21 of 24), food
taken +23% (higher on 20 of 24), born 151 -> 150. It was left off when every
narrow home carried less food home in the lab; that was before packed lunch
and dig down. The foraging lane was told.

### 8f. What is next

The step that would make B cheap is **ants that stay home**. Real ants
divide the work by age: young workers work inside, older ones forage. Here
that would be young ants that do not forage for a while, and carry and dig.
Then the carriers would not be the foragers or the breeders. It changes when
an ant starts to forage, which is the foraging lane's region, so it is put to
the owner and that lane first.
