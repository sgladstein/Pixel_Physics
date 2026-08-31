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
`examples/labshot.rs`. Both call `LabBox::build`, never a private copy, and
both take every bed knob's default *from `LabBox::default()`* rather than from
a literal — see §8 for the half-day that rule cost.

---

## 1. The finding

**Gate 1 is met: the box lives.** Plants germinate, grow, flower, fruit, set
seed, breed to **generation 4**, die and are replaced — **334 organisms born
against 305 dead** across 90,000 frames from eight founders, with the turnover
still running at the last tile. **The ants do not breed, and the colony goes
extinct**: 52 founded, 52 dead, 0 born, gone by frame 66,000.

**And the two halves are coupled.** The colony is the single largest thing
acting on the stand while it lasts, and the bed only settles once it is gone.

| at frame 90,000 | shipped box (1 colony) | same bed, `colonies=0` |
|---|---|---|
| plant organisms | 29 | 42 |
| plant cells | **598, settled** (−0.8%, −1.6%) | **1,146** (−6.3%, −0.7%) |
| deepest plant generation | 3 | **4** |
| seeds set | 265 | 563 |
| ants | **0** | — |

Seven results, in the order they change a decision.

1. **The founders that go missing are eaten — not ungerminated, and not
   merely too small to draw.** All eight germinate (`placed: 8 of 8`, `still a
   seed 0` by frame 900); one is dead by frame 900, three by 5,000 and **five
   by 66,000**. The identical bed with `colonies=0` still has seven of the
   eight at frame 66,000, and the effect holds **12 seeds of 12 on every
   column**. §3.
2. **Gate 0 is not reachable in this bed, and the reason is *reach*, not
   economy.** The shipped ant's margin here is **−820**. Fit the matched gut
   `creature-stamp-routes-2026-08-30.md` prices and the colony gets visibly
   richer — bank 219 → **568**, survivors at 48,000 frames 3 → 9 — and
   **births stay at exactly zero**, because 568 is the *leaf* ceiling (580)
   and the bar is 1,040. §4.
3. **Fruit stands, 22 to 40 rows up a stem, where a walking ant cannot go.**
   The lab already does what #162's step 1 asks — it sows a fruiting species.
   What it does not produce is **windfall**: dropped fruit is the only
   fruit-class food at ground level, and across 90,000 frames the standing
   windfall count never exceeds **1**. §4.3.
4. **Frame cost is the field's solve set, not biomass — so the box gets
   *cheaper* as it runs.** Median frame **1.84 ms** at 345 plant cells and
   **0.94 ms** at 598 after 90,000 frames, while `solved/f` falls 31.6 → 8.8.
   Over fifteen tiles the frame correlates **+0.03** with plant cells and
   **+0.92** with tiles solved. Gate 3's warning that a mature box is the
   expensive one is backwards here. §5.1.
5. **The draw costs three times the tick, and half the draw is `sky_light` in
   a box with a ceiling.** 4.78 ms a frame against a 1.28 ms tick, of which
   **2.8 ms is the sky-light pass**, the one phase that does not fall when
   less of the frame changes. It is the largest single thing between the speed
   dial and its ceiling. §5.3.
6. **The dial: about 8x real time on a fresh box, rising past 15x once it
   settles, at a 20 Hz display** — 6.4x to 12.7x at 60 Hz, 9x to 17.8x with the
   draw taken out. It rises through a session rather than falling. §5.4.
7. **Partitions contain the air exactly as §2c says, and it no longer buys a
   frame.** `solved/f` falls **39.4 → 25.1** across 1 → 16 compartments, a 36%
   cut — and the frame moves 1.69 → 1.41 → 1.92 ms, non-monotone, because the
   field is only **54%** of a 1.5 ms tick and there is not enough of it left to
   win. §6.

**The organism ceiling is a footnote.** High water **66 slots of 4,095 —
1.6%** — with `organisms_refused` **0** and `births_denied_no_space` **0** at
every tile of every run, so every zero in this report is an energy or a reach
result and never a space one.

---

## 2. The bed, and the run

`LabBox::default()` as it ships after Lane B's soil re-derivation: 512x320,
**40 rows of soil** on a stone base, ground line at row 160, a stone shell with
a ceiling, grow lights at 128-column spacing, the sky held at noon (frame
3,599, amplitude 4.000) and weather pinned clear. Eight `herb` founders and one
ant colony, both placed inside compartments. Seed 1.

