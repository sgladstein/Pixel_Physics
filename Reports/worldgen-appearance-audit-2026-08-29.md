# Why every generated world reads as the same picture

Measured 2026-08-29. Lane A of the worldgen revamp programme: a measurement
lane, not an implementation one — nothing outside `examples/` changed.

**The instrument is `examples/world_look.rs`**, new, built for this and
described in §7. Everything below is rendered through the shipped `Renderer`
at the player's own viewport (512x320 of an 8192x2560 world, 1/128th of it),
with daylight pinned at noon, so no arm can invent a colour or a scale the
engine would not have drawn.

---

## 1. The headline

**Of the 14 generation passes, 9 change less than 1% of the pixels in a
player's viewport, and 4 of those change none at all — in any preset, at any
seed.** `vaults` writes a 43,208-cell cave system and moves **zero** pixels.
`moisture_init` writes 298,239 cells and moves **zero**. `life_scatter` — the
entire biosphere — moves **817 pixels out of 2,621,440**, 0.031%.

The passes that do move the picture are three, and all three are **colour**
passes: `stone_massif` decides what the rock is (29–50% of the view),
`soil_blanket` puts soil on it (up to 19%), `soil_moisture` decides how dark
that soil is (up to 18%). **Every landform pass in the generator — cliff
brows, talus aprons, tors and stacks, boulders, caves — sits at or under
0.59%.**

That is `plant-appearance-design.md`'s finding, transposed exactly. There it
was *a lever that changes which cell gets a label cannot change a silhouette
that texture and colour set*. Here it is: **the generator's landform work is
0.6% of the picture and its palette work is 90% of it**, and six rounds of
worldgen effort went into the 0.6%.

---

## 2. The hypothesis, and what happened to it

The lane was opened on the hypothesis that worldgen has the plant line's
disease and that the evidence for it is the owner's six repeated *"I see no
difference"* verdicts. **The hypothesis survived; the reading of the evidence
did not, and the correction is what makes the finding actionable.**

### 2.1 What died: "every world looks the same" is false

Presets are **not** one picture. Over 6 presets x 4 seeds, a total-variation
distance between colour histograms of everything below the skyline gives:

| | colour TV | material TV | shape TV |
|---|---|---|---|
| same preset, different seed | 0.095 | 0.036 | 0.024 |
| different preset | 0.453 | 0.150 | 0.078 |
| **ratio** | **4.76x** | 4.15x | 3.18x |

Excluding `flat` — which ships `region_variation = 0` and is a structural test
bed rather than a world — the colour ratio is still **3.50x**. Read the TV as
*the fraction of the visible ground that would have to be repainted to turn
one into the other*: swapping `arid` for `wetland` repaints 71% of it.

**So the question as posed — "why does every generated world look like the
same picture?" — has the answer "it doesn't".** Anything scoped on that
premise would have been scoped on a false one.

### 2.2 What went wrong with the evidence

I pulled the nine "no difference" verdicts out of the review queue and read
what each card actually compared. **Not one of them is a comparison between
presets.** Every one is an A/B of a single change inside one preset, usually
at one seed:

| verdict | card | the pass or channel under test |
|---|---|---|
| "I see no difference" | deep rock grain, graded with depth | `render.rs` grain |
| "No significant difference" | formation palette — does one read as crystal? | `vaults` |
| "very little practical differences" | cave retune, reachability-first | `vaults` |
| "don't really see much difference" | cave formations, round 5 vs A3 taper | `vaults` |
| "Mostly more of the same" | four woody species instead of one | `life_scatter` |
| "no difference … except a few rock formations" | rock country at region vs country size | palette field |
| "I don't see a difference" | a world with a ground layer, against one without | `life_scatter` |

Set the two columns side by side against §3's table and the verdicts stop
being a mystery: **`vaults` moves 0.000% of the view and `life_scatter` moves
0.031%.** Three of those cards asked the owner to judge a retune of a pass
that is invisible at 100% strength. He was right every time, and the pictures
could not have shown him anything else.

