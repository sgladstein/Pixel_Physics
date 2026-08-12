# Tree rewrite design: resolving `organism-substrate-design.md`'s `Divide` caveat, for real

**Audience:** the coding agent implementing this.
**Status:** design only, written just-in-time before implementation, per
`design-philosophy.md` §3's own explicit instruction ("the same way each of
the other four reports was written right before its subject was tackled").
**Revision 2**, after an independent design review of revision 1 found four
blocking problems and one moderate one — every finding is addressed below,
each one named explicitly at the point it's fixed rather than silently
folded in, per this session's own standing practice of not papering over
review findings. Companion to `organism-substrate-design.md` (§1's
`Divide`) and `design-philosophy.md` §3 (the committed direction this
report makes concrete).

---

## 0. The decision, stated first, corrected from revision 1

**`organism-substrate-design.md` §1 asked whether moss's discrete
grid-candidate `Divide` and a tree's continuous-position space-colonization
growth are "the same algorithm wearing different data," flagging that if
not, the honest outcome is splitting into two named behaviors rather than
forcing a false unification.**

**Revision 1's answer — force one `Divide` struct to cover both — did not
survive review.** The review's finding 6 is right: moss's semantics
(uniform-random candidate pick, then a chance gated on that candidate's
local moisture) and a tree's semantics (score-then-sample candidates by
direction bias, resource-gated) are different selection algorithms, not
the same one with more parameters — cramming both onto one struct either
breaks RON's all-fields-required deserialization or silently changes
moss's own already-shipped, already-tested behavior. **Revision 2 takes
`organism-substrate-design.md` §1's own pre-sanctioned fallback: `Divide`
stays exactly as it is today (moss's behavior, unchanged, `moss.ron`
unchanged), and a new named behavior, `Grow`, is added for
direction-biased, resource-gated candidate selection** — trees and roots
use `Grow`, not `Divide`. This is still one shared library, still
composable, still not tree-specific (any future species wanting directed,
resource-gated growth uses `Grow`; any future species wanting moss's
"random pick, chance gate" shape uses `Divide`) — it is a second reusable
behavior, not a bespoke tree mechanism, which is what
`organism-substrate-design.md` §1 actually asked to preserve.

**What's unchanged from revision 1, because the review found it sound:**
the core move away from continuous-position space colonization toward a
discrete, per-cell rule remains — `Grow`, like `Divide`, is dispatched
from a real `GrowingTip`/`RootTip` *cell*, on the M16 schedule, writing
directly into the grid. Nothing here reintroduces `Tip`/`RootTip`'s
private float-position state.

---

## 1. What gets deleted, and what replaces it — corrected retrofit order

Same deletion target as revision 1 (`Tip`, `RootTip`, `TreeState`,
`tree_tip_tick`, `root_tip_tick`) — **but revision 1's retrofit order
deleted this before the visual-verification gate that would catch a
broken result, which the review's finding 5 correctly flagged as removing
the fallback exactly when it's most likely to be needed.** Corrected
order, binding for this revision: **the old implementation stays in the
tree completely untouched through every implementation step, reachable
only via its own existing `plant_tree`/`T` key. The new implementation is
built and wired to a separate, explicit planting entry point
(`plant_tree_v2`/a temporary debug key) for its entire development and
verification. Only after live-verification screenshots (§11) confirm the
new system produces recognizably tree-like output does `T`/`plant_tree`
get repointed to the new system, and only *then* does the old code get
deleted.** This is a real, if temporary, duplication cost — accepted
deliberately as the price of an actual rollback path, not an oversight.

---

## 2. Direction and self-avoidance: two separate local signals, not one

Revision 1 proposed a `score()` blend of continuation/light/wind/upward
terms and called it done. **The review's finding 1 is right that this
silently deleted the old algorithm's actual space-filling mechanism** —
the attractor-consumption term that made tips avoid clustering and spread
to fill available volume — **without any replacement**, leaving a real
risk of tangled, overcrowded growth. Revision 2 restores that role with a
mechanism that is *more*, not less, aligned with `design-philosophy.md`
§0's own core primitive, rather than reintroducing private state to get it
back.

