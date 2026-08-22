# Why three species look like one plant, and what actually sets a silhouette

Written after the owner's playtest reading of the Phase 2 sheets: *"variation
is still minimal — the shrub looks like a small version of the same tree, the
conifer a more narrow version of the same thing."*

This report is the **diagnosis and the appearance work that follows from it**.
`tree-architecture-variety-review.md` is the botany and the lever taxonomy;
`plant-night-session-handoff.md` is what was built against it. This is why
what was built did not move what the owner sees, and what does.

## 1. The finding, stated first

**The three species differ in height and mass and in essentially nothing
else.** Read off the megastudy logs (`target/megastudy/*-seed11.log`, 16
plants each, 45,000 frames):

| | tree | shrub | conifer |
|---|---|---|---|
| mean cells | 4,729 | 2,838 | 5,929 |
| mean height | 145 | 70 | 175 |
| **leaves as % of cells** | **4.8%** | **6.2%** | **3.2%** |
| rows >1 cell wide | 66% | 61% | 64% |

Every discriminating number is a magnitude. The *composition* is invariant:
each species is ~90% `MatureBody`, ~5% `Leaf`, and about two thirds of its
rows are wider than one cell. That is the numerical statement of the owner's
reading, and it is correct.

By eye (`target/filmstrips/p2-*.png`), all three render as the same object: a
dense mass of one-to-three-cell wavy brown strands with a thin green crust on
its lit upper surface. Tree and conifer are that texture at different
heights; shrub is that texture at hedge height.

## 2. Four causes, ranked by silhouette cost

### 2.1 Foliage is 5% of the plant

A tree's silhouette *is* its crown. Here 90% of every plant is wood and the
foliage is a one-cell skin on the lit boundary, so the rendered outline is
set by brown twig in all three species. **Six of sixteen trees in
`tree-seed11` carry zero leaves** and are still standing as wood.

No architectural lever can reach this. It is also a trade that was made
deliberately and for good reasons: `shade_death: 0.03` was a genuine interior
optimum on its own sweep (most mass *and* best crown separation *and* a lit
canopy) and it cut foliage from 11,179 to 2,336 cells in that same sweep. It
bought bole legibility with crown volume. The bole is legible now; the crown
is gone.

### 2.2 Every plant in the world is the same two palettes

`germinate` hardcoded `id_of("wood")`, and there was one `wood.ron` (four
browns) and one `leaf.ron` (four greens). No per-species colour, no
per-individual colour, no variation of any kind between a fir and a bush.

In a pixel game palette *is* species identity. This cause is entirely
independent of the growth model and was the cheapest unexploited lever in the
subsystem — it is what §3 below fixes.

### 2.3 Branch angle is unmodelled

`plant.rs`'s branch block picks the lateral as
`alt[rng.below(alt.len())]` — a **uniform** draw over the leftover
positively-scored neighbours. Branching *rate* is a per-order species
parameter; branching *angle* is noise.

Branch angle and its variance are top-tier silhouette parameters in every
prior art for procedural trees (L-systems, Weber–Penn, space colonization).
Here there is no parameter at all, and `upward_weight` plus `heading_inertia`
then re-score that lateral every subsequent step and bend it back toward the
tier's reference direction — which is how a crown of laterals comes out as a
bundle of near-parallel ropes.

### 2.4 There is one shape primitive

Every species is a biased random walk on an 8-neighbour lattice, scored by
four dot products (continuation, light, wind, tropic reference) with
per-order weights. Coefficients change the *statistics* of a meander; they
cannot change what a meander *is*. Nothing in the model represents a branch
as an object with a length, a plane, or a straightness budget — an internode.

`ByOrder` is the right instinct. It varies the coefficients of one walk, so
the reachable space is "meanders, more or less upright, more or less forky."

## 3. What this change does: colour as species and as individual