**This is a retrodiction, and it is the strongest control the audit has.** The
instrument was built without reference to these verdicts and it predicts all
seven of them.

### 2.3 What survived, unchanged

Both structural claims the hypothesis made turned out to hold exactly:

- **Composition.** In every preset, **94.6% or more** of the solid cells in a
  player's viewport are one of four minerals — stone, soil, sand, gravel
  (§4). The worldgen analogue of the plant line's "90% wood" is real and it is
  worse: on `arid` and `canyon` it is 99.9%.
- **Palette.** **Four to nine colours cover half of everything the player sees
  below the skyline** (§5), out of 104–180 that appear at all. On `flat` it is
  two.

### 2.4 What is new, and was not in the hypothesis

**Presets differ in colour and barely in form.** Between two presets, colour
TV is 0.453 and skyline-shape TV is **0.078** — 5.8x more difference in hue
than in landform. Excluding `flat`, shape falls to 0.059 against colour 0.440:
**94% of the skyline-step distribution is shared between any two presets.**
The mean absolute skyline step is between **0.20 and 0.48 cells per column in
every preset** — nowhere in any world does the ground line locally rise or
fall by half a pixel per column. Macro relief does differ (`canyon` 35–39
cells of skyline sd against `wetland` 8), but the texture of the ground line
does not.

**And three of the five real presets are one preset.** The pairwise matrix
(§6) puts `rolling` against `terraced` at **0.149 — below `rolling`'s own
seed-to-seed distance of 0.173.** Changing that preset moves the picture less
than changing the seed does. `canyon` joins them at 0.24–0.27 against its own
0.190. Only `arid` (0.065 within, 0.44–0.79 against everything) and `wetland`
(0.234 within, 0.39–0.71 against) separate — and both do it the same way, by
moving the **palette family** wholesale, not by building different landforms.

**So the hypothesis was right about the mechanism and wrong about where to
look for it.** The generator does not make one world six times. It makes
**two or three colour-schemes**, and inside each of them the changes it has
been shipping for six rounds are invisible.

---

## 3. Which passes move any pixels at all

Per-pass ablation, measured in **rendered pixels** rather than cells. Cameras
are computed from the un-ablated world and reused, so both arms look at the
same place and a pass that lowers the terrain cannot be scored for panning the
camera. 16 viewports per world — full coverage of the world's width, 2,621,440
pixels. Median over 3 seeds.

| pass | arid | canyon | flat | rolling | terraced | wetland | best % of view | cells written |
|---|---|---|---|---|---|---|---|---|
| `stone_massif` | 1,145,894 | 1,176,760 | 1,310,288 | 1,019,718 | 1,068,718 | 758,165 | **49.98%** | 19,300,014 |
| `soil_blanket` | 138,292 | 103,337 | 432 | 187,236 | 191,803 | 498,574 | **19.02%** | 497,702 |
| `soil_moisture` | 0 | 54,710 | 130 | 153,885 | 174,452 | 481,735 | **18.38%** | 481,994 |
| `ponds` | 0 | 1,303 | 0 | 121,574 | 67,495 | 71,766 | 4.64% | 88,118 |
| `pockets` | 41,009 | 41,603 | 0 | 23,750 | 27,176 | 14,218 | 1.59% | 359,627 |
| `brows` | 316 | 15,394 | 0 | 5,336 | 2,995 | 208 | 0.59% | 15,321 |
| `residuals` | 5,212 | 7,047 | 0 | 7,279 | 8,671 | 6,080 | 0.33% | 8,393 |
| `talus` | 184 | 5,743 | 0 | 2,264 | 1,599 | 146 | 0.22% | 5,298 |
| `life_scatter` | 0 | 166 | 0 | 343 | 509 | 817 | **0.031%** | 817 |
| `springs` | 0 | 220 | 0 | 151 | 179 | 0 | 0.008% | 109 |
| `bedrock_floor` | 0 | 0 | 0 | 0 | 0 | 0 | **0.000%** | 36,961 |
| `boulders` | 0 | 0 | 0 | 0 | 0 | 0 | **0.000%** | **0** |
| `vaults` | 0 | 0 | 0 | 0 | 0 | 0 | **0.000%** | 43,208 |
| `moisture_init` | 0 | 0 | 0 | 0 | 0 | 0 | **0.000%** | 298,239 |