### 2a. Direction: local neighbour-average, not "the parent" — fixing the branch-point ambiguity

**The review's finding 3 is a real bug**: "grow away from the single
parent neighbour" is undefined the moment a tip has more than one
same-organism neighbour (any branch point, by construction, and more
generally whenever 2D growth curves enough to bring non-lineage-adjacent
organism cells into the same 8-neighbourhood). Revision 2's fix: **use the
vector average of every same-organism `Plant` neighbour's position, not a
single designated "parent."**

```
away_from_growth = -normalize( mean( pos(n) - pos(self) for n in same_organism_8_neighbours ) )
```

This is always well-defined (one neighbour reduces to the old "away from
the parent" case exactly; zero neighbours — the seed's first `Grow`, before
anything else exists — falls back to `(0, -1)`, straight up, same as
revision 1; two or more neighbours, including the exact branch-point case
the review named, average cleanly with no special-casing needed). No
stored direction, no stored "which neighbour is the lineage parent" tag —
still a purely local read of the grid's own current state, still nothing
`design-philosophy.md` §2d would call private per-organism simulation
state.

### 2b. Self-avoidance: a real deposit-diffuse-decay-follow channel, per the review's own suggested fix

**This directly implements the review's finding-1 recommendation**, which
is also the single cleanest application of `design-philosophy.md` §0's
stated primitive anywhere in this document. Every organism-owned `Plant`
cell (any cell type, not just `GrowingTip`) deposits a small, fixed amount
into a new **canopy-density** scalar on `Grow`-driven organisms, packed
identically to the resource scalar (§3) but tracked separately — see §6 on
where the extra bits come from. It diffuses and decays exactly like a
plant's resource channel does (§3's `diffuse_resource`, generic over any
tagged scalar, not resource-specific — reused for this with a different
decay rate, not a second diffusion implementation). `Grow`'s candidate
scoring gains one more term:

```
score(candidate_dir) =
      dot(candidate_dir, away_from_growth)         * CONTINUATION_WEIGHT
    + dot(candidate_dir, phototropism_dir)          * LIGHT_WEIGHT
    + dot(candidate_dir, wind_lean_dir)             * WIND_WEIGHT
    + dot(candidate_dir, (0, -1))                   * UPWARD_WEIGHT
    - canopy_density(candidate_pos)                 * CROWDING_WEIGHT
```

A candidate cell sitting in already-dense canopy scores lower, the same
functional role `KILL_DISTANCE`'s attractor consumption played, but as a
real shared/diffusing/decaying field a tip *reads*, not a private point
cloud a tip *consumes* — canopy density rises where growth has already
happened, diffuses outward (so the *avoided* zone has soft edges, not a
hard consumed/not-consumed boundary the way attractors did), and decays
over time (so old growth's crowding signal fades, letting later growth
reclaim space near mature wood if nothing nearby is still actively
growing — a property the old attractor system didn't have at all, since a
consumed attractor never came back). This is a genuine mechanism with its
own emergent consequences, not a reimplementation of the old one under a
different name.

---

## 3. Resource: growth-rate limiter, not a dominance mechanism — walking back revision 1's central claim

**The review's finding 2 is correct and revision 1's claim does not
survive it: plain diffusion is an equalizing operator.** It cannot, by
itself, turn small early variance into a sustained dominant leader — that
requires either active suppression or a positive-feedback (self-
reinforcing) transport mechanism, and revision 1 already, honestly,
disclaimed implementing either. Revision 2 **drops the "apical dominance
emerges from diffusion" claim entirely** rather than patch around it.

