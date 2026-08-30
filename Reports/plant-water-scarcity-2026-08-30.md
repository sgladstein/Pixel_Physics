# Making water scarce, and what it does to root architecture (2026-08-30)

**Status: measurement.** Tests the one prediction
`plant-selection-teeth-2026-08-29.md` §4b made and did not test — *"making
water genuinely limiting — a bed at or below the wilting point, or a drought
cycle — should bring the root arm to life"*.

```
cargo run --release --example selection_arena -- bed=1 arm=same moisture=260 sky=clear
cargo run --release --example selection_arena -- arm=norootbranch moisture=260 sky=clear founders=8 seeds=18
```

**The prediction did not hold, and the reason is more useful than the
prediction was.** `norootbranch` reads **50.8%, 8 of 18 seeds, p=0.49** in a
bed carrying **5.5x less plant-available water** than the one the teeth report
called comfortable, and **50.8%, 8 of 18, p=0.93** in a further bed where
water genuinely limits growth — against its own 49.7%, 10 of 18, p=0.86 in the
wet one. Paired over shared seeds the beds differ by **+0.0 points**.

The world is not inert while this happens: in the same two dry beds
`nobranch` — a plant that grows, flowers and sets seed, and simply cannot
branch its shoot — loses **13 points on 12 of 12 seeds** and **10 points on 18
of 18**.

Three things came out of chasing it, and they are what the next attempt should
be built on:

- **A bed cannot be dried into drought.** Every species' `Germinate` rule puts
  a floor under how dry a *usable* bed can be, and for both `herb` and `tree`
  that floor sits **above** the availability at which their own uptake becomes
  limiting. Below it you get an empty bed, not a scarce one — `tree` at
  moisture 260 goes from 6 organisms to zero.
- **The lever that does bite is rooting volume, not moisture.** Same water,
  four rows of soil instead of thirty-four: water status falls **1.000 →
  0.678**, uptake halves, and the stand starts losing plants. A plant in a deep
  dry bed simply grows down into soil it has not drunk yet.
- **Root branching does not buy water here.** Income is `rate x available` per
  *wet neighbour*, so what earns is contact with wet soil — and in a
  drawn-down bed the bed sets that, not the root. The handicap costs **23% of
  root cells and 3% of uptake surface**. There is nothing for selection to act
  on.

**Read §5c before designing the next bed.** "Wetter or drier" is the axis this
report rules out.

---

## 0. What is new in the instrument, and why each piece had to exist

`selection_arena` could already vary a bed's moisture (`moisture=`) and its
rooting volume (`soil=`). It could not make a bed **scarce**, and it could not
say whether one was, so three things were added.

### 0a. `sky=clear` — because a bed under live weather gets wetter

**Rain is a one-way supply into this bed, and nothing dries a synthetic bed on
its own.** Two mechanisms, both already in the engine:

- `evaporation::schedule_damp_soil` is called where soil is **wetted** —
  `weather::step`'s rain soak and `update::update_soil_water`'s infiltration —
  and deliberately never from the sweep, because a settled damp bed sleeps
  like a dry one and a sweep hook there would fire exactly never. A bed
  written straight into the grid by `PlantScene::build` has been wetted by
  neither path, so no cell in it is ever scheduled to dry.
- `evaporation.rs`'s own `SOIL_DRY_FLOOR` doc records a plantless bed measured
  over ten weather epochs with soil `aux` **monotone non-decreasing on every
  seed**, one climbing 230,400 → 463,927.

Measured here on the flat bed, seed 1, to frame 15,000: the live sky
precipitated on **2,986 of 15,000 frames** and left **184,598 units of free
water** standing on the surface. `sky=clear` pins `Weather::CLEAR` — intensity
0, `Precipitation::None`, wind 0, chill 0 — so the bed holds a finite store
that only roots can spend.

**`sky=live` is byte-identical to not passing the argument**:
`World::set_weather_pin` returns early when the override is already `None`. So
every arena number taken before today still compares.