`found_colony` places **52 ants**, so the bed starts with 60 organisms: 8
plants and 52 animals.

**Everything in §§2–4 is a deterministic counter**, identical under any machine
load. §§5–6's milliseconds were taken at load average **0.85–2.8**; §8 has the
provenance, including the four hours of measurement at load 22–38 that this
report discards.

### 2.1 The census

```
  frame |   orgs   cells  seeds  gen | fruit flower windfl   leaf | ants births deaths | slot a/l  eats
      0 |      8       8      0    0 |     0      0      0      0 |   52      0      0 |  60/60       0
   6000 |     18     476     23    1 |     7      8      0    130 |   37      0     15 |  66/55      46
  12000 |     28     663     51    2 |     3      9      1    176 |   26      0     26 |  66/54      88
  18000 |     36     760     79    2 |     3      5      1    188 |   20      0     32 |  66/56     120
  24000 |     43     822    107    3 |     7      2      0    180 |   11      0     41 |  66/54     141
  30000 |     40     773    135    3 |     4      0      1    177 |    9      0     43 |  66/49     156
  36000 |     24     725    149    3 |     0      0      0    163 |    6      0     46 |  66/30     167
  48000 |     32     689    176    3 |     0      0      0    144 |    3      0     49 |  66/35     179
  60000 |     28     619    202    3 |     0      0      0    123 |    1      0     51 |  66/29     184
  66000 |     30     628    215    3 |     0      0      0    119 |    0      0     52 |  66/30     184
  78000 |     26     613    240    3 |     0      0      0    110 |    0      0     52 |  66/26     184
  90000 |     29     598    265    3 |     0      0      0    104 |    0      0     52 |  66/29     184
```

**Read `plant cells`, not `orgs`, for the settling question** — the same
reasoning `CLAUDE.md` gives for reading `rock` rather than `cells lost` in
`seedsweep`. `orgs` swings on germination and death events; `cells` is the
biomass and moves smoothly.

**The bed settles, and it settles once the colony is gone.** Plant cells peak
at 822 around frame 24,000, fall while the ants are eating, and hold at
598–628 from frame 60,000 — the last three tiles read 613 → 608 → 598, −0.8%
then −1.6%. The ants are extinct at 66,000. **Seeds set and organism turnover
keep climbing all the way to the last tile** (265 seeds, 334 born against 305
dead), so what settles is a *living* stand rather than a stopped one — which is
the distinction a standing count alone cannot make.

**How long is long enough**, for anyone measuring in this bed next: about
60,000 frames with a colony, and the `colonies=0` control is still moving at
90,000 (−6.3%, −0.7%).

### 2.2 The organism ceiling

| | shipped box | `colonies=0` | matched gut |
|---|---|---|---|
| slot high water | **66** | 131 | 66 |
| of the 4,095 ceiling | 1.6% | 3.2% | 1.6% |
| `organisms_refused` | **0** | 0 | 0 |
| `births_denied_no_space` | **0** | 0 | 0 |

Two orders of magnitude of headroom, and no run of any length or setting has
put a single organism near it. **The ceiling is a footnote in this bed, not a
design constraint** — worth saying plainly, because both the brief and the
guide flag it as a risk. It stays a footnote until either the ants breed or the
bed gets much bigger.

---

## 3. The founders: eight germinate, five are eaten

The open question handed to this lane was that the box plants 8 founders and 5
or 6 are visible at frame 900. **Germination failure and invisibility look
identical and mean opposite things**, so `labshot` now tracks each founder by
the organism id it held before the first tick: an id that no longer resolves is
a *death*, a small cell count is *invisibility*, and the photograph cannot tell
them apart.

Founder cell counts, same bed, same seed, with and without the colony —
`dead` means the id no longer resolves:

| frame | with 1 colony | with no colony |
|---|---|---|
| 0 | `1 1 1 1 1 1 1 1` | `1 1 1 1 1 1 1 1` |
| 900 | `28 24 21 15 15 dead 42 25` | `28 24 18 25 18 19 42 25` |
| 5,000 | `94 54 dead 69 dead dead 81 51` | `76 57 77 78 34 74 84 53` |
| 24,000 | `131 43 dead 63 dead dead 67 44` | `77 54 67 73 32 63 73 49` |
| 66,000 | `115 dead dead dead dead dead 63 42` | `114 48 dead 62 31 51 67 44` |