**What resource-gating actually does, stated at the honest strength the
mechanism supports:** `Grow`'s `resource < cost → skip this tick` gate
(the same shape `Divide` already uses) makes a tip's *growth rate* — not
its relative dominance — depend on locally available, diffusing,
decaying resource. A tip far from `Photosynthesize`d/`Absorb`ed sources,
or one sharing a crowded local resource pool with several siblings, grows
more slowly or stalls; a well-fed tip grows faster. **Whether this
produces one clearly dominant leader, several co-dominant branches, or a
fairly even canopy is left genuinely emergent and variable** — which is
an honest description of real tree form variation, not a gap. Nothing in
this design *claims* a single-leader outcome is guaranteed, correcting
revision 1's overreach. The combination that *does* still hold, and is
worth stating plainly: canopy-density avoidance (§2b) governs *where*
growth spreads (preventing tangling/overcrowding), and resource-gating
governs *how fast* it spreads there (preventing infinite, cost-free
growth) — two independent, honestly-scoped mechanisms, not one
overloaded claim.

Storage and transport are otherwise unchanged from revision 1:
`organism-substrate-design.md` §3's `diffuse_resource` (mirroring
`fire.rs`'s `diffuse_heat`), sourced by `Photosynthesize` (`Leaf` cells,
light field) and `Absorb` (`RootTip` cells, adjacent water — see §5 for
both of `Absorb`'s two mechanics, which the review's completeness check
found underspecified in revision 1), spent by `Grow`'s cost and
`SecondaryThicken`'s growth cost.

**What real auxin canalization actually is, and why this still doesn't
attempt it** (kept from revision 1, now correctly positioned as the
reason apical dominance *isn't* claimed rather than as a footnote after
claiming it anyway): real auxin transport is actively self-reinforcing —
channels that carry more flow become better conductors, a positive
feedback loop on conductance itself (Prusinkiewicz et al. 2009, PNAS,
already cited in `plant.rs`'s own module doc). Plain linear diffusion has
no such property. Implementing real canalization is out of scope here,
same as revision 1 said — the difference is revision 2 no longer claims
the weaker mechanism produces the stronger mechanism's signature effect.

---

## 4. The `GrowingTip → MatureBody` transition: made real, not just asserted

**The review's finding 4 is the most concrete bug found**: revision 1
described the transition in prose but the cited mechanism (the staleness
counter reaching its limit) only stops rescheduling in the current code —
it never rewrites the cell's own type. Left as revision 1 described it,
`StructuralAnchor` and `SecondaryThicken` (both gated on `CellType::
MatureBody`) would never fire on anything, which the review correctly
traced through to "trees could visibly break apart or never stabilize."

**Fixed explicitly:** the same tick that decides not to reschedule a
`GrowingTip` (staleness limit reached, or — new in this revision — every
8-neighbour candidate scores below a minimum threshold for a full
staleness-window's worth of checks, whichever condition the implementation
finds cleaner to track) **must call `world.set` to rewrite that cell's
`aux` to `CellType::MatureBody`**, carrying its current resource value
forward (`pack_aux(CellType::MatureBody, resource)`), before returning no
reschedule. This is not a new mechanism — `Divide`'s own dispatch already
demonstrates the "write the new type into `aux`, `world.set` it" pattern
for a freshly-created cell; the fix is applying that exact pattern to the
*existing* cell at its own staleness transition, which revision 1's prose
skipped. Call out explicitly in the implementation's own test suite: a
test that runs a tree to full canopy staleness and asserts a nonzero count
of `MatureBody`-typed cells exist afterward — the review's finding is
exactly the kind of gap that reads as "grew fine" in a shallow test and
only shows up once something (structural integrity, thickening) actually
depends on the type having changed.

---

## 5. `Absorb`'s two mechanics, and the tip-count cap — closing the review's remaining completeness gaps

**Both of the review's supplementary findings, addressed explicitly rather
than left implicit:**

**`Absorb` covers both of the old code's water-uptake mechanics**, kept
as two distinct checks rather than merged into one, since they're genuinely
different (one never moves the tip, one does):

1. **Drink-in-place**: every tick, regardless of whether `Grow` itself
   fires, a `RootTip`/`Leaf`-adjacent-to-water cell with `Absorb` drains
   every adjacent `Liquid`-kind neighbour directly (unchanged from the old
   code's neighbour-scan-before-moving step), crediting resource and
   depleting local moisture (`World::deplete_moisture`, already generic,
   unchanged).
2. **Grow-into-water**: `Grow`'s own candidate evaluation, for a
   `RootTip`-typed cell only, treats a `Liquid`-kind candidate as a third
   case alongside empty-and-open and blocked — absorbed on the spot
   (credits resource, depletes moisture, same as case 1) and the tip
   *advances into the now-empty space* rather than writing a body cell
   there, exactly the old `root_tip_tick`'s `target_kind` match arm.
   `Grow`'s own doc should name this as a `RootTip`-only branch, not a
   general property of every species using `Grow` — a canopy `GrowingTip`
   growing into water should behave as blocked, matching the old
   `tree_tip_tick`'s unconditional "blocked by any non-empty cell"
   check, roots being the one case where growing into a liquid is
   absorption rather than obstruction.

**A per-organism active-`GrowingTip`/`RootTip` cap restores
`MAX_TIPS_PER_TREE`/`MAX_ROOTS_PER_TREE`'s role**, silently dropped in
revision 1. Concretely: `Grow`'s branch-into-two-cells path (creating a
second new tip alongside the first, per the branch-chance roll) checks a
new per-species `max_active_tips`/`max_active_roots` parameter against a
count of currently-scheduled `GrowingTip`/`RootTip` active sites for that
`organism_id` before allowing the second cell — the same cap the old code
enforced (`tips.len() < MAX_TIPS_PER_TREE`), now read from the organism's
own live schedule rather than a `Vec` length, since there is no `Vec` of
tips left to measure. This requires a cheap per-organism count, which
`World`'s active-site structures don't currently expose — implementation
will need to decide whether that's a linear scan of due-soon sites (likely
fine at this scale, dozens of tips per tree, not thousands) or a small
per-`organism_id` counter maintained alongside `push_organism`/organism
state, a genuinely implementation-level call not worth deciding on paper
here.

