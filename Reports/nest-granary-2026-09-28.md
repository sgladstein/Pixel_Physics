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
  young born fall from 54 to 11, and the colony spends a third as much of its
  time carrying spoil (7.1% of decisions against 20.8%), because an ant
  carrying food never digs. (A share of decisions, between arms with
  different populations; cells dug were not counted.)
- **Only food picked up at home goes in** (foragers still drop at the door):
  **births rise only where storage fails, and fall where it works.** With
  today's room, which holds 0-1 cells (seed 1), born 30 -> 58, higher on 15 of
  24 seeds. With the bigger room, the only arm where food reaches the room
  (7-12 cells, seed 1), born 54 -> 40 (lower on 13, higher on 4), starved
  242 -> 295 (higher on 14, lower on 7), food taken 2,672 -> 2,518 (12 / 12).
  The carriers eat the loads on the way.
- **In the lab box**, looked at before the owner set the lab aside for this
  work, carrying food in kept the entrance open at 8 of 9 checks against 2 of
  9: three seeds at three stops, so n = 3 seeds, and `labshot`'s open count
  also counts a mouth filled by ants as open.
- **Every arm here ran before packed lunch shipped (#509)**, so with it off;
  the storeroom (§8) was measured with it on.

The owner chose **B**: foragers drop food at the door and ants at home carry
it in. **Built as a switch (§8), it fills the room, and it costs the
colony**: with a door over the mouth, 3 or more food cells in the room at
mid-run on 14 of 24 seeds, and against the door alone 14% less food taken and
a third fewer young. This colony has no ants that stay home, so the carriers
are the fed ants that also breed. Found on the way: the door alone now helps
the bed a great deal (starved 93 -> 15). **With a nest-worker caste and the
storeroom off one side of the entrance tunnel** (§8j), five times as many
loads get down and births rise 144 -> 203, but the room holds about as much
as the chamber at the tunnel's foot did: the nest workers eat it as it comes
in, fed or not. **Kept for the hungry** (`keep`, §8k), the room holds 4.9
cells on average against 1.8, with the same births and starvation.

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
nest in joules (the harness's `FOOD STORE`, each seed's mean from frame 6,000,
then the median over the 24 seeds -- the foraging lane's convention; this
column first printed the 24-seed sum); room food from **seed 1's census
only**.

| | food taken | starved | born | food at the nest | room holds (seed 1) |
|---|---:|---:|---:|---:|---:|
| today's ant | 3,057 | 152 | 30 | 6,670 J | -- |
| everyone carries in, today's room | 4,755 | 90 | 0 | 1,298 J | 2-7 cells |
| the bigger room, no granary | 2,672 | 242 | 54 | 6,338 J | -- |
| everyone carries in, the bigger room | 4,061 | 161 | 11 | 3,720 J | up to 12 |
| store loads only, today's room | 3,231 | 180 | 58 | 3,503 J | 0-1 |
| store loads only, the bigger room | 2,518 | 295 | 40 | 4,046 J | 7-12 |

"born 0" with everyone carrying into today's room is an exact zero over 24
seeds and was not traced (the foraging lane's guess: births held for want of
the nest, or of space, behind the jammed shaft).

Paired, against the arm without a granary on the same room: everyone carrying
in raises food taken on 21-22 of 24 seeds and lowers starvation on 16-17, and
lowers births on 20. Store loads only, today's room, against today's ant: food
taken higher on 15, starved 12 / 9, born higher on 15, food at the nest lower
on 23.

- **Today's room jams.** On seed 1 with everyone carrying in, laden ants at
  home spent 10,166 ticks holding a load they could not put down, 63% of
  their decisions at home were on the surface row, and those that reached the
  room found no space for 203 of their drops. The bigger room clears most of
  it (holding 10,166 -> 4,217 ticks). (The delivery counter, 6,177 -> 274,
  does not size it: it counts the same food again and again, 3.4-4.6x, and
  the granary rule removes the repeat drops by construction.)
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
| 140 | door + nest-bound young carry (`on,post,home,nestbound`), against the door | born within 20%; starved within +15; food taken within -10% | 125; 55; -25% | wrong |
| 141 | same | room >= 3 cells at 12,000 on at least 10 of 24 | 15 of 24 | right |
| 142 | young and 1 founder in 4, against young only | pick-ups at 12,000 at least twice, median | 4 against 0.5 | right |
| 143 | door + the stage and the room as home, nothing carried | food taken lower on at least 14 of 24 | 23 of 24 | right |
| 144 | door + the room as home alone | food taken lower on at least 14 of 24 | 20 of 24 | right |
| 145 | door + the stage alone | food taken within ±5% | -6% | wrong, close |
| 146 | door + nest-bound carry without the room as home | food taken within -10%; born within ±15% | -1%; -7% (but 0 loads delivered) | right |
| 147 | door + nest-bound carry walked down, room home | delivered at least twice the handed-down 15.5 | 11.5 | wrong |
| 148 | the mouth 5 columns west of the door, nothing else, against the door | food taken within ±5% | -5.2%; born 150 -> 105 | wrong, close |
| 149 | + nest workers carry (`on,post,caste=4,workerhome`), against 148's arm | room >= 3 at 12,000 on at least 12 of 24; food taken within -10%; born within ±15% | 7 of 24; -9.7%; -16% | mostly wrong |
| 150 | + the caste alone (`caste=4`), against 148's arm | food taken 5-15% lower | -11% | right |
| 151 | walked down against handed down | more delivered | 9 against 7.5 | right |
| 154 | side room against the foot chamber, mouth beside the door, nest workers walking (§8j) | room food (time-averaged) higher on at least 14 of 24 | 8 of 24 (under the door: 11) | wrong |
| 155 | same | food taken within ±10%; starved within ±25 | +4%; 92 against 89 | right |
| 156 | same | loads delivered a run at least twice the foot chamber's | 30.5 against 8 (under the door 31.5 against 5.5) | right |
| 157 | `keep`, side room, mouth under the door (§8k) | room food higher than without `keep` on at least 16 of 24; median at least 3 cells | 21 of 24; 4.87 | right |
| 158 | same | births below 203 | 226 (10 / 12) | wrong |
| 159 | `keep` at 80 founders | room food higher than without `keep` on at least 16 of 24 | 21 of 24 | right |

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

### 8g. Nest-bound ants (the owner's "nest-bound ants carry it in")

`nestbound=<frames>[/<k>]`: an ant born in the colony stays home for its first
`<frames>` (8,000, a fifth of the ant's half-life), and with `/<k>` so does one
founder in `k` from the start. A nest-bound ant is not sent out by the forage
drive; fed, it does not take the way out along a route and is pulled home when
it strays; hungry, it scouts for food as any ant does. Only it carries.

The colony bed, 24 seeds, **the door on in every arm**: food taken, starved,
born, food at the nest, the room at 12,000:

| | taken | starved | born | at the nest | room >= 3 cells |
|---|---:|---:|---:|---:|---:|
| the door alone | 5,611 | 15 | 150 | 6,279 J | -- |
| + the stage alone | 5,277 | 13 | 134 | 5,625 J | 1 of 24 |
| + the room as home alone | 4,590 | 49 | 139 | 6,384 J | 15 of 24 |
| + both, nothing carried | 4,173 | 58 | 120 | 5,790 J | 15 of 24 |
| + nest-bound young carry, room home, handed down | 4,221 | 55 | 125 | 5,682 J | 15 of 24 |
| + young and 1 founder in 4 carry, room home, handed down | 4,610 | 45 | 142 | 6,822 J | 17 of 24 |
| + the same, walked down | 4,804 | 67 | 142 | 6,820 J | 16 of 24 |
| + the same, handed down, room not home | 5,548 | 32 | 139 | 5,492 J | 3 of 24 |

- **The stay-home stage costs little on its own**: food taken -6%,
  starvation 13 against 15.
- **The room fills because it is home, not because it is carried to.**
  Made home, foragers who go in put food down there, as they do anywhere at
  home: 3 or more cells on 15 of 24 seeds with nothing carried at all. That
  is also what costs: food taken -18% against the door alone.
- **Nest-bound ants carry little**: 15-28 loads a run. There are few of them
  (2 to 6 at a time), and they reach food at the door only when it is not
  already buried.
- **Without the room as home the entrance is buried.** With the door over the
  mouth, every delivery lands at the mouth; crumbs fall into the shaft and a
  mound of food grows over it (seed 4, pictures). No load was delivered in 24
  runs, median: the shaft was never open.
- Against today's shipped ant (no door), the best storeroom form here, the
  door with the room as home and nest-bound carriers, starves fewer (93 -> 45)
  and takes as much food (4,574 -> 4,610), with 151 -> 142 born. Against the
  door alone it costs 18% of the food taken.

**What would let nest-bound ants do the work**: a mouth that food does not
fall into (the door beside the mouth rather than over it), and more of them.
Both are put to the owner.

### 8h. Two castes (the owner: "two different types of ants")

Asked whether "young" was right, the owner proposed castes, and they are the
better first step here. Most ants, the red harvester ant among them, divide
work by age, but this colony starts as 20 adults at once and breeds about six
young a run, so an age rule gave 2 to 6 home ants and none before the first
birth. A caste gives a home workforce from the start; *Messor* harvesters have
such a caste; and the engine already has the channel meant for one (a parent
hands its child `Provision`, which the child reads as `Made`), so it can go
into the genome once the switch shows it works.

Built as parts of `PIXEL_PHYSICS_STOREROOM`, off:

- **`caste=<k>`**: one ant in `k`, by its id, founders and the born alike, is
  a nest worker for life: nest-bound (§8g) with no end.
- **`workerhome`**: the founding cut (shaft, chamber and rim) is home to a
  nest worker and only to it; foragers keep the door.

And a founding dial, **`PIXEL_PHYSICS_NEST_SHAFT_OFFSET=<cells>`**: the shaft
is cut that many columns from the founding point, so the mouth sits beside the
door, not under it.

The colony bed, 24 seeds, the door on in every arm; food taken, starved,
born, room >= 3 food cells at 12,000, loads delivered a run:

| | taken | starved | born | room | delivered |
|---|---:|---:|---:|---:|---:|
| the door alone (mouth under it) | 5,611 | 15 | 150 | -- | -- |
| the mouth 5 columns west of it | 5,319 | 23 | 105 | -- | -- |
| + the caste, nothing carried | 4,728 | 20 | 86 | 4 of 24 | -- |
| + workers carry, handed down | 4,805 | 78 | 88 | 7 of 24 | 7.5 |
| + workers carry, walked down | 5,125 | 72 | 98 | 11 of 24 | 9 |
| mouth under the door, workers carry, handed down | 4,748 | 88 | 106 | 8 of 24 | 3.5 |

- **The caste costs what a quarter of the colony not foraging costs**: food
  taken -11%, born 105 -> 86. Starvation does not rise without the carry
  (23 -> 20).
- **Moving the mouth off the door costs births on its own** (150 -> 105,
  lower on 15 of 24). Not traced.
- **The workers barely fill the room: 7 to 9 loads a run get down.** A
  hand-down fails at the mouth because a shaft row has no open cell: 26 to
  824 such misses a run.
- **What blocks the shaft is food.** A census of its 12 cells (seeds 1, 3, 4)
  finds crumbs in most samples, 2 to 10 cells of them, with some lining,
  spoil and soil. The workers live in the cut, eat there and set crumbs down
  there, and a load that cannot get past is let go at the mouth and falls in.
  So food is stored inside the nest, but in the entrance shaft, blocking it,
  not in the room below.
- **The carry raises starvation** (20 -> 72-88), not traced per ant.

**Next, put to the owner**: a storeroom off to the side of the entrance
tunnel, so that stored food does not block the way down. Real harvester nests
keep their granaries in chambers off the main tunnel.

### 8i. Food in the room over the whole run

The owner: *"Track food in the room over time. Not at a single instance."*
The tables above read the room at frame 12,000, one sample of a quantity that
rises, dips and drifts. `trailfollow roomevery=250` now counts the food cells
in the founding chamber (and in the shaft) every 250 frames, for any arm
whatever its switches; nothing else in the run changes (colony numbers equal
the runs above, seed for seed).

Time-averaged over frames 3,000-24,000, median of 24 seeds, the door on
unless marked:

| | food in the room, cells | time with 3+ cells | food in the shaft | more than the door alone |
|---|---:|---:|---:|---:|
| today's ant, no door | 0.01 | 0% | 5.2 | 4 of 24 |
| the door alone | 1.09 | 0% | 4.5 | -- |
| mouth beside the door | 0.91 | 0% | 5.8 | 7 of 24 |
| + the caste, nothing carried | 0.66 | 0% | 5.6 | 10 of 24 |
| + nest workers carry, walked down | 1.74 | 31% | 4.5 | 17 of 24 |
| the room as home, nothing carried | 2.72 | 55% | 3.2 | 21 of 24 |
| nest-bound young and 1 founder in 4 carry, room home | 2.98 | 59% | 3.3 | 20 of 24 |
| every fed ant carries, room home | 3.54 | 69% | 3.6 | 23 of 24 |

- **Over the run the ordering holds and the gaps are clear.** Where every fed
  ant carries into a room that is home, the room fills within 6,000 frames
  and stays at 3-4.5 cells (mean of the 24 runs); with the room as home alone
  it fills as fast and drifts down to 2.5-3; the nest workers fill it slowly,
  to about 2.5 by frame 12,000-17,000, then 2.
- **The shaft holds food in every arm**, 3 to 6 cells on average, today's
  ant included: food falling into the entrance is not new, it is what the
  shipped nest already does.
- The colony-number costs of each arm are in §8c, §8g and §8h.

### 8j. A storeroom off one side of the entrance tunnel

The owner, to §8h's proposal: yes. Built as a part of
`PIXEL_PHYSICS_STOREROOM`, off:

- **`side`**: at founding a passage two rows tall leaves the entrance shaft
  halfway down, on the side away from the door, and runs past the end of the
  chamber at the shaft's foot. Beyond it is a room as wide as that chamber (7
  columns), its floor a row below the passage's, so food on it lies under
  the level ants walk at. The nest workers walk their loads down the shaft,
  along the passage and onto the room's floor. Every storeroom rule reads
  this room in place of the chamber (`ShaftFootprint::store_rect`); without
  `side` it is the chamber, and every arm measured before is unchanged bit
  for bit (the bed's default and the foot-chamber caste arm, 24 of 24 seeds
  each; `digbox`, 4 of 4).

The colony bed, 24 seeds, the door on, nest workers carrying
(`on,caste=4,workerhome`) in every arm. Food in the room counted every 250
frames, as in §8i:

| | foot chamber, mouth under the door | side room, mouth under the door | foot chamber, mouth beside the door | side room, mouth beside the door |
|---|---:|---:|---:|---:|
| loads delivered a run (median) | 5.5 | **31.5** | 8 | **30.5** |
| food in the room, frames 3,000-24,000 (median of time-averages) | 1.66 | 1.76 | 1.46 | 0.69 |
| ... frames 3,000-12,000 | 1.55 | 0.78 | 1.04 | 0.12 |
| ... frames 12,000-24,000 | 1.91 | 1.86 | 1.71 | 1.05 |
| food in the shaft (median) | 4.0 | 5.6 | 4.3 | 4.7 |
| food taken from the pile | 5,292 | 5,900 | 5,009 | 5,211 |
| starved | 61 | 75 | 89 | 92 |
| born | 144 | **203** | 110 | 119 |

For scale, the door alone on the same binary: 5,301 taken, 23 starved, 136
born.

- **The side room gets five times as many loads down**, 31.5 a run against
  5.5 with the mouth under the door, because the way in is no longer the
  store. **The room does not hold much more.** Seed for seed it fills later
  (less food in the first half on 17 of 24) and ties in the second half
  (more on 13, less on 11). The mean of the 24 runs is higher from about
  frame 15,000 (3.3 cells at the end against 2.7), because a few seeds store
  a lot.
- **The colony uses what goes in.** With the mouth under the door, births
  rise from 144 to 203 (higher on 18 of 24), the most of any arm measured on
  this bed, and food standing at the nest from 7,153 to 10,112 J (higher on
  18). Food taken rises 11% (14 / 9). Starvation rises too, 61 -> 75 (13 / 7).
  Who takes the stored food back out -- the nest workers who live in the
  cut, births paid from food within reach at home, or packed lunches -- was
  not traced.
- **With the mouth beside the door the side room is the worse room.** Loads
  still rise (8 -> 30.5), but it holds less (lower on 18 of 24 in the first
  half, 17 in the second) and births barely move (110 -> 119). The room
  then lies 10 to 16 columns from the door, against 5 to 11 with the mouth
  under it. Not traced further.
- **The shaft still holds 4-6 food cells in every arm**, with a storeroom or
  without: that food comes down from the door, not from the store.

### 8k. Where the stored food went, and a store kept for the hungry

**Traced bite by bite** (a scratch build logging every food cell an ant took
out of the storeroom, and who took it; the traced runs reproduce the measured
ones seed for seed). Side room, mouth under the door, 24 runs:

| food taken out of the room | side room | chamber at the foot |
|---|---:|---:|
| loads delivered in | 739 | 179 |
| bites by fed nest workers | 4,676 (69%) | 1,048 (41%) |
| bites by hungry nest workers | 703 (10%) | 669 (26%) |
| bites by fed foragers (a packed lunch) | 1,081 (16%) | 467 (18%) |
| bites by hungry foragers | 210 (3%) | 313 (12%) |
| dug out as spoil, or spent on a birth | 153 | 42 |

**The nest workers ate the store as it came in, fed or not.** A bite is a
mouthful, not a cell, so the rows count how often each kind of ant ate there.

**`keep`: food in the storeroom is eaten only by a hungry ant**
(`store_kept`, a part of `PIXEL_PHYSICS_STOREROOM`, off). A fed ant's won
feed roll on a storeroom cell takes nothing. Its "it fired" counter,
`store_kept`, reads 762-1,574 refusals on the first two seeds. Off, it reads
the switch and nothing else: the bed's default is unchanged on 24 of 24
seeds.

| side room, mouth under the door, 24 seeds | 20 founders | + `keep` | 80 founders | + `keep` |
|---|---:|---:|---:|---:|
| food in the room, frames 3,000-24,000 (median) | 1.76 | **4.87** | 2.43 | **3.66** |
| ... more than without `keep` | | 21 of 24 | | 21 of 24 |
| time with 3+ cells in the room | 27% | 64% | 36% | 57% |
| loads delivered a run | 31.5 | 15.5 | 32 | 27.5 |
| food taken from the pile | 5,900 | 6,088 | 8,413 | 8,218 |
| starved | 75 | 75 | 1,088 | 1,102 |
| born | 203 | 226 (10 / 12) | 73 | 72 |

- **The room becomes a granary**: in pictures it fills with food from about
  frame 12,000, where without `keep` it stays nearly empty.
- **Births and starvation do not move**, at 20 founders or at 80. Food
  standing at the nest rises 10,112 -> 12,536 J (higher on 19 of 24).
- **Fewer loads are needed** (31.5 -> 15.5 a run): the room holds what it is
  given.
- The rule is in the foraging lane's region (what an ant eats), so it lands
  as a switch they review. The whole granary is
  `NEST_DOOR=2 STOREROOM=on,caste=4,workerhome,side,keep`.

**Next**: whether the granary should come on by default is the owner's call
with the foraging lane: it costs the colony bed starvation (75 against 23 for
the door alone, the caste's cost) and raises births (226 against 136). And
at scale the room is one room: a bigger colony's store will need more.
