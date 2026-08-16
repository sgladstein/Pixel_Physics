# Second review: verifying the variety review, and what it missed

An independent second pass over `plant-substrate-v2` at `29e8984`, requested
against the same brief the untracked
`Reports/tree-architecture-variety-review.md` answers. That review is good,
and duplicating it would waste everyone's time — so this pass does the thing
a second reviewer is actually for: **verify its claims against the code and
the harness, correct the ones that are wrong, and look where it did not.**

Nothing on the "do not re-litigate" list is reopened here either.
Everything below is marked **measured** (run in this worktree this session)
or **read** (verified against source at `29e8984`) or **argued**.

---

## 0. Verdict

The branch is in good shape: 390/390 lib tests pass, clippy is clean, and
the headline numbers in the implementation plan reproduce exactly —
including the paired noon/night probe (71 live `GrowingTip`s at frame
28,800 against 28 at 30,000 on a *smaller* stand), which is the single most
consequential measurement on the branch, because it shows the day/night
oscillator, not growth, sets the size of the live frontier.

The prior review's architecture analysis is verified nearly wholesale — its
central discovery (the engine is already physically sympodial; the discrete
Hallé-Oldeman-Tomlinson axes are a few lines each because `Grow` already
retires its apex and already carries `heading`, `lineage_step`, `collar_y`
and a per-order table) is **confirmed line-by-line in source** and remains
the most valuable observation either review produced. But it has one
proposed fix that would reintroduce the disease it treats (§2.1 below), one
ordering that would transiently bring the canopy slab back (§2.2), two
quantitative claims that are wrong (§2.3, §2.4), and it missed a structural
cause of the `light_weight` inertness that a scene change cannot fix
(§3.1).

---

## 1. Claims verified

Each of these was checked against source or re-run, not taken on faith.

| claim (prior review §) | verdict | evidence |
|---|---|---|
| `occupancy` is orientation-blind; a 1-cell horizontal plate transmits 0.90 where a solid block transmits 0.20 | **confirmed** (read) | `field.rs:941-1006` counts cells filled over the 8×8 block; `apply_sky` (`:816`) attenuates by that fraction |
| `occupancy` has exactly one reader | **confirmed** (read) | grep: `occupancy_local` is called only at `field.rs:816` |
| Heat, pressure, moisture consumers unaffected | **confirmed** (read) | `step_pressure:1038`, `step_velocity:1081`, `step_diffusion:1165` gate on `blocked`, unchanged; moisture reads `moisture_source` |
| Powders/liquids transparent → buried seed passes the light gate at midnight | **confirmed** (read) | soil is `kind: Powder`; occupancy counts `Solid \| Plant` only; `NIGHT_LIGHT_FLOOR 0.2 ≥ light_threshold 0.1`. And `moisture_threshold: 0.0` means the *other* gate is degenerate too — germination is effectively unconditional on any resting surface |
| `ambient_light_above`'s one-block offset is a stale workaround post-occupancy | **confirmed** (read) | `plant.rs:505-524` doc still justifies the offset by the "own block reads 0.0" rule the occupancy change deliberately removed. Both guard tests (`plant.rs:3178`, `:3194`) still pass with the offset gone — they assert open-sky behaviour, which now holds either way |
| The crowding cliff is subtract-then-filter + stale-retirement, not ecology | **confirmed** (read) | `plant.rs:952-962` (`score > 0.0` filter, empty set → `continue` → `stale_ticks` → retire at 4) |
| The apex already dies every growth step — sympody is a labelling change | **confirmed** (read) | `plant.rs:1077-1083` retires parent; `:1110` child inherits order, `:1180` lateral takes `order+1`, `:1188` fresh heading |
| Every shoot's tropism reference is hardcoded `(0.0, -1.0)` | **confirmed** (read) | `plant.rs:929-936` |
| `collar_y`, `lineage_step`, bud bank, `q_peak` all exist where §3's mechanisms need them | **confirmed** (read) | `organism.rs:667`, `plant.rs:1007`, `:1790-1833`, `:1735-1741` |
| `q_now` is computed and discarded every tick | **confirmed** (read) | `accumulate_support` builds `q`, max-folds into `q_peak`, drops the vector; `break_buds` runs next in the same `step_organisms` call |
| `organism_id` is monotonic in planting order; generation wraps at 4 bits; no world serialization exists | **confirmed** (read) | `world.rs:486-503` (`free_organism_slots` never populated — no `free_organism` exists, `world.rs:540`); `encode_organism_id` 12+4 bits; grep for `Serialize`/save paths finds only screenshots and tunables |
| Headline numbers reproduce | **measured** | `plant_probe trees=8 frames=30000`: mean 2,541 cells, 1,064 leaves, median stem 14.5, thickest run 51, canopy top 69, 547 buds / 28 tips. `frames=28800`: 19,237 cells, **71 tips**, 530 buds |
| Tests/clippy | **measured** | 390 passed / 0 failed / 1 ignored (137 s); `clippy --all-targets -D warnings` clean |
| `tree.ron`'s `pipe_ratio` sweep table documents a different quantity than the live value | **confirmed** (read + history) | the full history is worse than the review knew: 10 (row-total) → 45 (per-stem, leaf count) → **22** (q in summed light, sweep in `1874f3c`) → **110** (leaf_cluster ×5, `372a97a`). The in-file table still marks 45 "<-- here"; the sweep that actually justifies the live value exists only in commit messages |