**And the control that matters is the one on a *dry* bed, not a wet one.**
The paragraph above was first written from a measurement on the wet bed, where
rain lands on soil already at field capacity and changes nothing — which
proves the sky is raining, not that it refills a dry bed. Re-run at
`moisture=260`, seed 1, `bed=1`:

| frame | mean available | rooting zone at wilting | free water standing | frames rained |
|---|---|---|---|---|
| 5,000 | 0.182 | 1.2% | 0 | 0 |
| 10,000 | 0.182 | 2.3% | 0 | 0 |
| 15,000 | **0.195** | 5.7% | **74,674** | 2,986 |
| 20,000 | **0.196** | 16.3% | 21,407 | **6,004** |

Under `sky=clear` the same bed ends at 0.183 with **0** free water and **0**
frames of rain. So a bed described as dry is, under live weather, a bed that
is rained on for **30% of the run** and carries tens of thousands of units of
free surface water — which roots drink directly through `absorb_water`'s
`Liquid` arm, not only through the soil.

One thing in that table is worth keeping because it is counter-intuitive:
the rooting zone ends up **drier** under rain (16.3% at the wilting point
against 3.9% under a clear sky). That is not a contradiction, it is
§0a's first mechanism seen from the other side — rain is what *schedules*
soil for evaporation, so a bed that is never rained on is also a bed that
never evaporates.

### 0b. `bed=1` — because "is this bed dry" is not a share

One world, no race, `arm=` applied to **every** founder rather than to
alternate ones, so `bed=1 arm=same` against `bed=1 arm=norootbranch` is a
phenotype A/B in one bed. It reports the supply side and the demand side
together:

- **supply** — `update::plant_available_fraction` over soil cells, *the same
  quantity a root drinks and a seed germinates on*: zero at or below
  `SOIL_WILTING_POINT` (180), one at `SOIL_FIELD_CAPACITY` (620).
- **demand** — `OrganismState::water_status`, which is `drawn / demand`: the
  fraction of transpiration demand the plant met. It multiplies every
  photosynthetic credit, so it is the whole coupling between having roots and
  being able to grow.

### 0c. Two columns are corrections to the first version of this census

Both were wrong in the direction that hides the effect, and both were caught
by running the wet bed first.

**The whole-bed mean is the wrong denominator.** The flat bed is ~34,000 soil
cells 34 rows deep and a herb's roots reach the top six, so 855 plants drying
the ground under themselves moved the mean from 0.998 to 0.976 — which reads
as "nothing is happening". `dryT%` reads the top 8 rows instead.

**A mean over the population is a statistic about seedlings.** The bed carries
**855 organisms at 9,538 cells** by frame 10,000 — a mean of 11 cells, and a
seedling has no architecture to differ in. Every plant column is taken over
**established** plants (≥20 cells) with the count (`estab`) printed beside it,
so a bed that killed the stand cannot hide inside a healthy-looking mean.

## 1. How dry a bed can be, and the floor that bounds it

**The bed cannot be taken to the wilting point, and the reason is
germination.** `herb`'s `Germinate` needs `soil_water_threshold: 0.15` and
`plant_available_fraction` is `(m − 180) / 440`, so **moisture 246 is the
floor**: below it nothing germinates and the bed is empty rather than scarce.
The driest usable bed is therefore a little above it, and `moisture=260`
(available fraction **0.182**) is the setting used throughout.

That is not "at the wilting point" as §4b's prediction wrote it. It is as
close as this engine allows while leaving a bed a plant can live in, and it is
**5.5x less plant-available water per soil cell** than the bed every previous
arena result was measured in.

**Three beds, `founders=16`, `sky=clear`, seed 1, frame 10,000.** Plant
columns are over established plants; `avail` is the mean available fraction
over the whole bed.

| moisture | avail | orgs | estab | cells | root cells | contact roots | root spread | root depth | **water status** |
|---|---|---|---|---|---|---|---|---|---|
| **620** (field capacity) | 0.976 | 829 | 91 | 9,221 | 14.2 | 9.9 | 4.9 | 5.2 | **0.988** |
| **350** | 0.387 | 849 | 102 | 10,576 | 17.3 | 10.8 | 5.7 | 5.3 | **0.976** |
| **260** | 0.184 | 783 | 89 | 9,703 | **23.9** | **16.4** | **7.0** | **6.2** | **0.977** |

