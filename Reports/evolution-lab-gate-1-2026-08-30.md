# Gate 1, measured: the lab box lives, and what it costs

*Lane D of the evolution-lab program, 2026-08-30. The census and the frame
cost of `lab::scene::LabBox` — the first scene in this repo that runs plants
and creatures together. Design of record:
`evolution-lab-design-guide-2026-08-30.md`; the numbers it is downstream of
are in `evolution-lab-feasibility-2026-08-30.md`. **Both are in flight on
`claude/evolution-lab-game-concept-q2rayk` and are named rather than linked
for that reason** — see the in-flight section of `Reports/README.md`.*

**Read §1 if you read nothing else.** Everything after it is the evidence, and
§7 is what the numbers ask somebody to decide.

The instrument is `examples/lab_cost.rs`; the picture half is
`examples/labshot.rs`. Both call `LabBox::build`, never a private copy.

---

## 1. The finding

**Gate 1 is met: the box lives.** Plants germinate, grow, flower, fruit, set
seed, breed to **generation 5**, die and are replaced — **285 organisms born
and 266 dead** across 90,000 frames from eight founders. The ants do not
breed, fall from 52 to 2, and hold there rather than going extinct.

**But it lives as two halves on different trajectories, and they are
coupled.** The plant half reaches rest; the animal half is a slow collapse
that takes a third of the stand down with it.

| at frame 90,000 | shipped box (1 colony) | same bed, `colonies=0` |
|---|---|---|
| plant organisms | 17 | 40 |
| plant cells | **685, still falling** (−0.8%, −10.3% across the last two tiles) | **1,287, settled** (−0.8%, −0.6%) |
| deepest plant generation | 3 | **5** |
| seeds set | 196 | 563 |
| ants | 2 | — |

Seven results, in the order they change a decision.

1. **The missing founders are neither germination failure nor invisibility.
   They are eaten.** All eight founders germinate — every one has cells at
   frame 900. One is dead by frame 900 and three by frame 5,000. Run the
   identical bed with `colonies=0` and **all eight are alive at frame
   45,000**. §3.
2. **Gate 0 is not reachable in this bed, and the reason is *reach*, not
   economy.** The shipped ant's margin here is **−820**, reproducing
   [creature-stamp-routes-2026-08-30.md](creature-stamp-routes-2026-08-30.md).
   Fit the matched gut that report's step 1 turns on and the colony gets
   visibly richer — bank 218 → **575**, standing colony energy 2,152 → 7,194,
   survivors at 48,000 frames 4 → 12 — and **births stay at exactly zero**,
   because 575 is the *leaf* ceiling and the bar is 1,040. §4.
3. **Fruit does stand — 22 to 40 rows up a stem, where a walking ant cannot
   go.** #162's step 1 is *sow a fruiting plant*, and the lab already does.
   What it does not produce is **windfall**: fruit that has dropped to the
   floor is the only fruit-class food an ant can reach, and across 90,000
   frames the standing windfall count is **0 at thirteen of sixteen census
   tiles and 1 or 2 at the other three**. §4.
4. **Frame cost is the field's solve set, not biomass — so the box gets
   *cheaper* as it runs.** Median frame **3.21 ms** at 345 plant cells and
   **1.97 ms** at 758, correlating **−0.80** with cells and **+0.95** with
   tiles solved. **The dial reaches 5.2x real time on a fresh box and 8.4x on
   a settled one with the draw taken out, 4.4x and 7.1x at a 20 Hz display.**
   Gate 3's warning that a mature box is the expensive one is backwards here.
   §5.
5. **The draw costs more than the tick, because the lab's air is drawn as
   sky.** 4.91 ms a frame at 512x320 — **30 ns/px against the feasibility
   report's 27.4 for empty sky** — against a 1.97 ms tick. It is the largest
   single thing between the speed dial and its ceiling, and the guide's own
   table already says not to do it. §5.3, §7.
6. **Partitions half reproduce, and the transferable quantity is the
   compartment's width in *chunks*.** Two chunks a compartment is worth
   **1.27x**; one chunk is **0.35x** and half a chunk **0.22x**, because the
   field solves one tile per chunk and a stone column through soil keeps the
   active-site scheduler busy for ever (**50x** that phase, reproduced with no
   creature in the box). §6.
