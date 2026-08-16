# Tree architecture: the implementation plan

The plan of record for the plant work on `plant-substrate-v2`. Written
after four parallel investigations (`tree-extension-biology.md`,
`tree-procedural-prior-art.md`, `tree-extension-audit.md`,
`tree-diagnosis-review.md`) and supersedes the ordering in
`tree-architecture-research.md` §5 and §7d.

## Why this is one design and not a list of fixes

**Every single-lever change attempted in the session that produced this
plan was wrong, circular, or scene-dependent.** Thickening tuning slid
between whip and blob without a middle; a rate cap converted exponential
growth into linear growth; ratio bounds bounded proportions rather than
size; flat maintenance respiration impoverished rather than shaped; and
child provisioning helped 32% in one scene and hurt 66% in another.

The prior art explains why. In every published model these are **not
separable knobs** — income is recomputed at the base each cycle, divided
competitively among the frontier, and girth is a *derived* consequence.
Take one piece alone and the feedback that bounds it is missing.

So the target is one mechanism, built in stages that each leave the tree in
a measurable state, but understood as a single design.

---

## Phase 0 — make measurement trustworthy (BLOCKING)

Nothing after this is worth measuring until this is done. Five separate
quantities on this branch have measured something other than their name,
and two harnesses that were compared are not the same scene.

**0a. One scene, built once.** `examples/plant_probe.rs` and
`examples/filmstrip.rs` (`grove`) currently differ in tree count (`trees=N`
vs hard-coded 3), spacing (56 vs 140 cells at 8 trees), seed placement
(resting vs dropped 25 rows), and soil depth (30 vs 34). Extract a single
scene builder both call. **This is why the crown-shyness commit was tuned
at one stand density and eyeballed at another.**

**0b. Size the world from evidence — BLOCKED, and the reason is a light
model defect, not a scene parameter.**

Measured, 8 trees / 30,000 frames, varying only ground depth:

| ground | cells | clearance at end |
|---|---|---|
| 200 | 8,529 | **3 rows** |
| 250 | 179 | 196 rows |
| 300 | 62 | 295 rows |
| 400 | 8 (nothing germinated) | 399 rows |

A cliff, not a curve. **No depth is both well-lit and un-ceilinged.**

The cause: `field.rs` seeds light on the topmost chunk's top row and
diffuses downward, so **light gets brighter as a tree climbs** — an
unbounded incentive to grow to the world's top edge, which is where every
scene has ended up pinned. Real sunlight is uniform above the canopy; the
gradient belongs *under* occluders.

**This is the same defect behind the `LIGHT_DECAY` mess** — the constant is
outcome-justified at 25x, but the germination gate is unreachable,
phototropism inverts for ~45% of each day/night cycle, and caves light up.
All four symptoms are one cause: attenuation is a function of *distance*
where it should be a function of *occlusion*.

**The fix is a per-column sky-visibility model** rather than pure
diffusion — a cell with unobstructed sky above gets full light regardless
of depth; attenuation comes from `is_blocked` cells in the column. The
field already tracks blocking for exactly this. That is a real change and
should be scoped on its own.

**Until it lands, every shape conclusion carries a ceiling caveat.** The
harness now prints `canopy top`; a run that reaches row 0 is void.

**0c. Report per-tree distributions, never stand sums.** Per-tree sizes
span **15 to 793 (53x)** with 2–3 of 8 trees failing to establish. Every
number quoted in the problem statement was a stand sum, which averages a
healthy population together with establishment failures.

**0d. Standing metrics.** Replace ad-hoc measures with:
- **lineage birth rate** (branch children per successful grow) and
  **lineage death rate** (tips aged out per tip created), reported
  separately — these are independent of each other and of the outcome,
  unlike `born/(born+seeds)`, which is algebraically tree size and was
  withdrawn as a tautology.
- **wood:leaf ratio** — a tree is a sparse skeleton carrying a mass of
  foliage; ours has run at 48:1.
