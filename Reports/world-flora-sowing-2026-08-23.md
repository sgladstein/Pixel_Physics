# Sowing the flora that already existed — and giving four species four looks

**Status: implemented (package W1 of `plant-implementation-split-2026-08-23.md`).**
Covers review-queue items A1-woody, C2 and the data half of A3, plus the B1
render. Written because three of its four findings are arithmetic that the
next session would otherwise re-derive, and one is a decision that has to be
visible to whoever picks up creeper's roots.

## 1. What was wrong

`life_scatter` sowed moss and the hardcoded string `"tree"`
(`passes.rs:3880`). **Conifer, shrub and creeper had never been planted in a
generated world** — `Reports/plant-project-review-2026-08-23.md` §2.0 found
it during a revision, and nothing in the record flagged it. Measured before
any change, over eight seeds at 2,048 columns:

| species | sown min/med/max |
|---|---|
| moss | 14 / 22 / 28 |
| tree | 25 / 33 / 46 |
| conifer, shrub, creeper | 0 / 0 / 0 |

The existing guard could not have caught it. `the_world_arrives_with_both_moss_and_trees_in_it`
counts **materials**, and every woody plant in this engine is made of `wood`
— so a world with one woody species and a world with four are the same
number to it. The census this work added counts *organisms by species*,
which is the only reading that tells those two worlds apart, and it is why
`examples/flora_census` exists.

## 2. The sowing rule: four readings of three facts, not a biome table

No new field on `Character`, no biome enum, no per-species terrain pass. Each
species is a **weight** over quantities the generator has already decided by
the time the pass runs — the same move `soil_blanket` makes when it lets
aridity decide sand against soil instead of asking for a "desert" flag.

The three facts, and their **measured** spreads over six worlds of the
default preset (`flora_census -- terrain=1`, restricted to columns the pass
can actually plant in — soil footing with sky above):

| fact | min | p10 | p50 | p90 | max |
|---|---|---|---|---|---|
| `aridity` | 0.00 | 0.02 | 0.27 | 0.58 | 0.62 |
| `elev` | −0.51 | −0.05 | 0.54 | 0.75 | 0.98 |
| `soil_depth` | 0 | 8 | 15 | 27 | 43 |

Those numbers are the whole reason the bands sit where they do. **Look before
you measure**: a band placed from an aspiration rather than from the spread
is either empty or the whole world, and both mistakes look identical on
screen — "that species is rare".

- **creeper** — `1 − ramp(blanket, 0.15, 0.60)`. The skin over rock, and
  nothing else.
- **shrub** — `ramp(aridity, 0.20, 0.50) × (1 − 0.5·blanket)`. The dry
  margin, thin ground preferred.
- **conifer** — `(1 − aridity) × ramp(upland, 0.55, 0.85)`. A damp upland
  belt; `elev` is regional, so it arrives as country rather than as a
  sprinkle.
- **tree** — `(1 − aridity) × ramp(blanket, 0.10, 0.35)`. The mesic
  generalist, and what the pass used to sow everywhere.

Three decisions inside that are not obvious from the code:

**Only `soil` has a `water_capacity`.** A seed resting on sand, gravel or
stone reads bone dry to `Germinate` whatever the weather has done, so there
is no such thing as a woody species differentiated by *footing material* —
every woody seed goes on soil or it never comes up. The differentiation has
to come from aridity, altitude and blanket depth, which is why footing does
not appear in any of the four weights.

**The weights agree with the germination ladder, and the agreement is
load-bearing.** `soil_water_threshold` runs conifer 0.35 > tree 0.25 >
shrub 0.20 > creeper 0.15. Sowing conifer into dry upland would have sown it
forever and germinated it never. `a_sown_woody_species_also_comes_up` is the
test that says the agreement holds rather than that it was intended; pooled
over eight worlds it measures conifer 45/46, creeper 45/46, shrub 12/16,
tree 67/71.