**All eight germinate in both arms**, and two independent readings say so: the
builder reports `placed: 8 of 8 founders`, and by frame 900 `still a seed`
reads **0** while every founder has grown from its 1-cell seed to 15–42 cells.
**So the answer is neither of the two readings the question offered: it is a
death.** With the colony, one founder is gone by frame 900, three by frame
5,000 and **five by frame 66,000**; without it, seven of the eight are still
standing at 66,000.

**Invisibility is a real second-order effect and does not account for the
complaint.** At frame 900 the two smallest survivors are 15-cell threads one
cell wide, which is why a viewer counts "5 or 6" where the census counts 7 —
but the census counts 7, not 8, and the eighth is gone.

### 3.1 The same result across twelve seeds

The two arms differ in more than the ants: `creature::step` draws from the
shared world RNG, so removing the colony makes it a different world after frame
1 — the *arms-differ-in-two-things* trap. A seed sweep settles it, because
stream luck does not have a sign.

Twelve seeds, 20,000 frames each, both arms, counters only:

| seed | plants w/ colony | w/o | plant cells w/ | w/o | seeds set w/ | w/o |
|---|---|---|---|---|---|---|
| 1 | 34 | 125 | 796 | 1,539 | 85 | 249 |
| 2 | 73 | 237 | 1,042 | 2,277 | 178 | 468 |
| 3 | 25 | 62 | 537 | 976 | 82 | 144 |
| 4 | **6** | 55 | 363 | 929 | 13 | 121 |
| 5 | 85 | 130 | 886 | 1,094 | 186 | 246 |
| 6 | 25 | 76 | 644 | 956 | 92 | 158 |
| 7 | 19 | 116 | 755 | 1,657 | 78 | 279 |
| 8 | 56 | 76 | 808 | 1,143 | 139 | 183 |
| 9 | 41 | 91 | 588 | 1,329 | 85 | 180 |
| 10 | 58 | 148 | 994 | 1,480 | 187 | 329 |
| 11 | 23 | 79 | 563 | 1,305 | 45 | 155 |
| 12 | 37 | 83 | 700 | 1,263 | 76 | 216 |
| **median** | **36** | **87** | **728** | **1,284** | **85** | **200** |

**Twelve of twelve, on all three columns.** Median ratios without the colony:
**2.8x** the organisms, **1.9x** the biomass, **2.4x** the seed set. Per seed
the spread is wide — the organism ratio runs 1.36 to 9.17, the biomass ratio
1.23 to 2.56 — which is what a real effect looks like in this engine, and the
reason to read the sign across seeds rather than the magnitude on one. Seed 4
with a colony ends at **6** plant organisms: fewer than the eight it was
planted with.

A sign that is unanimous over twelve independent worlds is not the RNG stream
moving under the arm.

---

## 4. Gate 0 in this bed: fruit stands, and no ant can reach it

`lab_cost` prices a birth the way
[creature-stamp-routes-2026-08-30.md](creature-stamp-routes-2026-08-30.md)
does — `ceiling − bar`, where `ceiling` is `hunger_fraction × start_energy` plus
the best mouthful **standing in this bed**, and `bar` is `birth_cost` read from
the engine rather than restated (`grant + body_energy × cells` = 80 + 960 =
**1,040**). It reads the gut back off a live founder, so a run cannot silently
measure the neutral gut.

### 4.1 The shipped ant

```
gut +0.00 (founder reads +0.00) | start_energy 200 hunger_fraction 0.50 grant 80 body_energy 480
  leaf x104  yield 120.0    seed  yield 120.0    corpse  yield 120.0
ceiling 220 (satiety 100 + best mouthful standing here 120) against a bar of 1040
  => margin -820      richest bank actually reached: 219
```

Every food in the bed yields the same 120 to a neutral gut, because
`diet_yield`'s matched filter flattens the material table for it. #162's −880
for the shipped ant reproduces here as **−820**; the difference is which foods
happen to be standing, and both are far outside any tuning.

### 4.2 The matched gut — the positive control

`gut=-1.0` builds the identical bed, writes the diet gene into the species
**before** `found_colony` stamps a founder's traits, and founds the colonies at
the same positions. The control that this is the same bed: `gut=0.0` through
the same path reproduces the default arm **exactly** at every tile, so the only
thing the arm changes is the gut.

