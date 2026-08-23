# Sowing the ground layer, and an instrument that can tell two patches apart

**Status: implemented (package W3 of `plant-implementation-split-2026-08-23.md`).**
Covers review-queue item A1's grass half and item A4. The first half extends
`world-flora-sowing-2026-08-23.md` (W1) rather than paralleling it; the second
builds the measurement `physical-trees-design-2026-08-23.md` §11.6 is waiting
on, and deliberately does not build the thing it is eventually for.

---

## 1. Grass: a layer, not a fifth woody species

W1 left `life_scatter` sowing four woody species — creeper, shrub, conifer,
tree — each by a weight over three terrain facts, each into its own country,
all four in 8 of 8 seeds. Grass was explicitly deferred: it had no mortality
path, and a plantable grass that cannot die is an organism-slot leak ending in
silent id corruption at the 4,095 ceiling. P3 closed that (shade kills a blade;
the seed bank decays on an 18,000-frame half-life), which is what unblocked
this.

**The one structural decision, and it is not the weight.** Grass is sown as
its own layer off its own `grass_density`, after the woody loop and before
moss — not as a fifth row of `WOODY`. The four woody weights split *one*
budget (`weight / max(1, Σ weights)`), so a fifth entry would have taken its
columns from conifer, shrub, creeper and tree, thinning the four species W1
had just finished putting into the world and changing what `tree_density`
means in the same edit. Moss has always been its own layer for the same
reason; grass joins it there. The paired measurement in §2 is what says this
worked rather than that it was intended.

### The weight is one reading of the woody sum

`1 - ramp(woody_budget, 1.0, 2.0)`, where `woody_budget` is the plain
**unclamped** sum of the four woody weights. Grass is the ground layer of open
country, and "open" is not "no tree in this column" — it is the whole woody
preference summing low.

**The band is cut through a measured spread, and the first version was cut
through the wrong one.** `life_scatter`'s own `budget` is `min(1.0, Σ)`, and
the obvious rule — grass gets `1 - budget` — is a rule that almost never
fires. Measured over eight worlds of the default preset, restricted to
plantable columns (`flora_census -- terrain=1`, extended by this package to
print it):

| fact | min | p10 | p50 | p90 | max |
|---|---|---|---|---|---|
| `aridity` | 0.00 | 0.01 | 0.26 | 0.56 | 0.62 |
| `elev` | −0.51 | −0.08 | 0.46 | 0.75 | 0.98 |
| `soil_depth` | 0 | 8 | 16 | 29 | 43 |
| **woody sum, unclamped** | **0.81** | **1.00** | **1.59** | **2.06** | **2.51** |

**p10 is already 1.00** — the clamped budget saturates across ninety per cent
of plantable columns, so `1 - budget` is zero almost everywhere and would have
sown grass into the margins only. The unclamped sum is the quantity that can
tell forest country from ground that only just supports a tree, and `1.0 → 2.0`
spans p10 to roughly p90: grass unopposed on the tenth of the world that
supports woody cover least, absent from the tenth that supports it most,
graded across the eighty per cent between.

**Two niches fall out with no term of their own, and writing either would have
been double-counting.** Past `ramp(aridity, 0.20, 0.50)` only shrub scores, so
the woody sum drops and grass takes the dry margin — which is what grass's
`soil_water_threshold` of 0.10, the lowest of the five, is for. At
`blanket ≈ 0` only creeper scores, so a sward runs over the rock shelf.

**Footing is still soil**, for two independent reasons that happen to agree:
`soil` is the only material with a `water_capacity`, so a seed on sand reads
bone dry to `Germinate`; and grass's `penetration_force` of 1.0 clears soil's
resistance of 0.8 and does *not* clear sand's 1.4, so a grass seed on sand
could not root even if it germinated. Two numbers, one niche boundary, no rule.

## 2. What it produces, paired against main in the same session

Sixteen seeds at 2,048 columns, generation only. The branch was measured, then
`main` was rebuilt and measured on the same machine in the same session:

| species | main (min/med/max) | branch (min/med/max) |
|---|---|---|
| conifer | 2 / 6 / 27 | 2 / 6 / 27 |
| creeper | 2 / 12 / 23 | 2 / 12 / 23 |
| shrub | 1 / 6 / 25 | 1 / 6 / 25 |
| tree | 1 / 16 / 35 | 1 / 16 / 35 |
| moss | 8 / **20** / 35 | 8 / **19** / 35 |
| **grass** | **—** | **7 / 24 / 60**, present in 16/16 |

