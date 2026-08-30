# The evolution lab: is a second game on this engine possible? (2026-08-30)

**Status: feasibility measurement, not a plan and not a decision.** Answers a
question put by the owner: strip the gnome, worldgen, rock, tunnelling,
explosions and collapse from this engine and build a second game on the plant
and creature arms alone — a sealed lab box of deep soil under grow lights, a
future scientist evolving colonies across generations, with an interaction
phase at normal speed and an experiment phase run fast.

Every number below was measured on this branch on 2026-08-30, on the machine
that ran it, in one session. Nothing is extrapolated from an earlier report;
where an earlier report is quoted it is named.

**The short answer is yes, and the reason is not the one the question
assumes.** Speed is not what stands in the way, and the systems the concept
proposes to delete are not what costs. What stands in the way is one arm of
the biology: **plants breed and ants do not**, measured today.

---

## 0. Summary, stated first

1. **A sealed box with nothing alive in it costs 0.001 ms/frame — 18,000x
   real time.** The engine's sleeping machinery is already perfect. Every
   frame of cost in this game is bought by something being alive.

2. **Under a held grow light, cost follows living biomass rather than world
   size** — a 2048-wide box measured *cheaper* than a 512-wide one at fixed
   founders, so shrinking the world buys nothing. **Under a moving sun the
   opposite holds and world width is the dominant term**, which is why the
   shipped game is slow (§2, §3b). The per-cell figure is **not a constant**:
   it runs 2.25 µs/cell at a 497-cell stand down to 0.91 at 5,684, and the
   model breaks entirely once the sun is running.

3. **Deleting rock and collapse is worth ~16% of the shipped frame** — the
   structural scheduler runs at 3.389 ms and falls to 0.197 ms when its load
   walks are switched off, a 17x drop no machine noise explains. The whole
   frame falls 30% in the same pair, but that half is one unpaired
   comparison and is a direction rather than a figure. **An earlier
   draft of this report said it was worth "approximately nothing", from a
   measurement taken in a bed with no rock in it; §3c is the correction.**
   You do not collect that 30% by deleting code, you collect it by not having
   rock — which is the same thing from the player's side and a different
   thing from the budget's.

4. **The organisms are 7% of the frame and the biosphere is 93%** — and the
   93% is not overhead, which is the correction §3a records. The CA sweep
   runs the soil water cycle; the field's passes carry light, heat and
   humidity and the channels plants and creatures actually read. The right
   reading is not *"the substrate is expensive"* but **"the environment costs
   thirteen times what the life in it costs"** — which is good news for a
   game about populations, because adding organisms is the cheap direction.

5. **Measured end to end: 5–7 herb generations in 4 minutes 17 seconds**,
   headless, one 1024x320 bed, 45,000 frames, stand grown to 20,913 cells.
   That is roughly **1.4 generations per wall-clock minute** with no
   optimisation at all, and the concept's own changes push it up rather than
   down. The "watchable evolution" premise is already met by the shipped
   engine.

6. **Ants do not breed, and that is the blocker.** Measured here today:
   `births 0`, `deepest generation 0`, richest bank **219** against a birth
   cost of **1,040**. `assets/species/ant.ron` says so in its own comment as
   of this morning. Plants are fine — `herb` reaches generation 5–7 — but
   `tree` reaches generation 1 in 200,000 frames and is unusable as a
   selection substrate.

7. **The grow light is a generation-clock lever, not a performance one, and
   it is a large one.** Holding the sky at full amplitude instead of a
   day/night cycle produced **1,037 seeds against 435** in the same 6,000
   frames — 2.4x the reproductive throughput — at 2.3x the population. It
   costs more per frame *because* more is alive, which is the trade the game
   wants to be making.

---

## 1. The instrument

`examples/labbox_cost.rs`, written for this question and kept. Four arms over
one bed, each one switch off the one before, so a difference is attributable:

| arm | what it changes |
|---|---|
| `live` | nothing — the shipped outdoor world, in a box |
| `calm` | `set_weather_pin(Clear)` — no wind, no rain |
| `lab` | `calm` plus `set_sky_hold(noon)` — a grow light, not a sun |
| `floor` | `lab` with `step_fields` not called — the control on how much of the frame is *not* the field |

It prints `solved/frame`, `awake/frame` and the stand census (`cells`,
`orgs`, `seeds`) beside every timing, because a frame that got cheaper
because the field converged and one that got cheaper because the field
**stopped being asked** are identical in a timing (`CLAUDE.md`: *a cost that
vanishes may be work that vanished*).

**That control fired immediately and is why `floor` is not a result.**
Removing the field gives 0.007 ms/frame — 2,463x real time — and a stand of
**14 cells and 0 seeds**. Light is delivered through the field, so the arm
does not measure a cheaper lab; it measures a dead one. Its only valid use is
as the lower bound on everything that is not the field.

**Sensitivity, run before any of this was believed.** The instrument moves
across a 5,000x range on the quantity it claims drives cost (§2), and reports
exactly 0 solved tiles and 0 awake chunks on the empty box — so it is neither
pinned nor blind.

**One arm was wrong first and the census caught it.** `lab` initially held
the sky at `DAY_NIGHT_PERIOD_FRAMES / 4` on the assumption that a quarter of
the way through the cycle is noon. It is not — the hump's phase belongs to
`sun_elevation`, not to the caller — so the first run pinned a dim sky, and
reported the lab arm as both cheaper *and* carrying a smaller stand. The
harness now finds noon by maximising `sky_light_amplitude` over the period
and prints the value it held (4.000, against a cycle running 0.200–4.000).
The corrected arm reverses the sign of the result, which is §0.7.

## 2. What actually costs

`labbox_cost frames=3000 warm=300 width=1024 soil=120 species=herb arms=lab`,
varying only the number of founders:

| founders | plant cells | ms/tick | speed-up over 60 Hz | solved/f | awake/f |
|---|---|---|---|---|---|
| 0 | 0 | **0.001** | **18,230x** | 0.0 | 0.0 |
| 4 | 497 | 1.118 | 14.9x | 17.9 | 2.9 |
| 16 | 1,932 | 2.942 | 5.7x | 38.4 | 12.9 |
| 48 | 5,684 | 5.156 | 3.2x | 54.8 | 21.5 |

**An empty sealed box is free.** Not cheap — free, at a thousandth of a
millisecond, with the field solving nothing and no chunk awake. Everything
after that is bought by life.

**But "0.7 µs per living plant cell" is not a constant, and an earlier draft
quoted it as one.** The three arms it was read off (`live` 0.813, `calm`
0.743, `lab` 0.635 µs/cell) all ran at the *same* width and the *same* founder
count, so they cannot test a per-cell model at all — that is this file's own
*ask what your number counts* rule, applied to a ratio. Read off the sweep
that does vary the stand, it is strongly **sublinear**:

| plant cells | ms/tick | µs per cell |
|---|---|---|
| 497 | 1.118 | **2.25** |
| 1,932 | 2.942 | 1.52 |
| 5,684 | 5.156 | **0.91** |

An 11x stand costs 4.6x. And in the `live` arm the model breaks outright —
1,622 cells at 4.818 ms (2.97 µs/cell) against 1,247 cells at 7.178 ms (5.76),
because there the sun's width-driven cost dominates and biomass is not the
term at all. **So: use ~1–2 µs/cell as a sizing rule for a grow-lit box, know
it falls as the stand grows, and do not use it under a moving sun.**

**World size is not a term in it — but only once the sun stops moving, and
that qualifier is the whole reason the shipped game is slow.** An earlier
draft of this section stated the first half flatly, from a sweep run on the
`lab` arm alone. Re-run with both arms, founders held at 16:

| width | `live` ms/tick | `live` solved/f | `lab` ms/tick | `lab` solved/f | tiles in world |
|---|---|---|---|---|---|
| 512 | 4.818 | **40.0** | 5.092 | 24.7 | 40 |
| 1024 | 5.133 | **80.0** | 3.897 | 40.3 | 80 |
| 2048 | 5.484 | **160.0** | 2.679 | 53.4 | 160 |
| 4096 | 7.178 | **320.0** | 4.082 | 123.2 | 320 |