7. **The organism ceiling is not a live constraint here.** High water **66
   slots of 4,095 — 1.6%** — and `organisms_refused` is **0** at every tile of
   every run. The 1,812–2,503 live organisms the guide quotes for `herb` are a
   *generated world* figure; this bed runs two orders of magnitude below it,
   and `births_denied_no_space` is 0 everywhere too, so every zero in this
   report is an energy or a reach result and never a space one. §2.

---

## 2. The bed, and the run

`LabBox::default()` unchanged: 512x320, 80 rows of soil under a stone floor,
ground line at row 160, a stone shell with a ceiling, the sky held at noon
(frame 3,599, amplitude 4.000) and weather pinned clear. Eight `herb`
founders spread across the usable width, one ant colony at the middle. Seed 1.

`found_colony` places **52 ants**, so the bed starts with 60 organisms: 8
plants and 52 animals.

**Everything in §§2–4 is a deterministic counter**, identical under any
machine load, and every figure was re-taken on the merged head that carries
#167's beetle sight sense and #156's motion decoys. The census reproduced
**row for row, unchanged** — which is itself the control saying those two
landings do not reach this bed.

### 2.1 The census

```
  frame |   orgs   cells  seeds  gen | fruit flower windfl   leaf | ants births deaths |  slot a/l   eats
      0 |      8       8      0    0 |     0      0      0      0 |   52      0      0 |   60/60        0
   6000 |     15     395     23    1 |     7      8      0    125 |   38      0     14 |   66/53       44
  12000 |     26     758     58    2 |     3     11      0    234 |   26      0     26 |   66/52       89
  18000 |     37    1002     97    3 |     6      1      0    235 |   19      0     33 |   66/56      117
  24000 |     44    1005    122    2 |    12      1      0    223 |   14      0     38 |   66/58      144
  30000 |     41    1036    154    3 |     3      3      1    217 |   11      0     41 |   66/52      163
  36000 |     34     976    170    4 |     2      0      2    201 |    8      0     44 |   66/42      179
  42000 |     19     945    178    3 |     0      0      0    189 |    6      0     46 |   66/25      190
  48000 |     22     938    184    3 |     0      0      0    176 |    4      0     48 |   66/26      198
  60000 |     19     851    192    3 |     0      0      0    147 |    4      0     48 |   66/23      210
  72000 |     17     794    203    3 |     0      0      0    131 |    2      0     50 |   66/19      219
  84000 |     19     764    211    3 |     0      0      0    119 |    2      0     50 |   66/21      227
  90000 |     17     685    196    3 |     0      0      0    108 |    2      0     50 |   66/19      230
```

**Read `plant cells`, not `orgs`, for the settling question** — the same
reasoning `CLAUDE.md` gives for reading `rock` rather than `cells lost` in
`seedsweep`. `orgs` swings 44 → 19 → 22 → 19 on germination and death events;
`cells` is the biomass and moves smoothly.

**The shipped box has not reached rest at 90,000 frames.** The last three
tiles read 770 → 764 → **685**, and the trend from frame 42,000 is a steady
decline: 945 → 685, −27% over 48,000 frames. The `colonies=0` control settles
cleanly at 1,305 → 1,295 → 1,287. So the answer to *how long is long enough*
is **~50,000 frames without a colony, and the box with one does not converge
at all inside 90,000** — it is still being eaten down.

### 2.2 The organism ceiling

| | shipped box | `colonies=0` | matched gut |
|---|---|---|---|
| slot high water | **66** | 119 | 70 |
| of the 4,095 ceiling | 1.6% | 2.9% | 1.7% |
| `organisms_refused` | **0** | 0 | 0 |
| `births_denied_no_space` | **0** | 0 | 0 |

Two orders of magnitude of headroom, and no run of any length or setting has
put a single organism near it. **The ceiling is a footnote in this bed, not a
design constraint** — and it will stay one until either the ants breed or the
bed gets much bigger, which is worth saying plainly because the brief and the
guide both flag it as a risk.

---

## 3. The founders: eight germinate, three are eaten

The open question handed to this lane was that the box plants 8 founders and
5 or 6 are visible at frame 900. **Germination failure and invisibility look
identical and mean opposite things**, so `labshot` now tracks each founder by
the organism id it held before the first tick: an id that no longer resolves
is a *death*, a small cell count is *invisibility*, and the photograph cannot
tell them apart.

Founder cell counts, same bed, same seed, with and without the colony:

