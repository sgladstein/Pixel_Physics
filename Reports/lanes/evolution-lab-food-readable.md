# Lane A, round 36 — the food economy, made readable

*Branch `claude/evolution-lab-food-readable`. Brief:
[`../evolution-lab-round-36-brief-2026-09-14.md`](../evolution-lab-round-36-brief-2026-09-14.md)
§"Lane A". What round 35 shipped is
[§4 and §5 of its record](../evolution-lab-round-35-2026-09-14.md).*

**The three verdicts this lane exists to answer**, in his words:

| card | his words |
|---|---|
| `…063034460Z-989679` road + harvest, planted bed | *"No i cannot really tell. the amber hatch it bad. is it too much to track actual paths and make trail?"* |
| `…063101470Z-06554c` road + harvest, far larder | *"Why is the amber hatch drop like a huge box?"* |
| `…063026275Z-e7b8f3` the FOOD page | *"Nope. I don't understand what these visuals are trying to tell"* |
| `…063118804Z-711e5a` empty-handed walking | *"both is interesting"* — keep both arms, settled |

---

## 1. Reproducing his reading, before changing anything

The brief's instruction, and it paid immediately: **render the two channels
separately.** Four arms, one bed, one seed, one frame — `played_bed` seed 1,
warmed 12,000 frames and captured at 17,600, mode swapped between runs and
nothing else:

```
./target/release/examples/foodroad scenario=played_bed seed=1 \
    start=12000 frames=5600 every=5600 mode={off|road|harvest|both} \
    png_dir=… up=4 crop=120,130,256,60
```

Identical world in all four (the overlay only reads), so the pictures differ
by exactly the thing under test. Counters, identical across the three live
arms and the number that says it fired at all:

```
laden steps 622 | unladen steps 657 | harvest bites 980 | face value taken 141,720
365 road cells, 36 harvest tiles held at the captured frame
```

**Finding 1 — the road works, and the hatch was burying it.** On the `road`
arm alone the surface carries a legible red-orange line with blue stretches
running along it: *carried this way, walked back that way*. On the `both` arm
that same line is underneath an amber checkerboard **in an adjacent hue**, and
it is gone. Nothing had to be built to see this; it is what the brief
suspected, and it means a large part of the repair is **what to draw**.

**Finding 2 — the hatch paints empty sky, and that is the "huge box".** Every
one of the 36 live harvest tiles sits at tile row `y=156` or `y=164` — the
surface band. A harvest tile is 8 cells square; at the surface that is about
one row of ground and **seven rows of air above it**. The wash covers the
whole tile, so what is drawn is a rectangle whose top edge is the *tile grid*
and not the ground: a hard-edged 8x8 block hanging over the terrain, brightest
tiles nearly solid at `HARVEST_MAX_COVER = 0.8`. Dropped like a huge box is a
literal description of it.