**All four woody species are bit-identical.** That is the claim the layer
split was made for, and it is a measurement rather than an argument: grass
takes its columns from *moss*, whose median goes 20 to 19, because those two
are the ones competing for the same leftover ground. Grass placement is
independent of woody placement by construction — the woody loop `continue`s a
column it planted, grass never touches the woody spacing cursor, and its
placement roll is its own salt.

Establishment, eight worlds at 1,024 columns and 300 frames: grass sown 4/9/17
per world, established 4/9/16, **8 of 8 worlds** — a pooled germination rate of
0.96. Grass's hazard is the opposite of a woody species': it has the lowest
germination bar of the five so it comes up almost anywhere it lands, and what
kills it is shade. So the rule that would fail is sowing a sward under a closed
canopy, and the establishment rate is what says the woody-sum reading keeps it
out from under one.

## 3. The guards, and the check that they guard anything

Two new tests in `tests/worldgen.rs`, in the shape W1 used — sixteen seeds,
an order statistic, one world allowed to miss the species outright:

- `grass_is_sown_across_a_seed_sweep` — present in ≥15 of 16, median ≥ 8
  (measured 24), **and median ≤ 72**.
- `sown_grass_also_comes_up` — pooled establishment ≥ 20 plants and ≥ 50% of
  sown (measured 0.96).

W1's `every_woody_species_is_sown_across_a_seed_sweep` is left exactly as it
was, so grass cannot silently weaken the guard on the four species it sits
beside.

**The upper bound has no woody equivalent and is the one worth explaining.**
Grass is the only sown species that breeds fast enough to matter to the 4,095
organism-slot ceiling, and what happens past that ceiling is silent id
corruption rather than an ugly picture. The bar is a *sowing* bound at 2,047
columns and therefore only a proxy: it catches a density or band change that
blankets the world, not a runaway seeding rate.

**Both bars were checked against the artifact they are supposed to catch,
because a guard nobody has seen fail is a guard nobody has tested**
(`CLAUDE.md`: all eight acceptance scenes once stayed green through a change
that made one world lose 26× more material, because `seed=` reached only two
of them):

| deliberate break | result |
|---|---|
| `grass_density: 0.0` | both tests fail — "grass established 0 plants across the whole sweep (0 sown)" |
| `grass_density: 3.0` | upper bound fails — "grass's median world holds 167 … the sward is blanketing the world" |

And the sweep genuinely sweeps the procedure rather than re-running one scene:
grass's per-world count runs **7 to 60** across the sixteen seeds, an 8.6×
spread, which is what makes the order statistic worth taking.

---

## 4. The A4 two-patch divergence instrument

`examples/divergence.rs`. **Same founders, two patches differing in one
environmental axis, scored on morphology.**

The named consumer is `physical-trees-design-2026-08-23.md` §11.6 — the owner
wants biomes that rarely storm to grow thinner-rooted, more slender trees, and
lane S's wind-throw work turns on being able to measure divergence between two
patches. So this is built as a general instrument with the axis as a
parameter, not as a wind instrument.

### Why the axis is moisture and not wind

`weather::at(seed, frame)` takes no position: wind is one value for the entire
world (§11.5). A windy patch and a sheltered patch are not expressible today
and no amount of instrument design makes them so. Terrain-derived exposure is
another package's work; this one picked the axis that exists so the instrument
is proven and landed rather than blocked. **What it takes to point this at
wind once exposure lands** is in §7.

### Three things it is shaped around

**1. Two worlds, not two halves of one world.** "Same founders" has to be
literal: genotypes are drawn from `(world seed, germination coordinate)`, so
two patches at different x in one world are founded by *different* plants and
the comparison would be measuring genotype draw alongside environment. Two
separately-built worlds at the same seed, the same geometry and the same seed
coordinates are founded by the same individuals. It is also what makes the
control below exact rather than approximate.

**2. The control comes first.** Both patches on the same setting; the only
correct answer is zero.

**3. An order statistic over seeds, and the spread, never a difference of
means.** Twelve identical trees from one genome span 31 to 153 cells. Every
figure is reported with quartiles beside it, per-seed divergences are printed
individually, and the headline is **how many seeds moved the same way**.