- **stem thickness above the base**, never `rows >1 cell wide`.
- **canopy top row** — the ceiling detector. If it reaches 0, the run is
  contaminated and the numbers are void.
- **`height` currently conflates canopy and roots** (`max_y − min_y` over
  all cells), which is why it reads 97 in a 96-row sky. Split it.

**0e. Fix the day/night sampling.** Every plant run to date sampled an
arbitrary phase of a 3,600-frame oscillator whose deep-field relaxation
(~3,300 steps) exceeds its period. Either warm up and sample at a fixed
phase, or report the phase alongside every number.

**0f. The light model — now the blocking item for 0b.** Attenuation is a
function of *distance* where it should be a function of *occlusion*. One
cause, four symptoms: no un-ceilinged well-lit scene exists, the
germination gate is unreachable, phototropism inverts for ~45% of each
day/night cycle, and caves light up.

The fix is per-column sky visibility rather than pure diffusion — a cell
with unobstructed sky above gets full light regardless of depth, and
attenuation comes from `is_blocked` cells above it. `field.rs` already
maintains that blocking bitmap for its own purposes, so the data exists.
Diffusion stays for lateral bleed under a canopy, which is what makes
shade soft rather than a hard shadow edge.

Scope it on its own and A/B it on tree outcomes, not on a depth profile —
the depth-profile argument has been wrong twice.

**Acceptance:** two harnesses, one scene; `canopyTop > 0` at end of run;
per-tree distributions printed; birth and death rates printed separately.

**Status (2026-08-15):** 0a done (`examples/common/mod.rs`). **0f done** —
`field.rs::apply_sky` is a per-column downward cast now, so light no longer
brightens with height; that removed the false climb incentive that had
pinned every scene against the world's top edge. **0b done as a
consequence**, plus the turgor height bound in `Behavior::Grow`, which
gives a *derived* ceiling (~120 rows) instead of a scene one: the 200-row
harness ends with 78 rows of clearance. 0d partly done — canopy top, stem
thickness above the base and per-tree distributions all print; birth and
death rates do not. 0c effectively done via the per-tree lists. 0e
outstanding.

**A ceiling the plant carries is still a ceiling.** With the world ceiling
gone, every tree now reaches the *turgor* bound instead (heights 127..131
against a derived ~129) and the crowns fuse laterally along it into a
single slab — the same artifact, from a different bound. See Phase 2's
status.

---

## Phase 1 — branch order (independent, ship first) — **DONE** (`36d2e17`)

Highest value per line in the whole survey, and testable alone. It was.

**Shipped as `organism::ByOrder<T>`** — `Copy`, deserialized from a RON list
of 1..=`BRANCH_ORDERS` values, padded with the *last* one so a short list
means "and so on" rather than "zero the rest". `branch_chance`,
`light_weight`, `upward_weight` and `plastochron` are per-order;
`OrganismCell::order` holds the tier, inherited straight ahead and
incremented sideways. In the sidecar, not `aux` bits 4-15.

**Measured:** the establishment lottery disappeared — smallest tree 39 ->
157 cells, heights 17..143 -> 127..131, stand leaves 521 -> 663. By eye,
eight bare vertical boles with crowns on top. Wood:leaf went the wrong way
(9.7 -> 12.9) because the extra mass arrived as trunk.

**The problem it solves:** 14 tips draw a whip because they are 14 copies
of the same rule. Classical tree L-systems are parameterised by arrays
indexed by **branch order**, and that is where architecture comes from.

**Data:** a `u4` order on each organism cell. `pack_cell_type` uses `aux`
bits 0–3 and the doc records **bits 4–15 as free**, so this costs nothing.
Alternatively an `f32` on `OrganismCell` beside `carbon`.

**Rule:** order is *inherited* on straight continuation and *incremented*
on a lateral branch. Fully local — a tip reads only its own order.