---

## 6. `Cell::aux` layout: one more scalar than revision 1 planned for

`organism-substrate-design.md` §2's layout (4 bits cell type, 8 bits
resource) has no room left for canopy density (§2b) as a *second* per-cell
scalar packed the same way. Two honest options, an implementation-time
choice rather than a design one: **(a)** halve the resource scalar's
precision (4 bits each for resource and canopy-density, still `u16`-sized
`aux`, coarser fixed-point on both), or **(b)** track canopy density as a
*field*-like second byte-array keyed by position, parallel to how
`field.rs`'s `FieldTile` already tracks per-chunk scalars, rather than
packed into the cell itself — arguably a better fit anyway, since canopy
density is explicitly a *diffusing, decaying, spatial* signal, exactly
`field.rs`'s own domain, and `diffuse_resource`'s per-cell CA-resolution
diffusion (§3, kept deliberately *not* field-grid-resolution because
`FIELD_SCALE = 8` would smear a one-cell branch) is a different concern
from canopy density's own resolution needs — canopy density plausibly
*can* tolerate `FIELD_SCALE`-level coarseness (it only needs to answer
"is this general area already crowded," not "is this exact cell full"),
where per-cell resource transport cannot. If so, canopy density might
belong on `field.rs`'s existing grid after all, sidestepping the `aux`
bit-budget question entirely. **Flagged as the one remaining open
question this document does not resolve** — worth 15 minutes of thought
at implementation time before picking (a) by default, since (b) may be
both simpler to fit and a better architectural home.

**Correction from the second review, before either option is picked:**
option (b) is not actually a symmetric tradeoff against (a) as written
above — it needs one more piece to work at all. `Grow`'s crowding term is
read once per candidate among a single tip's own 8-neighbourhood, cells
that almost always sit inside one `FIELD_SCALE = 8` field block; a
block-nearest read of a coarse grid would make all 8 candidates see the
*same* canopy-density value, silently disabling the directional
self-avoidance effect entirely — the exact resolution mismatch
`organism-substrate-design.md` §3 already rejected `field.rs`'s grid for
in the first place, for `diffuse_resource`. This is fixable — `plant.rs`
already establishes the fix for this identical class of problem
(`field_at_bilinear`, used for phototropism/wind/moisture precisely
because block-nearest reads make nearby candidates compare equal) — but
if option (b) is chosen, bilinear sampling is a required part of that
choice, not a separate later optimization.