**With the sun running, `solved/frame` is exactly every tile in the world,
every frame** — 40, 80, 160, 320 against worlds holding 40, 80, 160 and 320 —
and the field's cost tracks it linearly: 2.32 → 3.59 → 4.75 → 6.52 ms. With
the sky held it is 24.7 → 123.2 for the same widths, sublinear, and the field
runs 2.04 → 3.30 ms.

The mechanism is `sky_drifted`, and `frame-cost-audit-2026-08.md` named it in
2026-08-24: *"the sun wakes tiles over rock that has not moved in ten thousand
years"* — a lit tile whose stored amplitude is stale goes into the solve set
whether or not anything in it moved. At 8192 wide that is the entire surface
of the world.

So the correct statement is conditional, and both halves matter:

- **Under a moving sun, world width is the dominant cost term.** This is what
  the shipped game runs, and §3b is what it costs there.
- **Under a held grow light, it is not** — a wider bed is *cheaper* at fixed
  founders (2.679 against 5.092 ms), because the same stand spreads thinner
  and cost follows the stand rather than the grid. A small box *concentrates*
  the stand, which is the expensive direction.

The lab concept sits on the second row by construction: a sealed box has no
sun to drift. That is not an optimisation to be built, it is the arm measured
above.

**Soil depth is a real cost and, at this stand size, buys nothing.**
Same bed at 1024x512, founders fixed:

| soil rows | ms/tick | plant cells | orgs | seeds |
|---|---|---|---|---|
| 40 | 2.294 | 2,071 | 117 | 109 |
| 120 | 3.455 | 2,071 | 117 | 109 |
| 240 | 4.380 | 2,071 | 117 | 109 |

Six times the depth, **1.9x the frame, and a byte-identical stand** — herb's
roots never reach past 40 rows, so every row below that is paid for and
unused. The concept explicitly asks for deep soil (deep-rooted trees, dug
colony structures); this says to pay for depth **when something reaches it**,
and that a depth slider is a performance knob whether or not it is presented
as one.

## 3. Where the frame goes

Per-phase mean, ms, `width=1024 soil=120 trees=16 species=herb frames=6000`:

| arm | ca_sweep | chunk_bodies | active_sites | particles | field | pheromones | whole |
|---|---|---|---|---|---|---|---|
| `live` | 1.569 | 0.001 | 0.324 | 0.001 | 2.350 | 0.000 | 4.246 |
| `calm` | 1.471 | 0.001 | 0.314 | 0.001 | 1.999 | 0.000 | 3.786 |
| `lab` | 2.115 | 0.003 | 0.472 | 0.001 | 2.145 | 0.000 | 4.737 |

`active_sites` is the plants' and creatures' own biology — growth, transport,
allocation, reproduction, every brain tick. **It is 7% of the frame.** The
other 93% is the CA sweep and the coarse air field — see §3a, which is a
correction to what an earlier draft of this report said that 93% *was*.

Everything the concept proposes to delete is in the columns reading 0.000 to
0.003: chunk bodies, particles, blasts, the player. At 1024x320 with a
generated world, `scale_probe phases=1` puts the same phases at `player
0.001ms, rigid bodies 0.001ms, blasts 0.000ms, particles 0.000ms, liquid
bodies 0.000ms` against a 3.677 ms frame. **Deleting the gnome, explosions,
rigid bodies and the collapse scheduler is worth approximately nothing in
frame time**, because a chunk with nothing happening in it already costs
nothing.

This is worth stating plainly because it inverts the concept's stated
premise. The case for stripping them is real, but it is a case about **scope,
risk and cadence freedom** (§4), not about speed.

### 3a. The 93% is the biosphere, not overhead — a correction

