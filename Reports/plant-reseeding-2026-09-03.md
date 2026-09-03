# Why the lab's plants do not spread — the reseeding funnel, measured

*2026-09-03. Owner's report: "Plants create lots of seeds, many of them
germinate but rarely do they actually grow into full plants. I think the #1
issue is that they just drop under the plant so most seeds are under shade or
not touching dirt because they are in a big pile of seeds, or they sprout but
immediately next to the plant so they just combine into one plant. Nothing
spreads them."*

*Two questions were asked. **Q1** — can a plant evolve better seed spreading
inside this engine, or does the engine need changing? **Q2** — is dispersal the
only reason plants are not spreading? This report answers both from
measurement, in the lab bed, with `examples/reseed_probe.rs` (new) and
`examples/labshot.rs`.*

**Everything below is `herb`, the lab's default crop and the only shipped plant
whose life cycle evolution can act on. Nothing here is about the outdoor game.**

---

## The short answers

**Q1. No — and it is not a tuning gap, it is a missing channel.** Not one step
of a seed's journey has a heritable dial, and two of the three steps have no
dial at all, heritable or otherwise. Selection cannot act on dispersal because
there is nothing for it to act on. **The engine has to grow the mechanism
first; the genome hook is then cheap and is not a hardcode.** §1.

**Q2. No.** Dispersal is real and it is measured here at roughly a **1.4x**
germination effect and a **1.4x** coverage effect — worth having and not the
headline. Four other things are in front of it, three of them measured for the
first time in this report: the germination gate can only ever open on two
materials in the whole set (§2.1); the grow lamps leave **32-column dead bands**
in which a founder dies without ever setting a seed (§2.2); the colony is a
**seed predator** and cuts the stand by 2.8x (§2.3); and "they combine into one
plant" is `open-bugs-handoff.md` §Z, already owner-judged, and is a *rendering*
finding — two plants never merge in the simulation (§2.4).

**And one new defect fell out of the instrument**: `World::germinations` can
exceed the number of seeds that ever existed — 164 against 79 on one arm — which
points at a fate mutation relabelling a live cell to `Seed` in place. §4.

---

## 0. The instrument, and one number it had to stop lying about

`examples/reseed_probe.rs` follows the funnel stage by stage: seeds borne →
where they came to rest and **which gate is shut on each one** → germinations →
seedlings → established plants → **how much of the bed the stand holds**. It
reports the germination gate's *inputs* (is anything underneath, the ambient
light, the plant-available water in the cell below, and what that cell is made
of) against the species' own thresholds, rather than re-deriving the verdict.

`scatter=1` is its positive control and the reason it exists: every seed the
world creates is set down once, on open ground, at a uniformly random column.
Dispersal limitation is then gone and nothing else has changed.

**`World::seeds_borne` had to be added to the engine, and the reason is the
recurring one.** Two outside-in counts of "how many seeds were made" — the
parents' own `seeds_set`, and watching for organism ids appearing as fresh
single-`Seed` organisms — are both keyed on the organism slot, and
`World::push_organism` re-uses the slot a dead plant has just released. Both
miss the recycled births, **agree with each other while doing so**, and
disagreed with `germinations` by more than 2x. Two independent instruments
sharing one blind spot corroborate each other into a wrong answer; the counter
now sits in `plant::bear_seed_at` beside `World::fruit_dropped`, where both seed
paths meet.

---

## 1. Q1 — what the genome can reach, and what it cannot

A seed's journey has three steps. Here is what controls each.

| step | what decides it | heritable? |
|---|---|---|
| **where the seed appears** | `plant::set_seed` picks a uniformly random **free 8-neighbour** of the bearing `MatureBody` cell. The fruit path (`drop_organ`) does not even do that — a fruit lets go exactly where it hangs | only through crown shape, and see below |
| **the fall** | `Powder`. One cell down per frame, a diagonal step when blocked. No velocity, no momentum. **Wind reaches gases only** — `update_gas`'s bias is, in its own words, "the first thing in this engine that lets the M13 field move *material*", and nothing else displaces a cell | **no** |
| **the roll** | `roll_along_slope`, reach `1/tan(friction_angle)`. `seed` is 55°, so **0.70 cells** — position-jittered to 0 or 1. `windfall` is 26°, so 2.05 | **no** — `friction_angle` is a *material* property |

**So the maximum lateral travel a seed can make under its own rules is one
cell per roll step, on a bed that is flat.** What spreads a pile at all is the
diagonal fall, which builds a 45° cone: reaching 28 columns — half the founder
spacing — needs on the order of a thousand seeds in one pile, against a
`seed_half_life` of 14,000 frames.

