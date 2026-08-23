# Grassfire, and the desert decision

**Lane W, package W2 — Arc E items E5 and E6 of
`Reports/plant-project-review-2026-08-23.md`. 2026-08-23.**

Two subjects, one session, because they are the two halves of "does water
decide anything": E5 asks whether a meadow's wetness decides whether it
burns, and E6 asks whether a desert's dryness decides whether anything can
live in it. Both turned out to hinge on the same thing — **which channel a
rule reads, and whether that channel says anything at that cell.**

---

## Part one — E5, the grassfire

### 1. The standing verdict, split

`Reports/open-bugs-handoff.md` §G carries the owner's verdict in full:

> **"The fire looks bad. Just looks like you are cycling colors. It also
> doesn't spread at all (if we are going to do this, moisture vs dryness
> should play a role."**

Three claims, and they wanted separating before anything was touched. Two
of them had causes nobody had measured, and neither cause was the one the
record guessed at. The record's own guesses were the 0.9 resistance
constant being too weak and the `include_str!` rebuild trap. **Both are
wrong.** The constant was connected and doing exactly what it says; the
binary was fresh.

### 2. "It doesn't spread at all" — a sward is not a fuel bed

The strip speeds already on the record are right and are not about a
meadow: over a *contiguous* row of one material, grass crosses at 1.34
cells/frame (`fire::tests::a_fire_front_crosses_grass_faster_than_
foliage_and_not_at_all_over_soil`). Over a real sward the burnt band was
measured at ~0.12 c/f — a tenth of it — and the charitable reading was "a
patchy meadow is a firebreak".

It is simpler than that. `fire::try_ignite` scans **four neighbours** for
something burning, so what a front can reach is one 4-connected component
of fuel and nothing else. `examples/fire_probe.rs` was built to census
that, and the census is the whole answer:

| founders | grass cells | columns occupied | largest column gap | **4-connected islands** | largest island |
|---|---|---|---|---|---|
| 64 | 927 | 364 / 512 | 6 | **194** | 43 cells (4.6%) |
| 160 | 1,993 | 484 / 512 | **1** | **71** | 321 cells (16.1%) |
| 256 | 1,241 | 262 / 512 | **0** | **24** | 292 cells (23.5%) |

Read the 160-founder row twice. By the column census that sward is
*continuous* — one empty column in a 484-column span. By the census the
ignition rule actually performs it is **71 separate islands**, because
blades in neighbouring columns sit at different heights and share no face.
So a fire lit in one burns that island and stops. Measured on the
64-founder sward (largest island 43 cells): **14 grass cells consumed**,
`alight 0` by frame 300, and the front never left x=53 — one island's worth,
and then nothing for the remaining 2,100 frames.

That is not slow spread. It is **a fire that goes out**, which on a contact
sheet looks identical to a fire creeping — and is the reason the earlier
0.12 c/f figure existed at all: it is the average of one burnt tussock over
the time somebody watched.

This is `CLAUDE.md`'s "which object does this rule evaluate" in a new
costume. The rule evaluates *a shared face*. The thing being asked about is
*a meadow*.

### 3. "Moisture should play a role" — the term was reading a channel that is zero wherever there is fuel

`MOISTURE_IGNITION_RESISTANCE` was 0.9, applied as
`flammability * (1 - field_moisture_at(x, y) / 4.0 * 0.9)`. The paired
measurement — **one grown sward, 1,993 cells, re-wetted to four levels with
600 frames for the field to settle each time, so the fuel is identical and
one number differs** — sampled at the grass cells themselves:

| soil water | humidity **at the fuel** | fuel cells reading **exactly** 0.000 | effective flammability (mean) |
|---|---|---|---|
| 0 (bone dry) | mean 0.000, median 0.000 | 100% | 0.850 |
| 180 (wilting point) | mean 0.023, median 0.000 | **96.8%** | 0.8456 |
| 620 (field capacity) | mean 0.080, median 0.000 | **96.8%** | 0.8347 |
| 1000 (saturated) | mean 0.128, median 0.000 | **96.8%** | 0.8255 |

