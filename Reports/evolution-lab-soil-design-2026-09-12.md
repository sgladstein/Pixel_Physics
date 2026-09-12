# Two soils: what a mound is, and what it should do — design, round thirty

*Owner's ask, 2026-09-12, verbatim: "I want a lane to explore the soil that
is created when plants, leaves, other things decompose compared to the
pre-existing ground. creatures cannot dig through it and have it stay in
place, right? But when creatures dig the ground and place it outside it does
stay in place. I think we need to think this through and I am honestly not
sure what is best. I think maybe it should behave the same but I don't want
to end up with a bunch of individual pieces of soil floating in the air. I am
also wondering if after ants dig and place soil it should float. They create
cool towers, but it also engulfs plants and creates a ground that new plants
dont grow in. Maybe the solution is that buried plants should die (from no
sun or other reasons), decompose to soil, then new plants should be able to
grow through a nest with lots of cavieties (probably moisture issues) or just
make it so placed soil doesn't stay which may be simplier (although may not
also fully solve the problem). Lets really think think through."*

*Status: **design of record for the two soils, with the measurements every
recommendation is taken from.** Nothing here is built except the instrument,
`examples/soilfork`. Lane U of round thirty; upstream of round-30 brief 3
(room and weathering), which this report asks to re-cut in one respect.*

## The answer

**Two of the owner's three symptoms survive measurement and one does not.**
The towers are real and so is the engulfing. *"A ground that new plants don't
grow in"* is not: the bare patch over a nest is there before the mound is,
and on the bed that grows the biggest mound the plants stand *on* it. So the
question is not what to do about a sterile substrate. It is what a heap of
tamped spoil should do with time, and what a plant inside one should do.

1. **Keep the tamped spoil — the nest cannot exist without it — and let the
   spoil that is *dumped* weather back to loose soil, at a rate that is a
   dial.** This is round-30 brief 3, with one correction: **it cannot be a
   `decays_into` on `packedsoil`**, because the chunk-settle scan schedules a
   decay site for *every* cell whose material decays (`world.rs`,
   `end_step`), so the tunnel lining would weather too and the nest would go
   with it. The dumped pellet and the tunnel wall have to be **two
   materials**: `spoil` (self-supporting, decays to soil) and `packedsoil`
   (self-supporting, permanent). That is the "second flag" the brief asked
   for, and it is a material name rather than a rule. Measured as a staged
   end state on the played bed, the dial's two ends are a mound that stands
   for a session and one that cones over a few thousand frames at the cost
   of a fifth to a third of the shallow galleries (§3b). Its zero is
   "leave the towers", so nothing the owner likes is lost by shipping it.
2. **Make loose and packed soil opaque to sunlight, as a material property.**
   Burial cannot kill a plant today and *cannot* be made to with any rule
   about the plant: `field.rs` counts only `Solid` and `Plant` cells as
   occluders, so a leaf inside a heap of `packedsoil` sees the full lamp.
   Measured with a stone positive control (§2b): the same burial in stone
   cuts a tree's income to **8%** of the control and starts its starvation
   clock, in packed soil its income is unchanged. An `opaque` flag on the
   material costs nothing per frame — the scan already reads the material —
   and gives the buried plant the graded death `rot_remains` already
   delivers. It is second, not first, because it reallocates three things
   calibrated against transparent ground: the germination light gate, the
   ant's own light sense, and every cave in the outdoor game (§3d).
3. **Do not build the moisture candidate, the richer-soil candidate, or the
   gated crumb rule.** The mound is as wet as the bank beside it or wetter
   (0.29 against 0.22 of capacity on seed 3, 0.65 against 0.62 on seed 1),
   the crumb rule that the brief calls untried
   has been the shipped rule since 2026-09-01, and a richer decomposed soil
   needs a nutrient economy the engine does not have. Each is in §3 with its
   number.