| frame | with 1 colony | with no colony |
|---|---|---|
| 0 | `1 1 1 1 1 1 1 1` | `1 1 1 1 1 1 1 1` |
| 900 | `28 24 21 15 15 dead 42 25` | `28 24 18 25 18 19 42 25` |
| 5,000 | `75 54 dead 69 dead dead 81 51` | `76 57 77 78 34 74 84 53` |
| 20,000 | `130 49 dead 75 dead dead 69 41` | `85 55 68 77 32 67 81 49` |
| 45,000 | `120 46 dead 77 dead dead 62 40` | `102 52 66 69 32 68 70 45` |

**All eight germinate in both arms** — every founder has grown from its 1-cell
seed to 15–42 cells by frame 900. **So the answer is neither of the two
readings the question offered: it is a death.** Three of the eight are killed,
and the killer is the colony — at `colonies=0` the same eight are all standing
at frame 45,000, continuously alive throughout.

**Invisibility is a real second-order effect and does not account for the
complaint.** At frame 900 the two smallest survivors are 15-cell threads one
cell wide, which is why a viewer counts "5 or 6" where the census counts 7 —
but the census counts 7, not 8, and the eighth is gone. The missing founder is
**founder 5, at x = 340**, and the gap at that x is plain in the render.

### 3.1 The same result across twelve seeds

The two arms differ in more than the ants: `creature::step` draws from the
shared world RNG, so removing the colony makes it a different world after
frame 1 — the *arms-differ-in-two-things* trap. A seed sweep settles it,
because stream luck does not have a sign.

Twelve seeds, 20,000 frames each, both arms, counters only:

| seed | plants w/ colony | w/o | plant cells w/ | w/o | seeds set w/ | w/o |
|---|---|---|---|---|---|---|
| 1 | 42 | 95 | 1,042 | 1,162 | 107 | 224 |
| 2 | 48 | 169 | 737 | 1,795 | 136 | 389 |
| 3 | 50 | 79 | 665 | 969 | 110 | 172 |
| 4 | **8** | 53 | 434 | 986 | 13 | 118 |
| 5 | 40 | 116 | 357 | 1,086 | 107 | 237 |
| 6 | 17 | 49 | 444 | 866 | 46 | 111 |
| 7 | 45 | 132 | 1,213 | 1,694 | 137 | 296 |
| 8 | 15 | 83 | 302 | 914 | 77 | 172 |
| 9 | 44 | 84 | 645 | 1,135 | 84 | 183 |
| 10 | 46 | 162 | 542 | 1,807 | 131 | 326 |
| 11 | 26 | 91 | 861 | 1,462 | 64 | 163 |
| 12 | 47 | 130 | 600 | 1,532 | 86 | 274 |
| **median** | **43** | **93** | **623** | **1,149** | **97** | **204** |

**Twelve of twelve, on all three columns.** Median ratios: **2.9x** the
organisms, **2.1x** the biomass, **2.3x** the seed set without the colony.
Per-seed the spread is wide — the organism ratio runs 1.58 to 6.62, the
biomass ratio 1.12 to 3.33 — which is what a real effect looks like in this
engine, and the reason to read the sign across seeds rather than the
magnitude on one. Seed 4 with a colony ends at **8** plant organisms, i.e.
the founders and nothing else.

A sign that is unanimous over twelve independent worlds is not the RNG stream
moving under the arm.

---

## 4. Gate 0 in this bed: fruit stands, and no ant can reach it

`lab_cost` prices a birth the way
[creature-stamp-routes-2026-08-30.md](creature-stamp-routes-2026-08-30.md)
does — `ceiling − bar`, where `ceiling` is `hunger_fraction × start_energy`
plus the best mouthful **standing in this bed**, and `bar` is `birth_cost`
read from the engine rather than restated — `grant + body_energy × cells`,
80 + 960 = **1,040** — and it reads the gut back off a live founder so a run
cannot silently measure the neutral gut.

### 4.1 The shipped ant

```
gut +0.00 (founder reads +0.00) | start_energy 200 hunger_fraction 0.50 grant 80 body_energy 480
  leaf x108  yield 120.0   ant x4  yield 120.0   seed x9  yield 120.0
  corpse x19 yield 120.0   litter x4 yield 120.0  deadleaf x3 yield 120.0
ceiling 220 (satiety 100 + best mouthful standing here 120) against a bar of 1040
  => margin -820
richest bank actually reached in this run: 218
```