## 2. Corrections — where the prior review is wrong

### 2.1 Its occupancy fix overshoots and reintroduces binary shade, rotated 90°

The diagnosis (orientation-blindness) is right. The proposed fix — count
*columns hit* instead of cells filled — is wrong in exactly the way the
original `blocked` flag was wrong: it makes **any 1-cell-deep structure
fully opaque**. A thin horizontal plate, a diagonal twig, or a sparse spray
of single leaves scattered one per column would all transmit 0.2, same as
solid rock. That is the "one twig makes a whole block opaque" disease the
occupancy channel was built to cure, restored in the horizontal direction —
and per the repo's own rule, pairing graded rules with a binary light
signal is a recorded failure mode.

The consistent fix is **per-column Beer-Lambert with the block-stack
convention already in force**. `SKY_TRANSMISSION = 0.2` says 8 cells of
depth transmit 0.2, so one cell of depth transmits `0.2^(1/8) ≈ 0.818`.
Count depth per CA column in the scan that already runs, look up
`T[d] = 0.2^(d/8)` in a 9-entry table, store the mean over the 8 columns
(one u8, as now — the field just becomes "transmission" instead of
"fullness"):

| block contents | current | review's cols-hit | per-column Beer-Lambert |
|---|---|---|---|
| vertical 8-cell trunk | 0.90 | 0.90 | 0.90 |
| horizontal 1-cell plate | 0.90 | **0.20** | **0.818** |
| 1-cell diagonal twig | 0.90 | **0.20** | 0.818 |
| 5-cell leaf cluster (3 columns) | 0.94 | 0.70 | 0.89 |
| solid rock | 0.20 | 0.20 | 0.20 |

A `leaf_cluster: 5` canopy two or three clusters deep then attenuates to
~0.4–0.6 rather than ~0.85, which is the teeth self-shading needs — without
any 1-cell-thick structure casting rock-grade shadow. Same scan, 8 u8
counters and 8 table lookups per block, no new state. The prediction to
A/B on tree outcomes stands as the review framed it, with this formula in
place of theirs.

### 2.2 Its recommended order lands multiplicative crowding before the light fix — that gap is where the slab comes back