**An earlier draft of this report called the CA sweep and the field "the
substrate that exists to carry an outdoor world", and that is wrong.** The
owner challenged it; it does not survive reading the code, and the sentence
would have licensed deleting the game's environment as if it were scaffolding.
Recorded here rather than quietly fixed, because the wrong version is the
intuitive one and someone will arrive at it again.

**Every channel in the field has a biological consumer.**

| channel | written by | spread by | who reads it |
|---|---|---|---|
| `light` | `apply_sky` | `step_diffusion`, `step_advection` | photosynthesis, phototropism, `noon_equivalent_light` |
| `temperature` / `sky_temperature` | `apply_sky_temperature` | `step_diffusion`, `step_advection` | plant physiology; the ant brain's `TempAboveAmb` input |
| `moisture` | `apply_moisture_sources` | `step_diffusion`, `step_advection` | `organism::moisture_pull` — root hydrotropism |
| `vx` / `vy` | `step_pressure`, `step_velocity` | `step_advection` | `organism::wind_lean_dir` — trees lean; particle and litter drift |

`step_diffusion`'s own code says it: a blocked cell *"stays ambient — a wall
has no temperature or light of its own"*. It is the pass that spreads light
into a canopy and heat through a room. `step_advection` blends **pressure, vx,
vy, temperature, sky_temperature, light and moisture** along the velocity
field — it is how heat and humidity move on the air, not an air toy sitting
beside the biology.

**And the CA sweep is the water cycle.** `update.rs` runs infiltration
("*liquid infiltrates soil, soil holds and drains it, roots drink it*"),
drainage between soil cells, and the scheduling half of evaporation. In a bed
of soil that is most of what it is doing.

So the honest decomposition is not *biosphere versus overhead*. It is
**organisms (7%) versus the environment they live in (93%)**, and the second
number is the game.

**One part is separable, and it is much narrower than the earlier draft
claimed** — see §4a, which is rewritten around the measurement that settled
it.

### 3b. Why the shipped game is so much slower than any of this

The owner asked it directly: *"you are saying headless you can grow plants at
1.4 generations per minute, but I cannot do that in the actual game."*
Correct, and the gap is three separate multipliers, none of which the lab bed
pays.

**The shipped world, this machine, `scale_probe size=8192x2560 phases=1
warm=600 frames=1800`:**

```
                   phase       mean        p90      worst     share
                   field   13.950ms   25.774ms   39.572ms     68.9%
 active sites: scheduler    3.389ms    7.558ms   21.575ms     16.7%
 active sites: organisms    1.486ms    3.125ms   10.648ms      7.3%
  sweep (parallel::step)    1.403ms    2.308ms    5.690ms      6.9%
      (the other seven)     <0.02ms                            0.1%
             WHOLE FRAME   20.262ms   32.768ms   51.301ms
live organisms: 325   chunks: 5120   awake chunks: 24
1087 of 1800 frames (60.4%) exceeded the 16.6 ms budget
```

1. **The sun makes the world's size a per-frame cost.** 5,120 chunks resident,
   **24 awake** — under half a percent of the world has anything moving in it
   — and the field still costs 13.95 ms, 69% of the frame. §2's table is the
   controlled version of the same effect. This is the largest of the three and
   the lab removes it outright.
2. **The game renders and the harness does not.** `render_cost` measures a
   full 512x320 redraw at 2.407 ms on this tree, and the renderer redraws
   essentially every frame while the gnome walks, because a camera move
   invalidates every pixel. `frame-cost-the-render-half-2026-08-29.md` is the
   record for that half and puts the shipped world's full redraw at ~7.5 ms
   after its glow-halo fix (from ~42 ms before it). Nothing in this report's
   own timings includes a draw.
3. **The game is capped at 60 ticks per second by design, and does not reach
   it.** `main.rs` runs a fixed timestep with `MAX_TICKS_PER_FRAME = 5`, so a
   real-time session advances the simulation 60 times a second at best — where
   the headless bed runs as fast as it can and measured 175–256. With 60% of
   frames over budget it lands nearer 35–50.