Every food in the bed yields the same 120 to a neutral gut, because
`diet_yield`'s matched filter flattens the material table for it. #162's −880
for the shipped ant reproduces here as **−820**; the difference is which foods
happen to be standing, and both are far outside any tuning.

### 4.2 The matched gut — the positive control

`gut=-1.0` builds the identical bed, writes the diet gene into the species
**before** `found_colony` stamps a founder's traits, and founds the colonies
at the same positions. The control that this is the same bed: `gut=0.0`
through the same path reproduces the default arm **exactly** at every tile
(orgs 44, cells 1005, seeds 122, ants 14, deaths 38, eats 144, richest 218 at
frame 24,000), so the only thing the arm changes is the gut.

| | shipped gut | matched gut (−1.0) |
|---|---|---|
| best mouthful standing, mid-run | 120 (leaf) | **1,440 (flower)** |
| ceiling, mid-run | 220 | **1,540** |
| margin against the 1,040 bar | −820 | **+500** |
| richest bank actually reached | 218 | **575** |
| ants alive at 48,000 | 4 | **12** |
| colony energy at 24,000 | 2,152 | **7,194** |
| **births** | **0** | **0** |

**The margin says a birth is affordable and the bank says no ant ever got
near it.** 575 is the leaf ceiling almost exactly (100 + 480 = 580); a
flower-eating ant would sit at 1,540 and clear the bar on the spot. `births`
is a cumulative counter, so it closes the sampling hole the periodic `richest`
reading leaves: not one ant, in 48,000 frames, ever held 1,040 at a
reproduction check.

This is the case
[creature-stamp-routes-2026-08-30.md](creature-stamp-routes-2026-08-30.md) §5
names in advance: *"If fruit cells stand in the world and R3 still reads 0
births, the fruit is out of foraging range and this step is dead — which is a
foraging problem, not an economy one."* **It is that, and this lane can name
the mechanism.**

### 4.3 The mechanism: fruit stands up a stem, and ants walk on the floor

`lab_cost` reports how high the fruit-class food stands above the soil line,
as `(lowest, highest)` rows:

```
  frame | fruit flower windfall | ant->food  food up
   6000 |     7      8        0 |        18   24..40
  12000 |     3     11        0 |        19   22..33
  18000 |     6      1        0 |        18   22..32
  24000 |    12      1        0 |        17   27..32
  30000 |     3      3        1 |         7    5..31
  36000 |     2      0        2 |         5    2..29
  42000+|     0      0        0 |   no food        -
```

**Every flower and every attached fruit stands 22 to 40 rows above the soil,
on a herb's stem.** The two tiles where the nearest ant is 5 or 7 cells away
are exactly the two tiles where `windfall` is non-zero — dropped fruit, on the
ground, where an ant walks. Standing windfall across the whole 90,000-frame
run is **1 cell at frame 30,000 and 2 at frame 36,000, and zero everywhere
else**; the matched-gut arm produced **none at all** in 48,000 frames.

So the reachable fruit supply in this bed is, to two significant figures,
**nothing**. That is not a foraging-range problem an ant could solve by
walking further, and it is not an economy problem: it is a **delivery**
problem, and #162's own step 1 names the delivery it depends on — *"it ripens,
it falls as `windfall` to where the ants walk, and it is gone"*. The falling
is the half that is not happening at a rate any colony could live on.

**What would settle it, cheapest first**, none of which this lane built:

- **Count what happens to a ripe fruit.** `Behavior::Ripen`'s fruit→windfall
  transition has no counter, so "fruit rarely drops" and "fruit drops and is
  eaten or buried within a frame" are indistinguishable from a standing
  census. That distinction decides everything below it.
- **If fruit does not drop**, the herb's fruit `Ripen(rate: 0.012)` clock and
  the stand's short life are the two candidates — a plant grazed to death
  before its fruit ripens never drops one, which would make this a
  *consequence* of §3 rather than an independent fault.
- **If it drops and vanishes**, windfall on soil is the suspect: it is a
  falling material landing on a `Powder`, and the litter line has already paid
  for one census of where shed matter comes to rest (`litter_probe`).
- **A lab-only `ant.ron` does not fix this.** The gut arm is the proof: the
  gene that #162 prices is already worth +500 of margin here and buys zero
  births, because the food it unlocks is 22 rows over an ant's head.