**The thing to hold on to** — an ant must be able to make a hole that stays,
and a heap that is permanent and sterile is a scar — is satisfied by 1 and 2
together and by neither alone. The lining stays packed for ever; the heap
weathers into ground; what the heap buried dies in the dark and rots into the
heap. The verb still delivers a visible mound. The mound has a middle.

## 1. What the two soils are today

**One flag.** `packedsoil` is the only material in the game with
`self_supporting: true`. Everything that decomposes — litter, log, windfall,
ash, corpse, pip, and deadleaf and deadwood via litter — ends as plain
`soil`, a `Powder` that falls and piles at 33 degrees. Ant spoil is a
`packedsoil` pellet: the dig converts the dug cell through `soil.packs_into`,
the ant carries it, and the dump verb sets it down as worked ground, tamped,
wherever `drop_urge` fires and the cell will take it.

**What reads the flag** (all in `update.rs`'s `update_powder`, and only a
self-supporting cell pays any of it):

- **It does not fall.** The unconditional straight-down move that closes a
  gallery in loose soil in five frames is skipped. This is the whole of why
  a burrow is a place rather than an event, and it is the hard constraint on
  the owner's "maybe it should behave the same": measured on `labnest`,
  loose spoil leaves **2/1 and 2/3** cells of roofed void against **54/58**
  tamped. That is the nest, gone. Nothing about repose reaches it — at 89
  degrees the roll goes to zero and the straight-down move still fires.
- **The crumb rule** — a self-supporting cell with *nothing beneath it* and
  fewer than three of eight neighbours filled goes back to loose soil where
  it stands. Live, on by default, gated on the empty cell below — **this is
  the "correctly gated version" the brief calls untried; it shipped on
  2026-09-01 and the ungated version is the dead end.** It removes the
  single hanging pellet and, by construction, nothing resting on anything.
- **The wet rule**, off by owner ruling since 2026-09-06 (a roof that falls
  in was declined; a burrow fills with water instead).