### The two metrics

- **root:shoot** — root cells over shoot cells, root being `reinforces_powder`
  tissue or a `RootTip`, the same test `plant_probe` uses.
- **slenderness** — height above the anchor plate over stem width at the base.

**What "the anchor plate" is here, stated rather than implied.**
`plant::is_structural_anchor` is the engine's own answer and it is private to
`plant.rs` — a file two other packages are live in, which this one was under
instruction to stay out of. So the plate is read off the plant instead: the
topmost row holding root tissue, which for a rooted plant is the collar. Stem
width is the count of shoot cells in the lowest shoot row, which is the
quantity `thicken`/`pipe_ratio` moves at the trunk base. The day
`anchor_support` exposes the anchor set it already enumerates and throws away,
this should read that instead; the numbers should barely move, and if they do
that is worth knowing.

Both metrics are **ratios** on purpose. A raw cell count is dominated by how
big the plant got, which is the noisiest quantity in this engine.

## 5. The control: exactly zero, against a metric that is not flat

`divergence -- control=1 seeds=3 frames=3000 founders=8`:

```
CONTROL VERDICT: PASS — two identical patches diverge by exactly zero on both metrics.
```

per-seed divergence `[0.0, 0.0, 0.0]` on both metrics — while the pooled
sample it is drawn from spans **root:shoot 0.009 to 0.576** and
**slenderness 1.26 to 57.00** across 24 individuals.

Both halves of that matter. A metric that finds a difference between two
identical patches is measuring its own noise, and this project has shipped
exactly that: the whisker hunt defined a film as "a water cell with air above
and below", which is what falling water looks like, so it counted every
droplet in the world. Its numbers were real and meant nothing. But a metric
that reports zero because it cannot see anything would pass the same test — so
the spread is printed beside the zero, and it is wide.

## 6. Two traps the instrument now reports on, both found by running it

**The dry arm has to clear the species' germination bar.** The first setting
used moisture 260. `plant_available_fraction` is `(m − 180) / (620 − 180)`, so
260 is 0.18 — under `tree`'s `soil_water_threshold` of 0.25. Measured, the dry
patch established **0, 0, 2 and 1** of twelve founders against a wet patch's
12, so the two morphology columns were reading three plants and the
"comparison" was a stand against an empty field. The default dry arm is now
380 (0.45), clear of every species' bar — conifer's 0.35 is the highest of the
five — and still a real deficit against 1.00. The instrument now also **warns
on the line** whenever one patch establishes less than half the other, because
a quiet 5-against-12 is the dangerous middle: it is selection at germination
wearing a morphology costume.

**The axis can wash out during the run, and a washed-out axis reads exactly
like an axis that does nothing.** Soil water is not static: it drains, plants
drink it, and `weather` rains on it. At the default seed the first rain lands
at frame 14,400, which is *inside* the window a confirming 25,200-frame run
uses. So the instrument reports plant-available soil water **as set at the
start and as measured at the end**, per patch, and says so loudly if the gap
has closed to under a quarter of what it was. `CLAUDE.md`: when a mechanism
appears inert, check the scene still contains the situation you think it does
before touching the mechanism. This is that check, printed rather than
remembered.

## 7. What it says about moisture — one result and one refusal

`divergence -- seeds=8 frames=10800 founders=12`, twelve founders per patch,
all twelve established in both patches on every seed:

```
divergence: species=tree founders=12 frames=10800 worldseed=1..8 axis=moisture
            lo=380 hi=620 soil=100 width=768
```

| | dry (0.45) | wet (1.00) |
|---|---|---|
| root:shoot — min / p25 / **med** / p75 / max | 0.031 / 0.054 / **0.084** / 0.145 / 0.710 | 0.015 / 0.033 / **0.049** / 0.083 / 0.395 |
| slenderness — min / p25 / **med** / p75 / max | 2.52 / 8.15 / **16.71** / 30.00 / 65.00 | 2.34 / 7.71 / **18.00** / 30.67 / 122.00 |

| metric | median divergence (wet − dry) | seeds moving the same way |
|---|---|---|
| **root:shoot** | **−0.044** | **8 of 8** |
| slenderness | +2.75 | 5 of 8 |