---

## 7. `CellType`, `structural.rs` integration — unchanged from revision 1, review found these sound

`CellType` grows from `GrowingTip` alone to `Seed | GrowingTip |
MatureBody | Leaf | RootTip`, matching `organism-substrate-design.md` §2's
original sketch exactly — the review raised no objection to this part.
`structural::tick`'s new organism branch (bounded reachability from a
tree's own `RootTip` cells via the already-built, already-tested
`reachable_from_anchors`) is unchanged from `organism-substrate-design.md`
§2/§5 and revision 1 — also unchallenged by review. `SecondaryThicken`'s
downstream-`Leaf`-count flood fill, same primitive again, unchanged.

---

## 8. `tree.ron`, revised for the `Divide`/`Grow` split

```ron
(
    name: "tree",
    cell_types: [
        (Seed, [
            Germinate(light_threshold: 0.1, moisture_threshold: 0.0, instant: false),
        ]),
        (GrowingTip, [
            Grow(
                cost: 4.0,
                branch_chance: 0.12,
                continuation_weight: 0.75,
                light_weight: 0.35,
                wind_weight: 0.2,
                upward_weight: 0.0,
                crowding_weight: <new, tuned empirically>,
                max_active_tips: 14,        // was MAX_TIPS_PER_TREE
            ),
        ]),
        (RootTip, [
            Absorb(rate: 5.0),
            Grow(
                cost: 1.5,
                branch_chance: 0.0,         // roots branch on the oscillator-prime condition, not Grow's own roll -- see revision 1's still-open note on this, unchanged
                continuation_weight: 0.7,
                light_weight: 0.0,
                wind_weight: 0.0,
                upward_weight: -1.0,        // gravitropism default; overridden by the MIZ1 hydrotropism switch
                crowding_weight: 0.0,       // roots don't self-avoid the way canopy does -- no citation for root-root avoidance in the research this engine already cites, so not invented here
                max_active_tips: 10,        // was MAX_ROOTS_PER_TREE
            ),
        ]),
        (MatureBody, [
            SecondaryThicken(pipe_ratio: <new, tuned empirically>),
            StructuralAnchor,
        ]),
        (Leaf, [
            Photosynthesize(rate: <new, tuned empirically>),
        ]),
    ],
)
```

`moss.ron` is **unchanged** from what's already shipped — `Divide` keeps
its exact current shape and parameters, per §0's resolution.

---

## 9. Flexibility: `Grow`'s parameter surface across genuinely different plant forms

**Added at the owner's explicit request, made a checkable design
commitment rather than an assumption**: the whole point of composing
species from a fixed behavior library (`organism-substrate-design.md` §1's
original thesis) is that a materially different plant silhouette should
require only a new `.ron` file, never a new Rust match arm. `tree.ron`
(§8) exercises one point in `Grow`'s parameter space; the sketches below
exercise three more, deliberately chosen to be structurally distinct from
a tree and from each other — not just "a smaller tree" — as the actual
test of whether the parameter surface spans real diversity or only
resembles it. None of these are commitments to build in this pass (§11's
retrofit order stays tree-then-worm); they exist to pressure-test the
design on paper before implementation, the cheapest point to catch a
parameter that turns out not to generalize.

