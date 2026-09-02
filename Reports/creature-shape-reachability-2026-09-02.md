# Measuring whether an articulated body is reachable, before anyone designs one

Answers three pre-checks `Reports/creature-genome-flexibility-2026-09-02.md`
§13e names before its articulated-body proposal (§13c) is built: does
articulation recover mobility, is shape legible at all at this cell scale,
and can the owner tell the candidates apart. Read alongside
`Reports/creature-appearance-design.md` §1-§5, which is the evidence §13
rests on and partly argues with.

**Measurement only.** No body plan was implemented and neither `src/` nor
`assets/` was touched — another session is live in `creature.rs`, `brain.rs`,
the species files and the material table, and §12a's rule is that body and
metabolism are one shared budget, so two sessions moving it cannot be read
apart. Three new `examples/*.rs` harnesses were added (read-only against the
engine, listed in `Reports/instruments.md`) and one existing species
(`beetle`, already a shipped 2x2 rigid body) was measured for the first time.

## 0. The three numbers, stated first

1. **Does articulation recover mobility?** Partially, and only in one
   direction. A rigid footprint's blocked-move rate is set by its **width**,
   not its height or cell count, and the relationship is a step rather than a
   gradient: **≤2 cells wide blocks 8-13% median; ≥3 cells wide blocks 47-58%
   median**, on 12 seeds each. So an articulated body built entirely from
   parts ≤2 cells wide could plausibly land near that 8-13% bucket — a real,
   substantial win over a plain rigid block, though still 1.5-2x worse than
   the shipped chain's ~6%. **The moment any part, front or back, is ≥3 cells
   wide, the whole body's measured mobility is indistinguishable from a plain
   rigid block of that width** (§1.4) — which is exactly the "small head, big
   abdomen" insect silhouette the proposal is reaching for, and §13c's own
   corrected argument already said this case gets no help. Articulation's
   measured value is **length at near-chain mobility, provided every part
   stays narrow — not width, and not the insect outline.**
2. **Is shape legible at all at 36 cells?** No, on the instrument that
   exists. Two 36-cell bodies of different composition (a filled 6x6 block, a
   three-segment waisted "insect" silhouette) came out **~0.5% apart on ink**
   across 5 seeds (range 0.1-1.6%), not the ~15% §13d's threshold-crossing
   prediction needs. `creature-appearance-design.md`'s finding — shape at
   constant extent moves nothing measurable — generalises from 9 cells to 36
   rather than being an artifact of the smaller size. This is a real result,
   not a failure of the check: **on this evidence D1's lever is extent, not
   architecture**, for findability at least. §2 also states what effect size
   this design had the power to detect, since a null with no power statement
   is not evidence of absence. **§2.1 closes the follow-up question this
   raises**: extent grown as *length at the mobility-safe width §1 found*
   (2 cells wide, 18 tall, still 36 cells) scores within the same noise band
   as extent grown as a compact block — the mobility recommendation and the
   legibility recommendation are not in tension, provided the economics are
   fixed first (below).
3. **Can the owner tell them apart?** Posted, not yet answered. Six body
   shapes (`ant`, `beetle`, `uniform3`, `uniform5`, `forward_taper`,
   `backward_taper`) as a blind gallery card, `20260902T194120383Z-3860b1`,
   board `creatures`. This is the check no metric here can substitute for:
   whether any of these reads as an animal rather than a smudge.

**Recommendation:** build the narrow-part case if the goal is a body longer
than a chain that still reads as an animal once §3 answers whether these
silhouettes clear that bar — but do not expect it to produce the owner's
stated insect silhouette (small head, big abdomen), because that specific
shape is measured to cost the same mobility as a plain rigid block, with or
without articulation. If the real goal is "creatures that read as more than
a chain," on §2's evidence (and §2.1's follow-up) that is answered by making
them **bigger, as a long narrow body**, not by giving them a different
architecture while narrow, and not by making them wider — width is exactly
what §1 measured as expensive. **One prerequisite precedes all of this**:
`creature-genome-flexibility-2026-09-02.md` §11e/§12d already found that
body size buys nothing in today's economy (more meat for a predator and
nothing else), so a heritable size gene would shrink rather than grow until
that is fixed — extent is the right *shape* lever, but it needs a reason to
be selected for before it is worth building.