**Three things to read off it.**

1. **Cells written and pixels moved are not the same quantity, and the gap is
   two orders of magnitude wide.** `pockets` writes 359,627 cells for 41,603
   pixels (0.12 px per cell); `soil_moisture` writes 481,994 for 481,735
   (1.00); `moisture_init` writes 298,239 for 0. Any pass table denominated in
   cells — which is what `pass_ablation` reports, correctly, for the
   interference question it answers — ranks these three the same way and is
   silent about the only difference that reaches the owner.
2. **`boulders` writes zero cells in every preset at every seed.** That is not
   an invisibility finding, it is a pass that never fires, and it is a
   different and possibly larger problem. It is **not** simply that there is
   nothing to place: erosion sheds 0–3 boulder markers per world (`canyon` 3,
   `terraced` 1) and `boulders-seated` is **0 in every preset**, so the markers
   exist and nothing takes them up. Not diagnosed here — flagged.
3. **The zeros are not noise.** Control 6 (§7) puts the same machinery through
   an ablation of *nothing* and gets exactly 0 differing pixels; control 5
   ablates `stone_massif` and gets 43% of the frame. A zero in this table
   means the pass changed nothing a player can see, not that the harness
   failed to look.

### 3.1 The vegetation number, from the shipped instrument

`flora_census` at the real world size, `rolling`, 2 seeds, at generation:

```
seed 1  slots 343/4095  conifer sown 34 | creeper 37 | grass 116 | moss 66 | shrub 35 | tree 55
seed 2  slots 236/4095  conifer sown 32 | creeper 53 | grass  44 | moss 61 | shrub 15 | tree 31
```

**The entire flora of an 8192-column world is 343 one-cell seeds.** That
cross-checks the ablation exactly — `life_scatter` on `rolling` writes 343
cells and moves 343 pixels, so every planted cell *is* visible and there are
simply 343 of them. Only `moss` establishes without a long run (66 of 66); the
other five species are sown and ungerminated at the timescale any review card
has ever been rendered at.

---

## 4. Composition census

6 presets x 4 seeds x 8 viewports, aimed so the ground line sits mid-screen.
Shares are of the cells that hold material; **the ~50% sky share is a property
of that camera rule, not a finding.**

| preset | stone | soil | sand | gravel | water | **four minerals** |
|---|---|---|---|---|---|---|
| arid | 87.4% | – | 10.2% | 2.5% | – | **100.0%** |
| canyon | 89.4% | 3.6% | 4.4% | 2.5% | 0.1% | **99.9%** |
| flat | 100.0% | – | – | – | – | **100.0%** |
| rolling | 80.7% | 8.8% | 3.2% | 1.9% | 5.3% | **94.6%** |
| terraced | 82.2% | 12.3% | 1.8% | 1.9% | 1.7% | **98.2%** |
| wetland | 60.7% | 33.7% | 0.7% | 1.6% | 3.1% | **96.7%** |

The **skin** — the top 8 cells of material in each column, what actually reads
as the ground surface — is the one place composition genuinely varies, and it
is worth stating because it is the counter-evidence to a naive "it's all
stone" reading:

| preset | 1st | 2nd | 3rd | top two |
|---|---|---|---|---|
| arid | sand 61.2% | stone 32.0% | gravel 6.8% | 93.2% |
| canyon | stone 34.8% | soil 28.7% | sand 24.6% | 63.6% |
| flat | stone 99.2% | gravel 0.5% | soil 0.3% | 99.7% |
| rolling | water 37.6% | soil 33.5% | stone 18.6% | 71.0% |
| terraced | soil 55.4% | stone 20.1% | water 16.2% | 75.5% |
| wetland | soil 55.4% | water 34.8% | stone 7.3% | 90.2% |