**Colour is not physics, so a needle is not a new material.** `leaf.ron`'s own
doc states this engine's test for when a material is warranted — "its
*physics* genuinely differ on numbers that already exist" — and a fir and an
oak differ on none of them. What differs is hue. So hue is what varies.

`Cell::shade` is a full byte, wrapped modulo the palette length at render
time. That means a longer palette needs **no engine change at all**: band `b`
is simply palette entries `4b..4b+4`. The mechanism is therefore free of both
per-cell state and render work — the byte the cell already carried for grain
now carries identity as well.

- `leaf.ron` becomes **six hue bands of four tonal steps**, ordered along one
  axis (blue-green → green → yellow-green → olive) so a species' range is a
  contiguous slice and widening it never jumps a hue.
- `wood.ron` becomes **four bark bands**. Shorter on purpose: bark carries
  less of the silhouette than foliage, and it should separate species at the
  trunk without pulling attention off the crown.
- A species declares `foliage_bands` and `bark_bands` as `(first, count)`.
- **An individual draws one band from that range at germination**, keyed on
  `(world seed, germination coordinate)` — the same key the genotype uses,
  for the same reason: colour should be a property of the plant, not of the
  world's planting order.

Band 2 of `leaf` and band 0 of `wood` are the original palettes unchanged,
and `count: 0` means "undeclared" and restores the uniform draw over the whole
palette. So moss and any asset set predating bands keep their exact look.

Species ranges are **disjoint**, so no two species can ever draw the same
colour however their individuals fall:

| species | foliage | bark |
|---|---|---|
| tree | bands 2–3, mid → yellow-green | 0–1, mid → red-brown |
| conifer | bands 0–1, deep → mid blue-green | 1–2, red-brown → near-black |
| shrub | bands 4–5, olive → dark olive | 2–3, near-black → grey-brown |

### The property that makes this verifiable

`banded_shade` consumes **exactly one `rng.below(4)` per cell**, which is what
the code it replaced consumed (both palettes were four long). The
per-individual band is drawn from independent `rng::stream`s, which do not
touch the shared sequence. So the change is a pure recolour and **must not
move a single cell**: per-tree sizes, heights and thicknesses have to come
back bit-identical against the baseline. Anything else means the RNG was
perturbed and the recolour has smuggled in a geometry confound.

## 4. What the probe could not see, and now can

Every number `plant_probe` produced was a magnitude — cells, leaves, height,
thickness. **A study cannot answer a question it does not measure**, and the
genetic-variability megastudy was built without a single shape descriptor, so
it can regress traits onto *size* beautifully and cannot address the
complaint that prompted it.

Three descriptors added, the first two scale-free by construction (ratios
taken within one individual, so none can be satisfied by growing the plant
bigger):

- **crown profile** — foliage width in five height bands, top first, each as
  a percentage of that plant's widest band. A fir is wide at the bottom
  (descending), a bare-boled broadleaf is top-heavy (ascending), a shrub is
  flat.
- **foliage centre** — mean leaf height as a fraction of the plant's own
  vertical span, 0 at the collar and 1 at the apex.
- **foliage share** — leaves as a percentage of cells. Not a shape number,
  but the one that governs whether the silhouette is set by foliage or by
  twig.

And a **palette-band counter**, because a picture cannot answer "did this
fire": a banded stand and an unbanded one differ by a few colour bytes per
cell, which is invisible at the zoom a contact sheet is read at. If a species
declares two bands and the counter prints one, the draw is not reaching the
cells.

## 4a. The megastudy ran a stale binary, and is three runs repeated eight times

Found while extracting its results. **All eight world seeds produced
byte-identical logs within each species** — same md5, same genotype table,
same outcomes:

```
898af2515ef7379471d5f88fe2cec821 *tree-seed11.log
898af2515ef7379471d5f88fe2cec821 *tree-seed88.log      (all eight identical)
```