## 1. Q1 — does articulation recover mobility?

### 1.1 Method

`examples/creature_scale.rs`'s `mode=walk` is the standing instrument
(`Reports/creature-appearance-design.md` §5's `moves_blocked / (moves +
moves_blocked)` over a colony on generated terrain), extended with a new
`examples/creature_body_probe.rs` that clones `ant_block`'s `CreatureDef`
and overrides only `body` (`SpeciesRegistry::set_creature`, the same
technique `creature_scale.rs`'s own control arm uses), so every shape below
differs from every other **only in its footprint**. All runs: `rolling`
preset, `count=24`, `frames=4000`, **12 seeds each** (not four — the trap
this line has paid for twice, most recently reversing a rooms result
outright). Binaries were rebuilt fresh (`cargo build --release --examples`)
before any of this was measured.

### 1.2 Reproducing the standing controls

| shape | plan | cells | n | mean | median | min | max |
|---|---|---|---|---|---|---|---|
| `ant` | Chain(2) | 2 | 12 | 6.5% | 6.2% | 3.1% | 10.7% |
| `ant_block` | Rigid 3x3 | 9 | 12 | 53.1% | 55.8% | 21.1% | 67.8% |
| `ant_wide` | Rigid 5x2 (waisted) | 9 | 12 | 58.3% | 56.6% | 37.7% | 76.5% |

`creature-appearance-design.md` §5 cites 5% / 43% / 41% for these three (one
seed, a different harness: 40 attempted placements over 600 frames rather
than this instrument's 24 over 4,000). `ant` reproduces within noise. The two
rigid bodies reproduce in **order of magnitude** but run higher at the
median than the single-seed citation — and this is itself the finding the
citation's own report warned about: `ant_block`'s 12-seed range is
21.1-67.8%, which contains 43% comfortably below its median, so the
single-seed number was a real sample from a wide distribution, not a
different measurement. It is also **no longer ordered**: `ant_block` and
`ant_wide` land within a point of each other (55.8% vs 56.6%), matching
`creature-appearance-design.md` §4's own finding that the ranking between
these two reverses tree to tree and does not survive. **The 12-seed median
is the number to cite going forward**, not the single-seed figure either
report used before this one.

### 1.3 The never-measured 2x2, and what actually drives it

| shape | plan | cells | n | mean | median | min | max |
|---|---|---|---|---|---|---|---|
| `beetle` | Rigid 2x2 | 4 | 12 | 13.0% | **12.4%** | 6.4% | 19.2% |

§13e named this the number that "converts this whole argument from a
prediction into a number." **12.4% median** — about 2x the shipped chain's
rate, and a full order of magnitude below what any ≥3-wide rigid body costs
(§1.2). On its own this leaves open whether the win comes from being small
(4 cells) or from being narrow (2 wide) — `ant_block` and `ant_wide` are
both larger *and* wider than `beetle`, so cell count and width were never
separated. Four more shapes separate them, built the same way (a
`CreatureDef` cloned from `ant_block`, `body` replaced), 12 seeds each:

| shape | plan | cells | width | n | mean | median | min | max |
|---|---|---|---|---|---|---|---|---|
| `domino_v` | Rigid 1x2 | 2 | 1 | 12 | 10.0% | 8.5% | 2.8% | 26.7% |
| `domino_h` | Rigid 2x1 | 2 | 2 | 12 | 8.6% | 8.3% | 2.4% | 16.4% |
| `strip3` | Rigid 3x1 | 3 | 3 | 12 | 45.6% | 52.5% | 18.1% | 69.1% |
| `strip4` | Rigid 4x1 | 4 | 4 | 12 | 50.2% | 46.9% | 22.1% | 73.3% |

`domino_v` and `domino_h` hold cell count fixed at 2 (matching `ant`'s own
chain length) and only move width 1 vs 2: both land in the same ~8-10%
bucket. `strip3` and `strip4` hold height fixed at 1 and only move width 3
vs 4: both land in the same ~47-53% bucket, matching `ant_block`'s 3x3 and
`ant_wide`'s 5x2 despite 2-3x fewer cells and a third the height. **Height
and cell count do not move the number; width does, and it is a step rather
than a gradient.** Pooling every shape by width bucket (12 seeds each,
`ant_block`/`ant_wide` included in the wide bucket):

| bucket | n | mean | median | min | max |
|---|---|---|---|---|---|
| width ≤2 (`domino_v`, `domino_h`, `beetle`) | 36 | 10.5% | 10.3% | 2.4% | 26.7% |
| width ≥3 (`strip3`, `strip4`, `ant_block`, `ant_wide`) | 48 | 51.8% | 54.2% | 18.1% | 76.5% |

The two buckets barely overlap (18-27% is the only shared range) and the
medians sit 5x apart, on a preset whose terrain roughness this threshold is
presumably scaled to — a different preset was not tested, so treat "3" as
specific to `rolling` and "there is a sharp width threshold, not a gradient"
as the transferable claim.

### 1.4 The taper question, measured rather than assumed

§13c's corrected mobility argument: blocking is set by the **leading** part,
because it moves into fresh ground while a trailing part can reuse ground
the part ahead already proved passable — so a rearward taper (each part
narrower than the one ahead) is free, and a forward taper (a trailing part
*wider* than the one ahead, "a small head, a big abdomen") is not, because
that wider part is not moving into vacated ground.

There is no multi-part `BodyPlan` to build the real test on, so this
measures the floor a genuinely decoupled body would be compared against:
both taper shapes as a single **monolithic** `Rigid` body — a 2x2 head plus
a 3x3 abdomen (`forward_taper`) and its mirror, a 3x3 head plus a 2x2 tail
(`backward_taper`), 14 cells each, matching the shapes
`examples/creature_candidate_render.rs` renders for §3 exactly:

| shape | cells | n | mean | median | min | max |
|---|---|---|---|---|---|---|
| `forward_taper` (small head, big abdomen) | 14 | 12 | 53.6% | 56.6% | 22.5% | 73.5% |
| `backward_taper` (big head, shrinking tail) | 14 | 12 | 55.6% | 56.2% | 35.0% | 68.0% |

**Both land at the same ~54-57%, statistically indistinguishable from each
other and from the plain `ant_block`/`ant_wide` numbers above, regardless of
which end carries the wide segment.** For a monolithic body this is expected
— the whole footprint has to fit at every step, so where the wide segment
sits cannot matter — but it is also the control the *articulated* prediction
needs: a genuinely decoupled `backward_taper` could do no better than its
own leading (wide) part's standalone rate, because the leading part still
has to find fresh 3-wide footing every step whatever trails it, and §1.3's
data already gives that rate directly (~47-58% for any ≥3-wide footprint).
So even under §13c's own "backward taper is free" framing, "free" means the
narrow trailing parts add no *extra* cost over the head's own rate — it does
not mean the body becomes cheap. **Getting near the 8-13% bucket requires
every part, front and back, to stay ≤2 cells wide; a single wide part
anywhere sets the whole body's cost, independent of position.** This is a
sharper and more pessimistic statement than §13c's "taper backward is free"
reads on its own, and it is measured rather than argued.

## 2. Q2 — is shape legible at all at 36 cells?

`creature-appearance-design.md` §4: two 9-cell bodies (a filled 3x3, a
waisted 5x2) scored **0.8% apart on ink** and inside the noise on contrast,
concluding "extent is the only lever" at that size. §13d's counter-argument:
the owner's "it is a perfect cube" verdict was a shape reading delivered at
36 cells, so the two findings may sit on opposite sides of a legibility
threshold rather than contradicting each other — falsifiable as "0.8%
becomes ~15%" (threshold holds) against "stays near 1%" (the appearance
report generalises, and articulation's value is extent alone).

New instrument, `examples/creature_shape36_probe.rs`, reusing
`creature_look.rs`'s method (paired with/without render on real, grown
terrain; `ink` and `|contrast|` against the surround) without touching that
file: a filled 6x6 block against a three-segment waisted "insect" (a 3x4
head, a pinched 2x2 waist, a 4x5 abdomen — the same *kind* of composition
`ant_wide.ron` uses at 9 cells, scaled up with a real pinch), both 36 cells,
5 seeds:

| seed | ink diff | \|contrast\| diff |
|---|---|---|
| 1 | 1.6% | 2.3% |
| 2 | 0.1% | 20.4% |
| 3 | 0.5% | 12.9% |
| 4 | 1.1% | 22.2% |
| 5 | 0.4% | 17.9% |
| **median** | **0.5%** | 17.9% |
| **mean** | **0.7%** | 15.1% |

**Ink stays near 1% (0.1-1.6%), not ~15%.** §13d's falsifiable prediction
did not hold: the appearance report's finding generalises from 9 cells to 36
rather than being a small-size artifact. `|contrast|` is noisy (2-22%), but
it was already "inside the noise" at 9 cells in the original report for the
same structural reason — the two shapes cannot share a placement (they do
not fit in the same footprint), so each samples a different patch of
terrain for its surround, and that placement variance is exactly what
`|contrast|`, unlike `ink`, does not average out over the body's own cells.
Read `ink`, per the original report's own emphasis.

**What effect size would this design have detected?** The observed spread
across 5 seeds tops out at 1.6% — if the true effect were anywhere near the
predicted ~15%, it would sit roughly 10x above that spread and would not
have been missed. This design had the power to detect the effect §13d
predicted, and did not detect it; that is a real null, not an underpowered
one.

**This is a real result, not a failure of the check.** On findability, D1's
lever remains extent, not architecture, at both 9 and 36 cells. It leaves
open exactly what §13d's own "standing gap" names: no instrument here
measures whether a body *reads as an animal* rather than being merely
*findable* — which is what §3 asks instead.

### 2.1 Does "extent" have to mean compact?

Added after a follow-up question about §0's recommendation ("if the goal is
a creature that reads as more than a chain, the evidence points at extent,
not architecture"): every extent measurement above, and every one in
`creature-appearance-design.md` §2, grew the body as a **compact block** (up
to 4x4 at 16 cells there, a 6x6 at 36 here). §1.3 found the mobility-safe
zone is the opposite shape — **width ≤2, however long** — so before
recommending extent-via-length as the way to grow a creature without paying
§1's rigid-body mobility tax, the two findings need to be checked together:
does a long, narrow body keep the legibility win a compact block gets, or
was that win actually about being square-ish?

A third 36-cell arm, `narrow36` (`block(2, 18)` — 2 cells wide, 18 tall, the
same mobility-safe footprint §1.3 measured at 8-13% blocked), added to
`creature_shape36_probe.rs` and run against `block36` for 6 seeds:

| seed | block36 ink | narrow36 ink | diff |
|---|---|---|---|
| 1 | 5220 | 5233 | 0.2% |
| 2 | 5146 | 5157 | 0.2% |
| 3 | 5189 | 4506 | 13.2% |
| 4 | 5221 | 5163 | 1.1% |
| 5 | 5192 | 5073 | 2.3% |
| 6 | 5224 | 5070 | 2.9% |
| **mean** | | | **3.3%** |

Five of six seeds land at 0.2-2.9%, the same band `block36` vs `waisted36`
scored (0.1-1.9%, §2's table); seed 3's 13.2% is one placement-driven
outlier of exactly the kind this line keeps warning about (a single run is
a sample from a wide distribution), not a second data point. **Extent
pursued as length at a mobility-safe width keeps the same legibility as
extent pursued as a compact block.** The two findings are not in tension:
a long, narrow, jointed body (what §1.4's "every part ≤2 cells wide" case
actually looks like) is not a legibility compromise for being mobility-safe.

**What this does not check:** `ink` says the body puts the same amount of
"not-ground" on screen either way; it does not say a 2x18 silhouette *reads
as an animal* rather than as a root, a vine, or a crack in the rock — that
is §3's question, and `narrow36` is not one of the six shapes on that card.
Nor does it touch the economics: `idle_cost_per_cell` and
`move_cost_per_cell` are both charged per cell (`CLAUDE.md`, 2026-08-30), so
a 36-cell body of either shape costs 18x the shipped two-cell ant's upkeep,
and `creature-genome-flexibility-2026-09-02.md` §11e/§12d already flags that
size buys nothing in the current economy — an unguided heritable size gene
would shrink, not grow, whichever shape it grew into. Pursuing extent as a
design direction is contingent on that being fixed first; this section only
says the shape-legibility half of the trade is not the blocker.

## 3. Q3 — can the owner tell them apart?

New instrument, `examples/creature_candidate_render.rs`: since there is no
multi-part `BodyPlan` to render, and a static or smoothly-walking body's
silhouette does not depend on whether its segments are one rigid plan or
several parts following each other (articulation changes how a body decides
to *move*, not what its cells look like once placed), this stamps a single
`Rigid` body shaped like each candidate and crops/zooms it exactly as
`creature_scale mode=size` already ships.

Six shapes, one seed (7), at `ant_block`'s economics throughout:

- `ant` — shipped, Chain(2), 2 cells
- `beetle` — shipped, Rigid 2x2, 4 cells
- `uniform3` — three equal 2x2 segments with pinched waists, 14 cells
- `uniform5` — five equal segments with pinched waists, 14 cells
- `forward_taper` — small head (2x2), big abdomen (3x3), 14 cells
- `backward_taper` — big head (3x3), shrinking tail (2x2), 14 cells

Posted as a blind gallery card (labels hidden until the owner answers),
`board=creatures`, id **`20260902T194120383Z-3860b1`**, cell counts in each
item's `meta`, asking which reads most like an animal and which reads as a
blob. **Fire-and-forget — not yet answered.** Check `python3 scripts/review.py
inbox` (or `get 20260902T194120383Z-3860b1`) in a later session for the
verdict; nothing else in this report depends on it, but the recommendation
in §0 should be revisited once it lands.

## 4. What this does and does not settle

**Settled:** a rigid footprint's mobility is governed by width, sharply
rather than gradually, independent of height or cell count (§1.3); a single
wide part costs the same whether it leads or trails a monolithic body
(§1.4); shape at constant extent does not move findability at 36 cells any
more than it did at 9 (§2).

**Not settled, and not this session's to build:** whether a genuinely
decoupled (multi-part, each-follows-the-one-ahead) body actually reaches the
8-13% bucket for its narrow-part case, rather than merely being predicted to
by §1.3's atomic measurements — that needs the real mechanism, which is
`Reports/creature-genome-flexibility-2026-09-02.md` §13's Track B, explicitly
out of scope here per §12a (body and metabolism share one budget, and the
creature session is moving it now). Whether any of these six silhouettes
reads as an animal to the owner (§3, pending). Whether the width-3 threshold
found here on `rolling` holds on other presets.

**Not attempted, deliberately:** no literature (Toffin or otherwise) was
used as evidence that any mechanism here works — every number above came
from this engine, on this terrain, this session.

## 5. Instruments

Three new `examples/*.rs` files, none touching `src/` or `assets/`, all
listed in `Reports/instruments.md`: `creature_body_probe` (§1.3-1.4),
`creature_shape36_probe` (§2), `creature_candidate_render` (§3).