`seed` never exceeds **0.9%** of the skin and `moss` never exceeds **0.4%**.
No `wood`, `leaf` or `grassblade` cell appears in any census, at settle 0 or
settle 60.

---

## 5. Colour census

Ground pixels only — sky is excluded because it is the same gradient in every
preset and leaving it in drags every distance toward zero for a reason that
has nothing to do with the ground. Bins are 16 levels per channel, chosen to
be **the width of `render.rs`'s own grain jitter** (`JITTER_STRENGTH = 0.12`,
about ±15 levels on a mid-grey), so a rock family stays one or two bins rather
than dozens.

| preset | bins occupied | **b50** | b90 | entropy | mean luma | luma sd | skyline sd | mean \|step\| | speckle | speckle / luma |
|---|---|---|---|---|---|---|---|---|---|---|
| arid | 104 | **7** | 30–35 | 4.89–5.01 | 139.9–143.5 | 25.3–26.2 | 9.6–11.0 | 0.26–0.32 | 9.5–10.0 | 6.8–7.0% |
| canyon | 137–149 | **6–9** | 30–41 | 4.79–5.33 | 128.3–137.0 | 28.7–33.5 | 35.5–39.0 | 0.35–0.48 | 9.0–9.8 | 6.8–7.2% |
| flat | 21–25 | **2–3** | 6–7 | 2.76–2.87 | 121.9–124.6 | 12.1–13.0 | 0.0 | 0.00 | 7.4–7.8 | 6.1–6.2% |
| rolling | 150–180 | **5–7** | 32–45 | 4.73–5.20 | 115.7–122.4 | 30.7–41.5 | 15.7–17.4 | 0.24–0.28 | 8.3–9.1 | 7.0–7.4% |
| terraced | 153–179 | **4–6** | 25–31 | 4.52–4.74 | 112.8–115.5 | 33.3–40.4 | 17.4–23.4 | 0.33–0.39 | 8.0–8.5 | 7.0–7.5% |
| wetland | 141–180 | **4–5** | 17–19 | 4.15–4.33 | 70.0–96.4 | 38.4–46.6 | 7.8–8.8 | 0.20–0.28 | 5.9–7.1 | 7.4–8.4% |

`b50` is the number of colour bins covering half the visible ground.

**Two findings beyond the headline.**

- **The texture of the ground is the render grain, not the geology.** Local
  speckle (`pixel_stat`'s statistic — mean absolute deviation of a pixel's
  luma from its own 3x3 neighbourhood, computed inline here) sits at **6.1% to
  8.4% of mean luma across all 24 worlds**. The bottom of that range is
  `flat` — one material, one palette family, one seedless heightfield, whose
  speckle is therefore *pure grain*. A fully varied `wetland` reaches 8.4%.
  **A world made of literally one rock, at one palette family, carries 73–92%
  of the relative local texture that a fully varied world does** (`flat` 6.08–
  6.23% against 6.78% for the least-textured generated preset and 8.40% for the
  most). Whatever the eye reads as surface detail here, the geology contributes
  between a tenth and a quarter of it and the grain contributes the rest.
- **The grain also inflates any raw colour count.** 2,675–4,154 distinct RGB
  values appear on ground pixels; 104–180 bins. The 25x gap is entirely
  per-cell jitter (control 4). A revamp that counted raw RGB values would
  conclude the palette is rich. It is not.

---

## 6. Between-preset against within-preset: the crux

Median colour TV, every preset against every preset. **The diagonal is the
same preset at a different seed** — the bar each column has to clear.

| | arid | canyon | flat | rolling | terraced | wetland |
|---|---|---|---|---|---|---|
| **arid** | *0.065* | 0.438 | 0.791 | 0.596 | 0.633 | 0.711 |
| **canyon** | 0.438 | *0.190* | 0.449 | 0.241 | 0.267 | 0.557 |
| **flat** | 0.791 | 0.449 | *0.071* | 0.435 | 0.386 | 0.619 |
| **rolling** | 0.596 | 0.241 | 0.435 | *0.173* | **0.149** | 0.404 |
| **terraced** | 0.633 | 0.267 | 0.386 | **0.149** | *0.138* | 0.389 |
| **wetland** | 0.711 | 0.557 | 0.619 | 0.404 | 0.389 | *0.234* |