The cause is not in the model. `examples/plant_probe.rs` was last written at
02:54; `target/release/examples/plant_probe.exe` was built at 02:40. The
binary the study ran **never had the `worldseed=` argument**, and an unknown
argument is silently ignored. Verified both ways: the stale binary gives
identical genotype draws for seeds 11 and 999, a fresh build of the same
source gives different ones.

So the study is **3 populations, not 24** — 16 individuals per species rather
than 128. What survives is the cross-species comparison and the within-16
trait→outcome relationship, both at one world each. What does not survive is
anything the replication was for: the spread of outcomes across worlds,
establishment-failure rates as a population statistic, and the question the
conifer runs were included to answer (does the lean side vary by seed?).

This is `CLAUDE.md`'s existing "editing a `.ron` does nothing until the next
build — identical output across settings is the tell" rule, one level up: the
**harness is as stale-able as the assets it reads**, and a detached multi-hour
run is exactly where nobody is watching for it. The defence added here is not
discipline but a line of output — `plant_probe` now echoes
`species/trees/frames/worldseed/width` as its first line, so a log that does
not name its own seed was written by a binary that never had one.

## 5. The method lesson this phase paid for

Phase 2 built three architectural levers — sympody, tropism, acrotony — each
ranked "very high" or "high" on silhouette by the variety review. All three
**fired**: the counters record 46–186 sympodial forks per shrub and
1,797–2,750 plagiotropic steps per conifer. The conifer sheet is unchanged in
character and the shrub reads different mostly because `turgor_source: 0.4`
makes it short.

This is the third time in this repo a mechanism has fired and produced no
visible change, and it has a statable form:

> **A lever that changes which cell gets a label cannot change a silhouette
> that is set by texture and colour.** Before building the next one, ask
> which *pixels* change.

It is a sibling of the existing rule "check that a planned step can
demonstrate itself" — that one asks *which object does this rule evaluate*;
this one asks *which pixels does this rule move*.

## 5a. Foliage volume: `leaf_cluster`, and what it actually costs

**`leaf_cluster` is a pure appearance knob now, and that is a dividend of the
node currency.** Income (`intercepted / l_node`), the bud-break gate and the
pipe-model ratio (`q_peak / l_node`) are *all* normalised by `leaf_cluster`,
so a bigger cluster is more leaf cells at the same economy. Before the node
pass it quintupled income and fused the stand; the normalisation was done to
end the constant treadmill and this is a second payoff from it.

It is not free, and the cost is not economic but **optical**: every leaf cell
deposits canopy density, so a denser crown shades its own interior harder.

Three-point sweep, 8 trees / 30,000 frames, `tree` only, everything else
held, rebuilt between points:

| | cluster 5 | cluster 7 | cluster 10 |
|---|---|---|---|
| **foliage share** | 7% | 9% | **11%** |
| `Leaf` cells | 2,617 | — | 3,184 |
| `MatureBody` cells | 29,835 | — | 22,469 |
| wood : leaf | 11.4 : 1 | — | **7.1 : 1** |
| **thickest fused run** | 95 | 54 | **41** |
| mean cells | 4,263 | 4,175 | 3,414 |
| smallest individual | 3,136 | **202** | 670 |
| crown profile | [100, 77, 36, 0, 0] | — | [100, 80, 41, 0, 0] |

**Two quantities are monotone and one is noise, and the sweep is what shows
which.** Foliage share (7 → 9 → 11) and crown fusion (95 → 54 → 41) move
cleanly with the knob. The smallest individual does not: 3,136 → **202** →
670, worse at the middle setting than at the extreme.

That third row was very nearly written up as "10 sits on a cliff where an
individual collapses". It is not a cliff, it is `CLAUDE.md`'s own warning
about spread — outcomes here span 31 to 153 cells from one genome, and a
min-of-8 is a sample from the tail of a wide distribution. **Adding the
middle point is the only reason this was caught**, and the lesson is the
existing one sharpened: a monotone trend across three points is evidence, an
order statistic over eight individuals at one setting is not.