| | shipped gut | matched gut (−1.0) |
|---|---|---|
| best mouthful standing, mid-run | 120 (leaf) | **1,440 (flower)** |
| ceiling, mid-run | 220 | **1,540** |
| margin against the 1,040 bar | −820 | **+500** |
| richest bank actually reached | 219 | **568** |
| ants alive at 48,000 | 3 | **9** |
| **births** | **0** | **0** |

**The margin says a birth is affordable and the bank says no ant ever got near
it.** 568 is the leaf ceiling almost exactly (100 + 480 = 580); a
flower-eating ant would sit at 1,540 and clear the bar on the spot. `births` is
cumulative, so it closes the sampling hole the periodic `richest` reading
leaves: not one ant, in 48,000 frames, ever held 1,040 at a reproduction check
— and `births_denied_no_space` is 0, so it is not space either.

This is the case
[creature-stamp-routes-2026-08-30.md](creature-stamp-routes-2026-08-30.md) §5
names in advance: *"If fruit cells stand in the world and R3 still reads 0
births, the fruit is out of foraging range and this step is dead — which is a
foraging problem, not an economy one."* **It is that, and this lane can name
the mechanism.**

**What the gut does buy is survival**, and that is a result rather than a
consolation: survivors at 48,000 frames **3 → 9**, three times the colony left
alive. Route 3 keeps the colony going. It does not start it breeding.

### 4.3 The mechanism: fruit stands up a stem, and ants walk on the floor

`lab_cost` reports how high the fruit-class food stands above the soil line, as
`(lowest, highest)` rows:

```
  frame | fruit flower windfall | ant->food  food up
   6000 |     7      8        0 |        18   24..40
  12000 |     3      9        1 |         6    1..32
  18000 |     3      5        1 |         5    5..31
  24000 |     7      2        0 |        20   23..31
  30000 |     4      0        1 |        24    1..28
  36000+|     0      0        0 |   no food        -
```

**Every flower and every attached fruit stands 22 to 40 rows above the soil, on
a herb's stem.** The tiles where the nearest ant is 5 or 6 cells away are
exactly the tiles where `windfall` is non-zero — dropped fruit, on the ground,
where an ant walks. Standing windfall across the whole 90,000-frame run never
exceeds **1 cell**.

So the reachable fruit supply in this bed is, to two significant figures,
**nothing**. That is not a foraging-range problem an ant could solve by walking
further, and it is not an economy problem: it is a **delivery** problem, and
#162's own step 1 names the delivery it depends on — *"it ripens, it falls as
`windfall` to where the ants walk, and it is gone"*. The falling is the half
that is not happening at a rate any colony could live on.

**What would settle it, cheapest first**, none of which this lane built:

- **Count what happens to a ripe fruit.** `Behavior::Ripen`'s fruit→windfall
  transition has no counter, so *"fruit rarely drops"* and *"fruit drops and is
  eaten or buried within a frame"* are indistinguishable from a standing
  census. That distinction decides everything below it.
- **If fruit does not drop**, the herb's `Ripen(rate: 0.012)` clock and the
  stand's short life are the two candidates — a plant grazed to death before
  its fruit ripens never drops one, which would make this a *consequence* of §3
  rather than an independent fault.
- **If it drops and vanishes**, windfall landing on soil is the suspect;
  `litter_probe` has already paid for one census of where shed matter comes to
  rest.
- **A lab-only `ant.ron` does not fix this.** The gut arm is the proof: the
  gene #162 prices is already worth +500 of margin here and buys zero births,
  because the food it unlocks is 22 rows over an ant's head.

### 4.4 "The deadlock is one heritable step wide" — the arithmetic holds and the conclusion does not

A claim reached this lane mid-run, from the ant line: that the standard lab
bed's breeding margin reads −640 rather than an ants-only bed's −880, that the
240-point difference implies a **360-point best standing mouthful**, that 360
at a neutral gut implies a **1,440 flower standing in the box**, and therefore
that *a gut drifted to −1 would draw the whole 1,440 and clear the bar outright
— one mutation, not an engine change.*

**Every step of that arithmetic is correct.** Checked at the source rather than
taken on relay:

```rust
// creature.rs::diet_yield
let quality = (1.0 - (gut_bias - class).abs() / 2.0).clamp(0.0, 1.0);
worth * quality * quality
```

