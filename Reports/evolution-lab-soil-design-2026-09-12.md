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
`examples/soilfork`. Lane U of round thirty. The first cut landed as PR
#346; this revision folds in the coordinator's reframing of the same day —
**the direction is open** (the owner: *"I don't know if we want more loose
or more fixed, but I want the agent to explore the options and downstream
effects"*), the owner's four issues are the criteria every option is judged
against, and the towers are a preference to weigh rather than a rule. Brief
3's weathering half was cut; this report owns everything about what soil is
and does, and a build lane is cut from whatever it recommends.*

## The owner's four issues, and what the measurements say about each

These are the acceptance criteria, in the owner's words. Every option in §3
is scored against them in §4.

| # | the issue, as the owner put it | survives measurement? | what is actually happening |
|---|---|---|---|
| 1 | *You cannot tunnel through decomposed soil and have it stay; bank soil dug and placed does stay* | **partly, with a different mechanism than "two soils"** | Decomposed *soil* is `soil` and lines a tunnel exactly as the bank does — there is no second soil to behave differently. What cannot be tunnelled is **what has not yet become soil**: litter, carrion, ash, dead wood. None of them has a `packs_into`, so a gallery cut through them is never tamped and falls in five frames (§2e). On the lab bed that layer is thin — 81–123 cells of litter over 650–987 of rot-made soil — so the asymmetry is real but small there, and larger wherever leaves pile deep. It is between *tamped* and *untamped*, not between two soils |
| 2 | *Individual pieces of soil floating in the air* | **yes, at scale** | The single hanging pellet is already removed by the crumb rule (live since 2026-09-01). What remains is spoil on spoil and spoil in branches: 91 cells at 82 ants, **1,074 in 165 pieces** at 3,182 ants (§2c) |
| 3 | *Spoil engulfs plants* | **yes — and the plant lives on inside** | 1,391 shoot cells enclosed on the big colony. Burial cannot kill: soil casts no shade in this engine, so a plant inside a heap keeps its full light and income; only room stops it growing (§2b). On a small colony most of what stands above the old ground line is the plants' own rot, not spoil (§2f) |
| 4 | *The mound is ground new plants don't grow in* | **no** | The bare patch is there before the mound is, plants stand on the biggest heap in the report, and the heap is as wet as the bank beside it (§2a). The bare patch is the colony eating seedlings |

**And the preference:** *"they create cool towers … I like them, but don't
know if they are worth the problem they cause."* Treated as an input to
weigh. Two of the options below remove the towers and say so in those words.

**The recommendation, in one paragraph, with its trade** (the full case is
§4): let *dumped* spoil weather back to loose soil at a rate that is a dial
whose zero is today's towers, as a second material so the tunnel lining never
weathers; and make soil cast shade, as a material property, so a buried
plant dies the graded death the engine already has. **The trade:** at any
rate above zero the towers come down over a session — that is what
weathering *is* — and every loose cell that lands in a shallow gallery closes
it, measured at 1% of the nest per 6,000 frames at the slow end and 30% at
the fast end. Issue 1 is a third, separate fix: give the decomposing
materials a `packs_into`, or accept that a drift of leaves is not ground you
can tunnel. Neither direction — more fixed, more loose — wins on its own:
the "fixed" options (§3i–3k) close issues 1 and 2 and make the whole bed a
wall; the "loose" option (§3c) closes 2 and 3 and takes a third of the nest
with it. The recommendation is the middle, and the dial is what lets the
owner move along it in play rather than in a rebuild.

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

So the asymmetry between *bank* soil and *spoil* is exactly one bit on one
material, and every consequence below follows from the two things that bit
does: the lining stands, and so does everything else the colony tamps.

**But the asymmetry the owner noticed is a different one, and only part
of it is a soil.** "The soil that is created when plants, leaves, other
things decompose" ends as `soil` — the same material as the bank, with the
same `packs_into`, so a colony lines a gallery in it exactly as it does in
the bank. What it *cannot* line is anything still on the way: `litter`,
`deadleaf`, `deadwood`, `ash`, `corpse`, `log`, `windfall` and `pip` have no
`packs_into`, so `line_burrow` cannot tamp a wall in any of them and a
gallery cut through a leaf drift is loose on every side. How much of the
forest floor is in that state is a matter of rate and yield: litter rots
fast (a damp cell has a half-life of about 400 frames) and only one cell in
twenty becomes soil (`decay_yield` **0.05**, set because 1:1 rot *"buried the
owner's trees to their crowns"*), so the standing layer of untampable
matter is thin on the lab bed (§2f) and deep only where leaves fall faster
than they rot. §2e states the mechanism; §3i costs the fix.

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

### 2e. "You cannot tunnel through decomposed soil and have it stay" — survives, and it is the litter

Read from the assets rather than run, because the reading is decisive:
`packs_into` exists on `soil` alone (§1), so the only material a colony can
line a gallery in is soil — bank soil and rot-made soil alike, since both are
the one material. Litter, carrion, ash and dead wood cannot be lined at all.
`burrow_probe`'s own measurement then applies to them unchanged: an unlined
gallery in a `Powder` is 63–82% open one frame later and gone in five. **The
number that would settle it by measurement rather than by reading** is a
`litter` arm on `burrow_probe` with `line_burrow` applied — not built here,
one instrument being the budget — and the prediction is the bank's 100%
against the drift's 0% within five frames.

There is a second, smaller mechanism worth naming: a gallery is one to three
cells tall and the rot-made layer above the old surface on the lab bed is
one to two cells deep (§2f), so a tunnel cut *along* it has no roof to tamp,
whatever the material. Neither mechanism is "decomposed soil is different
ground". So issue 1 survives in part: the owner is comparing tamped ground
with untamped ground, and the untamped ground is the leaves and bodies that
have not become soil yet, or a drift too thin to roof. "Behave the same" has
two readings — make the drift tampable, or make the bank untampable — and
only the first keeps the nest (§1's 2/1-against-54/58 measurement is the
second).

### 2f. Who built the heap — the ants, or the plants

`soilfork` counts three things standing above the original surface: tamped
spoil, loose soil, and decomposing matter still on its way to being soil.

| bed | packed spoil | loose soil (rot-made, or weathered) | litter and other rotting matter |
|---|---|---|---|
| seed 3, 100,000 frames, 89 ants | **88** | 650 | 81 |
| seed 3, 200,000 frames, 82 ants | 541 | 987 | 123 |
| seed 1, 300,000 frames, 3,182 ants | **3,124** | 669 | 13 |

On a colony of a hundred, **most of what stands above the old ground line is
the plants' own doing** — rot-made soil outnumbers spoil seven to one at
100,000 frames and two to one at 200,000, with a thin skin of litter over it
— and "the ants built this heap" is partly wrong. On a colony of three
thousand the ants dominate five to one. This reframes issue 3 for the
ordinary bed: a seedling on a small colony's bed is buried by the forest
floor more often than by spoil, which is the burial `decay_yield` was set to
slow. And it reframes issue 1: the rot-made ground is overwhelmingly `soil`
already, so the untampable fraction is small here. The remedy for both
burials is the same one (§3d); the remedy for the untampable fraction is
§3i, or accepting it.

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

**The plain consequence, stated because it is bigger than this question:
depth does not exist for the light model except through rock and
vegetation.** A seed under fifty cells of soil reads full daylight. A root
ten rows down is as lit as a leaf. A cave dug into a hillside of soil is lit
to its floor; only one cut into stone is dark. Nothing in the engine has ever
been calibrated with ground that casts shade, which is exactly why this is
the biggest risk in the report and not the smallest change in it.

*What it reallocates, and why it is second.* This is the change `CLAUDE.md`
warns about — the `phototropism_dir` case: the prescribed repair, correct,
and reproduction went to **zero** at inherited constants with every gate
green but one. A term whose *expression* changes moves every constant
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

**Is the re-derivation affordable?** Yes, as one lane, on these terms, and
not otherwise. The constants are enumerable — one `light_threshold` per
species (seven files: conifer, creeper, grass, herb, scrambler, shrub, tree),
`shade_factor`'s floor, and whatever `LightHere` weight any shipped brain
carries — and the measurement that says whether they moved already exists:
`latecensus` and `labforage` on the played bed, three seeds, flag off against
flag on, read at the seed bank (seeds borne, germinations, bank size) and at
the stand. If germinations move by less than the seed-to-seed spread the
gates are re-derived by construction; if they move by more, the germination
threshold is re-set so a seed *on* the surface passes as it does today and
only a seed *under* soil waits, which is the intended change. **What makes it
unaffordable is skipping that measurement**: the flag with inherited
constants is the regression `CLAUDE.md` names, and nothing downstream would
notice, because every gate would stay green while the bank quietly shrank.
So the lane is the flag *and* the three-seed paired census, or it is not
scoped. Positive control: `soilfork mode=bury arms=none,packed` with the flag
on must put the packed arm where the stone arm is now.

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

### 3i. Make the drift tampable: a `packs_into` on decomposing matter (issue 1's direct fix)

*Sees:* a colony can tunnel a leaf drift or a carcass pile and the tunnel
stays; a gallery in litter is lined like one in the bank. On the lab bed the
untampable layer is thin (§2f), so this buys little there and more under an
outdoor forest. *Cost:* none per frame — the
dig and `line_burrow` already read `packs_into` off the material. *Kind:* a
material property, one line per asset. **The downstream effect is the whole
decision, and it is not free:** litter is **food** (480 J a cell, the
colony's second larder), and a tamped litter cell would become `packedsoil`
— the colony converting its own larder into wall as it digs. Two shapes:
`litter.packs_into: packedsoil` (simple; leaf mould pressed into ground is
what a real nest wall in a drift is; the larder loss is the cost) or a
`packedlitter` that stays edible and self-supporting (keeps the food, adds a
material and a food class). The first is the recommendation if issue 1 is to
be closed at all, with the larder loss measured on `labforage` before it
ships. **Or accept that leaves are not ground**, which is what the engine
says today and is physically defensible — a burrow in a leaf pile is not a
thing. That is a legitimate answer to issue 1 and the cheapest, and it should
be put to the owner as one.

### 3j. Loose soil sets when it comes to rest (the "fixed" direction, one rule for both soils)

The coordinator's candidate: the distinction is not *which* soil but *has
this cell come to rest*. Tilth falls until it lands, then sets; a pellet is at
rest the moment it is set down; nothing sets in mid-air because nothing is
at rest there. *Sees:* every hole in every soil holds — a dug pit keeps
vertical walls, a gallery needs no tamping, the forest floor is a wall once
it has lain still. *Cost:* the cheap form is a conversion `soil ->
packedsoil` on the chunk-settle scan that already runs for decay, so nothing
per cell per frame; the expensive form is a per-cell rest counter, and there
is no room for one (`aux` is moisture on a `Powder`). *Kind:* a rule, with
the settle interval as its dial. **Downstream, traced:** the lab's bed is
built settled, so it becomes `packedsoil` on the first settle — the whole
bed at 0.95 resistance and 1.19x root cost; repose stops existing after
first rest, so litter drifts, pits and spoil never cone again and every heap
is permanent; the crumb rule keeps the single hanging cell out, and the
lattice (issue 2) is made *legal everywhere*, since anything that lands on
anything sets; in the outdoor game every landslide freezes where it stops
and every dug hole keeps its walls with no ant in it, which is the "roof
that never falls" the four support models were built to avoid. It closes
issue 1 completely and issue 2 not at all, and it is the freeze
`packedsoil.ron` refused `reinforces_powder` for, applied to the world.
**Not recommended**, and it is the clearest statement of what "more fixed"
costs: a ground that can never slump is a ground with no middle.

### 3k. Rot-made soil is born packed (`decays_into: packedsoil`)

The narrow form of 3j: only what the plants make sets, the bank stays loose.
*Sees:* the forest floor becomes worked ground as it forms; a tunnel through
it holds without lining. *Cost:* none per frame; one field on each
decomposing material. *Kind:* material property. **Downstream:** it does not
touch issue 1's actual mechanism (the drift is litter, which does not decay
into anything for most of its life) — it only changes the 5% that becomes
soil; a dead tree's remains become a packed lump that never cones; and
`latecensus`'s `packed_above` stops meaning spoil. It buys almost nothing
issue 1 wants and costs the drift its repose. **Not recommended.**

## 4. Recommendation, and the first thing to build

**The scorecard**, every option against the owner's four issues, the
preference, and the two costs that trade against them:

| option | 1 tunnel the drift | 2 floating pieces | 3 engulfing | 4 sterile mound | towers | galleries lost | per frame |
|---|---|---|---|---|---|---|---|
| 3a leave it | no | no (91 / 1,074 hang) | no | not an issue | kept | 0 | 0 |
| 3b weather dumped spoil, slow | no | slowly (−20% in 6k frames) | over a session | — | **removed over a session** | −1% / 6k frames | ~0 |
| 3b weather dumped spoil, fast | no | mostly (404 of 1,073) | faster | — | **removed in thousands of frames** | −19% / −30% | ~0 |
| 3c loose now | no | yes (57 of 1,073) | yes | — | **removed at once** | **−31% / −39%** | 0 |
| 3d soil casts shade | no | no | **yes, graded** | — | kept | 0 | 0, plus a re-derivation lane |
| 3i drift tampable | **yes** (the litter fraction) | no | no | — | kept | 0 | 0, larder converts to wall |
| 3j all soil sets at rest | **yes** | worse (lattice legal everywhere) | no | — | kept, permanent | 0 | 0, repose gone from the world |
| 3k rot born packed | barely | no | no | — | kept | 0 | 0 |
| 3e / 3f / 3g | — | — | — | already true | — | — | — |

No single option closes more than two of the three live issues. **The
recommendation is 3b slow plus 3d, with 3i put to the owner as a yes-or-no**,
and it removes the towers — over a session at the slow end of the dial,
never at its zero. That is stated in those words because it is the one thing
he likes that the answer costs.

**First: weathering.** A second material `spoil` rather than a flag on
`packedsoil`, with `decays_into: soil` and the two decay chances as
lab-exposed dials, default set by a sweep of the rate against **roofed void
lost over a session** on both seeds above, and the sweep's table in the PR.
Room-per-ant (#359) is already on `main` and is untouched by this.
Deliverables that already exist for it:
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

**Third, a question rather than a build:** issue 1. Either the decomposing
materials get a `packs_into` (3i, with the larder loss measured first), or
the answer to *"you cannot tunnel through decomposed soil"* is *"that is
leaves, not soil, and it is right that a burrow in leaves falls in"* — and
the owner picks.

**Kept as the zero of the dial:** the towers. Any setting above zero takes
them down over time; that is the trade, and it is the owner's to make in
play.

**The card** (`20260912T181533048Z-7a5aab`) asks the fork the 2026-08-31
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
  pellet is a different material. (Brief 3's weathering half was cut on the
  same day; the finding stands for whoever builds it.)
- **The brief's "the two soils" as the asymmetry.** Rot-made soil *is* bank
  soil, and on the lab bed it is most of what the plants leave above the
  old surface. The ground the owner cannot tunnel is what has not become
  soil yet, and the property it lacks is `packs_into`, not
  `self_supporting`.
- **The card's first posting** (`20260912T051541289Z-3b03d3`) was overwritten
  in the shared queue by another lane's card under the same id, and the
  owner's comment recorded against it is about that card; it was re-posted
  as `20260912T181533048Z-7a5aab`.

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