**Specialists first, generalist last.** A column is claimed by the first
species whose roll succeeds. Ordering `tree` first would let it take the
thin, dry and upland columns that are the *only* country the other three
have, and the world would go back to one woody species with three rarities.
Least-tolerant first is also what the germination ladder already says.

**`tree_density` stays the woody density of the best ground.** Each species'
share is `weight / max(1, Σ weights)`, so a column that suits three species
carries about as many plants as one that suits only tree. Without the divisor
four weights of 0.7 would sow four times the density and the same preset
number would mean something different than it did.

Each species draws its **own** cluster field — the same squared low-frequency
noise, offset by 137.4 wavelengths per species — so their thickets fall in
different places. Four species sharing one set of clumps would read as one
mixed thicket repeated across the world, which is the opposite of the point.

### What it produces

Sixteen seeds at 2,048 columns, generation only: every woody species sown in
**all sixteen** worlds. Per-world medians conifer 6, creeper 14, shrub 6,
tree 15; per-world minima 2 / 2 / 1 / 1.

The shipped 8,192 × 2,560 world, seed 1, 3,000 frames, paired against
`origin/main` on the same seed and the same frame count:

| | before | after |
|---|---|---|
| tree established | 87 | 49 |
| conifer / shrub / creeper established | 0 / 0 / 0 | 30 / 28 / 28 |
| moss | 65 | 65 |
| **plant cells** | **37,297** | **55,782** |

So 87 standing plants become 135, and plant cells rise 50%. That is a real
cost rather than a rounding error — §7 has the frame bill. Tree itself
roughly halves, which is the intended half of the trade: it keeps the damp
deep country and gives up the thin, dry and upland columns it used to take
by default.

## 3. Grass is not sown, and that is not an oversight

`grass` has no mortality path (review report §F4): a plantable grass that
cannot die is an organism-slot leak that ends in silent id corruption at the
4,095 ceiling. It waits on lane P's package P3, not on anything in worldgen.
The `WOODY` table is named for what it holds so that adding grass is an
explicit act rather than a plausible-looking edit.

## 4. Four species, four looks (item C2)

### The palette arithmetic, which is the whole finding

`shrub` and `creeper` declared **identical** `foliage_bands: (first: 4,
count: 2)` and `bark_bands: (first: 2, count: 2)`; `conifer`'s bark overlapped
`tree`'s on band 1 and `shrub`'s on band 2, under a comment claiming it was
disjoint from both.

Four species at `count: 2` need **eight** bands to be disjoint. Leaf had six
and wood had four. There are only two ways out and one of them is wrong:

- **Halve the ranges to `count: 1`.** Rejected. On the leaf axis the band
  *is* the leaf-economy allele a founder starts on (`plant.rs`'s
  `LOCUS_LEAF_ECONOMY`), so `count: 1` would found every plant on one
  economy and turn a colour edit into a silent economy change. On the bark
  axis `count` is what `bark_band_for_density` spends to show a dense
  individual apart from a pioneer *within* a species.
- **Lengthen the palettes.** Taken. Leaf 6 → 8, wood 4 → 8.

Assignment after the pass, disjoint on both axes: `tree` foliage 2-3 / bark
0-1 (**unchanged**), `shrub` 4-5 / 2-3 (**unchanged**), `conifer` 0-1 / 4-5
(bark moved), `creeper` 6-7 / 6-7 (both new). Only the two species that were
actually colliding moved, which is why three of the four shipped looks are
byte-identical to what they were.

Leaf bands 6-7 step **off** the hue axis rather than continuing it, and the
break is deliberate: continuing past dark olive means going brown, which
reads as dead foliage. Desaturating instead gives a sage that is
unmistakably a leaf and unmistakably not one of the four saturated greens.
The contiguous-slice property still holds where anything reads it — inside a
single species' range.

`banded_shade` draws **one** rng value per cell whatever the band is
(`index * PALETTE_BAND + rng.below(PALETTE_BAND)`), so none of this can move
a cell. That discipline is what makes a recolour free to make.