### 4.4 "The deadlock is one heritable step wide" — the arithmetic holds and the conclusion does not

A claim reached this lane mid-run, from the ant line: that the standard lab
bed's breeding margin reads −640 rather than the ants-only bed's −880, that
the 240-point difference implies a **360-point best standing mouthful**, that
360 at a neutral gut implies a **1,440 flower standing in the box**, and
therefore that *a gut drifted to −1 would draw the whole 1,440 and clear the
bar outright — one mutation, not an engine change.*

**Every step of that arithmetic is correct.** Checked at the source rather
than taken on relay:

```rust
// creature.rs::diet_yield
let quality = (1.0 - (gut_bias - class).abs() / 2.0).clamp(0.0, 1.0);
worth * quality * quality
```

`flower.ron` carries `food_energy: 1440.0, food_class: -1.0`. A neutral gut
scores `quality = 1 − 1/2 = 0.5`, squared **0.25**, so a flower yields
**360** — the inferred number, exactly. A gut at −1.0 scores `quality = 1.0`
and yields the whole **1,440**. `birth_cost` is `grant + body_energy × cells`
= 80 + 960 = **1,040** (the 1,100 in the relayed version is
`reproduce_threshold`, which `birth_cost` does not read; either bar is
cleared). So a matched gut that eats one flower sits at **1,540** against a
1,040 bar.

**And the census confirms the flower is really there**, counted as cells in
the grid rather than inferred from a margin or from organs built: 1 to 12
standing `flower` cells and 1 to 12 standing `fruit` cells at every tile up to
frame 36,000 (§4.3). The inference was right about the world.

**The conclusion is still wrong, and this lane ran the experiment.** `gut=-1.0`
*is* the drifted gut, applied to the founders before the colony is stamped.
The harness prints the margin it produces — **+500** — and 48,000 frames later
the birth counter reads **zero**, the richest bank ever held reads **575**,
and `births_denied_no_space` reads **0**.

575 is not a near miss. It is `hunger_fraction × start_energy + leaf`
= 100 + 480 = **580**, to within a rounding of the satiety line. **The colony
ate leaves for 48,000 frames and never once ate the flower**, and the two
counters that would show otherwise cannot both stay at zero if it had: an ant
at 1,540 either reproduces (`births`) or is refused for space (`denied`).

So the deadlock in this bed is **not** one heritable step wide. The gene is
already worth +500 of margin, and it buys nothing, because the food it unlocks
grows 22 to 40 rows above an ant's head (§4.3) and the ground-level form of it
— `windfall` — stands at 1 or 2 cells for two census tiles out of sixteen and
at zero for the rest.

**What the gut does buy is survival**, and that is a real result rather than a
consolation: colony energy 2,152 → **7,194** at frame 24,000, and survivors at
48,000 frames **4 → 12**. Route 3 keeps the colony alive three times longer.
It does not start it breeding.

---

## 5. What it costs

**Taken on a quiet box**, load average **3.5–4.9** across the whole batch,
after four hours in which it ran 22–38. Every arm is the median of three or
five alternating repetitions, and every timing column below is the **median
frame**, not the mean — under any residual contention the mean carries the
tail and the median does not. §8 has the provenance for the loud figures the
earlier runs produced, and why none of them is quoted here.

### 5.1 The frame gets *cheaper* as the run goes on — Gate 3's warning is backwards here

Gate 3 says to measure at the population the lab actually runs, *"not at a
founder cohort — cost follows biomass and a mature box is the expensive
one"*. **In this bed the founder cohort is the expensive one.**

Median of five alternating runs, quiet box:

| frame | plant cells | p50 ms | mean ms | solved/f (of 40) | awake/f (of 40) | µs/cell | x real time |
|---|---|---|---|---|---|---|---|
| 3,000 | 345 | **3.21** | 4.12 | 32.6 | 4.7 | 10.4 | 5.2x |
| 6,000 | 395 | 2.38 | 3.22 | 25.7 | 2.9 | 7.4 | 7.0x |
| 9,000 | 528 | 2.18 | 2.61 | 25.8 | 3.1 | 4.7 | 7.7x |
| 12,000 | 758 | **1.97** | 2.51 | 20.2 | 3.1 | 3.2 | **8.4x** |