So the setting is chosen on the two monotone quantities and on the sheet:

- Foliage share up 57% relative, wood:leaf from 11.4:1 to 7.1:1 — the
  composition problem of §2.1 is the thing that moved.
- **Crown fusion more than halves** (95 → 41). Unlooked-for, and it is the
  metric the previous session fought hardest for: a denser crown self-shades
  harder, so graded abscission thins the interior and neighbouring crowns
  separate. `target/crown/foliage-tree.png` against `band-tree.png` is the
  whole argument — the stand goes from one continuous hedge to a row of
  separable trees with visible boles and sky between them.
- Mean stand mass falls 20%, which is the honest price.

**What is genuinely unsettled is the establishment rate.** Leafless plants
were already a known problem (5/16 `tree`, 4/16 `conifer` in the megastudy)
and denser crowns can only push on it, but no n=8 single-world run can
measure it — that needs the multi-seed study, which is now possible for the
first time (§4a). Flagged rather than claimed either way.

## 6. What is not done here
- **§2.3 branch angle** and **§2.4 internode length** — the model refinements.
  A scored departure angle with a per-order reference and variance, and a
  straightness budget before a lateral may re-score, are the two parameters
  that would let the walk produce something other than a meander.
- **Rhythmic growth / whorls** — variety review item #10, deferred because it
  was the only lever with a nonzero frame cost. It is also the only one of
  the set that changes *texture* rather than statistics, and it is what makes
  a conifer read as tiers.
- There is no `wiki/plants.md`. Every other subsystem has a page describing
  what it looks like when it is right; plants do not, and the appearance work
  is exactly the kind this repo's own convention says should have one.

## 7. The end state: colour as a readout, and heritable

Stated by the owner as a later goal, recorded here and in `PLAN.md`'s settled
decisions so the intermediate steps do not build away from it.

**Today's colour is authored data.** A species declares a band range; an
individual draws a band inside it. That is the right *first* move — it is
what separates three species that were previously identical, and it costs
nothing — but it says nothing true about the plant. The end state is that
appearance is **derived from physical state**: foliage hue from nitrogen and
chlorophyll status and light history, autumn colour from the temperature
channel (which oscillates and would need `noon_equivalent_light`'s treatment
— see `CLAUDE.md`), bark from age and accumulated thickening, pallor from
drought. A sick plant should look sick because it *is* sick, not because a
rule paints it.

Two blockers are worth stating now rather than discovering later.

**1. The current draw is positional, so it cannot evolve.** The band is keyed
on `(world seed, germination coordinate)` — deliberately, matching the
genotype, so a plant's colour survives planting order and save/load. But that
key is the *place*, not the *parent*. Offspring of a dark-leaved individual
draw whatever their landing spot dictates, so selection has nothing to act
on. Colour has to move onto the heritable genome before "do they change with
evolution" is even a question the engine can answer.

Because genotype slots are positional forever (renumbering one rewrites every
genome ever measured), that widening should ride along with the root-trait
widening already queued for slots 6+ in the handoff, not happen as a second
pass.

**2. A derived colour must stay legible.** This is `CLAUDE.md`'s
debug-overlay lesson in a new costume. A hue computed as a continuous
function of four physical channels converges on the same muddy average across
a whole stand — every plant sits near the population mean on most channels,
so the differences that survive are a few colour bytes and read as no
variation at all, which is the exact complaint this work started from.

The band structure is the defence: a physical derivation should **select a
band and modulate within it**, keeping the coarse, legible separation while
the fine variation carries the physiology. Bands are already four tonal steps
wide with room to grow, and `Cell::shade` is a full byte, so there is space
for this without another engine change.

Not scheduled. It wants the light and temperature economy and a real
heritable genome under it first — and, on the evidence of §5, it wants the
foliage to be a large enough fraction of the plant that a hue change is
visible at all.