**Nothing in the genome touches any of it.** The genome is ten continuous slots
(`organism::GENOTYPE_TRAITS`: shoot/root branch chance, plastochron, turgor,
pipe ratio, root tropism gain, root:shoot bias, stomatal closure, root
penetration, strain response) and six discrete loci (`DISCRETE_LOCI`: leaf
economy, branch angle, internode, sympody, tropism, wood density). **Not one is
about seeds.** `set_seed`/`bear_seed_at` copy `genotype_draws`, `alleles`,
`fates` and `lineage` and copy the **species id unchanged** — so
`windfall_material`, `seed_half_life`, `seed_cost`, `reproductive_allocation`,
`seed_maturity` and both `Germinate` thresholds are fixed per species and are
outside heredity entirely.

**The one indirect lever is crown width, and on `herb` it is off by
construction.** A wider crown rains seeds over a wider footprint, and crown
width *is* heritable — `LOCUS_BRANCH_ANGLE`, `LOCUS_INTERNODE`,
`LOCUS_SYMPODIAL`, `LOCUS_TROPISM` and slot 0 all move it. But every continuous
slot is a **multiplier on an authored species constant** (`plant::genotype`
returns `1 ± variance`), and `herb.ron`'s shoot declares
`branch_chance: [0.0, 0.0]`. **Zero times any genome is zero**, so no mutation
can ever make a herb branch, and slot 0's authored variance of 0.4 is
multiplying nothing. That is the general shape of the answer as much as the
specific one: **the continuous genome can only scale a behaviour the species
file already has; it cannot switch one on from a zero base.**

The `FateGenome`'s `Insert` operator *is* the exception — it can add a rule a
species never had, which is how a `tree` lineage could acquire a flower. But a
fate rule changes a **cell type sequence**. It cannot change a material, a
friction angle, or a position, so it cannot reach dispersal either.

### What this means for the fix

**Not a hardcode, and not a tuning pass.** The missing piece is a mechanism,
and the genome hook is a few lines once the mechanism exists. Three candidate
mechanisms, cheapest first:

1. **A launch offset in `bear_seed_at`.** The seed is placed at `x ± d` rather
   than at a free 8-neighbour, `d` drawn from a heritable distance. One draw,
   no per-frame cost, and a *distribution* rather than a binary — the ethos's
   first law. Against it: a seed appearing eight cells from the plant with
   nothing in between reads as magic rather than as a mechanism.
2. **Wind on light powders.** The field's velocity channel exists, has a
   writer, and already biases one movement rule (`update_gas`'s
   `wind_biased_order`). Extending that bias to powders whose material opts in
   is the smallest change that produces dispersal the player can *watch*, and
   it comes with a verb — the ethos's second law — the moment weather or a fan
   is something the player can turn on. It is also the version that costs
   frame time, and that cost has to be measured before it is promised: it
   keeps chunks awake.
3. **Animal dispersal.** Ants already carry food and `windfall` already is
   food. This is the most satisfying answer and it is furthest away — and note
   §2.3 below, which says that *today* the colony is the opposite of a
   disperser.

Whichever lands, the genome side is the same and is already precedented: append
**slot 10** to `GENOTYPE_TRAITS` (appending is exempt from the never-renumber
rule for the mechanical reason slot 9's own doc gives — each slot's founding
draw is keyed on its own index), give the species files a **non-zero** authored
base so the multiplier has something to scale, and expose the base on the
parameters page, which is where the owner's standing direction puts it.

---

## 2. Q2 — the other four reasons, measured

### 2.1 The germination gate can only ever open on two materials

`Behavior::Germinate` reads `update::plant_available_fraction` on **the cell the
seed is resting on**, and that read is gated on `water_capacity > 0`. Across the
whole material set, **`soil` and `packedsoil` are the only two materials that
declare it.** A seed resting on wood, leaf, litter, deadwood, stone, sand,
another seed or a windfall reads **bone dry however wet the world is**, and can
never germinate, at any moisture, for ever.

Standing-seed census, 8 founders, no ants, frame 36,000 (seed 1) — 332 seeds
standing:

| resting on | count |
|---|---|
| a material with **zero** water capacity | **313** |
| …of which **living or dead plant tissue** | **183** |
| …of which **another seed or a windfall** | 86 |
| soil, but below the 0.15 threshold | 14 |
| soil and wet enough, but too dark | 5 |
| still in the air | 0 |