**Root:shoot is a result. Slenderness is not, and saying so is the point of
the instrument.**

The dry patch puts a materially larger share of itself below ground — a
median root:shoot of 0.084 against 0.049, and every one of the eight seeds
moves that way. Unanimity across eight seeds is the statistic that survives a
distribution this wide; a difference of medians on its own would not be worth
quoting, because the pooled samples overlap heavily (the dry patch's p25 of
0.054 sits above the wet patch's median).

Slenderness went the other way on three of the eight seeds, swinging from
−5.15 to +3.94, against a pooled spread of 2.5 to 122. A difference of means
there would have read as "wet trees are 8% more slender" and it would have
been noise wearing a number. The per-seed column is printed precisely so that
this case is visible rather than averaged away.

**Both numbers are honestly bounded by the run length.** 10,800 frames is a
*scouting* run by `plant-species-authoring.md` §8's own rule (scout at 10,000,
confirm at 30,000), and W1 measured the slot-5 root axis peaking at 25,200 and
washing out entirely by 43,200 — so "root:shoot diverges" is established here
at scouting length and is not yet established at maturity. That is the next
run, not a caveat to argue around. The axis itself survived this one: mean
plant-available soil water ended at 0.42 against 0.92, from 0.45 against 1.00.

## 8. What it takes to point this at wind

Everything downstream of the axis is already axis-agnostic — the founders, the
control, the two metrics, the seed sweep, the imbalance warning and the
axis-survival check do not know what is being varied. Pointing the instrument
at wind is therefore **one arm on `Axis` and nothing else**, and it needs
exactly one thing that does not exist yet:

1. **Terrain-derived exposure** (`physical-trees-design-2026-08-23.md` §11.5)
   — open fetch upwind and height above local ground, queried at gust time for
   the organisms a 26-cell gust overlaps. Until an organism's experienced wind
   is a function of *where it is*, the two patches cannot differ on it: today
   `weather::at(seed, frame)` returns one value for the whole world, so the
   windy patch and the sheltered patch are the same patch.
2. Then an `Axis::Exposure` arm sets each patch's exposure the way
   `Axis::Moisture` sets its soil water, and the axis-survival check in §6
   reports mean realised exposure per patch instead of mean soil water.

**The one thing worth flagging to whoever lands exposure:** the survival check
matters more there than it does here, not less. Gusts fire on 41.6% of frames
at the default seed, so a "sheltered" patch that is only sheltered from *some*
gust directions will converge on the exposed one over a long run, and the
instrument would report a null that belongs to the scene. Report realised
exposure, not the exposure that was requested.

The plasticity mechanism §11.6 recommends (a repeatedly-shaken tree putting
carbon into root and stem instead of height) is lane P's economy work, not
this instrument's. This measures the divergence; it does not create it.

## 9. Grass is a transient in a mature world, and the control says why

The long run is the finding this package would have shipped without if it had
stopped at the establishment rate. Two seeds, 2,048 columns, **45,000 frames**:

| | sown | established at 45,000 | grass cells |
|---|---|---|---|
| **with the woody layer** (shipped) | 40 / 42 | **3 / 4** | 72 / 20 |
| **woody layer off** (control) | 43 / 43 | **63 / 44** | 1,186 / 885 |

Grass establishes at 96% and is down to **3 of 40** by frame 45,000 in a
shipped world. The obvious reading — the sowing rule is wrong — is the wrong
one, and the control is what says so: with `treedensity=0 mossdensity=0` on the
same worlds, grass ends at **63 established from 43 sown**. More than were
sown, so it bred; 1,186 cells standing.

So grass does not fail on its own. **It is shaded out as the woody flora
matures** — conifer alone ends those runs at 49,070 cells — which is exactly
the succession story `grass.ron`'s own header says P3's shade death was for:
*"a sward thins where a canopy closes over it"*. It is the mechanism working,
observed at a scale nobody had run it at.

**It is still a limitation worth stating plainly rather than filing as a
success.** A world 45,000 frames old has almost no grass in it, and the sowing
rule cannot fix that on its own: grass is already placed where the woody sum is
lowest, and the canopy grows over it anyway because the sum is a *country*
reading and a stand is not. Three levers exist and none of them is this
package's:

1. **Woody density** (`tree_density`) is a preset knob and a one-line change —
   but "is this world too full" is a question the owner answers by eye, and
   W1's panorama card already asked it.
2. **Grass's shade tolerance** (`Photosynthesize::shade_death`, 0.004) is an
   economy constant and belongs to lane P's single re-derivation.
3. **Disturbance** — fire is W2's, and a grassfire that clears a canopy patch
   is exactly what would keep a sward in a mature world. That is the
   ecologically right answer and it is already queued.

**The organism-slot ceiling is not a concern, measured rather than assumed.**
High-water 229 and 112 of 4,095 with the woody layer, 76 and 65 in the control
where grass is breeding freely, and **0 births refused** in all four runs. The
`grass_density: 3.0` guard bar remains the thing standing between a future
density edit and that ceiling.

## 10. Cost

`examples/ascii`, this branch against `origin/main`, both rebuilt and run **in
the same session on the same machine**, and run **four times each** — because
at two runs a side the worst-frame column was unreadable: one branch run
reported 42.4 ms on a 512x320 scene whose other three runs read 6.1, 4.7 and
4.7, and a later `main` run reported 25.3 ms on the same scene. Both sides
produce container hiccups, and a two-run comparison would have attributed one
of them to the change.

Medians of four runs a side:

| `examples/ascii` line | main | branch |
|---|---|---|
| river-cost 8,192x2,560, spring OFF, **mean** | 11.16 ms | 11.18 ms |
| river-cost 8,192x2,560, spring ON, **mean** | 13.08 ms | 13.23 ms |
| river-cost 512x320, spring OFF, **mean** | 1.29 ms | 1.26 ms |
| river-cost 512x320, spring ON, **mean** | 2.87 ms | 2.81 ms |
| organism scene, **mean** (61 vs 79 live organisms) | 3.57 ms | **3.15 ms** |
| river-cost 8,192x2,560, spring OFF, **worst** | 48.9 ms | 45.1 ms |
| river-cost 8,192x2,560, spring ON, **worst** | 51.1 ms | **68.0 ms** |

Three readings, and they are not the same reading:

- **Mean frame cost is unchanged on every scene.** The largest gap is
  +0.15 ms on the shipped world with the spring on, against a branch spread of
  0.59 ms across four runs of the *same* binary. Inside the noise floor.
- **The organism scene is 0.43 ms (12%) cheaper on the branch, in all four
  runs against all four**, and that is *not* "grass is free". The branch's
  scene holds 79 live organisms against main's 61 — but adding organisms
  reshuffles every subsequent RNG draw, so the woody plants in it are not the
  same plants. Measured on the shipped world at 3,600 frames, total plant cells
  come out 70,996 on the branch against 72,610 on main: the branch's stand is
  simply a slightly smaller one. Read this as "the scene changed", not as a
  speedup.
- **The worst frame on the spring-ON shipped scene is the one number that may
  be a real cost**: median 51.1 to 68.0 ms, with branch maxima of 81.8 and 84.6
  against main's 60.9. Four runs a side is not enough to call that definitively
  — the same column carries 40 ms outliers on both sides — but it is the same
  direction, the same scene and the same stated cause as W1's own finding
  (53 to 62 ms, *"more organisms means more organism ticks landing together on
  some frames"*), so it is named rather than averaged away. A worst frame is
  what a player feels.

**The lever, if that is too much, is `grass_density`** — a preset field, thinning
the sward without moving where it goes.

## 11. What this leaves open

- **Grass's long-run fate** (§9) — the mature world is nearly grass-free, and
  the three levers are all other packages'. Worth an owner call on whether a
  45,000-frame world *should* be a closed forest.
- **The divergence instrument is proven at scouting length only** (§7).
  Root:shoot diverges 8 of 8 at 10,800 frames; whether it survives to 25,200
  and 43,200 is one run and has not been made.
- **Slenderness has no confirmed driver.** It did not respond to moisture, and
  the plasticity mechanism that §11.6 expects to move it is lane P's economy
  work.
- **Pointing the instrument at wind** needs terrain-derived exposure and
  nothing else (§8).
- **`plant_tree_species`'s doc** now understates what it plants — it says the
  seed germinates into a `wood`-material `GrowingTip`, and grass germinates
  into `grassblade`. Left for whoever is next in `plant.rs`, which two other
  packages are live in.