- **`rolling` vs `terraced` = 0.149, under `rolling`'s own 0.173.** By this
  measure they are not two presets. A pooled between-preset median of 0.453
  hides this completely, which is why the matrix is here and not just the two
  medians.
- **`canyon` sits 1.27–1.41x its own diagonal from both of them.** The three
  form one cluster.
- **`arid` is the cleanest separation in the set** — the tightest diagonal
  (0.065) and 6.7–12.2x that against everything else. It is also the preset
  that shifts the soil and stone palette *families* hardest.
- The **worst within-preset pair in the whole sweep is 0.346** — larger than
  three of the fifteen between-preset cells, and all three of those are
  inside the `canyon`/`rolling`/`terraced` cluster.

Two review cards are queued on this, `worldgen` board, posted 2026-08-29:
*"Two different presets, one country"* (the `rolling`/`terraced` pair) and
*"All five presets, one seed, one camera"* as its control. **The metric is not
calibrated to a human eye and those cards are the calibration.** If the owner
separates the second and not the first, the matrix can be trusted to scope the
revamp; if he separates both, the colour histogram is finer than his eye and
every distance in §2.1 is an upper bound.

---

## 7. The instrument, and every control it was put through

`examples/world_look.rs`. `mode=composition|colour|distance|passes|shot|control`.
It echoes its own parameters on the first line, so a log that does not name its
seed count was written by a binary that never had one.

**Existing instruments were used where they fit and are named where they
did not.** `flora_census` answered §3.1 unchanged. `pixel_stat`'s statistic is
reproduced inline rather than through a PNG round trip, because the census
needs it per-view over ground pixels only. `pass_ablation` supplied the ablation
entry point (`worldgen::generate_ablated`) but reports **cells**, which is the
one thing §3 exists to say is the wrong unit. `creature_look` supplied the
pinned-daylight render and the paired with/without design. `viewshot` answers
"what does the player's viewport show" but composes a sheet rather than
returning the pixels, and every number here needs the pixels.

`mode=control` runs six checks, all green:

| control | asks | result |
|---|---|---|
| 1 / 1b | can the distance report **zero**? Same world, twice | colour TV 0.000000, material TV 0.000000 |
| 2 | can it report **large**? The same world repainted in one material | colour TV **0.962**, material TV 0.854 |
| 3 | is the pixel↔cell mask **aligned**? One cell repainted lava | **exactly 1 pixel moved, at exactly the predicted (100, 300)** |
| 4 | how much of the colour count is grain? | 2,473 exact RGB against 104 bins |
| 5 | can the ablation arm report large? Without `stone_massif` | 43.0% of pixels differ (≈ the whole non-sky half) |
| 6 | can it report zero? Ablation of **nothing** | **0** pixels differ |

Controls 2/5 are the sensitivity half and 1/6 the specificity half —
CLAUDE.md's rule that a number must both *move when something is wrong* and
*stay quiet when nothing is*. Control 3 is the one that would have silently
poisoned every colour figure: a mask off by one row misclassifies sky as
ground.

**Two further controls on the measurement itself:**

- **Settle.** Every census above is at settle 0. Re-run at **settle=60** — what
  `viewshot`, and therefore every review card the owner has seen, actually
  renders — `b50`, `b90`, entropy, mean luma, luma sd, skyline sd, mean step
  and speckle are **unchanged to three figures** on all three presets tested.
  Only the raw bin and RGB counts rise (150→199, 4,154→4,860), and `b90` does
  not move, so the extra bins are tail. The settle-0 numbers describe the
  pictures that were judged.