Biomass **doubles** while the frame gets **39% cheaper**. Over these four
tiles the median frame correlates **−0.80** with plant cells and **+0.95**
with the field's solve set. Over the fifteen tiles of a 90,000-frame run the
same pair reads **−0.02** and **+0.90**, with `awake/f` at **+0.89**, and the
frame falls 3.26 ms → 1.45 ms — about **12x real time** — while biomass goes
395 → 1,036 → 685.

**Cost in the lab box is the field's solve set, not biomass.** The early
frames are expensive because 80 rows of freshly written soil are still
settling and most of the box is awake; once it settles the box is cheap, and
stays cheap while the stand doubles.

That is a real correction to the guide's sizing rule (§2b: *"roughly 1–2 µs
per living plant cell per tick, falling as the stand grows"*), which that
section itself labels *"a sizing rule, not a model — do not trust this past a
factor of two"*. The µs-per-cell figure here runs **10.4 early and 3.2 late**,
so the magnitude is the right order and falling; the *variable* is wrong. What
falls is not the cost per cell — it is the number of tiles the field still has
anything to say about.

**The practical consequence is the pleasant direction**: a lab session gets
faster the longer it runs, and the worst frame a player ever sees is in the
first few thousand ticks of a fresh box.

### 5.2 Which phase is paying

`frame::step` split at its own seams, and the split checked against
`frame::step` itself by a full-grid hash before any of these numbers is
printed (§8). Means, at the 758-cell tile, quiet box:

| phase | ms | share |
|---|---|---|
| **field** | **2.245** | **69%** |
| pheromones | 0.461 | 14% |
| ca_sweep | 0.360 | 11% |
| active_sites | 0.177 | 5% |
| liquid_bodies, chunk_bodies, player, particles | **0.000** | 0% |

The four zeroes are the feasibility report's §3c prediction landing exactly: a
sealed box with no rock, no blast and no gnome pays nothing for those phases,
which is why the lab can run the shipped tick unmodified instead of forking
it.

**The field is the lab's whole simulation cost**, and it is already running
with both of the things that drive it removed — the sky is held and the
weather is pinned. Anything that makes the *simulation* faster has to come out
of `step_fields`, or out of the number of chunks it runs over.

### 5.3 The draw costs more than the tick

Measured in the lab bed through the shipped `Renderer` with the dirty-rect
skip live, quiet box, median of three:

| how often the box is drawn | ms per draw | ns per pixel |
|---|---|---|
| every tick (a **Tending** display) | **4.91** | 30 |
| every 20 ticks (a **Running** display) | **8.12** | 50 |

30 ns/px against the feasibility report's **27.4 ns/px for empty sky** is not
a coincidence: **the lab's 156 rows of air above the soil are being drawn as
sky**, gradient and star hash included, which is the one thing the guide's own
measurement table says must not happen — *"whatever fills the air above the
soil must not draw as sky"*. It is visible in every `labshot` panel. §7.

The second row is higher for a good reason rather than a bad one: 20 ticks of
simulation dirty more chunks than one does, so the skip has less to skip.

### 5.4 The multiplier, which is the number the speed dial needs

A tick is 1/60th of a simulated second, so with a draw costing `R` at a
display rate `hz`, simulated seconds per real second is
`(1000/hz − R) / tick_ms`. At `R = 0` the two rates agree — the arithmetic's
own check, since the whole advantage of a slower display is paying the draw
less often rather than running more ticks per tick.

| | fresh box (345 cells) | settled box (758 cells) |
|---|---|---|
| tick, median frame | 3.21 ms | 1.97 ms |
| **simulation only** | **5.2x** | **8.4x** |
| **60 Hz display** (draw 4.91) | **3.7x** | **6.0x** |
| **20 Hz display** (draw 8.12) | **4.4x** | **7.1x** |

At the 90,000-frame run's settled end the simulation-only figure reaches
**~12x**.

Three things to carry out of that table:

- **The dial's ceiling rises during a session**, from roughly 4x to 7x at a
  20 Hz display and higher on a long run. Quote a range, not a number.
- **The display rate is worth about 19%, not the tripling §2b expects.** That
  estimate assumed the draw dominates the budget, and it does — but dropping
  to 20 Hz pays a 5–8 ms draw three times less often against a tick that is
  already only 2 ms, and the ratio lands at 1.19x rather than 3x.
- **The draw is the larger term, and it is the one with an obvious fix.** At
  the settled tick the simulation alone would run 8.4x and the renderer takes
  it to 6.0x. Stopping the lab interior drawing as sky is worth more to the
  speed dial than anything available inside the simulation.