`flower.ron` carries `food_energy: 1440.0, food_class: -1.0`. A neutral gut
scores `quality = 1 − 1/2 = 0.5`, squared **0.25**, so a flower yields **360** —
the inferred number, exactly. A gut at −1.0 scores `quality = 1.0` and yields
the whole **1,440**. `birth_cost` is `grant + body_energy × cells` = 80 + 960 =
**1,040** (the 1,100 in the relayed version is `reproduce_threshold`, which
`birth_cost` does not read; either bar is cleared). So a matched gut that eats
one flower sits at **1,540** against a 1,040 bar.

**And the census confirms the flower is really there**, counted as cells in the
grid rather than inferred from a margin or from organs built: up to 9 standing
`flower` cells and up to 7 standing `fruit` cells at every tile to frame 30,000
(§4.3). The inference was right about the world.

**The conclusion is still wrong, and this lane ran the experiment.**
`gut=-1.0` *is* the drifted gut, applied to the founders before the colony is
stamped. The harness prints the margin it produces — **+500** — and 48,000
frames later the birth counter reads **zero**, the richest bank ever held reads
**568**, and `births_denied_no_space` reads **0**.

568 is not a near miss. It is `hunger_fraction × start_energy + leaf` = 100 +
480 = **580**, to within a rounding of the satiety line. **The colony ate leaves
for 48,000 frames and never once ate the flower**, and the two counters that
would show otherwise cannot both stay at zero if it had: an ant at 1,540 either
reproduces (`births`) or is refused for space (`denied`).

So the deadlock in this bed is **not** one heritable step wide. The gene is
already worth +500 of margin, and it buys nothing, because the food it unlocks
grows 22 to 40 rows above an ant's head and the ground-level form of it —
`windfall` — never exceeds one standing cell.

---

## 5. What it costs

**Taken on a quiet box**, load average **0.85–2.8** across the whole batch,
after four hours in which it ran 22–38. Every arm is the median of three or
five alternating repetitions, and every timing column below is the **median
frame**; the mean is printed beside it so the gap between them is visible.
§8 has the provenance for the loud figures the earlier passes produced and why
none of them is quoted.

### 5.1 The frame gets *cheaper* as the run goes on — Gate 3's warning is backwards here

Gate 3 says to measure at the population the lab actually runs, *"not at a
founder cohort — cost follows biomass and a mature box is the expensive
one"*. **In this bed the founder cohort is the expensive one.**

Median of five alternating runs:

| frame | plant cells | p50 ms | mean ms | solved/f (of 40) | awake/f (of 40) | µs/cell | x real time |
|---|---|---|---|---|---|---|---|
| 3,000 | 345 | **1.84** | 2.27 | 31.6 | 4.0 | 5.7 | 9.0x |
| 6,000 | 476 | 1.63 | 2.03 | 25.4 | 2.9 | 4.0 | 10.2x |
| 9,000 | 609 | **1.26** | 1.60 | 15.2 | 1.5 | 2.5 | **13.2x** |
| 12,000 | 663 | 1.28 | 1.63 | 13.5 | 1.4 | 2.4 | 13.1x |

Biomass **doubles** while the frame gets **31% cheaper** and the field's solve
set **more than halves**.

Run it out to 90,000 frames and the trend keeps going — same bed, same quiet
box, fifteen tiles:

| frame | plant cells | p50 ms | solved/f | awake/f | x real time |
|---|---|---|---|---|---|
| 6,000 | 476 | 1.86 | 28.5 | 3.5 | 9.0x |
| 24,000 | 822 | 1.28 | 13.9 | 2.0 | 13.1x |
| 48,000 | 689 | 1.03 | 14.3 | 0.8 | 16.2x |
| 72,000 | 623 | 0.71 | 8.9 | 0.4 | 23.4x |
| 90,000 | 598 | **0.94** | 8.8 | 0.4 | **17.8x** |

Over those fifteen tiles the median frame correlates:

| | with the median frame |
|---|---|
| **plant cells** | **+0.03** |
| **field tiles solved per frame** | **+0.92** |
| **awake chunks per frame** | **+0.93** |

**+0.03 against biomass is no relationship at all.** The frame tracks the solve
set and the awake set, and nothing else in this bed comes close.

**Cost in the lab box is the field's solve set, not biomass.** The early frames
are expensive because freshly written soil is still settling and most of the
box is awake; once it settles the box is cheap, and stays cheap while the stand
doubles.