**Multiplied out**: the shipped game advances maybe 40 ticks a second against
the bed's 175–256, so the same herb generation that takes 43 seconds headless
takes **four to seven minutes** in the app. That is the discrepancy, and none
of it is a mystery — it is a 21-million-cell world under a moving sun, drawn
every frame, advancing at wall-clock speed.

**Every one of the three is removed by the concept rather than optimised
away.** No sun (§2), a box small enough to draw cheaply and a wall instead of
a sky (§4d), and an experiment phase that is explicitly not real-time (§4c).
That is the strongest argument in this report for the lab being a different
*game* rather than a smaller world: the shipped game's cost is dominated by
things a lab does not have.

### 3c. Deleting the destruction half is *not* free — a second correction

**The §3 paragraph above says the phases the concept deletes measure
0.000–0.003 ms, and concludes that deleting them is worth "approximately
nothing in frame time". That conclusion is wrong, and it is wrong
circularly**: it was measured in a hand-built bed of soil and stone that has
nothing to collapse, so the collapse system idles by construction. Asking a
bed with no rock in it what the rock system costs is not a measurement.

Asked of the shipped world instead — `scale_probe size=8192x2560 phases=1`,
`PROBE_NO_LOAD=1` zeroing `load_budget` so the structural checks stop walking
load — the answer is large:

| | default | `PROBE_NO_LOAD=1` |
|---|---|---|
| `active sites: scheduler` | **3.389 ms** (16.7%) | **0.197 ms** (1.4%) |
| field | 13.950 ms | 11.309 ms |
| **whole frame** | **20.262 ms** | **14.263 ms** |
| awake chunks | 24 | 22 |

`scheduler::step` is the structural-check scheduler — its own code names the
load walks as *"the expensive half of this phase"*. **The direct saving is
3.19 ms, 15.7% of the frame, and a 17x drop is not something machine state
explains.** The whole-frame figure is weaker evidence and should be read as
indicative: it is **one unpaired pair**, not the alternating paired runs this
file's own rules ask for on a whole-frame delta, and the field's 2.6 ms of it
is a knock-on that cannot be separated anyway — with nothing collapsing fewer
chunks wake, and the two runs' worlds genuinely diverge (325 organisms against
326). Quote the 16%; treat the 30% as a direction.

**What this changes and what it does not.** The concept's conclusion survives
— a lab box is fast — but the reason given for it was wrong. You do not get
the 30% by *deleting code*; you get it by **not having rock**, which a lab bed
does not have anyway. So:

- Against the shipped game, the destruction half is ~16% of the frame
  directly and ~30% with its knock-on.
- Against a lab bed, deleting it saves ~0.03 ms, because there is nothing
  left to save.

Both are true and they answer different questions. The first is what the
owner feels when playing; the second is what a lab-box budget looks like. The
earlier draft quoted the second as though it answered the first.

## 4. Where the speed actually is

Two levers, and they are not the same size. **4b is the large one and 4a is
the interesting one** — the first draft had that the other way round, which
§3a is the correction to.

**4a. The air's own motion is a dormant mechanic, not overhead — and it is
the only separable part.** `FIELD_PASS` inside the `lab` arm, at frame 3,000:

```
solved 35  momentum 35  total 1.73ms | blocked 0.20  pressure 0.13
velocity 0.20  diffusion 0.37  advection 0.41  sky 0.21  sky temperature 0.19
moisture 0.01
```

**The earlier draft read pressure + velocity + diffusion + advection — 1.11 ms,
64% of the field — as "work no plant consumes". That was wrong twice over**,
and §3a is why: `step_diffusion` is the light-and-heat pass, and
`step_advection` transports light, temperature and moisture as well as
momentum. Only `pressure` and `velocity` (0.33 ms) are the air's own
mechanics, and even those exist to drive transport something reads.

**So it was measured instead of argued.** `FIELD_MOMENTUM=0` (new, in
`field.rs`, a control and never a setting) switches the three momentum passes
off for a whole run. Ablated against the shipped default, `width=1024
soil=120 trees=16 species=herb frames=6000`, wind running:

| seed | ms/tick, on → off | plant cells | organisms | seeds set |
|---|---|---|---|---|
| 1 | 4.209 → **3.365** | 5,220 → 5,337 | 370 → 376 | 435 → 428 |
| 2 | 3.474 → **3.075** | 3,993 → 4,492 | 264 → 274 | 311 → 315 |
| 3 | 5.880 → **5.228** | 4,530 → 4,577 | 274 → 290 | 329 → 344 |

**11–20% of the frame, and the stand comes out slightly *larger* without it** —
more cells and more organisms in 3 of 3 seeds, seeds set within ±2%. So in
this bed the air's motion is real cost doing mildly negative biological work,
presumably by smearing the light and heat fields plants would otherwise keep
above themselves.

**The effect is weak and scale-dependent, and the counter is what shows it.**
On a small bed — 512 wide, 60 rows of soil, 8 founders, 1,400 frames — the
same ablation leaves the stand **byte-identical** at 557 cells / 15 organisms
/ 8 seeds, which reads exactly like a dead knob. It is not: `FIELD_PASS` shows
`momentum 40 → 0`, pressure 0.10 → 0.00, advection 0.35 → 0.06, and the solve
set itself diverging (17 tiles against 6 by frame 1,200). So the air's state
changes and the plants do not notice, at that size. Both pairs above
reproduce byte-for-byte on a re-run, so every difference quoted is the knob
and not variance. Quote the counter beside any census here — a null on the
stand alone is indistinguishable from a switch that was never wired.

**Read all of that as a statement about this bed, not about the concept.** The bed
has nothing in it that drives air: no fire, no fans, no heaters, no
humidifiers, no wind-dispersed seed, no scent plume, and creatures that do
not read wind. Outdoors the weather forces these passes for free and they
earn their cost in tree lean, smoke and drift. **In a sealed box nothing
forces them, so they run at full price and return nothing — until the player
installs something that moves air.**

That is the interesting shape for the concept rather than a saving to bank:
**the air simulation stops being ambient and becomes player-driven**, which
is precisely the equipment-and-resource layer the brief describes. A fan is
then not set dressing; it is the thing that switches a pass on.

The reason it cannot sleep on its own today is in `field::step`:
`skip_momentum` requires `!any_fluid` — **no chunk awake anywhere** — and
every tile in range at exactly zero pressure and velocity. A growing plant
marks its chunk dirty, so in any living world the passes run permanently. A
per-tile gate is the obvious repair and is worth ~1.1x; the `FIELD_MOMENTUM`
control above is what would tell you whether a given box wants it.

**4b. The sweep runs at 60 Hz for a world whose fastest customer is 10 Hz.**
`ORGANISM_TICK_INTERVAL` is 45 frames; the shipped ant's `tick_interval` is 6.
The CA sweep — 1.5–2.1 ms, the largest single phase — runs **every** frame,
and in the outdoor game it must, because falling rock, a collapsing beam and a
thrown blast all need per-frame resolution. **Delete those and nothing in the
box needs 60 Hz except creature movement and falling litter.**

This is the lever the concept genuinely unlocks, and it is the reason to strip
the destruction half even though stripping it saves no time directly. It is
also the one that needs care: `clock.rs` records a paired 8-seed sweep in
which the same number of organism ticks at 4x `growth_slowdown` produced a
median **0.61x** final cells — *"a slowed subsystem is not the same subsystem
later."* Cadence changes are **not** free re-timings; they change outcomes.
Anything here has to be measured as a behaviour change, not assumed as a
speed-up.

**4c. What is already free and needs no work.** The fast-forward the concept
describes — pause interaction, run the experiment at speed — needs no new
machinery and introduces no error. `main.rs` already runs a fixed-timestep
catch-up loop, capped at `MAX_TICKS_PER_FRAME = 5`. Raising that cap runs the
*identical* tick sequence in the identical order; determinism is required
same-build (`PLAN.md`), so a fast-forwarded experiment and a real-time one are
the same simulation. The only thing given up is rendering every frame, and
rendering is cheap and per *displayed* frame: `render_cost` measures a full
512x320 redraw at **2.407 ms**, so at a 30 Hz display it is ~7% overhead
regardless of the tick multiplier.

