# The entrance papers in a vertical section — what transfers, what needs a factor, and what cannot be posed here

*2026-09-19. Written against the owner's entrance research report, which
closes the gap both prior reports named. **Nothing here is built.** This is
the dimensional audit that has to happen before any of its formulas become
code, and it exists because the report's numbers were measured in two
perpendicular planes and only one of them is ours.*

Companion to `nest-digging-plan-2026-09-19.md` (the stages) — **which is
not in this directory: it is on `claude/sweet-tesla-ommknn`, unmerged and
with no PR, along with `examples/digbox.rs` and the lane handoff. Every
test this report names has to run in that harness.** Also to [`nest-biology-2026-09-19.md`](nest-biology-2026-09-19.md)
(§2.5 is the cell-scale derivation this report uses rather than repeating),
`nest-biology-digging-signals-2026-09-19.md` and `nest-build-plan-2026-09-19.md`
(PR #472, open), and [`stigmergy-research.md`](stigmergy-research.md) §5 and
§7, which already carry Toffin and already state the plane principle this
report makes quantitative.

---

## 0. The one rule, and the three corrections it produces

**Sort every quantity by the plane it was measured in.** This world is a
vertical section — horizontal × depth, with gravity *in* the plane. The axis
we do not have is a second horizontal one. So:

- anything measured in a **vertical** plane is in our geometry and transfers;
- anything measured in a **horizontal** plane needs a projection factor, or
  cannot be posed here at all.

`stigmergy-research.md` §7 states this already, in the sentence that matters
most for the entrance work: *"Foraging is natively top-down — trails
spreading across a floor. Nest architecture is natively side view — every
real nest cross-section, and every one of Toffin's digging experiments (§5),
is a vertical section."* What is new here is that the entrance report is the
first document to put a set of **formulas** on the table, and three of them
change under that sort.

**The corrections, in the order they bite:**

1. **Keep `A^0.5`; do not take the offered `V^(2/3)`.** The report suggests
   substituting `V^(2/3)` for the crowding term "in 3D". We are not in 3D,
   and neither was Toffin. Taking the substitution would be correcting a
   formula that was already right for us.
2. **The crater needs a factor of `1/r`** if it is to read as a section
   through a real mound rather than as a natively-2D heap. §3 below.
3. **The amplification kernel is wider than the shaft it is meant to
   create**, at our lattice resolution. §4 below, and it is arithmetic
   rather than biology.

---

## 1. The plane table

Every quantity in the entrance report, sorted. **The middle column is the
finding**; the rest follows from it.

| quantity | measured in | for us |
|---|---|---|
| Toffin's dig rule — diggability, marker sum, Hill response | **vertical** (sand sandwich; his own gravity remark, and `stigmergy-research.md` §5) | transfers as-is |
| `A(t) = A_M·t^α/(β^α + t^α)` and the generative `τ = θ·A^0.5·(1−A/A_M)` | **vertical** | shape transfers; **`A^0.5` is correct, not `V^(2/3)`** |
| Branching onset at `A ≈ 0.6·A_M` | **vertical** | transfers (dimensionless) |
| Shaft diameter ≈ 1 body length (Gravish 2013) | **vertical** | transfers; **1–2 cells** (§2) |
| Piecewise-linear downward descent; arching and low-stress grains | **vertical** | transfers |
| Moisture threshold `W ≥ 0.05` (Monaenkova 2015) | substrate property | dimensionless in principle, **unmapped to our 0–1000 unit** |
| Reversal probability 0.34 when blocked (Aguilar 2018) | **vertical** shaft | transfers (dimensionless) |
| Active diggers ≈ √N | fixed arena, **vertical** | transfers (dimensionless) |
| Entrance chamber: ~5 cm long, 2–3 cm below surface | depth is **vertical**, length is **horizontal** | depth transfers; length is an upper bound on what a section shows (§4) |
| Crater / spoil profile from drop-distance `p(r)` | **horizontal** (radial walk) | **needs `1/r`** (§3) |
| Khuong's marker lifetime → pillar spacing | **horizontal** (spacing is set across the floor) | the deposit-near-fresh bias transfers; the spacing calibration does not |
| "2D nest patterns under flat rocks" | **horizontal** | **not our plane, despite the same label** |
| Entrance count; 15 cm clustering; the spanning tree of 1–6 adjacent chambers | **horizontal** (the ground surface) | largely **cannot be posed** (§5) |

**The trap is the two perpendicular "2D"s.** The report uses the label for
Toffin's vertical sandwich *and* for nest plans under flat rocks. They are
90° apart. A session porting "the 2D results" without checking which is
which will take a floor-plan result into a cross-section.

---

## 2. The conversion tiers, and the anchor

There are four tiers and **only the first two are safe**.

**Tier 1 — dimensionless. Use as-is.** The Hill exponent (2), reversal
`R = 0.34`, active diggers `≈ √N`, branching onset at `0.6·A_M`, the
exponents α (1.38 at 50 ants, 1.72 at 300).

**Tier 2 — denominated in body lengths. Convert at the engine's own anchor.**
Only one row: shaft diameter ≈ 1 body length. `ant.ron` authors
`body: Chain(2)`, so **a shaft is 1–2 cells wide** — and note this
conclusion **survives `nest-biology-2026-09-19.md` §2.5's open ambiguity**
about whether `Chain(2)` is two ant-lengths or a head-plus-body abstraction.
Under the first reading a shaft is 2 cells; under the second, 1. Either way
it is single-digit, which is what makes it safe to build on while the
convention is unsettled.

**Tier 3 — denominated in centimetres. Rough, and the error bars are wide.**
§2.5 derives **2–5 mm per cell** and that is the span to carry. It also
records that *nothing in the tree states a metres-per-cell convention*,
which is the real gate here. At 2–5 mm/cell:

| report's figure | cells |
|---|---|
| entrance chamber depth, 2–3 cm | **4–15** |
| entrance chamber length, 5 cm | **10–25** |
| entrance clustering, ≥15 cm | **30–75** |
| deep chambers, 1–2 m | **200–500** (§2.5's own table gives ~670 for a *Pogonomyrmex* nest) |

**And the per-row species is missing.** The report does not state a body
length per row, and its rows span *Myrmica* to *Pogonomyrmex* — roughly
3 mm to 10 mm. So each figure above carries the 2–5 mm/cell span *and* an
unstated species factor on top. Treat Tier 3 as order-of-magnitude.

**Tier 4 — bound to Toffin's lattice and timestep. Do not port.**
`K = 50`, `ν = 0.05`/step, the 100 marker units per dug cell, `ξ = 0.001`,
`θ = 0.38`/`1.08`, and the 0.04 h⁻¹ activity density. His cell is 0.265 mm
(0.07 mm² cells) against our implied 2–5 mm, and his step is one minute
against our `tick_interval: 6` frames. **The dimensionally sound way to
carry a decay rate is in units of the dig cycle** — grab, ascend, exit,
drop, return — not in minutes and not in frames. Khuong's ">10 minutes for
structure, 20 used" is the same statement: a marker must outlive some tens
of cycles. Our cycle length is not measured; that is the number to take
before any decay constant is chosen.

**Two consequences worth having.** Our door is **46 cells** across. At
30–75 cells for the ≥15 cm minimum spacing between two *separate* real
entrances, our single door is comparable to the gap real nests keep between
two of them — and it is 23–46 shafts wide. And at 200–500 cells the deep
chambers are deeper than the world is tall (320 rows), so the entrance
chamber and shaft are representable here and the full depth profile is not
until the world grows (M10 streaming).

---

## 3. The crater, which is the one formula that is wrong if ported straight

The report's §3 rule is: each exiting ant walks a distance `r` from the hole,
drawn from something like a Gamma with a mean of a few body lengths, and
drops its pellet. That is a **radial** walk on a horizontal surface.

**In 3D**, the mass dropped at radius `r` is spread around a circumference
`2πr`, so the surface density is `σ(r) ∝ p(r)/(2πr)`. A vertical section
through the resulting ring mound reads `h(x) ∝ p(|x|)/|x|`.

**In our section**, an ant leaving the hole can go left or right and nowhere
else. Mass dropped at distance `r` lands at exactly two points, `±r`. So
porting `p(r)` directly gives `h(x) ∝ p(|x|)`.

**The difference is a factor of `r`, and it is visible rather than
subtle.** `p(r)` peaks at some `r* > 0`, so porting it straight puts a
*ridge* at `r*` with a **dip between the ridge and the hole** — read as two
detached humps flanking a gap, not a crater. Dividing by `r` pulls the peak
inward and steepens the outer decay, which is the profile a section through
a real mound has.

**So: deposit on `p(r)/r` if the section is to read as a slice of an
anthill; on `p(r)` if we are declaring this a natively 2D world.** These are
different pictures and the choice should be made deliberately.

**This is judge-by-eye, not a metric.** It is exactly the case
`CLAUDE.md` reserves for the review queue — post both profiles as a paired
comparison and let the owner say which reads as a mound. A residual against
a fitted curve would answer a question nobody asked.

**Unbuilt, and flagged by the report's own §8**: there is no published
equation for entrance spoil. Both the drop-distance rule and this correction
to it are modelling proposals.

---

## 4. The kernel is wider than the shaft, and this is arithmetic

Toffin's rule sums the marker over a candidate cell's **8 neighbours** — a
3×3 kernel, 3 cells across. On his lattice **an ant is 4 cells wide**, and a
shaft is about one ant.

> kernel 3 cells < ant 4 cells < shaft. The kernel resolves the feature.

Ours: `body: Chain(2)`, so an ant is **2 cells** and a shaft is 1–2 cells
(§2). The same 8-neighbour kernel is still 3 cells across.

> kernel 3 cells > shaft 1–2 cells. **The kernel cannot resolve the feature
> it exists to create.**

**This is the spatial form of a failure `CLAUDE.md` already names** — *a
channel that decays and is read as a gradient needs range for both*, and
*narrow storage takes the gradient away first, and silently*. That rule is
about a channel's **value** resolution; this is the same defect in its
**spatial** resolution, and it has the same signature: the code stays
correct and every gate stays green while the structure the mechanism is
supposed to concentrate is smeared across a kernel wider than itself.

**It also predicts exactly when the endogenous mechanism fails.** Toffin's
Hill function breaks symmetry *endogenously* — whichever site was dug first
wins, and the squared term is what makes it win decisively. That needs the
lattice to be able to hold "one site" distinct from its neighbours. At 1–2
cells per shaft against a 3-cell kernel, it cannot.

**Three responses, and they are not equal:**

- **(a) Resolve the ant better.** Widen `body: Chain(N)` or shrink the cell.
  Correct, and it touches every creature in both games and every constant
  calibrated against a two-cell body — `Reports/creature-body-extent-2026-08-30.md`
  is the record of what that costs. Not a nest-lane change.
- **(b) Let the measured ant-level mechanism carry the direction instead.**
  The downward dig bias is *measured* (the PNAS piecewise-linear result,
  already validated on the plan's Stage 1) and acts on the dig **target**,
  not on a kernel, so it is unaffected by kernel width. **This is the one to
  try first**, and it is already Stage 1.
- **(c) Break the kernel's isotropy** — weight the vertical neighbours above
  the horizontal so amplification propagates down rather than sideways.
  **This is a proposal of mine and nothing in the literature supports it.**
  See §6.

---

## 5. What our geometry cannot pose — which is the least-settled part of the report

The entrance report's §6 is honest that **there is no published rule for
opening a new entrance**, and its §8 repeats it. That vindicates both prior
reports' refusal to invent a convergence rule.

**And it is the part our geometry can least use, which is a relief.**
Entrance *count* and the 15 cm clustering live on the horizontal ground
surface. In a vertical section that surface is a **line**. A nest with six
entrances scattered over a patch, cut by our plane, shows **one —
occasionally two**. So "how many entrances" is a sampling question for us,
not a biology one, and the invented hypothesis we were most at risk of
building is the one we need least.

The same applies to the entrance chamber's *architecture*: the minimum
spanning tree with 1–6 adjacent chambers around a hub is a **plan**, and a
section through it shows a hub with a couple of neighbours, not a tree. The
chamber's "5 cm long" is an upper bound on what a section reads, since a
slice off the long axis shows a shorter chord.

**What does survive, and it is the part that matters:** the *vertical*
architecture. A surface opening, a short shaft about one ant wide, an
entrance chamber 4–15 cells below it, and tunnels leaving that chamber
downward. Every one of those is a depth measurement, and depth is our axis.

---

## 6. What is mine rather than the literature's, stated so it can be refused

**The anisotropic kernel (§4c) is my invention.** The argument for it is
that gravity has already broken the isotropy in a vertical section, so an
isotropic kernel is the unphysical choice; the argument against it is
stronger and I want it recorded here rather than discovered later:

- **It is confounded with Stage 1.** A vertical kernel bias and a downward
  dig bias produce the same outcome by different routes. `CLAUDE.md` is
  explicit that two changes to the same outcome cannot be attributed apart.
  If Stage 1 produces the shaft, this is redundant.
- **Stage 1 is measured and this is not.** The ant-level downward
  preference is in the PNAS paper. An environmental directional prior on
  the amplification field is in no paper I have seen.
- **The repo's own precedent argues for endogenous symmetry breaking.** The
  plant line's flux-ratcheted per-face conductance broke isotropy
  *deliberately* because the isotropic rule could not produce a winner — but
  it broke it on **flux**, a quantity the system already had, not on an
  imposed prior. Toffin's marker is the digging analogue of flux.

**So the ordering is: (b) before (c).** Run the measured mechanism alone.
Only if it fails to concentrate — which §4 predicts it will, and for a
stated reason — does the fallback earn a test, and then it should be scored
as a *control arm against* Stage 1 rather than instead of it.

**And if it is built, somebody else should score it.** `CLAUDE.md`: *a
positive hides from motivated reasoning — it is the result you wanted, and
every check you reach for is one it passes.* I proposed it; I am the wrong
reader for its result.

---

## 7. What this does to the plan

- **Stage 1 gains a second reason and keeps its ordering.** It was ranked
  first on cheapness; §4 adds that it is the only directional mechanism
  immune to the kernel-resolution problem.
- **Stage 1's load-based arm weakens.** The report's §1 says arching makes
  tunnel-surface grains low-stress automatically, so an ant-side stability
  heuristic is probably unnecessary. **Conditional on our substrate
  actually arching**, which is untested: `structural.rs` keeps support
  *distances*, not force chains.
- **Stage 4 (fresh spoil attracts digging) is Toffin's marker in physical
  form**, and the report gives it a functional shape. It must stay
  *physical* — a material adjacency test — because a chemical field walks
  into Bruce 2015's measured null at the dig face, which the plan's §3
  already forbids.
- **A regime check the plan does not have.** `digbox` runs **1,200 ants**.
  Toffin fitted 50 and 300, with both α and θ still climbing at the top of
  that range, and the report's §5 says only `√N ≈ 35` should be digging at
  once while the robot result has performance *collapsing* at four diggers
  in one shaft. **We may be measuring a jam.** Run arms at 50 and 300
  before trusting anything fitted, and note this is a question about the
  *harness*, not the engine.
- **Founding is upstream of all of it.** `found_colony_of` calls
  `paint_nest_patch` and excavates **zero cells** — 53 columns of surface
  ground converted to nest material, a door 46 cells wide and (via
  `adjacent_nest`'s 8-adjacency to a one-cell skin) 2 rows deep. The
  biology's founding gesture is a lone queen digging a shaft *down* to a
  target depth on self-motion and timing alone. Ours paints a strip
  *across*. That is the shaft's dimensions transposed, and every
  measurement in this line was taken on a nest founded that way.

---

## 8. Sources

The owner's entrance research report, 2026-09-19 (not in this directory).
Its own recommendation is to pull **Toffin et al. 2009** (*PNAS*;
supplementary material has the full simulation details), **Khuong et al.
2016** (*PNAS*; 3D deposition), and **Aguilar et al. 2018** (*Science*;
traffic cellular automaton). The container's egress proxy returns 403 for
journal hosts; the `mcp__PubMed__*`, `mcp__Consensus__*` and
`mcp__bioRxiv__*` connectors are the way in — see the nest-digging handoff
for the workflow that worked.

**Claims in this report that rest on the entrance report rather than on a
paper read here**, and which a later session should not upgrade silently:
that Toffin's arena is vertical (consistent with `stigmergy-research.md` §5
and with his gravity remark, but the supplementary material was not read
here); the `p(r)` Gamma form for drop distance, which the report itself
flags as a proposal; and the √N active-digger law, whose derivation is not
given and which may be the 2D-specific form of the same boundary-scaling
argument that produces `A^0.5`.

**Checked in the tree for this report**, rather than taken from a note:
`found_colony_of` and `paint_nest_patch` (founding paints and never digs);
`adjacent_nest` (8-adjacency to a one-cell skin); `NestRoom::occupancy`
(`target/(target+room)` — a Hill response at exponent **1**, where Toffin's
is **2**); `COLONY_HALF_WIDTH = 26`; `ant.ron`'s `body: Chain(2)`; and that
no metres-per-cell convention exists anywhere in `src/`, `wiki/` or
`README.md`, which confirms §2.5's statement of the same.