**Species surface (`tree.ron`):** per-order arrays rather than scalars —
`branch_chance`, `upward_weight`, `light_weight`, `plastochron`,
`max_length`. A short array is a shrub; a long one is a tree. This is
`tree-architecture-research.md` §0b's requirement satisfied concretely.

**Acceptance:** trunk, limb and twig are visibly different in the celltype
overlay; the same species file with a 2-element array produces a shrub.

---

## Phase 2 — the bud bank — **DONE mechanically** (`ca3c414`), shape regressed

**Shipped:** `CellType::DormantBud` deposited at every plastochron node,
`Behavior::BudBreak { cost, thickening_survival }`, and `plant::break_buds`
gating on `supportable = floor(intercepted * income / cost)` once per
organism per tick. Thickening kills the buds it covers at
`1 - thickening_survival`.

**Acceptance met:** growth is still happening at frame 30,000 (9 live tips,
273 dormant buds, against 0 and 0 before), and buds track extension rather
than volume.

**And the shape is worse, for a reason worth having found.** Every tree
reaches the turgor bound, buds keep flushing, up is forbidden, so growth
goes sideways: the eight crowns fuse into one slab 136 cells across at
exactly that row, with limbs drooping off it. Nothing bounds lateral
spread — and the mechanism that was supposed to has never run.
`crowding_weight` is 0.5 in `tree.ron` and `canopy_density` measures
**0.000 max** on a grown stand, because decay erases the deposit before
anything reads it. **Fix self-avoidance before judging this phase's shape.**

**The problem it solves:** *nothing ever creates a new tip*. The audit
measured 1,419 tips created and 1,335 that grew, with lineages born at 5.7%
and dying at 5.9% — marginally sub-critical, hence drift to extinction.

**Biology:** the unit of shoot construction is the **metamer** — internode
+ leaf + axillary bud — so **extension manufactures its own future
meristems, one per node**. The reservoir (the *bud bank*) grows with the
shoot system. Buds persist for decades under bark via a vascular trace, and
are killed when the cambium outpaces them.

**Why it is runaway-proof by construction:** budding potential becomes
proportional to *extension already performed*, held in a depleting stock —
never to volume. **Thickening deposits no buds, so a blob generates no new
growth potential.** That asymmetry is exactly what the reverted bud-break
design lacked, and it comes from biology rather than tuning.

**Data:** a `DormantBud` variant on `CellType` — 5 of 16 variants are used,
so it is free.

**Rule:** every `plastochron` cells, a growing tip marks the cell it
vacates as `DormantBud`. A bud leaves that state at most once: flush,
abort, or die.

**Bundled, nearly free:** `SecondaryThicken` kills buds it covers, with a
per-species survival chance. Literal biology, and it **produces a clear
bole for free** — which `tree-architecture-research.md` §0 identifies as
the missing feature.

**Also here:** separate `max_active_tips` (a concurrency throttle, fine at
~14 and measured as *never binding*) from lifetime tip count (should be
unbounded and supply-driven). Conflating them was a stated defect and is
not actually what limits the tree.

**Acceptance:** growth still happening at frame 20,000 with `canopyTop > 0`;
bud count tracks cumulative extension, not cell count.

---

## Phase 3 — the basipetal pass, and girth becomes derived — **part done**

**Done (`19b290f`):** the `thicken()` row-total gate is retired. The
denominator is now the contiguous run of *woody* same-organism cells
through the cell, which is what Shinozaki names. `pipe_ratio` was
re-derived from a sweep (10 -> 45) because the correction changed what the
number means; the sweep table is in `tree.ron` beside the value.

**Not done:** girth as a *derived* quantity — the BFS parent ordering, the
reverse sweep accumulating `Q`, and the monotone high-water mark.
`SecondaryThicken` is still a free-standing process with its own knob.

**The problem it solves:** the blob. `SecondaryThicken` is a free-standing
process with its own knob — **the knob every attempt turned**. In every
published model, girth is a *derived quantity*: diameter is recomputed each
cycle from tips supported, so a plant that stops extending stops thickening
because there is no term that could add width.