Two things are already visible, and they are the whole result.

**The bed is dry and the plant knows it.** Root cells rise **68%** and
contact roots **66%** from the wet bed to the dry one, and the root system
gets wider (4.9 → 7.0) and deeper (5.2 → 6.2). This is not a bed the plants
cannot tell apart.

**And it costs them no water.** Water status is **0.988 / 0.976 / 0.977**
across a **5.3x** range of soil availability. The plants meet their
transpiration demand in the driest usable bed exactly as they do at field
capacity, by building root.

## 2. Lowering the moisture cannot make water bind. Taking away the soil can

**This section is the result, and the first version of it was wrong.** Written
from the moisture ladder alone it concluded that water status cannot be moved
at all. It can — by a lever the prediction did not name.

### 2a. What a root's income actually is

`absorb_water`'s `Powder` arm is the whole water supply of a plant with no
pond to reach:

```rust
let available = update::plant_available_fraction(n);   // (m - 180) / 440
if available > 0.0 {
    let drawn = (rate * available).min(capacity - water);
```

Three facts about how that runs, none of them guessable from the line:

- `rate` is **1.6** for `herb`, and `Absorb(rate: 1.6)` is declared on
  **`MatureBody` as well as `RootTip`** — so every mature root cell drinks,
  not only the ten-odd tips.
- It is dispatched from the whole-organism upkeep walk, once per organism
  tick per cell that declares it, per wet neighbour.
- `capacity` is `WATER_SCALE (4.0) x contact_root_cells`.

So an established plant's potential income per organism tick is at least
`1.6 x available x contact_root_cells`, and its expenditure is
`water_demand`. Both are measured columns.

### 2b. The headroom, in the driest bed a seed will germinate in

| bed | available | contact roots | **potential income / tick** | **demand / tick** | headroom |
|---|---|---|---|---|---|
| wet, 620 | ~0.95 | 12.2 | ≥ 18.5 | 0.85 | **22x** |
| dry, 260 | 0.182 | 21.2 | ≥ 6.2 | 0.81 | **7.6x** |

Making the bed 5.5x drier cuts the headroom by 3x — a real effect, in the
right direction — and leaves it at **seven times what the plant spends**.
Measured uptake in both beds tracks demand rather than availability, so the
term that actually binds is `capacity - water`: **the tank is full**, and it is full because the plant
built enough root surface to fill it.

**For availability to become the binding term at that root system you would
need `available < 0.024`** — soil moisture **below 191**, which is 11 units
above the permanent wilting point and **55 units below the floor at which a
seed will germinate at all**. There is no setting in that gap.

**And this is not a `herb` quirk — it is how the two thresholds are placed
relative to each other.** `tree` has `soil_water_threshold: 0.25`, so its
floor is moisture 290; at its measured root system (contact roots 50.8,
demand 9.9 at frame 10,000) availability would have to fall under 0.122 —
moisture 234 — to bind. Both species have a germination floor **well above**
the availability at which their own uptake becomes limiting. The two numbers
were never calibrated against each other, and the consequence is that no
species in this engine can be dried into drought while it is alive.

`tree` in the 260 bed is the visible form of that: **6 organisms at frame
5,000, one at 15,000, zero at 20,000** — an extinction by germination
failure, not by thirst.

### 2c. So water status does not move with moisture

`water_status` is `drawn / demand`, and it multiplies every carbon credit —
`plant.rs` calls it "the entire coupling between having roots and being able
to grow". Across the whole moisture range, `founders=8`, frame 20,000:

| soil moisture | available | water status |
|---|---|---|
| 620 (field capacity) | 0.945 | 0.985 |
| 350 | 0.387 | 0.976 |
| 260 | 0.183 | 1.000 |
| 248 | 0.155 | 0.966 |
| **246** (exactly the germination floor) | **0.149** | **0.989** |