**Read the median column, not the mean.** For 96.8% of fuel cells the term
reduces ignition by **exactly zero**, at every wetness including saturated;
averaged over all fuel cells it reduces it by **2.9%** at saturation, and
all of that 2.9% comes from the 3.2% of blades that happen to sit in the
soil's own field block. That is why it measured as changing nothing: it was
not weak, it was **blind**.

A band mean is what makes this hard to see and is what a scene report would
print. Sampled over the sward's *rows* rather than at its cells the same
four worlds read 0.000 / 0.041 / 0.142 / 0.230 — monotone, plausible, a 5.2%
span, and describing field blocks the fuel is not in.

The cause is one line in `field::step_diffusion`:

```rust
if tile.is_blocked_local(lx, ly) { continue; }   // stays ambient
```

and one line in `rebuild_blocked`, which marks a block blocked when any
`Solid` **or `Plant`** cell falls in it. A field block containing fuel
therefore never diffuses and holds ambient zero forever. **The presence of
fuel in a block is what makes that block read bone dry**, and a denser
sward reads drier. A band mean hides this completely — sampled over the
sward's *rows* the same worlds read 0.041 / 0.142 / 0.230, monotone and
plausible and describing blocks the fuel is not in.

On the *shipped* build, with flame off and the ground dry — the ablation
that reproduces `main` — the same 160-founder sward burns 262 of 1,993 cells
and stalls at x=110, which is the natural firebreak in that particular
sward. Both figures say the same thing in different sizes: the fire runs out
of touching fuel.

Nothing here fixes `rebuild_blocked`. That flag is load-bearing for light,
heat and pressure, and moss and phototropism read the channel it produces.

### 4. What was built

**One mechanism for the look and the spread, and one for the moisture.**

**`flame` (`assets/materials/flame.ron`) — a `Gas` that is created already
alight.** `fire::tick_burn` licks one into a nearby empty cell while a fuel
cell burns, at a per-material rate (`MaterialDef::flame_into` /
`flame_chance`, unset by default so nothing else in the world changes how
it burns). Being *burning* is what makes every existing piece of fire
machinery apply to it with no special case: it renders on the heat ramp,
`try_ignite`'s neighbour scan already asks `is_burning()` so a lick ignites
what it touches **at no added cost to that scan**, and its own `burns_into`
ages it into smoke, so the plume comes off the front rather than being a
second effect emitted separately.

The load-bearing detail is not the rate, it is that the **direction is
rolled**. The first version searched a fixed order and took the first empty
cell; in a sward the cell above a blade is nearly always empty, so every
lick went straight up, and the fire looked much better while spreading
exactly as badly as before. `FLAME_DIRECTIONS` now lists straight up twice
in six, so a third of licks go sideways — and a sideways lick is 4-adjacent
to three cells of the next column, which is exactly the vertical
misalignment that made the sward 71 islands.

**`CellSurface::ground_wetness_at` — the channel the gate reads instead.**
The moisture *source* the field rebuilds from the CA grid every frame (1.0
for standing liquid, `held / water_capacity` for damp soil), taken at the
cell's own field block and the one below it, because fuel takes its
dampness from what it stands in. Not advected, not evaporated, written
*for* blocked blocks rather than in spite of them, and on a clean 0..1
scale. At the fuel, over the same four beds:

| soil water | old channel: fuel cells reading exactly 0 | **new channel at the fuel** |
|---|---|---|
| 0 | 100% | 0.000 |
| 180 | 96.8% | **0.180** |
| 620 | 96.8% | **0.620** |
| 1000 | 96.8% | **1.000** |

**The gate is a cutoff, not a scale** (`FUEL_WETNESS_NO_IGNITION` = 0.8,
squared falloff below it). Fire spread here is a percolation — a sward
either carries a fire the width of the world or stops it inside a hundred
cells, with very little between — so a gate that shaves 10% off ignition
does nothing at all until it crosses the threshold and then does
everything. The deterministic `temperature >= ignition_temperature` path is
untouched, which is the escape hatch the old constant's doc argued for: a
fire hot enough to boil the water out of wet fuel still lights it.