That is a real correction to the guide's sizing rule (§2b: *"roughly 1–2 µs per
living plant cell per tick, falling as the stand grows"*), which that section
itself labels *"a sizing rule, not a model — do not trust this past a factor of
two"*. The µs-per-cell figure here runs **5.7 early and 2.4 late**, so the
magnitude is the right order and falling; the *variable* is wrong. What falls is
not the cost per cell — it is the number of tiles the field still has anything
to say about.

**The practical consequence is the pleasant direction**: a lab session gets
faster the longer it runs, and the worst frame a player ever sees is in the
first few thousand ticks of a fresh box.

### 5.2 Which phase is paying

`frame::step` split at its own seams, and the split checked against
`frame::step` itself by a full-grid hash before any of these numbers is printed
(§8). Means, at the 663-cell tile:

| phase | ms | share |
|---|---|---|
| **field** | **0.812** | **54%** |
| pheromones | 0.430 | 29% |
| active_sites | 0.136 | 9% |
| ca_sweep | 0.117 | 8% |
| liquid_bodies, chunk_bodies, player, particles | **0.000** | 0% |

The four zeroes are the feasibility report's §3c prediction landing exactly: a
sealed box with no rock, no blast and no gnome pays nothing for those phases,
which is why the lab can run the shipped tick unmodified instead of forking it.

**The field is still the largest phase, but it is no longer most of the
frame** — 54% here against 69% on the 80-row bed this was first measured on, so
halving the soil roughly halved it. The whole simulated tick is now **1.5 ms**,
and §5.3 is why that stops mattering.

### 5.3 The draw costs three times the tick

Measured in the lab bed through the shipped `Renderer` with the dirty-rect skip
live, median of three:

| how often the box is drawn | ms per draw | ns per pixel |
|---|---|---|
| every tick (a **Tending** display) | **4.78** | 29 |
| every 20 ticks (a **Running** display) | **6.26** | 38 |

The second row is higher for a good reason rather than a bad one: 20 ticks of
simulation dirty more chunks than one does, so the skip has less to skip.

**And `PIXEL_PHYSICS_DRAW_TIMING=1` says where it goes**, which is the part
worth acting on:

| draw phase | ms |
|---|---|
| **sky_light** | **2.77–2.82** |
| pixels | 0.45–2.48 (with how much is dirty) |
| glow scan + near_glow | 0.9 on the first rebuild, ~0 after |
| preamble, horizon, overlays | 0.04 |

**Well over half the draw is the sky-light pass, in a box whose whole design
premise is that it has a ceiling instead of a sky**, and it is the one phase
that does not vary with how much of the frame changed. The guide's own
measurement table says *"whatever fills the air above the soil must not draw as
sky"* — empty sky is 27.4 ns/px against stone's 6.7 — and at 29 ns/px this draw
is priced as sky. The bed *does* declare itself an enclosure and the interior
*is* painted as a room; whatever that buys, it is not showing up in
`sky_light`. §7.3.

### 5.4 The multiplier, which is the number the speed dial needs

A tick is 1/60th of a simulated second, so with a draw costing `R` at a display
rate `hz`, simulated seconds per real second is `(1000/hz − R) / tick_ms`. At
`R = 0` the two rates agree — the arithmetic's own check, since the whole
advantage of a slower display is paying the draw less often rather than running
more ticks per tick.

| | fresh box (~350 cells) | mid run (~660) | settled, 90,000 frames (598) |
|---|---|---|---|
| tick, median frame | 1.84 ms | 1.28 ms | **0.94 ms** |
| **simulation only** | **9.0x** | 13.1x | **17.8x** |
| **60 Hz display** (draw 4.78) | **6.4x** | 9.3x | **12.7x** |
| **20 Hz display** (draw 6.26) | **7.9x** | 11.4x | **15.6x** |

So the honest statement for Lane A's dial is **about 8x real time on a fresh
box, rising past 15x once it settles, at a 20 Hz display** — and 60 Hz is
*slower*, not faster, by about 19%. Mean rather than median frames move every
figure down by roughly 10–20% and change nothing about the shape.

Three things to carry out of that table:

- **The ceiling rises during a session.** Quote a range, not a number.
- **The display rate is worth about 22%, not the tripling §2b expects.** That
  estimate assumed the draw dominates the budget, and it does — but dropping to
  20 Hz pays a 4.8–6.3 ms draw three times less often against a tick that is
  only 1.3 ms, and the ratio lands at 1.22x rather than 3x.