Five beds spanning a **6.3x** range of plant-available water, and the
readout does not leave the top 3%.

### 2d. What does move it: rooting volume

Same moisture (260), same sky, same seed — **`soil=4` instead of `soil=34`**,
a four-row skin of soil over stone:

| frame | available | rooting zone at wilting | orgs | estab | root cells | contact roots | spread | depth | **water status** | demand | uptake |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 5,000 | 0.177 | 4.0% | 109 | 21 | 16.0 | 7.5 | 6.6 | 3.0 | **0.926** | 0.882 | 0.545 |
| 10,000 | 0.181 | 6.9% | 240 | 29 | 25.8 | 10.2 | 9.6 | 3.7 | **0.872** | 0.860 | 0.711 |
| 15,000 | 0.161 | 9.3% | 376 | 32 | 27.1 | 8.7 | 10.1 | 3.6 | **0.775** | 0.959 | 0.444 |
| 20,000 | 0.152 | 9.9% | 329 | 30 | 29.8 | 8.6 | 10.9 | 3.6 | **0.678** | 0.958 | 0.281 |

Against the 34-row bed at the identical moisture, which ends at water status
**1.000** with 740 organisms.

**Every column of that table is a drought.** Water status falls monotonically
and ends at 0.678 — a third of every carbon credit gone. Uptake **halves**
(0.545 → 0.281) while demand *rises* (0.882 → 0.958). The rooting zone at the
wilting point grows to 9.9% against 3.9% in the deep bed. The population
peaks at 376 and **declines to 329** — the only bed measured here that loses
plants. And contact roots go *down* (10.2 → 8.6) while the root system keeps
spreading sideways (6.6 → 10.9 wide in a bed 3.6 deep): the plant is running
out of unexhausted soil to touch.

**That is the mechanism the moisture ladder was missing.** A herb in a deep
dry bed is not water-limited because it *escapes downward* — it grows into
soil it has not drunk yet, and 34 rows is more soil than it can exhaust in
20,000 frames. Lowering the moisture lowers what each cell yields; it does
not reduce the number of cells. **The binding resource is rooting volume, and
soil moisture only sets the price per cell.**

`common/mod.rs`'s own `soil_depth` doc predicted exactly this and said why the
parameter exists — *"a shallow bed holds less water and gives a root system
nowhere to go, so it is the case where having roots is supposed to stop being
free"* — and no arena result had used it.

## 3. Does the handicap change the plant?

**Yes, and this had to be checked before any share was believed.** Three arms
on this line were vacuous before they were real. `arm=` here is applied to
**every** founder, so these are two beds each carrying one genome. Seed 1,
`founders=8`, `sky=clear`, frame 20,000:

| bed | arm | orgs | estab | cells | root cells | contact roots | root spread | root depth | water status |
|---|---|---|---|---|---|---|---|---|---|
| wet 620 | `same` | 942 | 60 | 8,013 | 19.5 | 12.2 | 5.5 | 6.5 | 0.985 |
| wet 620 | `norootbranch` | 729 | 52 | 6,171 | **12.3** | **9.8** | **4.9** | 5.6 | 0.995 |
| dry 260 | `same` | 740 | 46 | 7,321 | 32.3 | 21.2 | 9.1 | 8.0 | 1.000 |
| dry 260 | `norootbranch` | 930 | 55 | 8,601 | **25.7** | **15.9** | **7.9** | 6.9 | 0.979 |

**The handicap is not silent.** It costs 37% of root cells in the wet bed and
20% in the dry one; contact roots — the surface that actually earns — fall
20% wet and **25% dry**. The root system is narrower and shallower in both.

**And the bed does what it was built to do.** From wet to dry, an unhandicapped
plant grows **66% more root cells**, **74% more contact roots**, and a root
system **65% wider**. The dry bed makes root architecture cost more and do
more, which is exactly the condition under which a handicap on it should be
selectable.