**The owner's "big pile of seeds" is real and is the smaller half.** The
commonest place a stuck seed sits is **the parent plant itself** — 183 against
86. `seed.ron` does not set `falls_through_organisms`, so a seed borne beside a
stem twenty rows up comes to rest on the first branch under it and stays there.
`windfall.ron` *does* set that flag, on the 2D-slice argument that a branch one
cell wide is not a shelf spanning the wood's whole depth — an argument that
applies more strongly to a smaller, lighter object. **That inconsistency was not
designed; it was never noticed.** §3 measures what closing it buys.

Over a whole run this is where the crop goes: 8 founders, no ants, 45,000
frames, three world seeds — **2,596 / 4,300 / 2,528 seeds borne against 924 /
1,382 / 882 germinations**. Roughly a third germinate; a little over half rot
where they fell.

**This is not a new finding, it is a new face of a known one.** `seedbed_probe`
already measured 16/16 germination on bare soil against **0/16** on a deadwood
mat, and its sharpest result is that a mat of **soil** blocks just as totally,
because fresh ground is created dry — so "turn the debris into soil" is
measurably not a fix. The capillary change that would produce a real moisture
gradient was built, measured and reverted: it drains the bed to the wilting
point by frame 12,000 (`dead-ends.md`, `where-a-dead-plant-goes-2026-08-31.md`).

### 2.2 The grow lamps leave 32-column dead bands, and a plant founded in one dies

Bench light (`plant::ambient_light_above`, noon-equivalent) across the 512-wide
bed at frame 600, every 8 columns:

```
0.00 0.00 0.36 0.36 0.48 0.48 2.40 2.40 1.80 1.80 1.11 1.11 1.65 1.65 2.40 2.40
0.69 0.69 0.69 0.69 2.40 2.40 1.80 1.80 0.95 0.95 1.65 1.65 2.40 2.40 0.69 0.69
0.69 0.69 2.40 2.40 1.80 1.80 1.16 1.16 1.65 1.65 2.40 2.40 0.68 0.68 0.68 0.68
2.40 2.40 1.80 1.80 0.79 0.79 1.65 1.65 2.40 2.40 0.60 0.60 0.42 0.42 0.00 0.00
```

Under a fixture it is **2.40**. Two of the inter-lamp gaps read 1.1–1.8 and are
fine. Two of them are **four consecutive field blocks at 0.69** — a 32-column
band at **29% of the light under a lamp**. The lamps sit at 56-column spacing
and the field resolves light at 16, so the pools alias: some gaps tile and some
do not.

One founder, no ants, 13,500 frames, four world seeds, planted on a lamp column
against planted in a dead band:

| founder column | outcome |
|---|---|
| **60** (under a fixture, light 2.40) | **4 of 4** live and breed: 96–164 seeds borne, 19–39 germinations, 8–14 plants standing |
| **256** (mid dead band, light 0.69) | **4 of 4 dead before frame 4,500, having set exactly zero seeds** |

Not one seed, on any of four genomes. Both columns clear the *germination* bar
of 0.1 easily — what fails is income, which is `rate × light × water`, so a
plant in a dead band earns 29% of one under a lamp and cannot fund its nine
metamers.

**This is not an edge case in play.** The shipped lab opens empty and the player
plants by clicking, so roughly a sixth of the bed kills a plant outright with no
feedback. It is also a trap for anyone measuring here: `LabBox::spread(1)` puts
a single founder at column 256 — **the darkest column in the bed** — so a
one-founder run is not a smaller eight-founder run. `reseed_probe` grew a `col=`
argument for exactly this reason.

### 2.3 The colony is a seed predator, not a disperser

`seed` carries `food_energy: 480` and `food_class: -1.0`, and `ant.ron`'s
explicit `food:` list is gone — diet is `food_class` against `gut_bias` — so a
plant-leaning ant eats seeds. 8 founders, 27,000 frames, two world seeds,
colony against no colony:

| | seeds borne | germinations | plants alive | columns held |
|---|---|---|---|---|
| **no colony** | 1,586 / 2,589 | 473 / 852 | 73 / 90 | 256 / 351 of 512 |
| **one colony** | 680 / 1,128 | 142 / 308 | 26 / 44 | 122 / 178 of 512 |

**2.3x fewer seeds, 3.1x fewer germinations, 2.6x fewer plants, half the
coverage.** The shipped `LabBox::default()` has a colony in it, so every earlier
plant measurement in this bed that used the default was taken with a grazer in
the box.

### 2.4 "They combine into one plant" — they do not, and this is already judged

Two adjacent organisms never merge; nothing in the engine does that. What is
real is that the owner cannot *see* them apart, and that has a verdict already:
`open-bugs-handoff.md` §Z, *"No. Everything has merged together into a big mass.
I cannot identify individual trees."* It is a rendering and architecture
finding, it is open, and its entry records that a contiguous-run metric measures
whether crowns **touch** and cannot measure whether they are
**distinguishable** — believed once and overturned by looking.