**Cost, and why it is affordable:** `organism::transport` already builds,
once per organism per tick, a determinism-sorted cell vector plus a flat
4-neighbour adjacency table. Add one BFS from the base for a rooted parent
ordering (O(N)), then one reverse sweep accumulating `Q` upward. Two extra
linear passes over arrays already materialised. `OrganismState` sets the
precedent — `root_cells`/`shoot_cells` are refreshed in exactly this pass,
with a doc-comment sanctioning whole-plant quantities.

Note a 2D thickened trunk is a *blob* of cells, not a tree graph, so the
BFS yields a spanning tree; the existing sort makes it deterministic.

**Yields, from one sweep:** `Q_base` (total intercepted light), pipe-model
girth, and branch size for upkeep.

**Girth is a monotone high-water mark.** Palubicki is explicit: *"branch
width is not decreased when leaves and branches are shed… the model
requires a memory of past leaves and branches."* One per-cell scalar under
a max-accumulate.

**This also retires the `thicken()` row-total gate**, which the review
showed over-reads on branched rows: **53% of occupied rows contain more
than one separate run**, so a limb elsewhere on the same row silently
suppresses trunk thickening. Leaves are also counted as stem cross-section,
and the same cell appears on both sides of the ratio. Do not fix that gate
— replace it.

**Acceptance:** wood:leaf falls from 48:1 toward single digits; a tree that
stops extending stops gaining wood; the 53%-of-rows failure case is gone by
construction.

---

## Phase 4 — the acropetal pass: allocation, λ, and `n = ⌊v⌋` — **part done**

**Done (`ce86950`), out of order and deliberately:** frontier cells receive
a *share of income* rather than whatever diffusion delivered, and income is
intercepted light rather than leaf count. This was pulled forward because
tips were starving beside carbon-rich neighbours and nothing downstream
could be judged until they stopped. `break_buds`'s `supportable` is
`n = ⌊v⌋` in miniature.

**Not done:** `λ` (the basipetal/acropetal split between a shoot and its
laterals), which is the excurrent/decurrent control.

The sustain-and-bound mechanism, and the excurrent/decurrent control.

**The headline both research passes reached independently:**

> **A bud receives an allocation, not a reserve.**

Bud break failed because it asked a *local* question and every mature cell
answered yes at once. **No model in the literature ever asks that
question** — they ask what *share* of the plant's income a bud got, and
shares cannot saturate everywhere, because they sum to one.

**The split at a branch point:**

```
v_m = v · λQ_m / (λQ_m + (1−λ)Q_l)
v_l = v · (1−λ)Q_l / (λQ_m + (1−λ)Q_l)
```

`λ > 0.5` biases the main axis (**excurrent**, spruce); `λ < 0.5` biases
the lateral (**decurrent**, oak). Palubicki sweeps 0.46–0.54 and that ±0.04
band spans the entire range, because the bias compounds multiplicatively at
every branch point.

**This is the correct form of `β_x`.** Biasing *conductance* changes which
face gets more, but nothing enforces that the two shares sum to the
parent's supply — so it cannot compound. It must be a multiplicative split
of a conserved flow.

**The terminator, and the single most important line in the survey:**

```
n = ⌊v⌋        metamers this bud produces
l = v / n      length of each internode
```

A bud allocated `v = 0.9` produces **zero** metamers, does not grow, and
**does not die, is not retired, and costs nothing**. Next cycle its share
is recomputed; if a competitor is shed or shaded out, its share rises above
1 and it **resumes**. Soft, reversible, competitive dormancy.

**This is the answer to "bounded without an arbitrary cap".** It bounds
growth without ever killing anything: adding tips divides the same income
more ways, so the system self-throttles. And `m > 1` is not the goal — the
goal is `m` held *at* 1 by a feedback that responds to the plant's state.