**Do not read the biomass columns as fitness.** These are separate worlds
carrying one genome each, one seed apiece, and twelve identical trees from
one genome span 31 to 153 cells. The dry `norootbranch` bed happens to carry
*more* cells than the dry `same` bed; that is a sample from a wide
distribution and says nothing. Fitness is §4's job, where the two arms stand
in one bed and the mirror cancels position and draw.

## 4. The arms

`herb`, 8 founders, 20,000 frames, `sky=clear`, `mirror=on` except where the
row says so. Arm B's share of the final **biomass**; 50% is no effect. The
frame budget is the teeth report's own, deliberately, so these rows sit beside
its ladder — and they carry the same caveat, restated in §4d.

| bed | arm | n | median | quartiles | seeds B lost | p | per-seed sd |
|---|---|---|---|---|---|---|---|
| deep, 34 rows, dry 260 | `same` **unmirrored control** | 18 | **52.1%** | 47.6-55.3 | **5/18** | **0.396** | 7.2 |
| deep, 34 rows, dry 260 | `nobranch` (teeth control) | 12 | **37.2%** | 36.0-39.7 | **12/12** | **0.003** | 2.7 |
| deep, 34 rows, dry 260 | **`norootbranch`** | 18 | **50.8%** | 48.4-53.6 | **8/18** | **0.486** | 4.5 |
| deep, 34 rows, wet 620 | **`norootbranch`** | 10 | **54.1%** | 50.6-56.2 | **2/10** | **0.103** | 4.3 |
| thin, 4 rows, dry 260 | `same` **unmirrored control** | 18 | **53.7%** | 44.4-60.9 | **8/18** | **0.459** | 12.9 |
| thin, 4 rows, dry 260 | `nobranch` (teeth control) | 18 | **39.8%** | 37.1-42.6 | **18/18** | **0.000** | 4.1 |
| thin, 4 rows, dry 260 | **`norootbranch`** | 18 | **50.8%** | 45.3-53.3 | **8/18** | **0.931** | 4.7 |

### 4a. `norootbranch` is a null in both dry beds

**50.8%, 8 of 18, p=0.49** in the deep dry bed and **50.8%, 8 of 18, p=0.93**
in the thin one where water genuinely limits — against the teeth report's
**49.7%, 10 of 18, p=0.86** in the wet bed. Three independent 18-seed runs, on
beds spanning 5.5x in plant-available water and a water status from 1.000 to
0.678, all sitting on the null.

**The two 50.8%s are a coincidence of the summary, not of the data** — worth
saying, because two identical medians is exactly the tidiness `CLAUDE.md`
warns about. The quartiles differ (48.4–53.6 against 45.3–53.3) and the
per-seed differences between the two beds span **−9.4 to +11.9 points**:

```
paired arm_norb_dry minus arm_norb_wet over 10 shared seeds: median +0.0 pts, 5/10 lower, z=0.36, p=0.722
  diffs: -1.5 +5.9 -5.6 +0.0 -8.2 -6.6 +6.0 +1.1 -1.0 +1.6
paired arm_norb_thin minus arm_norb_dry over 18 shared seeds: median -2.0 pts, 11/18 lower, z=0.61, p=0.542
  diffs: +11.9 -9.4 -8.2 +2.1 +7.4 -2.6 -6.4 -4.4 +0.8 -9.2 +4.4 -3.0 -2.0 -3.1 -1.4 -7.4 +4.7 +10.6
```

The deep-versus-wet contrast is the cleaner one, because it is the comparison
the prediction was about: **median +0.0 points over 10 shared seeds, 5 of 10
in each direction.** Not a small effect the seed count cannot resolve — no
effect, and no sign.

### 4b. Both beds have teeth, including the one with the real drought

`nobranch` — the same plant with its bud unable to flush, which still grows,
flowers and sets seed — loses **13 points on 12 of 12 seeds (p=0.003)** in the
deep dry bed and **10 points on 18 of 18 (p<0.001)** in the thin one. The bed
that produced a real drought punishes a shoot handicap on every one of its
eighteen seeds. It is specifically root architecture that neither bed can
see.