- **The draw is the dominant term — by a factor of five at the settled tick —
  and it is the one with an obvious fix.** At 0.94 ms of simulation against a
  4.78 ms draw, the simulation alone would run 17.8x and the renderer takes it
  to 12.7x. **Getting `sky_light` out of a box with a
  ceiling is worth more to the speed dial than anything left in the
  simulation** — the whole field phase is 0.8 ms and that pass is 2.8 ms.

---

## 6. Partitions: the containment reproduces, the speed-up does not

§2c is the guide's strongest single finding — a fanned 2048-wide bed walled into
16 compartments went from 4.1x to 7.6x with the stand held to within 0.2%.
**Half of it reproduces here, and the half that fails is the half the design
was counting on.**

One fan, offset by a third of a spacing so it never sits on a partition (the
scene error §2c records having paid for), 12,000 frames, median of three
round-robin repetitions:

| compartments | width each | chunks each | p50 ms | solved/f (of 40) | plant cells | vs open |
|---|---|---|---|---|---|---|
| 1 | 512 | 8 | 1.69 | **39.4** | 778 | 1.00x |
| 2 | 256 | 4 | 1.91 | 32.2 | 631 | 0.89x |
| 4 | 128 | 2 | **1.41** | 26.6 | 543 | **1.20x** |
| 8 | 64 | 1 | 1.59 | 28.4 | 689 | 1.06x |
| 16 | 32 | 0.5 | 1.92 | **25.1** | 357 | 0.88x |

**The containment is real and it is exactly §2c's mechanism**: the field's solve
set falls **39.4 → 25.1**, a 36% cut, and it falls monotonically apart from one
step. A field tile is one 64-cell chunk (`FIELD_TILE_SIZE = CHUNK_SIZE /
FIELD_SCALE`), and walls stop the fan's disturbance crossing between them.

**The frame does not follow it.** 1.69 → 1.91 → 1.41 → 1.59 → 1.92 ms is not a
trend; the whole spread is 1.36x and the best arm (four compartments, 1.20x) is
flanked by two arms *slower* than the open box. The reason is §5.2: the field
is **0.8 ms of a 1.5 ms tick**, so cutting its solve set by a third is worth at
most ~0.3 ms, and the walls' own cost eats it. `active_sites` at sixteen
compartments means **5.12 ms** against 0.14 open — a stone column written
through soil produces intermittent scheduler storms, which the median frame
survives and the mean does not.

**Two cautions, both against the finding rather than for it.** The stand is
*not* held constant the way §2c held it — 778 / 631 / 543 / 689 / 357 cells —
so these arms differ in biomass as well as in walls, though §5.1 says biomass
is not what the frame tracks. And an earlier version of this sweep, on the bed
before Lane B made placement compartment-aware, produced a **catastrophic**
inversion instead of this mild one: 0.35x at eight compartments and 0.22x at
sixteen, with `active_sites` at 8.5 ms and a quarter of the box permanently
awake. That is gone, and the difference between the two sweeps is the scene, not
the walls.

**What this means for the design.** Partitions remain worth having for
evolutionary isolation and for §5 of the guide's scoring, and the air
containment §2c measured is real in this bed too. **They are not a performance
lever at 512 wide**, because there is not enough field left in the frame for a
third of it to matter. §2c's 4.1x → 7.6x is a **2048**-wide bed, where the
field is a much larger share and 16 compartments is still 128 cells each; the
quantity that transfers is the compartment's width in chunks and the field's
share of the frame, not the compartment count.

---

## 7. What this asks somebody to decide

1. **Is a colony that grazes the bed down and then dies out the opening the
   lab wants?** It is a real food web and the most alive thing in the box — but
   it costs the stand three of its eight founders, half its biomass and a
   generation of depth, and the colony does not survive it either: 52 founded,
   52 dead, 0 born, extinct by frame 66,000. Posted to the review queue as a
   paired A/B with the counts under each panel, card
   `20260830T141122049Z-d710bf`.
2. **Gate 0 needs the fruit to reach the floor, not a better gut.** §4.3–4.4.
   The next measurement is a counter on fruit→windfall, and it is small.
3. **Half the lab's draw is `sky_light`, in a box with a ceiling.** §5.3. The
   enclosure is declared and the interior is painted as a room, and the draw is
   still 4.78 ms of which 2.8 ms is the sky-light pass. Whatever
   `sky::Interior` buys, it is not showing up in *that* phase, and that phase is
   worth more to the speed dial than anything left in the simulation.