### The identity levers: what moved, and what was already fine

`Reports/plant-appearance-design.md` is unambiguous that composition and
colour outperform every architectural lever, so this pass spent its budget
there and checked the rest rather than jittering it:

- **`leaf_cluster` — spread, and it is now an explicit trade.** Height budget
  and cluster size run opposite ways: conifer 150 rows at 6, tree 120 at 10,
  shrub 40 at **12** (was 8), creeper 8 at **14** (was 8). What a plant
  cannot spend on height it spends on leaf area per node. Affordable because
  income, the bud-break gate and the pipe ratio are all normalised by
  `leaf_cluster` (tree.ron's own note) — it is more leaf cells at the same
  economy. It is **not** free, and the measurement says so: on one seed at
  3,000 frames shrub's cell count fell 11,147 → 8,562 (−23%), which is the
  denser-crown-shades-its-own-interior cost that note predicts. Conifer and
  tree, whose values did not change, still moved by +3% and +4% — neighbours
  compete for the same light. Nothing was byte-identical, which is the tell
  that the knob is connected at all (`CLAUDE.md`'s `include_str!` trap).
- **`plastochron` — already four distinct profiles** (conifer `[14,3,2,2]`,
  tree `[12,5,2,2]`, shrub `[3,2]`, creeper `[2,2]`). Left alone. It is the
  strongest size lever measured (3.9×, inverted), so moving it to satisfy a
  checklist would have moved a species' size for no stated reason.
- **`branch_angle` — already four distinct profiles** (`[70,60,50,45]`,
  `[85,60,45,45]`, `[50,45]`, `[80,75]`). Left alone, same reasoning.
- **`turgor_source` — 1.0 / 1.0 / 0.4 / 0.16.** `tree` and `conifer` tie
  here and still reach different heights (120 against 150 rows), because the
  difference is carried by `turgor_per_cell`, which is an economy constant
  and belongs to lane P. Not manufactured a difference on the identity knob
  to avoid saying so.

So the near-clone finding was **colour and composition**, not architecture —
which is exactly what §2.1 of the review report predicted and what the
appearance report measured a phase earlier.

## 5. Decision: creeper is sown as-is, with its dead root knob left alone

`creeper.ron`'s `RootTip` runs the **superseded** in-tick branching path —
`branch_chance: [0.05]` with no `branch_priming` — which the other three
species carry a comment explaining they abandoned because it "cleared that
twice in twelve thousand frames and fired zero times". The review report
asked for a decision before sowing it. The decision is **sow it and do not
touch the root block**, for three reasons:

1. **It is not blocking anything, measured.** Creeper establishes 45 of 46
   sown across the eight-world sweep and 28 of 28 in the shipped world — the
   highest rate of the four species. Its roots are a single unbranched
   strand, which for a plant eight rows tall is not the binding constraint.
2. **`branch_priming` is in the root block**, which the lane split assigns
   to lane P — and P4 is the package that rewrites root allocation. A root-
   mechanism edit landing on a world-data branch would collide with the
   package that owns it, for no measured gain.
3. **The hazard is the appearance of a live knob**, not the behaviour. That
   is fixed by saying so where the next reader will be — creeper.ron's
   header, this report, and `open-bugs-handoff.md`.

**Retry condition:** whoever lands P4 should set `branch_chance: [0.0]` and
`branch_priming: [3]` in `creeper.ron` in the same change, and re-measure
creeper's root cell count paired against this branch's.

## 6. B1: the one root axis that already exists, measured

Item B1 asked for a *render*, not a build — establish the heritable root
axis that is reachable today so Arc B has a baseline. It is reachable, it
works, and it is stronger than the record expected.

Genome slot 5 multiplies the root's `upward_weight` by `1 ± variance`, and
`tree.ron`'s root declares `upward_weight: [0.6]` with slot-5 variance 0.4.
So the **entire reachable axis is 0.36 to 0.84** — that is what a slot-5
draw of −1 and +1 buys, and quoting anything wider would be quoting a
morphology no genome in the world can produce. Both treatments were run at
those two values with slot-5 variance zeroed, so a pane shows the treatment
rather than eight coin flips. No root code was written; the two `.ron` edits
were reverted before anything landed.

| | gain 0.36 (draw −1) | gain 0.84 (draw +1) |
|---|---|---|
| median root cell, rows down | 16 | 8 |
| deepest row below surface, mean | 31.8 | 23.4 |
| depth histogram, surface→bedrock | 21 / 21 / 17 / 21 / 17 | 40 / 39 / 14 / 4 / 0 |
| root cells, min/median/max | 149 / 242 / 635 | 120 / 252 / 395 |
| lateral spread, min/median/max | 19 / 27 / 100 | 18 / 34 / 60 |
| root share of plant, mean | 9.2% | 7.2% |

**The sign is the opposite of the obvious reading, and it cost a mislabelled
render to find.** `upward_weight` on a root means what it says: a *high*
value pulls the root up, so the high end of slot 5 is the **shallow**
morphology and the low end is the deep one. The first pair of files went out
named backwards.

### The owner's verdict, and the control it forced

Posted as a blind A/B. The verdict: ***"They look identical. Have you
provided enough soil under the plant to really test differences."***

He was right to ask, and the control changes the conclusion rather than
confirming it. `common::SOIL_DEPTH` is **34** rows, and the low-gain
treatment's *deepest* individual measured exactly **34** — it was standing on
the floor of the scene. Re-run at `soil=100`, which the scene has room for
(`ground_y` 200 in a 320-row world):

| | gain 0.36 @ soil 34 | gain 0.36 @ soil 100 | gain 0.84 @ soil 34 | gain 0.84 @ soil 100 |
|---|---|---|---|---|
| deepest row below surface, mean | 31.8 | 32.0 | 23.4 | 21.6 |
| deepest, max | **34 (the floor)** | 57 | 34 | 32 |
| median root cell, rows down | 16 | 11 | 8 | 8 |
| depth histogram | 21/21/17/21/17 | 74/22/3/0/0 | 40/39/14/4/0 | 93/6/0/0/0 |

Three things fall out, and the second one retracts a claim already posted:

1. **The scene was clamping the deep treatment's outliers.** Max depth 34 is
   the soil floor exactly; given room it reaches 57. `SOIL_DEPTH`'s own
   comment claims it is "deep enough for a real root system to spread
   without hitting rock" — for this treatment that is measurably false.
2. **The depth histogram is normalised to the soil column, so it is not
   comparable across soil depths, and I read it as if it were.** The flat
   `21/21/17/21/17` I posted as "drives vertical strands down to the bedrock
   band" is what a *shallow* profile looks like when the column is only 34
   rows deep. The same treatment reads `74/22/3/0/0` given real depth. The
   claim was an artifact of the scene, and the picture the owner saw was
   telling him the truth.
3. **The axis is real but much smaller than posted.** Mean deepest row 32.0
   against 21.6, median root cell 11 against 8. That is a genuine ~1.5x on
   depth and it is *not* nothing — but it is nowhere near turf-against-
   prairie, and "they look identical" is the correct reading of it.

**So B1's answer is: the reachable slot-5 axis is not enough** — and the
maturity run below sharpens *why* into something more useful than "the
difference is small". Two prior owner verdicts already said tuning-level
root variety does not read; this is the third and fourth, with the scene
error ruled out rather than suspected.

### The presentation mattered as much as the bed

Re-shot a third time on the integrator's relay, at the presentation
`root-morphology-findings.md` lists *first* — **single plants at high zoom**
rather than the N-per-treatment band, in the same 100-row bed. The two do
separate more clearly there than at stand scale: the low-gain plants send
strands down and out, the high-gain plants build a crust near the surface
and stop. Same numbers underneath (32.0 against 21.6 mean deepest row), so
nothing quantitative changed — what changed is that a stand-scale band
averages the shape away and a single plant does not.

That is worth recording as method rather than as a result: the findings
report offers two presentations and they are **not** equivalent for a shape
question. The band was the weaker one, and it was the one this package
reached for twice.

Posted blind. The standing conclusion above (the reachable axis is not
enough) is unchanged until that verdict comes back — it is written from the
numbers, and the numbers did not move.

### The run was as short as the bed was shallow: the axis is a transient

The blind single-plant card came back ***"Similar. Give them more time to
grow to see if they differentiate when the root structure is bigger."***
Every run above was 10,800 frames, and `plant-species-authoring.md` §8 says
in as many words: *scout at 10,000, confirm at 30,000*. All three shots so
far were scouting runs.

Sampled at three ages in the 100-row bed, day-phase aligned:

| frames | low (0.36) deepest row | high (0.84) deepest row | gap | low root cells | high root cells |
|---|---|---|---|---|---|
| 10,800 | 32.0 | 21.6 | 1.48x | 327 | 319 |
| 25,200 | 45.8 | 24.8 | **1.85x** | 578 | 444 |
| 43,200 | 55.2 | 49.8 | **1.11x** | 1,045 | 1,086 |

**The axis peaks at 25,200 and washes out by 43,200.** At maturity the
median root cell sits 16 rows down for the "deep" treatment and **17 for the
"shallow" one** — the shallow arm is fractionally deeper — and the root
counts are level. Depth bias steers the system early and stops mattering
once it is large.

And the renders say why, more clearly than any statistic: **neither mature
system reads as a root system.** Both are dense amorphous masses that fill
whatever soil is available. `plant-species-authoring.md` §5 already names
this shape from the shoot side ("thin whip or big blob"); this is its root
counterpart. A random walk that keeps going fills space, and filled space
has no morphology — which is the same mechanism behind the owner's verdict
on the four species above ground (§9).

**So B1's answer stands, and now for a much better reason than "the
difference is small": the difference is *transient*.** Root form in this
engine is a volume, not a shape. No amount of slot-5 tuning reaches a
morphology, because the mature form is set by how long the walk ran, not by
where it was pointed.

**One caveat, stated rather than buried:** the low arm is censored from
25,200 onward — its deepest individual sits at exactly 100, the bed floor —
so its true depth is understated and the convergence is *partly* a
measurement limit. Settling that needs a bed of 200+ rows, which the 320-row
scene cannot provide; it would need a `height=` knob on `plant_probe` and
`filmstrip`. The blob, however, is not a measurement artifact.

**Three scene and run limits masqueraded as model limits in this one
investigation** — a 34-row bed, a stand-scale presentation, and a
10,800-frame run — and the owner caught two of the three from the picture
alone. That is the reusable lesson, and it is worth more than the root
result: *before concluding a mechanism is weak, check the bed, the
presentation and the clock.*

**Method note for whoever runs the next root comparison:** set `soil=` well
past the deepest root you expect *and* check the max against it, because a
root system resting on the scene floor and one that chose its depth are the
same picture. `plant_probe` takes `soil=N`; `filmstrip scene=grove` does not,
which is why the render and the numbers were taken at different depths.

## 7. The panorama verdict, and a lever that does not pay for itself

The generated-world panorama came back ***"Mostly more of the same"*** — the
four species are sown, established and counted, and crossing the world still
does not read as country changing. A count says a species is present; it
cannot say it is present *somewhere in particular*.

`examples/flora_census -- mix=1` was written to measure that, pooled over
eight worlds. Two readings, because they fail differently: what fraction of
a plant's nearest woody neighbours share its species (read against that
species' own share of the population, which is the no-structure baseline),
and how many consecutive plants of one species you pass.

| species | same-species neighbours | share of population | mean run |
|---|---|---|---|
| conifer | 0.41 | 0.21 | 1.72 |
| creeper | 0.43 | 0.28 | 1.75 |
| shrub | 0.36 | 0.15 | 1.62 |
| tree | 0.59 | 0.36 | 2.46 |

**The niche weights are not decorative** — every species is 1.5x to 2.4x more
likely to sit beside its own kind than chance. But a "belt" averages **1.6 to
2.5 plants**, and a viewport holds ten to twenty. You never cross a boundary;
you stand in a permanent mixture. That is the verdict, in a number.

The obvious lever is to sharpen the weights before normalising them, so a
column's best-suited species takes more of it. **Swept, and rejected**, with
one trap caught on the way: sharpening shrinks every weight (they are all
below 1), which thinned the world 38% and then inflated the very run-length
number being judged — two knobs moving one metric. Separating "how much"
(the unsharpened sum) from "which species" (the sharpened split) fixed that,
and the corrected sweep is in `NICHE_SHARPNESS`'s own doc.

It still does not pay. **At every setting from 2.0 up, `shrub` disappears
from 2 of 16 generated worlds** — the sixteen-seed guard caught what the
pooled eight-world numbers could not. Sharpening takes the rarest species'
marginal columns first, so the cost lands precisely on the species this
package just finished putting into the world, and the gain at settings that
survive is small (shrub's run 1.62 -> 2.04). Left at 1.0, documented, with
the sweep recorded so nobody re-derives it.

## 8. Cost

`examples/ascii`, this branch against its own base `origin/main`
(`a0fa433`), **built and run in the same session on the same machine**, and
run **twice** so the run-to-run spread is visible rather than assumed —
which matters, because on one of the four figures the spread is larger than
the difference being reported.

| `examples/ascii` line | main | branch |
|---|---|---|
| river-cost 8,192×2,560, spring OFF, **mean** | 11.24 ms | 11.02 ms |
| river-cost 8,192×2,560, spring ON, **mean** | 13.10 ms | 12.94 ms |
| river-cost 8,192×2,560, spring OFF, **worst** | 52.98 ms | **62.30 ms** |
| river-cost 512×320, spring OFF, **mean** | 0.95 ms | 1.13 ms |
| world generation, 8,192×2,560 | 5,880 ms | 5,253 ms |
| ant colony, 12,000 frames, mean | 3.18 ms | 2.98 ms |

Three readings, and they are not the same reading:

- **Mean frame cost at the shipped world size is unchanged.** The
  difference is 0.2 ms in the branch's *favour*, and two runs of the
  *same* binaries spread by 0.6 ms — so this is inside the noise floor,
  in both directions. Same for world generation and the structural pass,
  which also swing further between runs than between branches.
- **Mean frame cost on the 512×320 generated scene rises ~0.15 ms/frame
  (~15%)**, consistently across both runs and well outside that scene's
  own ~0.05 ms spread. That is the plant tick, and it is the honest price
  of 50% more plant cells: in a small world the organisms are a large
  share of the awake work, and at shipped size the field solve dominates
  them.
- **Worst frame at the shipped world size rises ~9 ms (53 → 62 ms),
  consistently across both runs.** This is the one number that is a real
  cost rather than noise, and it is worth naming rather than averaging
  away: a worst frame is what a player feels. It is the same cause —
  more organisms means more organism ticks landing together on some
  frames.

**The lever, if that is too much, is one divisor.** Total woody density is
`weight / max(1, Σ weights)`; raising the floor above 1.0 thins every
species proportionally without changing which species goes where. It has
deliberately not been pre-emptively tuned, because the question "is this
world too full" is one the owner answers by eye, and the panorama card asks
it directly.

## 9. The owner's question: could this ever grow a tomato plant?

Asked on the four-species card, and it deserves a real answer rather than a
deferral, because the answer sequences a lot of future work:

> *"Different-ish. The biggest differences are still size and color. That
> really different morphology. I think the issue is in the base design of the
> random walk growth. At some point we should consider flowers and fruits to
> add more variety, but could we ever realistically get to a tomato plant or
> sunflower or climbing vines with our implementation?"*

**The diagnosis is right, and the project's own record already agrees with it
three times over** — this is not a new finding, it is the third independent
confirmation. Sympody, tropism and acrotony all fired, all counted, and moved
nothing anyone could see (`plant-appearance-design.md` §5, review report
§2.1). `plant-species-authoring.md` §1 measures `light_weight` and
`upward_weight` as **inert**. This pass adds a fourth data point from the
other end: colour made all four species disjoint and `leaf_cluster` spread
leaf area 6→14, and the verdict is still "the biggest differences are size
and color".

### What the walk can and cannot express

`Grow` places one cell at a time from a tip, scored by continuation,
light, wind, upward bias, crowding and heading inertia, with `ByOrder`
parameters per branch order. That vocabulary can express **size, branching
density, height budget, leaf area per node, lean, and colour**. It has no way
to express **organ identity** — every cell in the world is stem, leaf, root,
bud or seed. `Reproduce` sits on `MatureBody` and emits a seed directly:
there is no flower and no fruit anywhere in the model, not even invisibly.

That is the gap the three named plants fall into, and they fall into it in
three *different* places:

- **A climbing vine is the closest, and the missing mechanism is already
  named in the codebase.** `creeper.ron`'s own header says it: *"a creeper
  that chases light climbs, and climbing is the one corner of this envelope
  that genuinely needs a mechanism (surface attraction) — out of scope
  here"*. That is thigmotropism, and it is a **scoring term**, not a new
  substrate: prefer candidate cells adjacent to solid non-organism material.
  It fits the existing walk exactly, which is why it is the cheapest of the
  three and the one to try first.
- **A sunflower needs determinacy**, which the engine does not have. Growth
  here is indeterminate — a tip runs until it retires on staleness. A
  sunflower is a single stout unbranched axis that *terminates* in one very
  large capitulum and stops. That needs a terminal organ and a rule that
  converts a tip into it, not a tuning of the walk.
- **A tomato needs fruit as a thing with mass hanging off an attachment** —
  and the trusses are most of what makes it read as a tomato plant.

### Why flowers and fruit are the highest-value move, not a garnish

The owner proposed them as "more variety"; the measured record says they are
considerably more than that. **Colour and composition are the only two levers
this project has ever measured to move a silhouette.** A flower is a small
patch of a wholly new hue placed at a structurally determined position, and a
fruit is a dense mass that hangs — so between them they move both levers at
once, which no architectural lever has managed. They also give `Reproduce`
something visible to be, closing the oddity that a plant currently sets seed
with no reproductive structure at all.

### The honest limit

Leaf **shape** is not reachable and should not be promised. A leaf is a blob
of `leaf_cluster` cells placed by one rule, so the engine can vary leaf
*size* and *colour* and cannot vary a leaf's outline — no compound pinnate
tomato leaf, no palmate vine leaf. Some of the variety a real garden has is
off the map until a leaf is a small authored or grown *shape* rather than a
count.

**Sequencing.** New cell types are `organism.rs`/`plant.rs`, which is lane
P's substrate; their materials and palettes are lane W's data. So this is a
cross-lane item for the integrator to schedule, not something this package
starts on its own initiative. Suggested order, cheapest and most certain
first: **surface attraction (vines) → flowers → fruit → determinacy**.

## 10. What this leaves open

- **Grass** (lane P's P3, then lane W's W3).
- **The `arid` preset sows nothing at all**, by construction: `tree_density`
  and `moss_density` are both 0 there, and its columns carry sand rather
  than soil. Item §X of the review report is where the three desert levers
  are costed; this pass deliberately did not touch it.
- **Species differentiation is still colour and composition.** No
  architectural lever was shown to move a silhouette here either. C3
  (whorls) is the one deferred lever that changes *texture*, and it is
  still deferred.