The review's §4 order is: multiplicative crowding (#2) → column-mask
occupancy (#7) → offset removal (#8). But under subtract-then-filter,
crowding is today the **only mechanism that stops lateral spread** — it
works precisely by dead-ending tips in the shyness zone until they retire.
The multiplicative form deliberately never dead-ends anything; a fully
crowded tip takes the least-bad direction and keeps building. Land it while
a horizontal canopy plate still transmits 0.9 and income still grows with
lit width, and nothing bounds the slab again until #7/#8 arrive.

Order it: **7 → 8 (light tells the truth) → 2 (crowding becomes a
preference) → 9 (normalise; re-derive `LEAF_INCOME_PER_TICK`, `pipe_ratio`,
`crowding_weight` once, in normalised units)** → then the silhouette levers
1/3/4. This also folds both income-moving changes into a single constant
re-derivation instead of two.

### 2.3 "pipe_ratio is ~20x its daily-mean value" — no, ~2.8x

`q_peak` is noon-latched (organism ticks sample every 45 frames, so the max
lands within 1.3% of the peak) — that part is right. But the daily mean of
`sky_light_amplitude` is `0.2 + 3.8/π ≈ 1.41` against a noon 4.0: the
noon-latch inflates `q_peak` by **~2.8x** over a daily-mean convention, not
20x. 20:1 is the night:noon range, which is the relevant number for the
*bud-break gate* (instantaneous) but not for a *latched maximum*. The
direction of the argument survives; the magnitude was wrong.

### 2.4 Its §1e dusk-lag claim is derived from a stale doc — the real constant is 30x smaller than documented

The review repeated the "deep field relaxes over ~3,300 steps" figure. That
figure belongs to `LIGHT_DECAY = 0.9997`, and **the constant in the file is
0.95** (`field.rs:195`) — under the column-cast model, diffusion no longer
carries sunlight, only lateral bleed, and the value was cut accordingly.
At 0.95 the field tracks dusk in ~60 frames (uniform decay 4.0 → 0.2 at
×0.95/step), i.e. ~1.7% of the cycle: night is *not* systematically
brighter than the amplitude says. The 0e problem is real but is purely the
20:1 oscillator against instantaneous gates, not field relaxation.

The larger finding here is the doc rot itself — see §3.2.

## 3. What both reviews missed until now

### 3.1 `phototropism_dir` cannot point sideways — `light_weight` is inert by construction, and no scene will fix it

`organism.rs:1666-1674`: the function returns `(0.0, -1.0)` when the probe
4 cells up is brighter, else `(0.0, 0.0)`. That is the entire codomain.
So in `Grow`'s score, the phototropism term is **the same basis vector as
`upward_weight`**, intermittently gated — the two levers are collinear, and
both are collinear with what heading-inertia plus the turgor gate already
control. This explains *both* inert genome traits at once, and it means the
queue's item 2 ("re-test `light_weight` against the new light channel") and
the prior review's §2.3 ("measure it on a slope first") are testing a lever
that **cannot respond**: a slope, a clearing, or a neighbour's shadow
produce *horizontal* light gradients, and the probe has no horizontal
component to express them with.

The fix is the shape `moisture_pull` already has (`organism.rs:1699-1708`):
sample ±offset in x and y, return the normalized gradient. Then
`light_weight` becomes the engine's first genuinely lateral silhouette
lever, gap-seeking works, and the §3.2 plagiotropy lever gets a light
signal to interact with. Cheap, and strictly more expressive. Re-run the
genome study only after this lands.

### 3.2 Load-bearing doc rot in three places, all pointing at the old light model

This repo treats source comments as records of why; these three now record
a world that no longer exists:

- **`field.rs:102-195`**: ~90 lines argue for `LIGHT_DECAY = 0.9997`
  (air must not attenuate; the 25x outcome table; "re-derive rather than
  trust"). The constant is `0.95`, changed when `apply_sky` became a column
  cast, and nothing in the comment says so. The next person to "re-derive"
  from that doc will be re-deriving the wrong constant. `plant.rs:3807`
  ("`LIGHT_DECAY` puts `Germinate`'s threshold around 75 rows below open
  sky") is the same rot — under the column cast, open air attenuates
  nothing at any depth.
- **`examples/common/mod.rs:52-77`**: `PlantScene`'s doc still states "There
  is currently no depth that is both well-lit and un-ceilinged" and blames
  the topmost-row seeding — the exact defect Phase 0f fixed. The
  implementation plan's own status section contradicts it (78 rows of
  clearance at ground 200).
- **`plant.rs:505-524`**: `ambient_light_above`'s justification, already
  covered in §1.

One more, smaller: `plant_probe` still prints "one quantization step of
canopy density is 0.267 (4 bits, 15 steps)" — the 4-bit packing it
describes was removed by the sidecar migration.

### 3.3 The implementation plan carries a claim its own later measurements refute

`tree-architecture-implementation-plan.md`, Deferred: "`canopy_density`
reads max 0.000 at end of run — decay erases it before anything reads it,
so `crowding_weight` is inert and the crown shyness change landed on a dead
channel." The crown-shyness sweeps (0.5→136 … 20.0→13) are direct proof the
knob is live — the channel is read *at candidate-evaluation time during
growth*, when deposits are fresh; an end-of-run standing readout of a
signal with a per-tick half-life necessarily reads 0. That is the repo's
own "measure the standing state vs the event" trap, inverted. The deferred
bullet should be rewritten before someone spends a session resurrecting a
channel that is not dead. (This session's probe reads max canopy 1.5 — one
fresh deposit — at frame 30,000, consistent with "alive and decaying", not
"dead".)

### 3.4 `relocated_seed`'s search cone has holes at ±1 column — and the honest fix already exists

`plant.rs:567`: `(x + dx * dy.min(2), y + dy)` with `dx ∈ {0, -1, 1}`
checks columns `x, x±1` at depth 1 but `x, x±2` — never `x±1` — at every
depth ≥ 2. A seed that falls 2+ rows and drifts exactly one column between
checks (4 frames of `Powder` motion; a topple does this) is missed, the
site dies, and the organism becomes a permanent inert seed cell. Rare on
the flat harness; will bite exactly when a `slope` scene (both reviews'
recommendation #11) makes seeds roll. The function's own doc says the cell
list is "the right fix later" — the cell list has existed since Decision 2,
is maintained under both drivers (there is a test for the parallel one),
and a seed organism has exactly one cell. Replace the cone with a read of
`OrganismState::cells` and delete the search.

### 3.5 Smaller code notes

- **`break_buds` self-pay edge** (`plant.rs:1812-1827`): if the richest
  cell *is* the flushing bud, `write_carbon(rx,ry, held-cost)` then
  `write_carbon(bx,by, cost)` overwrites — the bud ends at `cost`, not
  `held`, silently destroying `held - cost` carbon. One `if (rx,ry) == (bx,by)`
  guard.
- **`lineage_step` saturates at 255** (`plant.rs:1007`,
  `saturating_add`): after 255 steps a lineage stops hitting
  `is_multiple_of`, so leaves stop. Unreachable today (turgor bound ~129
  rows) but it becomes the modulus for §3.3's rhythmic-growth gate — note
  it there.
- **`apply_sky`'s `let _ = world;`** (`field.rs:822`) is dead, and
  `if carried <= 0.0 { continue }` cannot fire (amplitude ≥ 0.2,
  transmission ≥ 0.2). Tidiness only, as the prior review said.
- **Leaves carry structural load** (`structural.rs` BFS filters on
  `organism_id` + `Plant` kind only; `leaf.ron`'s own comment admits it).
  Known, documented, worth remembering when abscission lands: shedding a
  leaf that was a load path will schedule real collapses.

### 3.6 Merge debt is real and quantified

The branch is 81 ahead / **71 behind** master. `git merge-tree` reports
**16 textual conflict hunks across 13 both-changed files** — including
`src/sim/field.rs` and `src/sim/update.rs`, where the semantic risk lives:
master gained a sky/sunrise renderer (`65a5d5c`) and "ground stays damp"
water work (`53118dc`) while this branch rewrote `apply_sky`'s invariant
(occupied blocks now lit) and soil moisture consumption. Also inbound from
master: worldgen (whose generated soil profiles will meet the degenerate
germination gates of §1 row 4 the moment trees plant into generated
terrain), `tests/determinism.rs`, and a rewritten CLAUDE.md whose new
`wiki/` rule this branch's eventual merge will owe pages under. None of
this blocks the queue; all of it argues against letting the gap grow
another 70 commits.

---

## 4. The six concerns — where this pass agrees, extends, or corrects

- **B1 (treadmill):** Agree with the diagnosis and the normalisation fix,
  now with the full history as evidence (§1, last row): of `pipe_ratio`'s
  four values, the two unit-conversions (45→22 light-for-leaves, 22→110
  leaf_cluster) are exactly what `L_node = MAX_LIGHT × leaf_cluster`
  normalisation deletes; the other two were genuine mechanism changes and
  no formulation removes those. So: the honest answer to "is there a
  formulation where economy, girth and spacing are independent" is — the
  *couplings* are genuine and wanted (one `Q` feeding both is the pipe
  model), the *units* are not, and the treadmill was units three times out
  of five.
- **B2 (cliff):** Agree it is a code artifact doing three jobs; correction
  §2.2 on landing order. After 7/8/2/9, re-derive `crowding_weight` expecting
  the usable band to open downward — if it does not, *that* is the finding.
- **B3 (one scene):** Agree, with the sharpening that the `slope`/`gap`
  scenes are **blocked on §3.1** for anything phototropic: they vary the
  light field horizontally, and no current lever can read horizontal light.
  Scene work and the gradient probe should land together.
- **B4 (determinism):** Agree with position-keying (germination coordinate
  × world seed), verified all its premises (§1). One addition: keep the
  drawn genotype *cached on `OrganismState`* at germination rather than
  re-hashed per read — position-keying makes the genotype a function of a
  coordinate the organism must then carry anyway, and the cache makes the
  eventual serialiser's job "save one struct" instead of "reproduce a hash
  discipline".
- **B5 (q_peak):** Agree completely with the `q_now`/`q_peak` split — it is
  the cheapest high-value change on the branch (the vector already exists,
  §1). Correction §2.3 on the noon-latch magnitude. One sharpening: because
  `q_peak` latches noon and `q_now` swings 20:1 daily, the damage trigger
  must compare *like phases* (e.g. latch a daily max of `q_now` too, or
  gate the deficit test on amplitude) or every plant will read "damaged"
  every night — 0e again, and the same per-cell running mean fixes both.
- **B6 (2D foliage):** Agree with the projection framing and with not
  changing the mechanism now. Note the per-column Beer-Lambert fix (§2.1)
  quietly delivers the "occlusion weight > 1" idea for free — a 5-cell
  cluster stacked 2-3 deep in a column attenuates like 2-3 cells of depth,
  which is the optical-depth reading of `leaf_cluster` without any render
  change.

---

## 5. The variety programme (part C) — deltas only

The prior review's §3 mapping (sympodial ≈ 4 lines; plagiotropy = per-order
tropism reference reusing `heading`; acrotony = one signed term on the bud
score against `collar_y`; reiteration = flushed bud writes order 0 on a
`q_peak − q_now` trigger; whorls = gate branching on `lineage_step`) is
**verified against source in every particular** (§1) and stands as the
plan of record. Amendments:

1. **Order**: 7 → 8 → 2 → 9 (§2.2), then 1 (sympody) → 3.1's gradient probe
   → 3 (plagiotropy) → 4 (acrotony) → 5/6 (q_now, reiteration) → 11
   (scenes) → 10 (whorls) → 12 (position-keyed genotype).
2. **Counters with pictures**, per CLAUDE.md: sympodial forks resolved,
   plagiotropic axes live, reiterations fired, buds flushed — printed by
   `plant_probe` next to every sheet. All four are one-line counters.
3. **Botany fact-check**: see §6.

---

## 6. Botany fact-check of the prior review's sources

All eleven load-bearing citations were independently verified against the
actual papers (paywalled ones text-extracted, quotes grepped verbatim, not
taken from snippets). Bottom line: **every mechanism claim survives; four
citations are wrong about *which paper* says it; one is overstated.**

**Verified as cited:** the 23 HOT models and their categorical axes
(restated verbatim in Barthélémy & Caraglio 2007, p. 383/389); the
reiteration taxonomy (complete/partial, immediate/delayed,
opportunistic/sequential — B&C 2007); the delayed-traumatic-reiteration
definition (Lecigne, Delagrange & Messier, UF&UG 47 (2020) 126541); the
competition-allometry findings (del Río et al., Trees 2019: narrower
crowns, greater height, less taper, plasticity tracking shade tolerance —
with the nuance that taper does not follow shade tolerance for
*inter*specific competition); Palubicki's λ sweep (Fig. 7 is literally
0.46/0.48/0.50/0.52/0.54, decurrent→excurrent) and the branch-width memory
quote, both verbatim; and Bond — the 10-fold elongation recovery is real
(Bond et al. 2007, Tree Physiology 27:441–453: old-growth Douglas-fir
scions on seedling rootstock, 10x within two years; size, not age).

**Wrong paper, right content:** the acrotony/basitony "arborescent or
bushy growth habit" quote *and* its annual-shoot-scale caveat are verbatim
from **B&C 2007 p. 384**, not Costes et al. 2014; the adaptive-vs-traumatic
reiteration definition is verbatim from **B&C 2007 p. 394**, not the Ishii
Tree Physiology paper it was attributed to; and "developmental constraints
outrank competitive position" is the conclusion of **Osada et al. 2004**
(For. Ecol. Manage. 188:337–347), not Sterck & Bongers — whose 2001 paper
supports the same direction ("light availability cannot explain trait
changes with tree height") by a different route. B&C 2007 turns out to be
the single most load-bearing source in the whole set; it belongs in
`tree-procedural-prior-art.md`'s canon.

**Overstated:** Prusinkiewicz & Remphrey (2000) does *not* say all 23
models fall out of "relatively simple parametric variations" — the phrase
is absent, the paper deliberately drops the rhythmic/continuous axis and
omits two models (McClure's, Tomlinson's), and claims only to characterize
"most" HO models from apex fate, branching configuration and
ortho/plagiotropy. Weaker than quoted — though note the three axes it
*keeps* are exactly §5's items 1, 3 and the branch-point rule, so the
implementation programme is unaffected. Likewise the Nature Communications
2025 crown-spectrum paper *cites* the "species turnover dominates,
intraspecific plasticity secondary" finding from a prior 342-species study
rather than concluding it; its own analysis is species-level only. The
investment ordering (discrete architecture > plasticity > genotype) still
stands, but its quantitative anchor is:

**New numbers worth keeping:**

- **Genotype explains ~10–30% of within-stand crown-form variance.**
  Progeny-trial heritabilities for crown/branching traits: loblolly branch
  angle/diameter/frequency h² 0.06–0.22; slash pine crown width 0.11–0.27;
  Douglas-fir ALS crown metrics 0.014–0.315; radiata form ~0.2–0.3. The
  branch's six-scalar jitter is, per the field data, correctly a *minority*
  contributor — the missing majority is competition and light history,
  which is what §5's plasticity checks measure.
- **35% establishment failure is *low*, not high, for a dense even-aged
  stand followed to maturity** — Peet & Christensen's 50-year loblolly
  plots lose ~60–99% in the thinning phase (densest: 1,141 → ~25 per
  plot). 35% is realistic for a moderate-density plantation up to first
  canopy closure. Uniform spacing *delays* self-thinning onset (everyone
  hits the boundary together) but does not avoid it — so the prior
  review's spatial critique (regular-pitch failures read as fake;
  clustered mortality is the natural signature; jitter the planting
  positions) stands, and the *rate* should be expected to climb as runs
  lengthen.
- **The HOT model table, temperate-anchored:** Troll's model genuinely is
  the temperate broadleaf workhorse — Barthélémy/Édelin/Hallé 1989 call it
  "the most frequent in both tropical and temperate woody species", and
  Fagus/Ulmus/Carpinus/Tilia/Tsuga are assigned to it in the literature.
  Champagnat is confirmed as the arching-shrub model (orthotropic axes
  secondarily bending under weight — Sambucus, Forsythia, Spiraea);
  Leeuwenberg's temperate examples are lilac and sumac. Note **Troll's
  model is built from plagiotropic axes that secondarily erect** — so §5's
  plagiotropy lever plus heading momentum is what unlocks the single
  commonest temperate architecture, a stronger argument for it than the
  fir-tier case alone. Mesotony is a real third category but belongs to
  median branching in trees (Cedrus, Alnus), not to shrub habit; the
  acrotony=tree / basitony=bush dichotomy survives at plant scale with the
  shoot-scale caveat already noted.
- **Epicormic buds: the engine's `DormantBud` is better biology than
  either review claimed.** Meier et al. 2012: preventitious buds persist
  ≥40 years in oaks (eucalypt bark strands: the tree's whole life),
  maintained by a parenchymatous trace that **extends radially at the same
  rate the tree thickens** — i.e., real buds mostly *keep pace with* the
  cambium, and resprouting after topping is overwhelmingly from these
  preventitious buds, with true adventitious (callus) shoots a minor
  wound-margin contribution. Two consequences: `thickening_survival` is
  the right *kind* of parameter and its literature range is species-huge
  (≈1.0 for eucalypt-like resprouters, low for most conifers), and a
  reiteration mechanism fed by surviving buds (§5 item 6) is the
  documented pathway, not an invention.
- **Syllepsis note:** real temperate laterals are mostly *proleptic* — a
  bud waits at least a season before breaking; sylleptic (same-flush)
  branching is the tropical/juvenile exception. `break_buds` can flush a
  bud the tick after it forms, i.e. everything is sylleptic. At game
  timescale that is the right call; it just means "how long must a bud
  wait" is a species lever the literature prices (poplars/birches barely
  wait; oaks wait a season) if variety along that axis is ever wanted.
- **Whorls (for §5 item 10):** pine pseudo-whorls run ~5–6 branches
  (P. strobus, one flush/year); polycyclic pines (P. taeda) flush 2–3+
  times a year; internodes within a flush are *not* uniform (long first
  cycle, cataphyll-scarred proximal zone). A `whorl_count` of 3–5 in 2D
  and a shorter first-flush internode would read right.

---

## 7. Filmstrips, looked at

Three sheets rendered this session into `target/filmstrips/`
(`review-grove.png`, `review-cut.png`, `review-light.png`), looked at
before concluding anything from the numbers.

**Growth (`review-grove.png`, frames 2,000→30,000).** The stand is healthy
and the trees are *individuals* in size — and every one of them is the same
kind of object: a vertical column carrying foliage clusters along most of
its height. No readable limbs leave the trunk; no crown reads as a shape
sitting on a bole; heights differ (110–145) but silhouette does not. This
is the variety complaint, confirmed at the zoom the game is played at, and
it is what §5's discrete axes exist to change. Two more things the numbers
do not show: **roots read as flat pale mats** hugging the top few soil rows
and running tens of cells sideways (queue item 4, visually worse than
"runs flat" suggests — they read as a buried fence, not a root system), and
crown shyness is visibly working — seven of eight crowns separated, one
pair nearly touching, matching the thickest-run 51 against 57 spacing.

**Damage (`review-cut.png`, `cut=170,60,120,80` at frame 15,000, watched
to 30,000).** The cut counter reads **removed 1,362 cells, 1,362 of them
living tissue** — the rectangle landed squarely on the stand (the branch's
own earlier experiment removed 1,344). The cut visibly lands — the
mid-stand tree is topped between tiles 1 and 2 — and over the following 15,000 frames the topped tree does
not rebuild; its neighbours grow into the opened gap instead, and by the
final tile the *stand* has closed the hole while the victim stays stunted.
That is precisely the `break_buds` rich-get-richer defect its own doc
records (intercepted light fell, so the drive to rebuild fell), watched
happening rather than inferred. Queue item 3 (`q_now`/`q_peak` deficit) is
the fix, and B5's answer is the design for it.

**Light channel (`review-light.png`, dusk → noon → afternoon).** The new
invariant, visible: open-sky columns read uniformly bright at every depth,
canopy blocks read lit (an occluder now carries the light arriving at it),
and each crown casts a block-resolution shadow column beneath it. Also
visible: those shadows are *weak* — crowns that read materially solid cast
faint shade — which is §2.1's under-attenuation seen directly. The sheet is
the picture the per-column Beer-Lambert fix should be judged against
after it lands.

---

## 8. Measured this session (commit `29e8984`, this worktree)

- `cargo test --release --lib`: 390 passed, 0 failed, 1 ignored, 137 s.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `plant_probe trees=8 frames=30000`: 20,331 cells; per-tree 1,810–3,099;
  8,512 Leaf / 11,244 MatureBody / 547 DormantBud / 28 GrowingTip; thickest
  run above ground 51; canopy top y=69; vein p50 9.68, 59% vascular; max
  canopy density 1.5.
- `plant_probe trees=8 frames=28800` (noon): 19,237 cells, **71 GrowingTip**,
  530 DormantBud — the 2.5x frontier swing on a smaller stand, reproducing
  the prior review's paired-phase measurement.
- `git merge-tree abe9c2f master plant-substrate-v2`: 16 conflict hunks,
  13 both-changed files.

*Written 2026-08-16 against `29e8984`, in the plant-v2 worktree. The
botany verification in §6 was performed against the actual papers
(text-extracted where paywalled), not search snippets; §7's sheets are in
`target/filmstrips/` beside the branch's own numbered series.*