### 2.5 …and what dispersal itself is actually worth

The `scatter=1` control, 8 founders, no ants, 27,000 frames, two world seeds:

| | germination rate | plants alive | columns held | plants >15 columns from a founder |
|---|---|---|---|---|
| **baseline** | 29.8% / 32.9% | 73 / 90 | 256 / 351 | 12 / 8 |
| **every seed set down at a random column** | 43.1% / 58.4% | 72 / 125 | 403 / 377 | 14 / 33 |

Germination up about **1.5x**, coverage up about **1.3x**, and the count of
plants that reached ground their parents never stood on up **2.3x**. Real, and
smaller than one would guess from the picture — because the bed saturates
either way, and because the *lit* part of the bed is what saturates.

**The picture is the finding the numbers understate.** Eight founders, no ants,
frame 27,000: 407 organisms, and still **eight discrete clumps sitting under the
eight lamps with dark empty stripes between them** (`labshot founders=8
colonies=0 frames=0,4500,13500,27000`). "Columns held 351 of 512" reads as
coverage and is really eight wide clumps. **§2.2 and dispersal are the same
wall seen from two sides: seeds cannot cross the gap, and there would be
nothing for them if they did.**

---

## 3. One measured candidate, not landed

`seed.ron` gains `falls_through_organisms: true`, matching `windfall.ron`.
One founder at column 60, 13,500 frames, four world seeds, paired, one binary
per arm:

| world seed | seeds borne → germinations, baseline | with the flag |
|---|---|---|
| 1 | 164 → 39 (23.8%) | 154 → 75 (**48.7%**) |
| 2 | 147 → 33 (22.4%) | 138 → 62 (**44.9%**) |
| 4 | 96 → 22 (22.9%) | 98 → 39 (**39.8%**) |
| 3 | 103 → 19 (18.4%) | 79 → **164** — see §4 |

**Germination roughly doubles on three of four seeds and the stand does not
move**: span 40–88 / 45–77 / 46–75 against 43–88 / 44–81 / 46–80, and plants
more than 15 columns from the founder 2/1/0 against 0/1/1. Standing population
is inside the seed spread either way (12/14/11 against 18/11/13).

**That is exactly what the mechanism predicts and it is why this is a
candidate rather than a fix**: the flag moves seeds from stranded in the canopy
to lying at the stem base, which is where the soil is. It buys germinations, not
territory. It is left uncommitted because it is a physics change that reaches
**every forest in the outdoor game** as well, and nothing here has measured that
side.

---

## 4. A new defect the instrument found: germinations can exceed seeds

On one arm (`col=60 seed=3` with §3's flag on), **79 seeds were borne and
`World::germinations` reached 164**. All three independent counts of seed
creation agree on 79, so the excess is real germination events, not a lost
birth.

The likely route, unconfirmed: `becomes: Seed` is resolved by **detaching** only
under a `FateWhen::Ripe` rule, which routes through `drop_organ` →
`bear_seed_at`. Every other `when` falls through the general path
(`plant.rs:3212`, `self_type_after_grow = fate.becomes`), which **relabels the
cell in place on the living parent**. A `FateGenome` `Insert` mutation that
writes `becomes: Seed` on a `Grew` or `Node` rule would therefore turn a live
cell into a `Seed`, which germinates straight back into a `GrowingTip` — a free
germination, on a cell no one paid for, repeatable. `FateGenome::mutate`'s own
doc records that three of its four operators, `Insert` among them, are
**unmeasured**.

Reproduction: `cargo run --release --example reseed_probe -- frames=13500
every=13500 founders=1 col=60 seed=3`, with `falls_through_organisms: true` on
`seed.ron`. Filed as its own entry in `open-bugs-handoff.md`.

---

## 5. What to do next, in the order the evidence supports

1. **Light the whole bench, or show the player where the light is.** §2.2 is
   the largest single effect measured here and the only one that kills a plant
   outright. It is also the one that matches the owner's direction exactly —
   the fix is an overlay and a lamp the player can move, not a tuned constant.
2. **Decide the seed/branch inconsistency in §3**, which is one line and a
   doubled germination rate, once someone has looked at what it does outdoors.
3. **Then dispersal**, from §1's list — and note that until §2.2 is fixed,
   spreading seeds further mostly spreads them into the dark.
4. **File and fix §4**, which is cheap and which currently makes every
   germination-rate measurement in this engine an overcount of unknown size.

The germination gate (§2.1) is deliberately last: it is known, its obvious fix
is a recorded dead end, and the honest repair is the soil water model rather
than the gate.