---

## 6. Partitions: they work down to two chunks a compartment, and invert below that

§2c is the guide's strongest single finding — a fanned 2048-wide bed walled
into 16 compartments went from 4.1x to 7.6x with the stand held to within
0.2%. **It half reproduces here, and the half that fails is worth more than
the half that works.**

One fan, offset by a third of a spacing so it never sits on a partition (the
scene error §2c records having paid for), 12,000 frames, median of three
round-robin repetitions, quiet box:

| compartments | width each | **chunks each** | p50 ms | solved/f (of 40) | plant cells | vs open |
|---|---|---|---|---|---|---|
| 1 | 512 | 8 | 3.06 | 38.3 | 666 | 1.00x |
| 2 | 256 | 4 | 2.68 | 33.7 | 554 | **1.14x** |
| 4 | 128 | **2** | **2.40** | **28.1** | 663 | **1.27x** |
| 8 | 64 | 1 | 8.85 | 36.0 | 499 | **0.35x** |
| 16 | 32 | 0.5 | 13.84 | 38.6 | 412 | **0.22x** |

**Down to four compartments the mechanism is exactly §2c's**: `solved/f` falls
38.3 → 28.1, a 27% cut in the field's solve set, and the frame follows it to a
**1.27x** speed-up. Smaller than the 1.85x §2c reports, in the same direction
and for the same reason.

**At eight and sixteen it inverts, and two independent readings say why.**

**First, containment stops.** `solved/f` climbs back to 36.0 and 38.6 — the
whole box again. The field solves **one tile per 64-cell chunk**, so a
compartment narrower than a couple of chunks cannot hold a tile inside it and
every tile straddles a wall. Four compartments is 128 cells, two chunks, and
is the last row that contains anything; eight is exactly one chunk, sixteen is
half of one.

**Second, the wall has a cost of its own, and it is not the field.** The
per-phase split at the same tiles:

| compartments | ca_sweep | active_sites | field |
|---|---|---|---|
| 1 | 0.366 | **0.130** | 2.859 |
| 4 | 0.406 | **0.150** | 2.206 |
| 8 | 2.295 | **4.020** | 3.139 |
| 16 | 1.728 | **6.596** | 2.676 |

The field barely moves. **`active_sites` goes up 50x**, and `awake/f` goes
3.1 → 9.5: a quarter of the box never sleeps again.

**That is the walls and not the colony**, which is the control this needed.
`LabBox` founds its colony at `width/2`, which is *also* where the partition
goes at every power-of-two compartment count, so the obvious suspect was 52
ants stamped onto a stone column — the identical scene error §2c records for
its own fan. Re-run with `colonies=0`, no ant anywhere near a wall:

| compartments | p50 ms | active_sites | awake/f | plant cells |
|---|---|---|---|---|
| 1 | 2.06 | 0.151 | 3.5 | 1,095 |
| 8 | 10.75 | **8.503** | 9.5 | 992 |
| 16 | 11.33 | **6.175** | 8.0 | 687 |

The explosion reproduces with no creature in the box at all. **A stone column
written through soil keeps the active-site scheduler busy for ever**, and that
is the cost partitions actually carry here.

**Two cautions, both against the finding rather than for it.** The stand is
*not* held constant the way §2c held it — 666 / 554 / 663 / 499 / 412 cells —
so the expensive arms are expensive while carrying up to 38% *less* biomass,
and the cheap arms would be cheaper still at equal stands. And while the
colony-on-the-partition is not the cause of the inversion, **it is still a
live scene error in `LabBox::build`** at every power-of-two compartment count;
somebody should move one of the two.

**What this means for the design.** Partitions remain worth having for
evolutionary isolation and for §5 of the guide's scoring, and at lab scale
they are also worth about **1.27x** of frame — provided a compartment stays at
least two chunks, 128 cells, wide. Below that they are a **3–5x cost**. §2c's
16-compartment result is a 2048-wide bed, where 16 compartments is *still* 128
cells each; **the quantity that transfers is the compartment width in chunks,
not the compartment count.**

---

## 7. What this asks somebody to decide

1. **Is a colony that grazes the bed down and then starves the opening the
   lab wants?** It is a real food web and it is the most alive thing in the
   box — but it costs the stand a third of its founders, half its biomass and
   a whole generation of turnover, and the colony does not survive it either.
   Posted to the review queue as a paired A/B with the counts under each
   panel (card `20260830T102106955Z-327527`).