**A vine** — creeps along a surface rather than growing free-standing;
real vines solve support the way real vines do, by climbing, which this
model doesn't attempt, so this sketch is scoped to *shape* (low,
horizontal, wall-hugging) not climbing mechanics:
- `upward_weight` near `0.0` (no gravitropic pull), `continuation_weight`
  high (a vine holds a direction once established, more than a tree's
  canopy does), `crowding_weight` *low* (real vines tolerate — often
  prefer — growing directly alongside their own earlier growth, the
  opposite of a tree canopy's self-avoidance).
- `light_weight` high (strong phototropism is exactly how a real vine
  finds a lit surface to hug).
- No `RootTip`/`Absorb` cell type at all if the sketch is an air-plant/
  epiphyte-style vine with no root system — `Grow`'s dependency on
  `Absorb`-sourced resource is per-species opt-in (a species simply
  doesn't define a `RootTip` cell type), not a hardcoded requirement,
  confirming §0's "not tree-specific" claim for the *absence* of a
  mechanism, not just its presence.
- **Correction from the second review: this rootless case is not actually
  free, and the claim below that all three sketches need zero new code is
  wrong for this one specifically.** §7's structural integration (carried
  over from `organism-substrate-design.md` §2/§5 unchanged) anchors an
  organism's reachability search on its own `RootTip` cells. A rootless
  vine has none, so under that rule it has zero anchors and can never
  validate as structurally sound — not a subtle edge case, an
  organism-wide failure for exactly the species this sketch proposes.
  Two honest ways to close this, neither decided here: extend the anchor
  rule to also accept any organism-owned `Plant` cell directly adjacent
  to `Solid` (mirroring `has_growable_neighbour`'s own existing
  Solid-adjacency check in `plant.rs`, so a vine anchors wherever it
  physically touches a wall, the real-world analogue of what a root does
  for a tree), or accept that a rootless species is genuinely out of
  scope until that extension is built. Either is fine; silently assuming
  the existing `RootTip`-only rule already covers it is not — flagged
  here so a future pass building this sketch doesn't rediscover the gap
  from a broken vine instead of from this paragraph.

**A shrub** — squat and densely-branching rather than tall with a clear
trunk:
- `branch_chance` much higher than `tree.ron`'s `0.12`, `max_active_tips`
  much higher (a shrub reads as "many stems," which is exactly what a
  higher simultaneous-tip cap produces from the *same* branching
  mechanism, no new code).
- `upward_weight` weakly positive rather than the tree's near-neutral
  value (a shrub's overall silhouette leans less vertical), `crowding_
  weight` low-to-moderate (a shrub is expected to look bushy/dense, not
  avoid itself as strongly as a canopy tree does) — the same
  crowding-avoidance mechanism from §2b, just tuned to tolerate more
  density, not a different mechanism for "dense" vs. "sparse" growth.
- `SecondaryThicken`'s `pipe_ratio` set so multiple stems stay thin rather
  than one dominant trunk thickening — `organism-substrate-design.md` §4's
  own point that `pipe_ratio` is a per-species parameter precisely because
  real plants vary here, not a universal constant, doing real work in this
  sketch rather than being a formality.

**A cactus-like succulent** — slow, sparse, single-spire growth:
- `cost` much higher than a tree's `4.0` (deliberately slow growth —
  the same resource-gate mechanism §3 already uses to modulate *rate*,
  not a new "slowness" concept), `branch_chance` at or near `0.0` (a
  single spire, not a canopy), `light_weight`/`wind_weight` low (real
  succulents are less phototropically/wind-responsive than a broad-leafed
  canopy — a judgement call the way every other cross-species weight in
  this document is, not a cited number).
- No `Leaf` cell type — photosynthesis happens directly on `MatureBody`
  or `GrowingTip` cells instead (a species is free to attach
  `Photosynthesize` to whichever cell type makes sense for its own form;
  nothing in `Grow`/`Photosynthesize`'s own code assumes leaves are a
  separate cell type from stem, that's purely `tree.ron`'s own choice,
  not a constraint the library imposes).

**What this check actually confirms, and what it doesn't.** The shrub and
cactus sketches reuse exactly `Grow`, `Absorb`, `Photosynthesize`,
`SecondaryThicken`, `Germinate`, `StructuralAnchor` with zero new behavior
variants and zero new Rust code, every difference expressed as parameter
values and which cell types a species defines — the concrete, checkable
version of "flexible enough for a diverse array of plants." **The vine
sketch does not clear that bar as cleanly, and its own paragraph above
says so rather than this summary quietly excepting it**: a rootless
organism needs either a real (if small) extension to the anchor rule or
an accepted scope limit, so "zero new code" is true for two of the three
sketches, not all three — a more honest, and more useful, result than
three-for-three would have been, since it caught a real gap the same way
the sketches were meant to. Separately, none of these three should be
treated as validated until (if) a future pass actually builds and
screenshots one — that's an empirical question §11's live-verification
step exists for, same as it is for the tree itself.

---

## 10. Where the ported field-read functions live — unchanged from revision 1

`phototropism_dir`, `wind_lean_dir`, `moisture_pull` move from `plant.rs`
private functions to `organism.rs` as shared helpers, available to
`Grow`'s dispatch regardless of species — unchanged from revision 1, not
challenged by review.

---

## 11. Retrofit order for this pass, corrected per finding 5

1. Extend `organism.rs`: `CellType` (5 variants), the new `Grow` behavior
   (§0, §2, §3 — direction scoring, resource gate, canopy-density term),
   `Photosynthesize`/`Absorb`/`TransportChannel`/`SecondaryThicken`/
   `Germinate`/`StructuralAnchor`, `diffuse_resource` generalized to carry
   both the resource and canopy-density channels (§6's open question
   resolved first, since it affects this step directly), the ported
   field-read helpers. `Divide` untouched. Tests at this layer first, pure
   functions, cheapest to verify in isolation.
2. `tree.ron`. Parse-and-load test only.
3. Implement `Grow`'s dispatch (the direction-average rule from §2a, the
   canopy-density term from §2b, resource gating from §3, the explicit
   `MatureBody` transition from §4, the tip-count cap and both `Absorb`
   mechanics from §5) as `organism_tick`'s new branch alongside `Divide`'s
   existing one — **not replacing `plant_tree`/`tree_tip_tick` yet.**
4. `Germinate` on `Seed` cells; a new `plant_tree_v2` (or equivalent
   debug-only entry point, per §1) plants a `Seed` cell and schedules its
   germination check, running entirely in parallel with the old,
   untouched `plant_tree`.
5. `SecondaryThicken`, `StructuralAnchor`, `structural::tick`'s organism
   branch wired to `plant_tree_v2`'s trees as a real caller.
6. **Live-verification screenshots of `plant_tree_v2`'s output** — a tree
   grown from a seed, ticked forward, checked visually for: a connected
   trunk-to-canopy structure, believable (if not necessarily
   single-leader) branching, no visible tangling/overcrowding artifacts
   the canopy-density term should be preventing, roots that plausibly
   seek water and fail sensibly on bare stone. Compared side by side
   against a `plant_tree`-grown (old-system) tree in the same scene.
   Committed under `docs/screenshots/`, per this session's established
   practice — this is the actual gate, not a formality, given findings 1
   and 2 above mean "does this read as a tree" was a real open question
   revision 1 didn't earn the right to skip past.
7. **Only once step 6 passes**, repoint `T`/`plant_tree` to the new
   system, port/rewrite every existing `plant.rs` tree test against the
   new emergent shapes (not simply deleted), then delete
   `Tip`/`RootTip`/`TreeState`/`tree_tip_tick`/`root_tip_tick`/
   `plant_tree_v2`'s now-redundant separate entry point/
   `World::trees`/`push_tree`/`tree`/`tree_mut`.
8. Independent review before commit, per standing practice for a change
   this size — a second review, specifically re-checking findings 1, 2,
   3, and 4 above are actually resolved in the implementation, not just
   on paper.
9. The worm (`creature.rs` → `Locomote` on `organism.rs`) remains a
   separate, smaller pass after this one lands, unchanged from revision 1.

**What this pass does not attempt**, unchanged from revision 1: real
auxin canalization's self-reinforcing channel conductance (now correctly
positioned in §3 as the reason apical dominance isn't claimed, not
mentioned only in passing); Palubicki-style shadow-voxel light competition
between branches; a resistance-network transport solve. All three remain
out of scope per `organism-substrate-design.md` §7's own list.