**4d. A free render win the concept hands over.** Empty sky is the most
expensive thing this renderer draws — **27.4 ns/px** against stone's 6.7,
because of the gradient, moon and star hash — and an all-sky 512x320 frame
costs **12.370 ms** against 1.990 ms for all stone. A lab has walls and a
ceiling, not a sky. Whatever fills the air above the soil in this game should
not be the sky branch.

## 5. The generation clock, which is the real question

"Fast enough that a player can expect multiple generations in a short period"
is a question about **frames per generation**, and that is biology, not
performance.

**Plants: `herb` works, `tree` does not.** From
`plant-throughput-herb-2026-08-29.md`, `plant_probe trees=16 frames=45000`:

| species | seeds set | established carrying an inherited genome | deepest established generation |
|---|---|---|---|
| `tree` | 143–196 | **0 of 16** | **1** |
| `grass` | 0 | 0 of 8 | **0** |
| **`herb`** | 7,974–10,926 | **75–110 of 90–125** | **3, 5, 7** |

Herb's histogram is a life cycle running, not a population that once reached
generation 2: `[gen 0: 11, gen 1: 374, gen 2: 807, gen 3: 436, gen 4: 392,
gen 5: 191, gen 6: 52, gen 7: 30]`. **88% of established plants carry an
inherited genome.**

**Measured here today, end to end:** that same 45,000-frame herb run took
**4 minutes 17 seconds** of wall clock, headless, ending at 20,913 organism
cells — about **1.4 generations per minute** at 175 ticks/s averaged over a
stand that grew throughout. Nothing was optimised and nothing was stripped.

**And the grow light raises it.** `live` against `lab` over 6,000 identical
frames:

| arm | plant cells | orgs | **seeds set** |
|---|---|---|---|
| `live` (day/night) | 5,220 | 370 | 435 |
| `lab` (grow light held at 4.000) | 7,459 | 845 | **1,037** |

**2.4x the reproductive throughput per frame**, because the cycle it replaces
spends half its time at an amplitude of 0.2. The grow light costs 12% more
per frame and returns 2.4x the generations — that is the correct direction for
a game whose currency is generations watched per minute, and it is an argument
for the fake sun on *mechanical* grounds rather than fictional ones.

**Creatures: the ant does not breed.** Measured here today,
`creature_probe terrain=world frames=12000 ants=45`:

```
reproduction: births 0 denied-no-space 0 refused-no-slot 0
  live 27 deepest generation 0 | richest bank 216 against a birth cost of 1040
```

The machinery is in — S6 landed, `try_bud` runs, `mutation_rate` is live — and
the **economy does not close**. `ant.ron`'s own comment, written this morning,
states it: *"the shipped ant does not breed, at any grant and at any budget,
and this line cannot fix that"*. A birth costs the grant (80) plus the body
stamp (960); an ant's bank is capped by `hunger_fraction` at roughly half
`start_energy` plus one mouthful, measured at **219**. The stamp is invariant
to every knob in the file, so lowering `start_energy` lowers the ceiling
faster than the cost: bank-over-bar goes 0.30 → 0.21 → 0.16 as the budget is
cut from 900 to 200 to 90.

**Read the provenance honestly.** What was measured *here* is one scene —
`terrain=world`, 45 founders, 12,000 frames, one seed — reporting zero births.
The general claim ("at any grant and at any budget") is `ant.ron`'s, from a
sweep this session did not re-run, and the arithmetic above is what makes it
credible rather than the single run. A reader wanting to overturn it should
attack the bank ceiling, not the birth count.

**Subject to that, it is the single thing standing between the concept and its
premise.** A game about evolving creatures across generations, on a creature
that has never produced a second generation. The fixes named at the source are structural,
not tuning: a child born at one cell that grows into its plan, or a gut
specialised enough to draw a full leaf's 480. Full arithmetic in
`creature-birth-grant-2026-08-30.md`.