The unmirrored `same` control passes in both — 52.1% (p=0.40) deep, 53.7%
(p=0.46) thin — so neither bed's own asymmetry manufactures a winner. **Run
unmirrored deliberately**: mirrored, `arm=same` is one simulation with the
labels swapped and returns exactly 50.0% as an algebraic identity.

### 4c. And the reason is that the handicap does not cost the plant water

This is the finding, and it is measured rather than inferred. The two
phenotype censuses at frame 20,000, in the bed where water genuinely binds:

| thin dry bed, frame 20,000 | `same` | `norootbranch` |
|---|---|---|
| root cells | 29.8 | **22.9** (−23%) |
| root spread | 10.9 | 9.3 |
| **contact root cells** | **8.6** | **8.3** (−3%) |
| **water status** | **0.678** | **0.690** |

**The handicap costs a quarter of the root system and 3% of its uptake
surface.** In a bed the plants have drawn down, contact is limited by **how
much wet soil is left**, not by how much root has been built: income is
`1.6 x available` per *wet neighbour*, and branching buys root mass that lands
in soil already drunk. Water status is, if anything, marginally *higher* for
the handicapped plant.

In the deep bed contact does fall — 21.2 against 15.9, −25% — and it still
does not matter, because §2b's headroom is 7.6x either way. **So in neither
bed does removing root branching reduce the water the plant actually gets.
That is why there is nothing here for selection to act on.**

### 4d. Two limits on all of the above, both measured

**These are 20,000-frame readings and the system settles at 50,000–75,000.**
The harness says so per run — 12 of 18 world-runs in the deep control ended
with the share still moving. The budget is deliberate: it is the teeth
ladder's, so the rows compare. A settled re-run would move magnitudes; it is
not going to convert 8-of-18 into a direction.

**And the power is worse in the thin bed, though not fatally.** Its unmirrored
control's per-seed spread is **12.9 share-points against 7.2** in the deep bed
— the population there is small, declining and stochastic. That puts its
resolution at roughly **11 points rather than 6**, and `nobranch`'s 10-point
loss was detected there on every one of 18 seeds, so the bed is not blind. It
is blind to a *small* root effect, and so is the deep bed: nothing here rules
out a true cost of a few points, which is the wall the teeth report already
named and which needs a different design rather than more seeds.

## 5. What this does and does not settle

### 5a. The prediction, plainly

`plant-selection-teeth-2026-08-29.md` §4b predicted that *"making water
genuinely limiting — a bed at or below the wilting point, or a drought cycle —
should bring the root arm to life"*.

**It did not.** `norootbranch` reads **50.8% on 8 of 18 seeds (p=0.49)** in a
bed with 5.5x less plant-available water, and **50.8% on 8 of 18 (p=0.93)** in
a thin bed where water status falls to 0.678 and the stand loses plants —
against 49.7% on 10 of 18 (p=0.86) in the bed the prediction called
comfortable. The paired contrast over shared seeds is +0.0 points. This is the
second prediction on this line about the bed to be made in advance and refuted
— the first was `Relief::Varied` — and it is recorded the same way rather than
quietly replaced.

### 5b. What was wrong with the prediction, which is the useful part

**The premise "herb is carbon-limited, not water-limited" is right. The
inference "so make water scarce" does not follow, because soil moisture is not
the term that limits water here.**

Three things are now measured that were not:

1. **A bed cannot be dried into drought.** `Germinate`'s
   `soil_water_threshold` puts a floor under how dry a usable bed can be —
   moisture 246 for `herb`, 290 for `tree` — and both floors sit **above** the
   availability at which each species' own uptake becomes limiting (191 and
   234). The two constants were never calibrated against each other. Below the
   floor you get an empty bed, not a scarce one: `tree` at moisture 260 goes
   from 6 organisms to zero.
2. **The binding resource is rooting volume.** At the same moisture, 34 rows
   of soil gives water status 1.000 and 4 rows gives **0.678**, falling
   monotonically, with uptake halving and the population declining. A plant in
   a deep dry bed escapes downward into soil it has not drunk yet; take the
   depth away and the drought is real.