*(Kept here as a measurement rather than a guess: the tile rows come from
`foodroad`'s own `SOURCES` line, which prints world coordinates.)*

## 2. The repair: the wash marks the skin of the ground, and nothing else

Two clips, and they are one change — **either alone leaves a box.**

- `FoodRoad::harvest_on_ground` refuses **open air**. That removes the straight
  top edge hanging over the terrain.
- `food_road::near_open_air` refuses ground more than `HARVEST_SKIN` = 3 cells
  under the exposed face. Without it the block is simply anchored to the
  *bottom* of the tile instead of the top — the same box upside down, which is
  what the air-only clip actually drew on `far_larder`, where the surface is
  bare soil and the tile's lower half is solid.

`HARVEST_MAX_COVER` went 0.8 → 1.0 in the same change and for the same reason:
the fifth held back existed so a tile the colony lives off still showed the
ground *underneath* it, and clipped to the skin the wash no longer covers what
it points at. Holding it back only made the strongest patch in the bed read as
speckle. `ramp`'s floor keeps the middle — the quietest live patch is still 18%
dots.

**What was ruled out by measurement, not by argument:** *shrinking the tile*.
At `tile=4` the sheet still draws boxes in the sky, only smaller; at `tile=1`
the channel vanishes into the road entirely and says nothing. **The box is the
air, not the size** — which is why the fix is a clip and `tile` stays at 8,
where the region reading lives.

Guard:
`render::tests::the_harvest_wash_marks_the_skin_of_the_ground_and_not_the_sky_over_it`
— sky unchanged, skin changed, buried ground unchanged, and a fourth arm that
**puts the fault back** (`harvest_on_ground = false`) and requires the sky pixel
to move, so the first three cannot be green because the wash never fired.

**Card `20260914T193144118Z-d8e147`** — *"The amber hatch, no longer a box"*,
posted 2026-09-14 19:31Z, before/after frame sequences on the planted bed.
Verdict: *pending*.

## 3. "Is it too much to track actual paths?" — no, and it never was

**The road was already tracking actual paths.** It marks the exact cell an
animal stands on, every tick its `moves` counter advances. What it was doing
wrong is **forgetting them after ten seconds**: `road_half_life` was 600
frames, so what reached the screen was the last few hundred steps anybody
took, drawn as a scatter of specks along the surface. Raised to **3,600** — a
minute, which is already the harvest map's own half-life — the same run on the
same seed draws one unbroken line, red-orange where they carried and blue
where they came back empty.

Measured on the played bed, seed 1, the overlay armed at frame 12,000 and read
at 20,000: **279 road cells held at ten seconds against 1,024 at a minute**,
from an identical 995 laden and 887 unladen steps.

**And the longer memory is free**, which is the half of the answer he actually
asked for. `examples/foodroad cost=4`, `RAYON_NUM_THREADS=4`, arms alternated
inside each run and the order swapped every round:

| bed | memory | road cells held | delta over its own `off` arm |
|---|---|---|---|
| settled (dearest — the dirty-rect skip can fire) | ten seconds | 623 | **+0.82 ms** (+62%) |
| settled | no decay at all | 1,165 | **+0.69 / +0.71 ms** (twice) |
| settled | a minute (shipped now) | 1,165 | **+0.73 ms** (+53%) |
| whole running frame, tick **and** draw | ten seconds | 559 | **+0.56 ms** (+11%) |
| whole running frame | a minute | 1,100 | **+0.47 ms** (+9%) |

**The positive control that says the instrument can see drawing work at all:**
the second channel on the same settled bed moves the same delta to **+1.17 ms
(+87%)**, which reproduces round 35's +88% independently.

So **what this view costs is being switched on** — both channels decay every
tick and defeat the dirty-rect render skip — and that is paid whatever the map
remembers. Map size is not a term in it: 87% more road, twice, cost nothing
either instrument could see.

### One confound caught on the way, and it is `CLAUDE.md`'s own tell

The first pricing run reported the ten-second and the never-forgetting arms as
holding **420 and 431 cells** — *identical output across a change that must
have moved something*. `price`'s `off` arm **drops both maps by design**
(`FoodRoad::observe`), so every arm re-warms from nothing, and at the old
1,500-tick warm **every setting of `roadhalf` was priced on the same young
map**. The number was arithmetically correct and about nothing. `warm` now
defaults to 6,000 and the same two arms separate to 623 and 1,165.

`examples/foodroad` also gained **`whole=1`**, which puts the tick inside the
timed span. Without it the running row is a draw-only figure, and this repo has
already had the same field change measure −50% in its own harness and −27%
through the whole of `App::update`.

Guard: `food_road::tests::a_single_crossing_outlives_the_harvest_patch_it_leads_to`
— one cell crossed once must still be road when the harvest patch it leads to
is still lit, with the **fault put back** as a second arm at the old 600.

**Card `20260914T201530498Z-1a387e`** — *"How long should a food road
remember?"*, blind A/B, ten seconds against a minute. Verdict: *pending*.

## 4. Verdicts in

- **`20260914T193144118Z-d8e147`** (the amber hatch, no longer a box) —
  answered **rating 4**, no comment. The box is accepted; nothing is asked
  for.
- **`20260914T201530498Z-1a387e`** (how long should a food road remember) —
  **"Option A. Halves in a minute."** Blind card, `blind_was: [1, 0]`, so his
  displayed **A** is stored item **1**, which is the minute — and he named it
  in the prose as well, so the click and the sentence agree without needing
  the map. **The minute is confirmed as the default and is already shipped.**
- **`20260914T205807355Z-12829d`** (the FOOD page) — answered with a design,
  not a rating. See §6; it overturns §5's chart work.

## 5. The FOOD page: every row was true and none of them was an answer

*"I don't understand what these visuals are trying to tell"* is not a
complaint about a chart type. Reading the page cold, two things are wrong:

- **No row says whether the colony is alright.** It lists what a colony has
  eaten, spent and is holding, and never whether that adds up to a colony that
  can keep itself alive.
- **Neither chart has a scale anywhere on it.** `draw_lines` normalises every
  series to the tallest sample across all of them, and that number appeared
  nowhere — so the same peaked line means 600 J on one bed and 6 on another,
  and a colony collapsing to a tenth of its intake redraws at exactly the same
  height. It drew a shape, not a quantity.

**Both fixes ride on lines that already existed.** Each colony's own first line
now opens `FINDS n%` — the share of everything it has spent that it went out
and found in the world for itself — and each chart prints its peak, top right.
`FINDS` and not `FEEDS`, because `FED` on the row directly below is
mouth-to-mouth and two words a letter apart meaning opposite things is how a
dense page stops being read.

`feeding_itself` excludes three things and each exclusion is load-bearing on a
shipped bed: the **founding grant** (runs out once, and is most of what these
colonies ever have), **scavenged corpse** (84–95% of what they eat), and
**trophallaxis** (the colony's own joules going round again). Counting any of
them would answer *yes* for the whole bed. Against **spend**, not intake:
intake includes the grant, so a colony living on its endowment would score
100% and starve on schedule.

### The regression I nearly shipped, caught by rendering rather than by testing

The first build put the verdict on a **row of its own** plus a page-wide
headline row. Both colonies stopped fitting. Rendering the page from `HEAD`
and from the branch on one bed showed it in two pictures:

| | colony blocks drawn |
|---|---|
| `main` | **2** |
| verdict as its own row + page headline | **1**, and `MORE COLONIES +1` |
| verdict folded onto the block's first line | **2** |

The arithmetic afterwards: the budget is **228 px**, two charts take 108, the
`MORE COLONIES` row is held back at 9, and a colony block carrying the rivals
row is **49** — so two colonies fit in exactly the 98 px left, with nothing
spare. **One extra row anywhere on this page, per block or not, drops the
second colony**, silently, and looks like a design choice. Telling two colonies
apart is what the page is for.

`the_food_page_stays_on_the_screen` now asserts both blocks are drawn at two
colonies, and it was **watched going red** with a deliberate extra row in the
block (`2 colonies must both fit: 1 drawn, overflowed true, page 188 px of
228`). New guard
`finding_your_own_food_does_not_count_the_grant_the_dead_or_a_handout` covers
the three exclusions with a positive control (a colony that foraged its whole
bill reads 100%), the middle band, and the no-spend case.

**Card `20260914T205807355Z-12829d`** — *"The FOOD page, with the question it
never asked"*, before/after. Verdict: *pending*.

### One thing found and not fixed, because it is not this lane's

`examples/labui frames=20000 colonies=2` panics with *"the interface has no
button for RosterCompare"*; at `frames=6000` it does not. Reproduces on
`origin/main`'s `src/lab/ui.rs` (checked by rendering the before-picture with
exactly that file), so it predates this branch. It looks like the compare verb
needing two pinned individuals on a bed whose colonies have died back. Not
filed as a bug — it is a harness fragility, not engine behaviour — but it will
waste somebody's twenty minutes.

## 6. He answered the FOOD card with a design, and the graphs came out

Card `20260914T205807355Z-12829d`, verdict 2026-09-14, quoted in full because
it overturns §5's half-measure and because the diagnosis under it generalises:

> *"We really should have discussed this more. #1 issue is your graphs, (top)
> they are showing how much a colony is eating in unknown scale at a fast pace.
> This is not answering any questions that I would ask. (bottom) it shows what
> a colony is eating at any given time, but again the pace is too fast (if the
> graph is all spikes that isn't useful), the bigger issue is that there is no
> legend for me to know which line is which. In general, I prefer graphs to
> tables, but I don't think they are helping here. Here is my suggestion: top
> page is basically the stats you have (foraged/meat/fed, ate rivals/eaten by,
> etc.) for each colony. Get rid of the graphs. If you click a colony, it goes
> to a more detailed page for that specific colony, which lists what they are
> eating (deadleaf, ant, seed, corpse, etc.) and other detailed info. If this
> is possible, I would love to have another layer, so if the colony is eating
> lots of ants, i can click and see which colony they are coming from, I can
> click flower and see which plants the flowers are being eaten from, etc. I am
> guessing you chose the graphs because these change over time and that is
> valid but we are about what was eaten over the past 2-3 minutes, not the past
> 10 seconds. Maybe we keep all the stats but there is a range options (all
> time, 10 min, 5 min, 1 min, etc) for them."*

**Adding the peak label to the charts was the wrong fix and he named why.** The
charts plotted a window of `SAMPLE_EVERY` = 120 frames, `HISTORY` = 56 times —
**a rate**, where the quantity asked of them was **an amount**. So the picture
was drawing the sampling interval rather than the colony, which is the whole of
*"if the graph is all spikes that isn't useful"*. A scale label makes a
mis-framed picture readable, not right. **The general rule, worth carrying: a
chart of a differenced series answers "how fast", and a question of the form
"how much, lately" wants a number over a span the reader picks.**

**Shipped, to his spec:**

| | |
|---|---|
| top page | every colony's stats, no charts; each colony's line is a `Body::Choice` and opens it |
| colony page | what that colony eats, ranked, in joules, each source in its material's own colour, then spend / loads / trophallaxis |
| `COUNTING THE LAST` | a clickable stop on `RANGES` — 2 MIN, 1 MIN, 5 MIN, 10 MIN, ALL TIME — and **every figure on both layers is read over it** |

Two minutes is the default because it is the span he named, and because all
time is dominated by founding. `Body::Choice` gained a per-row tint in the same
change: a colony is identified by the colour it wears in the box, and a row
that opens it still has to say which colony it is.

**The range needed a ring of its own, which is the one non-obvious part.** The
population ring is 56 samples at 120 frames — **under two minutes** — so it
cannot answer a five-minute question at all, and deepening it would have moved
the ANTS page's chart span. `BooksSnap` is a second, coarser ring: every
colony's accounts and per-colony diet, every 600 frames, 64 deep = **10.7
minutes**, one sample of headroom over the longest stop. The population ring's
`food` and `diet` fields and its four series methods are **deleted**, not left
for a future caller — `CLAUDE.md` on a superseded mechanism.

### Two traps this hit, both caught

**`range_reaches` was wrong in the direction that reads as working.** Written
as *"is the oldest snapshot at or before `now - frames`"*, it says **yes** for
every range on a young box, because `now - frames` saturates to zero and the
first snapshot is trivially inside it — so a ten-minute reading over a
thirty-second box reported itself as a full ten minutes. Correct arithmetic,
different question. It now also requires `now >= frames`, and the row says
`(SO FAR)`.

**The fit guard's fixture stopped measuring the tall block, silently.** With
the rivals row now drawn only when raiding happened *inside the window*, and
`colonised` booking its raids before the first snapshot, every finite range put
the fixture's raid in the past — so the block being height-checked was the
short one. Caught by the guard's own `rivals == named` assertion going red,
which is the assertion added for exactly this. The fixture now pins ALL TIME.

### The third layer, and why it is not here

*"If the colony is eating lots of ants, i can click and see which colony they
are coming from, I can click flower and see which plants the flowers are being
eaten from."* **Neither is buildable on what the engine records today**, and
saying so is cheaper than building the wrong half:

- **Which colony the ants came off.** `ColonyBooks::raided` is a **scalar per
  colony**. With two colonies it is determined by subtraction; with three it is
  not, and the page would be guessing. The fix is small and additive — a
  `HashMap<u32, f64>` beside `raided`, filled at `World::book_raid`, which
  already has both colonies in hand — and it is in `src/sim/world.rs`, the most
  contested file in the repo and not this lane's. **Not done, deliberately, and
  it is one commit whenever it is wanted.**
- **Which plants the flowers came off.** Nothing attributes a harvested cell to
  the organism it grew on; the bite reads a *material*, not an individual. That
  is a real piece of engine work, not a readout.

**What the box can already answer is *where on the ground*** — the harvest map
on `F7`, which is per colony and per tile. Each diet row's note says so, so the
missing layer reads as a pointer rather than as a dead end.

**Card `20260914T221442287Z-1add4b`** — *"The FOOD page, rebuilt the way you
asked"*, both layers. Verdict: *pending*.