**It also directly deletes the mechanism the biology falsifies.** Bond et
al. grafted old-growth Douglas-fir tips onto seedling rootstock and got a
**10-fold elongation increase in two years** — size, not age. There is no
meristem senescence in trees, and `ORGANISM_STALE_LIMIT` retiring a tip
permanently implements exactly the disproven thing.

**Cheapest fallback** if the acropetal pass proves too costly: keep the
basipetal half and broadcast `v_i = v_total · Q_i / Q_base` per organism.
This loses λ (and so the excurrent/decurrent axis) but keeps the property
that matters — a competitive share that cannot saturate, and `⌊v⌋`
dormancy.

**Acceptance, and these are the two the existing metrics will not catch:**
- **The distribution of `v` across buds**, not the total. The failure mode
  is `Q_base` growing in proportion to bud count so every share stays above
  1 forever — degenerating back into unbounded extension. A healthy plant
  shows most buds below 1.0 and a few above.
- **Dormancy is reversible.** Cut a limb at frame 10,000 and confirm
  neighbouring dormant buds restart. If they do not, the mechanism has
  become `ORGANISM_STALE_LIMIT` with extra steps — and per `CLAUDE.md`'s
  ethos note, that visible resumption after damage *is* the payoff for
  choosing this over a cap.

---

## Deferred, with reasons

- **Self-pruning / maintenance respiration.** Still wanted for crown
  recession and the clear bole, but **two corrections apply**: a *flat*
  per-cell upkeep bounds nothing (cost linear in mass against income linear
  in leaf count balances at any size — Takenaka's exponent is **1.5**), and
  combining a graded shed rule with a *binary* space signal makes branches
  vanish the instant they stop extending. Cheapest superlinearity: charge
  upkeep proportional to girth, which Phase 3 already computes.
- **Apical dominance / the auxin channel.** Scoped in
  `plant-substrate-v2-design.md` §7i as a second `[f32; 4]` running the
  identical rule in reverse polarity. **If Phase 4 lands, the release
  signal is just `v ≥ 1` and this is redundant.** Do not build both.
- **Light-as-attractor-envelope.** Space colonization's envelope is what
  produces trunk-vs-blob — *"narrower trees have a clearly delineated
  trunk, whereas in widely spread trees even the main limbs are highly
  ramified"* — and we have no envelope. **Trap:** `canopy_density` is
  deposit–diffuse–decay, while an attractor field must be a *consumable
  stock with no decay*; reusing it would silently delete the bounding.
- **Child provisioning.** The conservation hole is real — a child is born
  holding zero while the parent's `cost` vanishes, and that was 52.8% of
  starvation refusals. Reverted because it was +32% in one scene and −66%
  in another. Revisit once Phase 4 makes allocation explicit, since it may
  be subsumed.
- **`LIGHT_DECAY` side effects.** The value is outcome-justified (25x) but
  the germination gate is unreachable, phototropism inverts for ~45% of
  each day/night cycle, and caves light up. The likely fix is to decouple
  *reach* from *amplitude* rather than move the constant.
- **`canopy_density` reads max 0.000** at end of run — decay erases it
  before anything reads it, so `crowding_weight` is inert and the crown
  shyness change landed on a dead channel. Fix the decay cadence or the
  read, then re-evaluate crown shyness.

---

## Standing discipline

- **Judge by the picture**, on the unified scene, with `canopyTop > 0`.
- **Never re-tune against `forest` or the default `plant_probe` ground** —
  both are 40-row scenes with a ceiling.
- **Five quantities on this branch have measured something other than their
  name.** Before trusting a new metric, ask what it reads when nothing is
  wrong, and check it is not algebraically the outcome.
- **A fix that trades one artifact for another needs a test that catches
  the trade** — the `thicken()` row-total gate did not have one.
- **Verify both drivers.** `update::step` is serial; `parallel::step` is
  what the app runs. A cell-list bug hid for a session because the guard
  test only ran the serial one.