**Only `grassblade` opts in.** `wood` and `leaf` are the obvious next
candidates and are deliberately left alone: a flame on either changes how
fast a fire crosses a crown *and* when a burning trunk's base gives way
(`structural::tests::burning_a_trees_base_collapses_the_rest_of_the_trunk`,
and the acceptance cases). That is a measurement, not a line to add while
passing, and W2's remit was the grassfire.

### 5. What it measures

**Paired, one strip, one variable** (`fire::tests::a_fire_crosses_a_dry_
sward_and_stops_on_a_wet_one`, the guard):

| ground | grass consumed (of 171) | front reached |
|---|---|---|
| bone dry | **171** | x = 180 (the far end) |
| saturated | **4** | x = 13 |

**Swept over the procedure**, because a meadow is procedural and a single
sward's outcome turns on one particular gap. `PlantScene` takes no seed, so
the two axes that redraw a stand are founder count and start frame (weather
is a pure function of `(seed, frame)`, so a different window grows the sward
under different rain): 4 founder counts x 3 start frames = **12 swards**, each
burnt at three wetnesses, `fire_probe ... frames=1500`.

Fraction of the sward consumed, as an order statistic over the 12:

| ground wetness | min | median | max | swards burnt out entirely |
|---|---|---|---|---|
| 0.00 (bone dry) | 11% | ~40% | 100% | **5 of 12** |
| 0.18 (wilting point) | 11% | ~37% | 100% | 4 of 12 |
| 0.62 (field capacity) | 0.2% | ~3% | **7.9%** | 0 of 12 |

**No sward at field capacity loses more than 8% of itself; when dry, five of
twelve burn out completely.** The front distance says the same thing
without the normalisation: 67–489 cells when dry, and **never past 74** when
damp — that is 23 cells from the ignition point, which is the ignition and
its immediate neighbourhood and nothing more.

**Frame cost**, paired on the same machine in the same session, same scene,
the only difference `grassblade.ron`'s `flame_chance`:

| | worst frame | at frame |
|---|---|---|
| `flame_chance: 0.0` (the ablation — this is main's fire) | 12.33 ms | 2,184 |
| `flame_chance: 0.18` (shipped) | 10.12 ms | 287 |

**Read the second column, not the first.** The ignition is at frame 3,000,
so in *neither* arm does the fire produce the worst frame — both worst
frames are in the growth phase, before anything is alight. What the pair
says is therefore not "the flame front is cheaper"; it is **the flame front
never became the most expensive frame in a scene that also grows 160
plants**, and the 2 ms between the arms is growth-phase variation between
two builds, which is exactly the trap `CLAUDE.md` records about comparing
against a remembered number.

`examples/ascii`, on the same machine in the same session, reports its
plant scene at **worst 59.681 ms, mean 3.617 ms over 12,000 frames with 76
live organisms** — none of its 31 scenes contains a grassfire, so it does
not bound this change; it is recorded as the standing figure this branch
did not move.

The standing gas population is the cost worth watching if this is ever
scaled up: a full-width burn holds **~180–230 flame cells** and peaks around
**2,000–3,400 smoke cells** before dissipation takes them, on a 512x320
world. Flame is bounded by its own `burn_duration`; smoke is bounded by
`dissipation` and is the larger of the two by an order of magnitude.

The ablation is worth stating precisely because it is also the honest
"before" for any comparison: with the ground dry, `ground_wetness_at`
returns 0, the gate multiplies ignition by exactly 1.0, and main's blind
term multiplied it by 0.965–1.0 — so **flame off, ground dry** reproduces
main's grassfire in the ignition rule bit for bit.

### 6. The look — what changed, and what is filed

Before this branch, a burning cell's entire visual was `render.rs` blending
the cell's *own* colour toward a fire tint with a flicker. The owner's
"just looks like you are cycling colors" was not a metaphor; it was a
description of the code. Fire could not extend past the silhouette of
whatever was alight, and grass went orange and then grey.

It now has a body above the fuel, a plume rising off the body, and a black
scar behind it. That part is shipped and is judge-by-eye; the card is with
the owner.

**Two things were tried for the *colour* and are not shipped**, both
recorded here and in `Reports/dead-ends.md`:

- **`flame` with `glow: 0.9`** — fire as a real light source, which is what
  `Material::glow` is for and which a night fire obviously wants. Rendered,
  it made the fire *worse*: the glow path is built for a crystal geode in a
  dark cave and does two things a fire in daylight must not — it lifts a
  lit cell multiplicatively (`rgb * (1 + glow * GLOW_SOLID_LIFT)`, taking
  smoke's grey 84 to 160) and blends the *air* around it toward
  `GLOW_AIR_TINT`, which is (226, 222, 208), near white. A grassfire drew
  as bright specks on a bleached sky and read as a **snow flurry**. There
  is a cost argument too: `near_glow` splats a disc of radius 14 per
  glowing *cell*, and a front carries hundreds of them where a geode has a
  lining that never moves. Left at 0.0.
- **Widening `HEAT_GLOW_RANGE` from 400 to 1000** — on the theory that
  every burning thing saturates the ramp (grass burns at 520C, flame at
  780C, the ramp tops out 400C above ambient), so fire has no gradient and
  draws one flat colour. True, and the fix is the wrong direction:
  rendered, the fire came out a murky **olive**, because at a low heat
  ratio the tint is `FIRE_TINT_LOW`-ward and is blended over the fuel's own
  green. Reverted.

**And the third thing tried is shipped, because the owner picked it.**
`FIRE_TINT_HIGH` was (255, 210, 110) — a pale yellow-white — and *every*
burning thing sits at the top of the ramp, so a burning meadow drew as
**straw**. That is most of what "cycling colors" was describing. The pair is
now LOW (150,30,12) / HIGH (255,138,36).

**It was put to the owner rather than decided here**, because those two
constants are not fire's alone — they colour lava, fresh quench crust and
warm water, three looks already judged — and that trade was not W2's to
make. Blind A/B, panes reversed; the owner chose the orange. The collateral
then went back as a second card, and this is what it shows:

| | old pair | the owner's pick |
|---|---|---|
| a grassfire | straw | orange, reads as fire at a glance |
| lava (`scene=lavadrop`, falling blob) | pale sandy cream | saturated orange — **better**, more molten rock and less sand |
| fresh quench crust at the waterline | muted tan | reads more clearly as hot |
| warm water (`scene=simmer`) | **unverified** | **unverified** |

**The warm-water row is recorded as unverified rather than as checked**,
which is the honest state: by the time the pan is worth photographing it has
cooled to ambient, and at the frames where it is hot the tint barely
registers against the blue. I could not construct a frame where the
difference was visible, so I did not claim one. It wants an eye on a scene
where a pan is actually hot in shot.

If any of the collateral turns out worse, the escape is cheap and named on
the card: give fire its own tint pair instead of sharing one, at the cost of
a second pair of constants.

### 6a. The in-between: it was already there, and the card showed the wrong thing

The owner's verdict on both burn cards was *"clear and good but an inbetween
everything burns and nothing burns would be good"*. Worth recording because
the mistake was in the **demo**, not the model: both cards paired 0.00
against 1.00 — the two extremes — to demonstrate the gate, which is exactly
the wrong pair for the question *is there a middle*.

**The setting is 0.35, and the whole transition was unsampled until
somebody looked.** The landing sweep took 0.00 / 0.18 / 0.62 and nothing
between, so the entire interesting band fell in a gap. Re-run on the same
12 swards at 0.25 / 0.35 / 0.45:

| ground wetness | min | median | max | burnt out (≥99%) | **partial (5–99%)** |
|---|---|---|---|---|---|
| 0.00 (landing sweep) | 11% | ~40% | 100% | **5 of 12** | 7 of 12 |
| 0.18 (landing sweep) | 11% | ~37% | 100% | 4 of 12 | 8 of 12 |
| **0.25** | 1.3% | 35.6% | 100% | 3 of 12 | 8 of 12 |
| **0.35** | 1.3% | **20.8%** | 71.5% | **0 of 12** | **11 of 12** |
| **0.45** | 0.8% | 26.5% | 99.9% | 1 of 12 | 10 of 12 |
| 0.62 (landing sweep) | 0.2% | ~3% | 7.9% | 0 of 12 | 0 of 12 |

**At 0.35, eleven of twelve swards burn partially and not one burns out.**
That is the in-between, it is one number, and it is available today. The
middle is a **slope**, not a cliff — the cliff was an artifact of sampling
0.18 and then 0.62.

**What decides the outcome is mostly *which meadow*, not *how wet*.**
Between swards at one wetness the consumed fraction spans **70–99 points**;
within one sward across the entire band it spans a median of **13.9**. But
that is a median over a bimodal set and should not be over-read: six swards
move less than 5 points across the whole band (wetness barely matters at
all), and six move 22–76 points, chaotically — `plants=128 frame0=0` goes
99.9% → 27.6% → 99.9% across 0.25/0.35/0.45, which is a sward sitting on
the percolation threshold and flipping. So wetness is not inert; its effect
is simply swamped by sward geometry except where the sward is marginal.

**The natural bed does not burn at all, and that is the finding under the
finding.** The obvious hope — that the world's own moisture field varies
enough across a sward to give a mosaic for free — is measured **false**, and
not because the variation is missing. A sward grown without any uniform
reset carries real spatial spread (min 0.374, median 0.576, p90 0.728 at
`moisture=250`), which straddles the band nicely. The problem is the
*level*: the bulk sits at 0.576–0.699, above the band, so the fire simply
refuses — 11 cells consumed, out by frame 300, at both `moisture=250` and
`moisture=300`.

The cause is §F8, arriving somewhere nobody predicted it: unplanted soil
has three moisture sources and one sink, so any bed ratchets toward field
capacity while the sward grows on it. **A meadow in a generated world would
therefore never carry fire**, whatever it was sown at — which is a fact
about P3's world, not about this branch, and is the thing to check the day
grass is actually sown.

Swept finely on one sward, cells consumed of 1,993:

| ground wetness | consumed | front |
|---|---|---|
| 0.00 | **1,993 (100%)** | x=488 |
| 0.20 | 442 (22%) | x=110 |
| 0.30 | 1,599 (80%) | x=386 |
| 0.40 | 440 (22%) | x=110 |
| 0.45 | 1,960 (98%) | x=484 |
| 0.50 | 229 (11%) | x=100 |
| 0.55 | 436 (22%) | x=110 |
| 0.60 | 137 (7%) | x=63 |
| 1.00 | 10 (0.5%) | x=53 |

**Partial burns are the common case; total ones are the extreme.** Across
the 12-sward sweep at bone dry, 5 burned out entirely and the other 7
landed between 11% and 42%.

**But it is not a dial, and that is inherent rather than untuned.** Spread
is a percolation, so at a given wetness the outcome turns on whether the
front happens to cross one particular gap — which is why 0.30 burns 80% and
0.40 burns 22% on the *same sward*. What the model can offer is "partial
burns are likely in the middle of the range"; what it cannot offer is "burn
exactly half". Anyone asked to make the in-between *reliable* rather than
*common* should know that going in: it means changing the shape of the
threshold, not the value of a constant.

### 7. Two answers lane S asked for, measured rather than read off the source

Both asked while wind-throw was being staged
(`Reports/physical-trees-design-2026-08-23.md` §11), because fire and
wind-throw would become two consumers of one channel.

**Yes, fire reads wind — and it barely matters today.** A flame is a `Gas`,
and `update::update_gas` steers every gas cell through
`wind_biased_order`, which reads `field_wind_at`. So flame licks lean
downwind for free and nothing in `fire.rs` had to ask for it.

**On "is there such a thing as a sheltered spot": the driving wind is
global, but the field is not, transiently.** `fire_probe` now prints the
horizontal wind across the sward, 64 samples at `FIELD_SCALE` spacing.
Three start frames, one instant each:

| frame | min | mean | max | **spread** |
|---|---|---|---|---|
| 3,600 | −0.0000 | −0.0000 | 0.0000 | **0.0000** |
| 7,200 | −0.0000 | 0.0006 | 0.0387 | **0.0387** |
| 10,800 | −0.0000 | −0.0000 | 0.0000 | **0.0000** |

Two of the three are *exactly* flat — no wind anywhere, so no shelter
because there is nothing to shelter from. The third is the interesting
one: one part of the sward reads 0.0387 while the rest reads zero, which
is a `weather::gust` dipole (radius 26) sitting in the field. So locality
**does** exist downstream of the global driver, at gust scale and for as
long as a gust lasts — what does not exist is *persistent* shelter, because
nothing positional (terrain, a stand, a wall) feeds the driving wind.

Stated with its limits: three instants on one seed, sampled at sward
height. It is enough to say the field is not uniform; it is not enough to
characterise the duty cycle, and the magnitudes here (0.04 peak) are small
enough that **this branch should not be described as having wind-driven
fire spread.** The lean is real, wired, and currently almost invisible.

**Litter is fuel, and it does not throw flame.** `litter.ron` carries
`flammability: 0.6` and no `flame_into`, deliberately — only `grassblade`
opts in. So the runaway lane S asked about (a windy epoch shedding litter
at a 41.6% duty cycle *and* fanning it) is **currently self-limiting**: a
litter layer burns only where it is contiguous, exactly as grass did
before this branch, because contact-only spread cannot cross the gaps in
a scatter of shed leaves. That is a property of a default, not a
guarantee — the day anyone gives `litter` a `flame_into`, the two
mechanisms meet and the question becomes real. Worth measuring then, not
now.

---

## Part two — E6, the desert decision

**This is a decision card, not an implementation.** `Reports/open-bugs-
handoff.md` §X is a design direction, and its own instruction is *do not
"fix" this by watering deserts*. What follows is the three candidate levers
with what each actually costs, measured against the code rather than
estimated — because two of the three costs on the record have changed.

### The situation, restated in one line

Arid country's ground is **sand**, `soil.ron` is the only material in the
asset directory that declares a `water_capacity`, so sand's is 0 — ground
with no water-holding capacity whatever. No wilting point, however low,
extracts water from a material whose capacity is zero, which is why §X
already rules out the species-wilting-point fix *for the desert* (it stays
worth doing for the wetland-to-canyon gradient, where the ground is really
soil at differing wetness).

### Lever (a) — sand gets a `water_capacity`

**The prerequisite the record names is already paid.** `MaterialDef::
water_capacity`'s own doc says widening this "meant teaching those tallies
about held water first, and that is **done**". Confirmed at the site:
`weather::water_equivalents` counts held water under
`MaterialKind::Powder if m.water_capacity > 0` — **keyed on the field, not
on a material name** — so a second water-holding powder joins the ledger
automatically. The conservation guards (`nothing_escapes_the_world`, the
multi-chunk sand-and-water settle) need re-running, not re-writing.

**The cost that is real, and is arithmetic rather than engineering.**
`update::plant_available_fraction` measures a cell's water against
`SOIL_WILTING_POINT` (180) and `SOIL_FIELD_CAPACITY` (620) as **absolute
aux values**, not as fractions of that material's own capacity. So:

- a "small" capacity does nothing. At `water_capacity: 150`, a *saturated*
  sand cell holds 150, which is below the wilting point, and plant-available
  water is exactly zero — the desert is dead in precisely the same way, now
  with an infiltration cost per frame over every sand cell in the world.
- **the threshold is 180 before a plant gets one drop**, and around 620
  before a sand cell reads as "field capacity" to a root. That is 18–62% of
  soil's own capacity, which is not a small number and is a real statement
  about what desert sand is.
- the alternative — making `plant_available_fraction` scale by the
  material's capacity — is a different and larger change, because it moves
  what every existing soil reading means and the economy is calibrated
  against those.

**What else moves:** sand is not only in deserts. Every beach and dune in
every preset starts absorbing adjacent water, which is realistic and is a
visible change to shorelines; damp sand becomes a moisture source, so it
darkens, feeds evaporation, and — as of this branch — **stops carrying
fire**.

### Lever (b) — roots reach the water table

**This is not a root-reach problem in a desert, and that is the finding.**
`assets/worldgen.ron`'s `arid` preset sets `table_offset: 4000.0`. The
table is placed four thousand cells below the datum — off the bottom of the
world — deliberately: `params.rs` names `arid` and `flat` as the two presets
that "put the water table past the world floor, so no cliff face can
intersect it". `flat` is the structural test bed; `arid` is the desert.

It is not an accident anyone can tidy away, either: **there is a test
guarding it** — `tests/worldgen.rs`'s
`the_dry_presets_keep_their_table_below_the_world_floor`. So this lever
begins by deciding to break that guard on purpose, which is the shape of
thing that should be the owner's call and not an implementer's.

So the decision inside lever (b) is **whether the desert gets a water table
at all**, which is a worldgen number, not a plant capability. Give it one
and the depth follows from the existing terms (`table_offset + aridity *
aridity_table_drop`): at the wetter presets' settings that lands the table
of the order of 90–100 cells below an arid surface, which is then a genuine
taproot-reach question and the one Arc B4 is about. **These two decide
together, and (b) cannot be scheduled before the worldgen half of it is
decided.**

Second-order, and worth saying because it is the fun part: a table inside
the world in arid country is also what would let a desert have the springs,
seeps and oasis hollows the cliff-daylighting pass already knows how to
build (`noise.rs`'s aquifer-daylighting is switched off for `arid` by that
same `0.0`, not absent).

### Lever (c) — stored-rain events

Rain already falls on the desert — the render on this card is the arid
preset *in rain*. What happens to it: `update::update_soil_water` requires
`water_capacity > 0`, so nothing infiltrates; the drops stay `Liquid`, run
downhill, pool where they can and evaporate. That is a flash flood, and it
is already correct desert behaviour.

**So (c) is not "add rain", it is "make a storm leave something behind for a
few thousand frames"** — an ephemeral pulse of plant-available water that a
short-lived species could germinate on and complete a life cycle inside.
The engine already has the shape of the storage: `FieldTile::moisture_floor`
is an authored lower bound on moisture that evaporation may not take a cell
below (worldgen writes it once, for the aquifer). A decaying floor written
by a storm is a small mechanism.

**The cost is that it needs a species that can use it**, and nothing in
`assets/species/` is built to live and die in a window: every plant here
is a perennial that accumulates. A desert annual is its own package. Rated
as the largest of the three for that reason, and as the one that buys the
most distinct *behaviour* if it lands — a desert that is dead for a season
and green for a week is a thing the other two levers do not produce.

### What this card asks

Which lever, and the two that pair. (a) and (b) are not exclusive — real
xerophytes do both — but (b) is entangled with a worldgen decision and with
Arc B4's taproot niche and should be taken with them.

---

## Dead ends recorded

Both in `Reports/dead-ends.md` with their conditions: the flame `glow`
(condition: `render.rs`'s glow path is daylight-blind and per-cell-splatted)
and the widened `HEAT_GLOW_RANGE` (condition: the tint blends over the
fuel's own colour, so a lower heat ratio is muddier, not deeper).

## Reproducing any of this

```
cargo build --release --examples          # ALWAYS -- the .ron files are include_str!'d
cargo run --release --example fire_probe -- moisture=620 burnmoisture=0    plants=160
cargo run --release --example fire_probe -- moisture=620 burnmoisture=1000 plants=160
cargo test --release --lib a_fire_crosses_a_dry_sward
cargo run --release --example filmstrip -- scene=grove species=grass plants=160 \
    moisture=620 dry=0,2400 ignite=51,194,3,3000 start=3000 every=45 count=8 \
    zoom=4 crop=20,170,220,44 cols=2 out=/tmp/burn.png
```

`fire_probe` echoes both its command line **and the fuel constants the
binary was built with**, which it does because a sweep over `grassblade.ron`
was killed by a timeout before its restore line ran and the next four
measurements were of a fuel nobody meant to test — they read as "the
moisture gate is inverted", which is a conclusion, not a typo.