- **Sampling.** 8 viewports covers 50% of a world's width; 16 covers all of it.
  Composition at 16 views moves `rolling`'s four-mineral share 94.6%→94.2% and
  `wetland`'s 96.7%→96.3%. The §4 sampling is representative. §3 was run at 16
  throughout, so no rare feature can have been missed by framing.

**Oscillators divided out.** Daylight is pinned at noon in every render here
(`sky::frame_for_daylight(1.0)`), for the reason `creature_look` records: its
first run landed at night and read a surround luma of 28 against 153 at
midday. No number in this report is a statement about the hour.

---

## 8. What I could not establish

- **Whether the distance metric matches the owner's eye.** It matches mine —
  the `rolling`/`terraced` pair reads as one country in the render and the
  `arid`/`wetland` pair as two — but that is one more agent's opinion. The two
  queued cards are the only thing that settles it, and they are unanswered at
  the time of writing.
- **What a *grown* world looks like.** No world measured here contains a
  single `wood`, `leaf` or `grassblade` cell, at settle 0 or 60, because
  `life_scatter` sows seeds and five of six species need thousands of frames to
  germinate. Everything in §4 and §5 is a statement about the **mineral**
  world. It is also a statement about the world every review card in the queue
  was rendered from, which is why it is the right thing to have measured — but
  a revamp that lands vegetation would invalidate the palette numbers, and
  should re-run this.
- **Whether an invisible pass is invisible or merely eaten.** §3 measures a
  pass's *marginal* contribution with all thirteen others present. A pass whose
  cells are overwritten by a later one scores 0 for a completely different
  reason than one that writes only underground, and this cannot tell them
  apart. `pass_ablation`'s interference matrix is the instrument for that, and
  crossing the two is the obvious next measurement — `vaults` in particular is
  a known victim (`pockets` eats it, round-5 review).
- **Why `boulders` seats nothing.** Confirmed zero across 6 presets x 3 seeds,
  with 0–3 markers shed upstream and none ever taken up. Not diagnosed.
- **Shape, beyond the skyline.** The shape channel is a 1-D per-column skyline
  step histogram. It cannot see cave silhouette, formation profile, or anything
  below the surface, so "presets barely differ in form" is a claim about the
  **ground line only** and should not be read wider.
- **Seed count.** 4 seeds for §4–§6, 3 for §3. CLAUDE.md's own bar is that six
  is not a sweep. §3's effect sizes are 0 against 500,000 and are not at risk;
  **§6's matrix rests on 6 within-preset pairs per preset and is the number
  most likely to move on a wider sweep.** The `rolling`/`terraced` cell (0.149
  against a 0.173 diagonal) is close enough that it should be re-run at 12+
  seeds before anything is built on it.
- **Cost.** Nothing here measures a frame. No claim in this report is a
  performance claim.

---

## 9. What this says to the revamp

Stated as findings, not as a plan — the implementation lanes own that.

1. **Stop spending on landform passes until something makes them visible.**
   Six rounds went into `vaults`, `residuals`, `brows`, `talus` and `boulders`.
   Together they are **at most 0.6% of the player's view**, and three of the
   owner's "no difference" verdicts are exactly that fact arriving as a
   playtest report.
2. **The lever that already works is palette family.** The two presets that
   separate — `arid` and `wetland` — separate by moving `palette_family`, and
   the three that collapse are the three whose families overlap. `stone.ron`
   has four families and `soil.ron` three; that is the whole colour vocabulary
   of every world, and it is what 90% of the picture is drawn from.
3. **A pass's own cell counter cannot be trusted as an appearance number** and
   the gap is 100x. If the generator gains a pass-visibility figure, §3's
   column is the shape it should take.
4. **Vegetation is 343 cells in an 8192-column world.** Any appearance work
   that assumes a green layer exists is assuming something the generator does
   not currently produce at a scale the eye can find.

---

*Freshness: written 2026-08-29 against `claude/worldgen-revamp-plan-dot67g`
at `3c464c2`. Every figure is reproducible from `examples/world_look.rs` and
`examples/flora_census.rs` at that commit; the invocations are in each mode's
doc comment.*
