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

The difference is visible, not merely numerical: at gain 0.84 the roots run
as a horizontal mat in the top few rows, and at 0.36 they drive vertical
strands down to the bedrock band. That is the turf-against-prairie
distinction `root-morphology-findings.md` named as the near-term win, and it
needed no mechanism at all — only the comparison run on the presentation the
findings report specified (single plants at zoom, never stand medians).

Posted as a blind A/B; the verdict is the thing that decides whether Arc B's
thickening and dominance work is still needed before root form is worth
showing again.

## 7. Cost

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

## 8. What this leaves open

- **Grass** (lane P's P3, then lane W's W3).
- **The `arid` preset sows nothing at all**, by construction: `tree_density`
  and `moss_density` are both 0 there, and its columns carry sand rather
  than soil. Item §X of the review report is where the three desert levers
  are costed; this pass deliberately did not touch it.
- **Species differentiation is still colour and composition.** No
  architectural lever was shown to move a silhouette here either. C3
  (whorls) is the one deferred lever that changes *texture*, and it is
  still deferred.