**Where a pellet may go** is one predicate about a cell, not a policy:
empty, two of the three cells under it filled, three rows of clear air over
it; failing that, the first such cell up the column to 160 rows. The owner's
ruling closed the placement question (*"an ant should be able to hold and
carry soil similar to how it holds and carries food"*), and the lift up the
column is how a pellet dug at the face leaves the nest without the walk
being simulated. PR #221 (open, 2026-09-03) measured that lift posting
pellets **52–101 rows** into a tree's crown, four seeds, against **4–11**
rows with no tree in the bed — that is the *engulfing*, and it is a live
mechanism on `main` today, not a stale one.

So the asymmetry the owner describes is exactly one bit on one material, and
every consequence below follows from the two things that bit does: the
lining stands, and so does everything else the colony tamps.

## 2. Which symptoms survive measurement

Everything here is from `examples/soilfork`, built for this report, on the
played bed at `RAYON_NUM_THREADS=1`. Two seeds, chosen for their shapes: seed
3 booms early and crashes (82 ants and 16 plants left at 200,000 frames on
this build), seed 1 booms late and enormously (**3,182 ants** at 300,000).
Two runs of seed 3 to 200,000 frames agreed to the cell, so every count below
is reproducible from one command.

### 2a. "A ground that new plants don't grow in" — does not survive

Three independent readings, all against it:

| reading | number | what it says |
|---|---|---|
| `latecensus`, late-game report §0, seed 2 | band closed **84/129 → 0/129 bare** while 386–456 packed cells stood above the surface | plants come back under a standing mound once the colony is small |
| seed 3 at 100,000 frames (this report) | **112 of 129** band columns bare with **88** packed cells above the surface and **650** of rot-made soil | the bare band is there before the mound is |
| seed 1 at 300,000 frames, 3,182 ants | **1 of 129** band columns bare; 1,313 plant cells in the band; 200 plants standing | the bed with the biggest mound in this report is green over it |

And the ground itself is not hostile. A pellet carries the water the dug cell
held, and it comes up from depth: the mound reads **0.29** of water capacity
against **0.22** in the top four rows of the bank beside it on seed 3, and
**0.65 against 0.62** on seed 1, where flooded galleries wet the bank too.
Roots enter packed soil at 0.95 against 0.8 for tilth, a 1.19x carbon cost.
Seeds germinate on it when it is wet, and it is wetter than what they
germinate on elsewhere. Round 28's garden-loop blocker — *452 of 454 pip
checks failed on water* — was the `nest` material, which holds no water at
all, and it was fixed by relocating the drop (#328). None of that applies to
spoil.

**What the owner is seeing is the colony, not the ground.** A colony at the
door eats every seed and seedling; when it thins, the band closes with the
mound still there. This is the late-game report's reading and this report
confirms it from the other side. Consequence: the two candidates that
address the *substrate* (a cavity nest plants can root into; a soil that is
better than the bank) are aimed at a symptom that is not there.

### 2b. "It engulfs plants" — survives, and the plant lives on inside

Seed 1 at 300,000 frames: **493 shoot cells touching packed soil, 1,391 shoot
cells enclosed on all eight sides.** Seed 3 at 200,000: 92 and 91. So spoil
does bury shoots, at scale on a big colony.

**But burial does not kill, and the reason is the light model, not the
plant.** `rebuild_blocked` in `field.rs` counts a CA column's opaque depth
from `Solid` and `Plant` cells only; `Powder` and `Liquid` pass. The field's
own comment records it as a known degenerate case for the germination gate
("a buried seed still passes it"). A leaf inside a heap of packed soil
therefore reads the same light as a leaf in open air.

Measured, `soilfork mode=bury`: the played bed's plants grown 6,000 frames
with no colony, then every shoot buried in a box eight columns wider than the
plant and eight rows over its top, to the surface, and run 12,000 frames
more. Three arms from one world:

| plant | control: income / light | in **packed soil** | in **stone** (positive control) |
|---|---|---|---|
| tree, 1,233 shoot cells | 3.27 / 0.23 | 1.60 / 0.34, growth stopped | **0.27 / 0.05**, starving 67 ticks |
| shrub, 654 | 1.19 / 0.33 | 0.93 / 0.45 | **0.31 / 0.15** |
| herb, 116 | 0.28 / 0.45 | 0.27 / 0.50 | **0.13 / 0.22** |
| grass, 14 | 0.63 / 0.60 | 0.63 / 0.60 | 0.39 / 0.37 |
| grass, 8 | 0.46 / 0.59 | 0.23 / 0.59 | dead |
| three shrub seedlings, 11–23 cells | 0.005–0.013 | unchanged | senescent at 267 starving ticks, light 0.10 |

Income is the plant's own `income` against its `maintenance`; light is the
field at its shoot cells over the lamp's maximum. Packed soil changes the
light not at all — it *rises* for several plants, because the fill stops
neighbours growing over them — and what it does cost is room: the tree in
packed soil kept 1,219 shoot cells while the control grew to 3,487. Stone
darkens every plant and the largest begin to starve within 12,000 frames
(the starvation clock is 200 organism ticks of 45 frames, so 9,000 frames of
sustained deficit, and the tree is 67 in). **The first version of this
harness buried to a fixed twelve rows in a box one column wider than the
plant, and its stone arm changed nothing on any grass** — the field's
transmission is a mean over each 8x8 block's eight columns, so a
three-column pillar passes five-eighths of the lamp. The positive control
caught the instrument; the numbers above are from the version it passed.

So the owner's candidate 1, *buried plants should die from no sun*, is not
what happens and is not reachable by any rule about the plant. A plant
engulfed by a mound stands inside it alive, at the income its leaves had,
for ever: a fossil in a scar, and a binary — thriving or entombed, with
nothing between.

### 2c. "Individual pieces of soil floating in the air" — survives at scale

Hanging spoil here is a packed cell above the original surface with no path
down to the bank through other ground (plant cells deliberately not
counting as ground, so spoil posted into a canopy is hanging). With the crumb
rule on:

| bed | packed above the surface | hanging | in pieces |
|---|---|---|---|
| seed 3, 200,000 frames, 82 ants | 541 | **91** | 43 |
| seed 1, 300,000 frames, 3,182 ants | 3,124 | **1,074** | 165 |

About two cells a piece on the small colony, six and a half on the big one.
The crumb rule is doing what it was built for — nothing single hangs — and
the lattice the 2026-08-31 card showed is what is left: spoil on spoil,
spoil on leaves, pairs and triples with three contacts each. A third of the
big colony's above-ground spoil has no ground under it.

### 2d. "They create cool towers" — survives, and on seed 1 it is the whole bed

The 3,182-ant colony has hollowed the bank to **6,541 cells of roofed void**
and heaped the spoil into a mound at least 48 rows tall (the census caps at
48) with plants standing on it and a flooded gallery pooling at its foot.
That is the owner's playtest report reproduced, and it is the sample the
card is taken from. On seed 3 the same rules give a shallow honeycomb along
the whole surface — packed lining in every 32-column bin of the bed, 8 to
111 cells each — under a crust of spoil one to three cells thick. Same flag,
two silhouettes; which one a bed gets is the colony's size.

## 3. The option space, each costed

For each: what the player sees / cost per frame / what it breaks or
reallocates / whether it is a dial, a material property or a gene. Dead ends
grepped for every mechanism named; the ones that bind are cited.

### 3a. Leave the towers (the owner's candidate 4)

*Sees:* what they see now — a heap that stands, a lattice at the top of a
worked bank, spoil in a crown. *Cost:* none. *Breaks:* nothing; the crumb
rule already removes the free-floating single. *Kind:* the zero of the dial
in 3b. **Not rejected.** It is `CLAUDE.md`'s second law — the verb delivers
something visible — and it is kept as the end of the range rather than
argued away.

### 3b. Dumped spoil weathers back to loose soil (the owner's candidate 3, brief 3's mechanism) — **recommended first**

*Sees:* a mound that cones at the angle of repose over a session, a lattice
that comes down piece by piece as its cells go loose and fall, a crater at
the shaft mouth that the colony re-digs. *Cost:* one decay site per dumped
pellet — 1,789 on seed 3 at 200,000 frames, 36,635 on seed 1 at 300,000 —
each checked once per 200 frames on the scheduler's decay channel, which is
the same channel litter already runs on and proportional to decayable
matter, not to the world. Nothing per cell per frame. *Kind:* two material
properties (`decays_into`, the damp and dry chances) — and the chances are
the dial, exposed in the lab's parameters like every other material rate.

**Staged, not built**: `soilfork mode=fork` forks the played bed from one
world state and lets exposed packed cells above the original surface revert
on the decay channel's schedule. What it costs the nest, 6,000 frames after
the fork:

| bed and arm | packed above | hanging | roofed void | roofed vs keep |
|---|---|---|---|---|
| seed 3, keep | 563 | 98 | 242 | — |
| seed 3, weather at 0.05 per check | 293 | 29 | 195 | **−19%** |
| seed 3, weather at 0.005 per check | 506 | 77 | 240 | −1% |
| seed 1, keep | 3,144 | 1,073 | 7,190 | — |
| seed 1, weather at 0.05 per check | 1,262 | 404 | 5,056 | **−30%** |

The loss is the crust running back into the shallow galleries beneath it —
on seed 3 the bins that lost void are the ones with workings a cell or two
under the surface. At a tenth of the rate the mound still loses a tenth of
its hanging pieces in 6,000 frames and the galleries lose nothing; over a
session (300,000 frames) an exposed cell at 0.005 per 200-frame check has a
half-life of about 28,000 frames, so the mound cones within a session and
the crust goes in a few. **That is the range to sweep, and the number to
gate the default on is roofed void lost over a session, not over 6,000
frames** — the harness prints it, and `latecensus` carries the same column.

**The correction to brief 3, and it is the reason this report is upstream of
it.** Brief 3 scopes the weathering as `packedsoil` gaining `decays_into:
soil` with the dump verb scheduling the site and the lining scheduling
nothing. That is not how decay sites are scheduled. `World::end_step` scans a
chunk on its awake-to-settled transition and schedules a decay site for
**every cell whose material has a `decays_into`** (`world.rs`, the
`decays_into.is_some()` test in the settle scan) — it was moved there because
a site scheduled at creation strands when its cell moves (`open-bugs-handoff`
§0e). Give `packedsoil` a `decays_into` and every lining cell in every chunk
that settles is scheduled, and the nest weathers from the inside. Two ways
out:

- **A second material, `spoil`** — what a dug cell becomes in the jaws, as
  `packedsoil` is what a wall becomes under the tamp. Same density, same
  resistance (an ant cuts back through either), same `water_capacity`, same
  palette, `self_supporting: true`, plus `decays_into: soil` and its two
  chances. The dig branch already rewrites the pellet's material through
  `packs_into`; it wants a sibling field on `soil` naming the pellet
  material (`spoils_into`), and `line_burrow` keeps `packs_into`. The
  lining and the heap are then told apart by *what they are*, which is
  `CLAUDE.md`'s "state the difference as data" — the same rule that put
  `self_supporting` on a material instead of inferring a wall from shape.
  **Recommended.** Cost: one asset, one material field, one line in the dig
  branch.
- An exposure gate in `decay.rs` — a packed cell decays only with three rows
  of clear air above it, `SPOIL_HEADROOM`'s own predicate. It works (a
  gallery is one to three cells tall by construction) and it is a rule
  about geometry that has to be kept in step with the dump's. Second choice.

Either way the *lining* never weathers, which is what keeps this from being
the wet rule the owner declined in a new costume: a roof does not fall in; a
heap on the surface goes back to being ground.

*Breaks / reallocates:* the roofed-void figure in every late-game census
(the harness above prints the loss); the pellet's own moisture must ride
across the conversion (`aux == 0` on a `Powder` means dry — the crumb rule's
own note); and `latecensus`'s `packed_above` column stops meaning "the
mound" and starts meaning "the mound not yet weathered", so it wants a
`soil_above` beside it, which `soilfork` already prints.

### 3c. Placed soil does not stay (the owner's candidate 3, literal form) — rejected, twice measured

*At the drop* it is the recorded dead end: loose spoil set down anywhere near
a gallery runs into it — roofed void **2/1 and 2/3 against 54/58** tamped on
`labnest`. *As an end state* — every packed cell above the surface made loose
at once, which is what "doesn't stay" converges to — measured here from the
same forks as 3b:

| bed | roofed void, keep | roofed void, loose | hanging, loose |
|---|---|---|---|
| seed 3 | 242 | 173 (**−31%**) | 12 |
| seed 1 | 7,190 | 4,353 (**−39%**) | 57 |

It does solve the floating pieces — 1,073 to 57 — and it takes a third of
the nest with it in 6,000 frames, on the big colony and the small one alike.
The owner's own caveat ("may not fully solve the problem") is right in the
other direction: it over-solves it. 3b at a low rate is this with a clock on
it, and the clock is the whole difference.

### 3d. Buried plants die, rot to soil, new plants grow through (the owner's candidate 1) — **recommended second, as a light-model property**

*Sees:* a plant a mound closes over goes yellow, sheds, and thins away into
the heap over a few thousand frames (`rot_remains` at the species half-life —
graded, the owner's own ruling on plant death), and its litter rots to soil
inside the mound. A plant half-buried loses half its income and holds at a
smaller size. A seed under a cell of soil waits, as the field's own comment
says it should and today does not. *Cost:* **zero per frame.** The
`rebuild_blocked` scan already reads every cell's material to find `Solid`
and `Plant`; an `opaque: bool` on `Material` is one more field in the same
match. *Kind:* a material property, default on for `soil`, `packedsoil`,
`spoil`, `sand`, `snow`; off for `litter` (insubstantial — a leaf drift is
not a roof) and every liquid.

*What it reallocates, and why it is second.* This is the change `CLAUDE.md`
warns about: a term whose *expression* changes moves every constant
calibrated against the old expression, and here three are:

- **`Germinate`'s light gate** (herb 0.1, grass 0.08 of maximum). Today a
  seed buried by drifting soil passes it; with opaque powder it waits until
  uncovered. The seed bank is the colony's staple and the late-game
  report's whole argument is measured against it — so the bank's size and
  germination rate (26,768 seeds borne, 16% germinating, unfed seed 2) must
  be re-taken with the flag on before anything downstream of it is
  believed.
- **The ant's `LightHere` input**: an ant underground would read dark for the
  first time. `ant.ron` does not name the input, so the shipped weights may
  be zero; check, and if any lineage's brain has evolved on it, its
  behaviour changes.
- **The outdoor game.** Every cave under soil is lit today and would be
  dark, which `field.rs` says it wants ("caves stay dark") and nobody has
  looked at. `shade_factor` for moss reads the same field.

None of these is a reason not to do it; all three are why it is a lane of its
own with `latecensus` and `labforage` run before and after, not a rider on
3b. Positive control: `soilfork mode=bury arms=none,packed` with the flag on
must put the packed arm where the stone arm is now.

*Not recommended instead:* a per-leaf rule ("a shoot cell with ground above
it earns nothing") in `plant.rs`. It is cheap — one `get` per shoot cell per
organism tick — and local, and it is the wrong shape: it answers "am I under
dirt" per cell with a neighbour test where the field already answers "how
much light reaches here" for everything at once, and it would leave the
germination gate and the ant's eye still seeing through ground. Two readers
of one question is how the wet rule got argued at two thresholds.

### 3e. A nest of cavities plants can root into, with the moisture fixed (the owner's candidate 2) — no build

The premise was moisture at the nest. Measured: the mound is as wet as the
bank beside it or wetter (0.29 against 0.22 of capacity on seed 3, 0.65
against 0.62 on seed 1, neither moved by weathering). Water is not blocking roots or seeds in spoil; it blocked pips
on the `nest` patch, which holds none, and that is closed (#328). And the
cavities are already rootable — a root pays 1.19x to enter a packed wall and
does, which is why `packedsoil` keeps `water_capacity: 1000` (its own header:
"a wall is still ground: plants root through it"). What plants do not do at a
nest is *survive the colony*, and that is round 29's sower lane, not soil.

### 3f. Decomposed soil is better to root in than bank soil — declined

Priced in the late-game report §6 and declined there: a per-cell nutrient
scalar fed by decay and drawn by roots is a moisture-pass-scale cost (the
perf line's second-largest block) and a new economy every plant species must
be re-derived against. This report adds one fact against it: burial in packed
soil does not even reduce a plant's income (§2b), so there is no *deficit* a
richer soil would be repairing. The visible half — the anthill as the
richest ground — is bought by 3b plus the sower's pips at the door.

### 3g. The gated crumb rule — already shipped; extending it is not worth a lane

The brief's most promising direction is the live rule (§1). What could be
built on it is a wider reach — "nothing beneath *within N rows*", or four
contacts instead of three — and both are rules about the shape of a lattice,
which is the family that produced four support models before
`self_supporting` was made data. 3b removes the lattice by time instead of
by shape and costs no per-cell test. Not filed as a dead end, because nothing
was tried; filed here so the next session does not re-read the brief's
sentence as an open lead.

### 3h. The two soils converge over time — this is 3b

"Converge" is weathering by another name: packed → loose is the only
direction that has a physical reading (worked ground slakes; tilth does not
compact itself under nothing). The other direction — loose soil that settles
long enough becomes self-supporting — would make every bank a wall and every
gallery in loose soil permanent, which is the freeze `packedsoil.ron` refused
`reinforces_powder` for.

## 4. Recommendation, and the first thing to build

**First: brief 3, re-cut.** Room-per-ant as scoped; weathering as a second
material `spoil` rather than a flag on `packedsoil`, with `decays_into: soil`
and the two decay chances as lab-exposed dials, default set by a sweep of the
rate against **roofed void lost over a session** on both seeds above, and the
sweep's table in the PR. Deliverables that already exist for it:
`soilfork mode=fork` is the staged version and its `weather` and
`weather_slow` arms are the two ends of the sweep; the positive control is a
mound placed with no colony coning within N frames (brief 3's own); the
card is the seed-1 heap before and after.

**What it would take:** `assets/materials/spoil.ron` (copy `packedsoil.ron`,
add the three decay fields, keep the header's reasoning); a `spoils_into`
field on `MaterialDef`/`Material` beside `packs_into`, read at one line in
`creature.rs`'s dig branch; the crumb rule and the dump predicate unchanged
(they read `self_supporting`, which `spoil` keeps); `latecensus` and
`soilfork` counting `spoil` with `packedsoil` in `packed_above`; `wiki/
powders.md` and `wiki/ants.md` updated in the same change. Half a day.

**Second: the `opaque` material property (3d), as its own lane**, with the
three re-derivations named in 3d budgeted as part of it, and `soilfork
mode=bury` as its gate.

**Kept as the zero of the dial, never removed:** the towers.

**The card** (`20260912T051541289Z-3b03d3`) asks the fork the 2026-08-31
card never had answered, on the bed that actually grows the heap: seed 1 at 300,000 frames with the two
soils tinted apart, and the same heap 6,000 frames after each rule. The
counts are in `meta`. *"Live with the heap and its hanging pieces, let it
weather over a session, or let it go loose now — knowing what each costs
the galleries under it?"*

## 5. What this rejects, and where it is filed

Three entries in `Reports/dead-ends.md`, each with the condition its
rejection depends on: loose spoil as an end state (3c, measured); burial
killing a plant under the current light model (2b, measured with a positive
control — re-test the moment `opaque` exists); and the moisture premise of
the cavity-nest candidate (3e, measured). Brief 3's `decays_into`-on-
`packedsoil` scoping is not filed there — nothing was built — but the
coordinator note carries the correction.

## 6. What this contradicts

- **The brief's "a correctly gated version is untried."** It is the shipped
  rule; `update.rs`'s crumb comment records the gate as "doing the work".
- **The owner's "creates a ground that new plants don't grow in."** Three
  readings against it in §2a. The bare patch is traffic.
- **The brief's candidate 1 as a plant rule.** Burial is invisible to a plant
  because it is invisible to the light field; no rule in `plant.rs` can see
  it without reading the neighbourhood itself, and that is the wrong reader.
- **Brief 3's scheduling of the decay site from the dump verb.** Sites are
  scheduled by material at chunk settle, so the lining weathers unless the
  pellet is a different material.

## 7. The instrument

`examples/soilfork`, two modes, documented in `Reports/instruments.md`. The
two things it learned that generalise: a burial or shading claim must be
checked against the light field's 8x8 block and not the cell, because the
first version's stone arm did not move and the harness was the reason; and
a crop centred on a colony's founding column can miss the colony entirely —
a colony digs where its ants are, so the crop centres on the packed cells'
centroid and a 32-column profile prints where the workings are. Deterministic
at `RAYON_NUM_THREADS=1`; two runs of seed 3 to 200,000 frames agreed to the
cell.