## 6. What the concept is really buying, restated

Stripped of the performance argument it does not need:

- **A closed box removes the two designed oscillators** — day/night and
  weather — that `CLAUDE.md` says must be divided out of every measurement
  taken in this engine. In a lab they are *replaced by player equipment*: a
  grow light on a schedule, a misting system, a heater. The oscillator becomes
  a dial the player sets rather than a confound every reading carries. That is
  worth more to this project than the milliseconds.
- **It removes the reason the sweep must run at 60 Hz** (§4b), which is the
  largest untaken speed lever and is unreachable while rock can fall.
- **It removes procedural terrain as a variance source.** `seedsweep.sh`
  exists because outcomes here are chaotic in the world seed. A hand-built bed
  is the same bed every run, which makes a selection experiment a controlled
  one — and `selection_arena`'s whole finding is that a world which does not
  discriminate invalidates every evolution result measured in it.
- **It turns the air simulation from an ambient system into a player-driven
  one** (§4a). Outdoors the weather forces those passes for free; in a sealed
  box nothing does, so they cost full price and return nothing until somebody
  installs a fan or a heater. That is an argument *for* the equipment layer
  rather than against the air sim.
- **It does not remove the environment's cost, and should not.** The 93% is
  light, heat, humidity and the soil water cycle — the biosphere itself, not
  scaffolding (§3a). What it removes is the *outdoor* forcing of that
  environment, replacing a weather system nobody can set with equipment a
  player can.

## 7. What would have to be true, in the order it would have to be tested

Stated as gates rather than as a plan, because the plan is not what was asked
for.

1. **Ants must reach generation 2.** Nothing else in the concept matters until
   this is true; every gene in `brain.rs` is inert without it. The gate is
   `creature_probe` reporting non-zero `births` and a `deepest generation ≥ 2`
   in a bed a player would recognise. §5 says why tuning `ant.ron` cannot
   reach it.
2. **A scene must exist that runs plants and creatures together in one hand-
   built box.** There is none today. `filmstrip scene=colony` is the closest —
   it generates a `wetland` world, grows it 2,400 frames, then founds a colony
   — and it depends on worldgen for the plants, which the concept deletes.
   This is the first thing to build and it is small.
3. **The sweep's cadence must be shown to be separable** (§4b), measured as a
   behaviour change against `clock.rs`'s 0.61x finding, not assumed as a
   re-timing.
4. **Selection must have teeth in the box.** `selection_arena` is the existing
   instrument and its null is a finding about the *world*: a bed that does not
   punish a plant known to be worse invalidates everything measured in it. A
   hand-built lab bed has never been run through it.
5. **Only then, resource management and equipment.** Every one of those is a
   verb, and `CLAUDE.md`'s second law applies: a verb that produces no visible
   consequence is not finished.

## 8. What this does not answer

- **Whether it is fun.** Nothing here is a judgement about play. The engine
  can carry it; that is a different claim.
- **Whether generations arrive evenly.** §5's "1.4 per minute" divides total
  generations by total wall clock, and generation depth is not linear in
  time — the founding cohort has to mature before anything can be a second
  generation, so the early minutes are slower than the figure and the later
  ones faster. Nothing here measured the curve.
- **What a mature stand costs.** Cost is per living cell and the stand grows,
  so a box at equilibrium has never been measured — the 45,000-frame run was
  still growing at the end. The relevant number for a long experiment is the
  *equilibrium* biomass the box supports, and nothing has measured it.
- **Whether ants dig usable colony structures in deep soil.** §2 shows depth
  costs 1.9x and herb does not use it. Whether an ant does is untested.
- **The creature side of the frame budget at a breeding population.** Every
  creature figure in this repo, including the one here, is at a population
  that does not breed. `creature-evolution-plan.md` §2.6 already flags this:
  *"creature work was measured free at 55 ants and a breeding population is
  not 55."*
- **Multi-species interaction cost.** One species of plant, one of creature,
  measured separately.