2. **Gate 0 needs the fruit to reach the floor, not a better gut.** §4.3. The
   next measurement is a counter on fruit→windfall, and it is small.
3. **The lab interior draws as sky, and it costs more than the simulation.**
   The guide's own table says *"whatever fills the air above the soil must not
   draw as sky"* — empty sky is 27.4 ns/px against stone's 6.7 — and the shell
   as built leaves 156 rows of open air that `Renderer` shades as a sky
   gradient. Measured: **30 ns/px, 4.91 ms a frame, against a 1.97 ms tick**
   (§5.3). It takes the settled dial from 8.4x to 6.0x. That is a Lane B
   decision on `scene.rs`, and it is the single largest thing standing between
   the speed dial and its ceiling.
4. **`LabBox` founds its colony exactly where its partition goes.**
   `colony_spacing` puts one colony at `width/2`, and so does the partition at
   every power-of-two compartment count — the same scene error §2c records
   having paid for with its fan, in a different file. It is **not** the cause
   of §6's inversion (the `colonies=0` control reproduces that), but it means
   no `compartments > 1` run has ever placed a colony in open ground. One of
   the two should move.
5. **Nothing has run Gate 2 in this bed.** `selection_arena`'s `arm=` ladder
   is the teeth-test, and its own finding is that a null there is a statement
   about the world rather than the genome. Everything in this report is about
   whether the bed is *alive*; none of it says whether it *discriminates*.

---

## 8. Provenance, and what this does not measure

**The counters are deterministic and the timings are not.** Every figure in
§§2–4, and every `cells`, `solved/f` and `awake/f` column in §§5–6, is a count
that reproduces exactly under any load. The milliseconds do not, and this
report was written on a shared four-core container running three other agent
sessions against the same repo.

**So the timings were taken twice, and only the quiet set is quoted.** Load
average over the day, from the harness's own log:

| batch | load average | what became of it |
|---|---|---|
| first cost pass | 22 → 25 | **discarded** — mean 4.46 ms against a p50 of 2.37, a 1.9x gap that is the machine |
| render pass | 35 → 38 | **discarded** — 22.3 and 84.5 ms per draw, i.e. 136 and 516 ns/px, against a shipped sky's 27.4 |
| partition pass | 37 | kept only as a cross-check on the quiet re-run; same ordering, same inversion |
| **the quoted batch** | **3.5 → 4.9** | §5 and §6 |

On the quiet box mean and median frame close to within 27% (2.51 against
1.97 ms) where the loud ones stood 90% apart, which is the tell that what was
being measured earlier was the other three sessions. Every worst-frame figure
this harness produced failed its own `mean x frames ~= worst` check — ratios
of 50 to 2,285 — and the harness refuses to let one be quoted; none is.
`Reports/measurement-under-contention.md` is the standing account of this
failure mode in this repo.

**What the instrument checks about itself.** `lab_cost selftest=1` runs four
positive controls, because a number that is arithmetically correct and cannot
move looks exactly like a result:

```
[PASS] split tick reproduces frame::step over 200 frames
[PASS] leaf census: 191 in a planted bed at frame 4000, 0 in an unplanted one
[PASS] one fan wakes the box: solved/f 2.5 with no fan, 39.6 with one
[PASS] slot high water moves with founders: 2 at 2, 16 at 16
```

The first is load-bearing for §5: the per-phase breakdown re-types
`frame::step`'s sequence, which is precisely the fork `sim/frame.rs` exists to
prevent, so the harness hashes a real lab box stepped both ways and prints the
comparison before any phase number is quoted.

**Not measured here**, and each is somebody's next question rather than a gap
in this one:

- **Gate 2** — whether selection has teeth in this bed. `selection_arena`'s
  `arm=` ladder has never been run in a hand-built lab bed, and this report
  says nothing about it.
- **Why plants die.** `p.died` is 266 across 90,000 frames and the census does
  not attribute a single one. Grazing, shading, starvation and senescence are
  all live and all indistinguishable in these columns.
- **Whether the decline reverses.** The shipped box was run to 90,000 frames
  and was still falling. Nothing here says where it stops.
- **Soil depth.** §2a of the guide asks that something reach the depth the
  bed pays 1.9x for; `root_contact` answers it and was not run.