4. **Partitions are an isolation and scoring feature here, not a performance
   one.** §6. They contain the air exactly as §2c says — 36% off the solve set
   — and the frame does not follow, because at 512 wide the field is already
   only half of a 1.5 ms tick. Keep them for what §5 of the guide wants them
   for; do not budget a speed-up against them at this bed size.
5. **Nothing has run Gate 2 in this bed.** `selection_arena`'s `arm=` ladder is
   the teeth-test, and its own finding is that a null there is a statement
   about the world rather than the genome. Everything in this report is about
   whether the bed is *alive*; none of it says whether it *discriminates*.

---

## 8. Provenance, and what this does not measure

**A harness that defaults a knob is a harness that can measure the wrong bed,
and this one did for half a day.** `lab_cost` was written with `soil` defaulted
to a literal `80` while Lane B re-derived `DEFAULT_SOIL_DEPTH` to **40** from a
measurement of where herb's roots actually stop. Nothing failed and nothing
looked wrong: the harness echoed `soil=80`, which is exactly what it was doing,
and the census was internally consistent. It is the `include_str!` gotcha
wearing a different hat — *a knob nobody can see the value of is a knob nobody
can tell is disconnected* — with the extra sting that **a defaulted knob looks
connected**. Every bed knob now defaults from `LabBox::default()`, and every
figure in this report was re-taken after that fix.

**The counters are deterministic and the timings are not.** Every figure in
§§2–4, and every `cells`, `solved/f` and `awake/f` column in §§5–6, is a count
that reproduces exactly under any load. The milliseconds do not, and this was
written on a shared four-core container running three other agent sessions
against the same repo.

**So the timings were taken twice, and only the quiet set is quoted.** Load
average, from the harness's own log:

| batch | load average | what became of it |
|---|---|---|
| first cost pass | 22 → 25 | **discarded** — mean 4.46 ms against a p50 of 2.37, a 1.9x gap that is the machine |
| render pass | 35 → 38 | **discarded** — 22.3 and 84.5 ms per draw, i.e. 136 and 516 ns/px against a shipped sky's 27.4 |
| partition pass | 37 | **discarded** |
| **the quoted batch** | **0.85 → 2.8** | §5 and §6 |

On the quiet box mean and median frame close to within 28% (1.63 against 1.28
ms) where the loud ones stood 90% apart, which is the tell that what was being
measured earlier was the other three sessions. **Every worst-frame figure this
harness produced failed its own `mean × frames ≈ worst` check** — ratios of 50
to 2,285 — and the harness refuses to let one be quoted; none is.
`Reports/measurement-under-contention.md` is the standing account of this
failure mode here.

**What the instrument checks about itself.** `lab_cost selftest=1` runs four
positive controls, because a number that is arithmetically correct and cannot
move looks exactly like a result:

```
[PASS] split tick reproduces frame::step over 200 frames
[PASS] leaf census: 191 in a planted bed at frame 4000, 0 in an unplanted one
[PASS] one fan wakes the box: solved/f 2.6 with no fan, 39.5 with one
[PASS] slot high water moves with founders: 2 at 2, 16 at 16
```

The first is load-bearing for §5.2: the per-phase breakdown re-types
`frame::step`'s sequence, which is precisely the fork `sim/frame.rs` exists to
prevent, so the harness hashes a real lab box stepped both ways and prints the
comparison before any phase number is printed.

**Not measured here**, and each is somebody's next question rather than a gap
in this one:

- **Gate 2** — whether selection has teeth in this bed. §7.5.
- **Why plants die.** `p.died` is 305 across 90,000 frames and the census does
  not attribute a single one. Grazing, shading, starvation and senescence are
  all live and all indistinguishable in these columns.
- **What happens to a ripe fruit.** §4.3 — the one measurement that would close
  Gate 0's remaining question, and it is small.
- **Whether the render change fired.** §5.3 reports the draw as it stands on
  the merged head. It is *not* an A/B of the enclosure work, because this lane
  cannot build the same bed without the enclosure; the phase split is the
  evidence, not a before/after.
- **Soil depth.** §2a of the guide asks that something reach the depth the bed
  pays for. Lane B answered it directly (12 rows used of 40 given); nothing
  here adds to that.