3. **Root branching does not buy water in this engine.** Uptake is
   `rate x available` per *wet neighbour*, so the quantity that earns is
   contact with wet soil — and in an exhausted bed that is set by the bed, not
   by the root. The handicap removes 23–25% of root cells and **3%** of uptake
   surface where it matters.

### 5c. So what would put roots under selection

Named because they follow from the mechanism above rather than from taste, and
none of them is "a drier bed".

- **Patchy water, not a gradient.** Branching pays when finding water is a
  *search*. `Relief::Varied` ramps moisture linearly across the bed, so every
  root finds the same thing whichever way it goes; scattered wet pockets in
  dry ground is the geometry a spreading root system is for. This is a new
  `Relief` variant and one more arm race.
- **Make contact scale with branching.** Today a root cell's income depends on
  its wet neighbours and nothing else, so a wide root system and a deep one
  earn the same per cell. If income saturated per *cell* — a depletion zone,
  which is what a real root has — reaching new soil would pay and reaching the
  same soil twice would not.
- **Lower the germination floor, or raise the uptake price.** Either one opens
  the empty band between "a seed will start" and "water limits growth". They
  are one number apart and neither was set with the other in view.

### 5d. What not to conclude

**Not "roots do not matter".** The engine's own water economy is real —
`water_status` moved from 1.000 to 0.678 in this session — and the thin bed
shows a stand losing plants to it. What is missing is a link between *root
architecture* and *water income*, not water itself.

**And not "18 seeds settles it".** The control's spread is 7.2 share-points
per seed in the deep bed and 12.9 in the thin one, so this design resolves
roughly 6 points and 11 points respectively — it caught `nobranch` on every
seed in both, and is blind well below that. A true 3-point cost to having no root
branching would be invisible here and would still matter over enough
generations. That is a design problem — a frequency trajectory over many
generations rather than an endpoint share — not a seed-count problem, and it
is the same wall the teeth report hit.

## 6. Reproducing it

`cargo build --release --example selection_arena` first — a stale
`target/release/examples/` binary runs happily and prints plausible numbers.

**The bed censuses** (one world each, no race; `arm=` applies to every
founder, so swapping it is a phenotype A/B):

```
selection_arena bed=1 arm=same         founders=8 frames=20000 sky=clear moisture=620          # wet
selection_arena bed=1 arm=same         founders=8 frames=20000 sky=clear moisture=260          # deep dry
selection_arena bed=1 arm=same         founders=8 frames=20000 sky=clear moisture=260 soil=4   # the drought
selection_arena bed=1 arm=norootbranch founders=8 frames=20000 sky=clear moisture=260 soil=4   # its arm-B twin
selection_arena bed=1 arm=same         founders=8 frames=20000 sky=live  moisture=260          # the rain control
```

**The arms** — each is 18 seeds x a mirrored pair, so chunk with `seed0=` if
the container may restart:

```
selection_arena arm=norootbranch founders=8 frames=20000 seeds=18 sky=clear moisture=260
selection_arena arm=norootbranch founders=8 frames=20000 seeds=18 sky=clear moisture=260 soil=4
selection_arena arm=nobranch     founders=8 frames=20000 seeds=18 sky=clear moisture=260
selection_arena arm=nobranch     founders=8 frames=20000 seeds=18 sky=clear moisture=260 soil=4
selection_arena arm=same mirror=off founders=8 frames=20000 seeds=18 sky=clear moisture=260
selection_arena arm=same mirror=off founders=8 frames=20000 seeds=18 sky=clear moisture=260 soil=4
```

**`mirror=off` on the `same` arm is not optional.** Mirrored, `arm=same` is
one simulation with the labels swapped and returns exactly 50.0% as an
algebraic identity — a control that cannot fail.

**Do not quote the selection-coefficient block** the harness prints after the
shares. It is known-broken for two independent reasons the harness itself
warns about: the generation axis is a mean over *living* organisms, so it
equilibrates instead of accumulating, and the share equilibrates too.
