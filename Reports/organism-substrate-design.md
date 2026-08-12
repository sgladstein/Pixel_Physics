# Organism substrate design: retiring `TreeState`/`CreatureState` for a shared, cell-typed model

Companion to `research/m16-plant-biology.md` and `research/m18-creature-biology.md`
(the biological grounding already cited by `plant.rs`/`creature.rs`) and to
`emergent-world-architecture.md` (the thin-organism/rich-world thesis this
document exists to extend to a third case). Read this before touching
`plant.rs`, `creature.rs`, or `structural.rs` — it explains what's being
replaced, why, and the two places the literature forced a real decision
rather than a straightforward translation.

**Why now, not at M16/M18 time.** `plant.rs` and `creature.rs` each solved
their own version of "per-organism state too big for a `Cell`" privately —
`TreeState` (attractor list, positional-float tips and roots, one energy
scalar) and `CreatureState` (one energy scalar) are structurally unrelated
despite representing the same underlying problem. `emergent-world-
architecture.md` §0's own words: *"Deep simulation is a local optimum that
is hard to climb out of."* Stigmergy needs this fixed before ants can be
built at all — an ant colony is exactly the shape `CreatureState` already
is (a chain of cells with shared per-colony state), and extending
`CreatureState` again rather than building the shared model would be the
third private solution to the same problem, permanently.

---

## 1. Species as data, parallel to materials, not inside the registry

**Decision, from the earlier planning-session discussion this report was
commissioned to settle:** a new `species/` registry (`assets/species/*.ron`,
hot-reloaded via the same `notify` pattern `MaterialRegistry` already uses),
architecturally parallel to `MaterialRegistry` rather than merged into it.

**Why parallel and not merged.** A material answers "how does this pixel
move and react" — a question every cell in the world asks, whether or not
it belongs to anything. A species answers "what is this organism, and what
does *this* cell of it do right now" — a question that only makes sense for
a cell with a nonzero `organism_id`. Folding species into `MaterialRegistry`
would mean every material lookup (millions per frame, on the CA sweep's hot
path) carries fields that are meaningless for sand, water, and stone. Kept
separate, `MaterialRegistry` stays exactly what it already is — data for
*substance* — and `SpeciesRegistry` is data for *organism*, looked up only
by the M16-style active-site scheduler, which already runs at
organism-count frequency, not cell-count frequency.

**Why reusability was the harder requirement, and how it's met.** The
brief asked for composable, reusable named behaviors — moss and a tree
sharing code, not two independent growth functions that happen to look
similar the way `moss_tick`/`tree_tip_tick` do today. The answer is a
fixed library of named behaviors (`src/sim/organism.rs`), each one a small
struct with its own parameters, and a species `.ron` file is nothing but a
list of `(cell_type_name, [behavior instances])` pairs. `tree.ron`'s
`leaf` type gets `[Photosynthesize { rate }, TransportChannel { decay }]`;
`moss.ron`'s one cell type gets `[Photosynthesize { rate: lower },
Divide { ... }]` — the *same* `Photosynthesize` code, different data. This
is the material-file precedent applied one level up: `material.rs`'s own
doc says behaviour comes from `kind` plus numeric parameters so adding
material never means adding a branch to the update loop; the organism
substrate makes the identical claim for species.

The full library this section refers to throughout: `Photosynthesize
{ rate }` (reads the light field, credits the resource scalar);
`Absorb { channel, rate }` (root-tip water uptake, `ROOT_WATER_ENERGY`'s
existing mechanism relocated onto generic data); `TransportChannel
{ decay }` (participates in §3's diffusion pass); `Divide { direction_bias,
cost, cooldown, branch_chance }` (creates a new same-`organism_id` cell —
see the honest caveat on this one below); `SecondaryThicken { pipe_ratio }`
(§4); `Germinate { light_threshold, moisture_threshold, instant }` (a
`Seed` cell's transition to `GrowingTip`/`RootTip`, checked on a schedule
against local field readings; `instant: true` is a test-only escape hatch
that fires unconditionally next tick, avoiding germination-condition
waits in every other test that just needs a grown organism to exist);
`StructuralAnchor` (marks a cell type as counting toward `is_body_material`,
no behavior of its own — a tag other systems read); `Locomote
{ move_cost_fn }` (the worm's existing `move_cost` relocated the same way
`Absorb` relocates root drinking).

**An honest caveat on `Divide`, the one behavior asked to do the most
divergent work.** Moss's growth (§7) is a discrete pick among 4-neighbour
candidates — `moss_tick`'s existing shape, unchanged in spirit. A tree's
growth (`tree_tip_tick`, ported in §7) is continuous-position space
colonization: a floating-point tip position and direction, a shared
`attractors` list consumed across tips, a per-tip `channel` scalar
competing for a shared energy pool. These are not the same algorithm
wearing different data — a single `Divide` implementation cannot cover
both without an internal mode split (a discrete grid-candidate mode for
moss-shaped species, a continuous space-colonization mode, carrying the
tip's own float position/direction/channel as extra per-tip state
`Divide`'s own parameters don't otherwise need), and this report is not
going to claim that split is trivial or fully worked out here. Flagged
explicitly as the biggest concrete risk in §7's retrofit order, not
papered over: if the two modes turn out not to share enough real code to
justify one behavior name, the honest outcome is splitting `Divide` into
two named behaviors (a plain grid one, a space-colonization one) rather
than forcing a false unification — a decision for the implementation
session, made with this risk named in advance rather than discovered
partway through.

---

## 2. `Cell::aux`: cell type, resource scalar, and the conflict this creates for two different reasons on two different kinds

The proposed layout, for a cell that is actually organism tissue:

- bits 0-3 — cell type (`Seed | GrowingTip | MatureBody | Leaf | RootTip`,
  room for 11 more, shared vocabulary across every species rather than a
  per-species enum, so the CA/scheduler code that dispatches on cell type
  never needs to know which species it's looking at)
- bits 4-11 — resource scalar, `u8` mapped 0-255 → 0.0-4.0 — a new
  fixed-point encoding, not one already in use elsewhere in this codebase:
  `Cell::temperature` is a raw unscaled `i16` and every M13 `FieldCell`
  channel is a plain `f32`, so this is a genuinely new pattern being
  introduced here, not a precedent being followed. Worth calling out
  explicitly since it's easy to assume otherwise.
- bits 12-15 — spare

**This only applies to organism-owned cells (`organism_id != 0`).** A
cell's `MaterialKind` and its organism ownership are separate, orthogonal
properties — `wood` is `Plant`-kind whether or not anything owns it, and
the material system has no concept of "belongs to organism 7." Concretely:

- **`organism_id == 0`** — inert material, whatever its `MaterialKind`.
  Hand-painted wood (the brush can paint any material, `wood` included,
  with no tree behind it) and a fully-reclaimed dead tree's former trunk
  (§6: `organism_id` reclaimed, cells left behind as ordinary wood) are
  both this case. `aux` keeps its *current*, pre-rewrite meaning
  unconditionally: anchor distance for `Solid`/`Plant`, unused for
  everything else. `structural.rs`'s existing incremental relaxation is
  completely untouched for these cells — this is the case the earlier
  draft of this section described as "stone and wood keep working exactly
  as today" without stating the condition that actually makes it true.
- **`organism_id != 0`** — living organism tissue, `Plant` or `Creature`
  kind. `aux` switches to the cell-type-plus-resource layout above.
  `structural.rs`'s aux-cached relaxation is *not* available to these
  cells (their `aux` no longer holds a distance at all) — see the
  reachability-search replacement below. `Creature` cells are always in
  this case: unlike wood, there is no "hand-painted worm" — every
  `Creature`-kind cell in the current codebase already carries an owning
  id in `aux` (`creature.rs`'s existing creature-index scheme), so
  `organism_id` simply takes over the job `aux` used to do for `Creature`
  cells specifically, freeing that kind's `aux` for the same
  cell-type-plus-resource layout `Plant` gets, with no conflict at all —
  a cleaner case than `Plant`'s, not a harder one, because `Creature` had
  no "unowned" case to preserve in the first place.

**`structural::tick` gains exactly one new branch to dispatch on this**:
today it treats every `is_body_material` cell (`Solid | Plant`)
uniformly; after this change it checks `organism_id` first — `0` keeps
the existing aux-relaxation path verbatim, nonzero routes to the
reachability search below instead. Scheduling is unchanged either way:
the same `schedule_structural_check`/`schedule_structural_check_around`
call sites (the brush, an explosion, fire burnout) feed both paths, so no
new `ActiveKind` or new `World` method is needed — only `structural::
tick`'s own body branches.

**Decision: for `organism_id != 0` Plant cells, structural integrity
moves off the per-cell cache entirely, onto the organism's own side-state
plus an event-triggered bounded reachability check** — not a smaller
cached distance stolen from the spare bits, and not a global recompute.
Concretely: each organism's side-state (§5 below) already tracks its own
root/anchor cell positions, the same way `TreeState::roots` does today;
`structural::tick`'s new branch, on any organism-owned `Plant` cell,
schedules a bounded BFS *from that organism's own anchors*, capped the
same way `SecondaryThicken` already needs to be capped (§4's
`MAX_THICKEN_SCAN_CELLS`), and any `Plant` cell of that `organism_id`
unreached within the cap breaks free exactly as the existing aux-relaxation
path already does for an unsupported `Solid` span.

This is a real, deliberate divergence from `Solid`'s (and unowned
`Plant`'s) strategy, not an oversight: `structural.rs`'s distance-
relaxation is incremental specifically *because* it has somewhere to
cache the last-known distance per cell, letting a disturbance's cost stay
local to the neighbourhood that actually changed. An organism-owned
`Plant` cell has nowhere left to cache that number once its `aux` is
spent on cell-type-plus-resource, so the alternative has to be an
on-demand bounded search instead of an O(1) relaxation step — acceptable
because organism structures are small relative to
`MAX_THICKEN_SCAN_CELLS`-scale terrain spans and the search only ever
runs in reaction to an actual disturbance, never speculatively.

---

## 3. Transport: explicit per-cell diffusion, and precisely what it is *not*

**Decision, restated from the plan and grounded here:** `organism::
diffuse_resource<S: CellSurface>` — per-cell finite-difference diffusion
between same-`organism_id` `Plant` neighbours, the same shape `fire.rs`'s
`diffuse_heat` already uses (generic over the CA sweep's serial and
parallel drivers via `CellSurface`, reusing the existing Fourier stability
clamp rather than re-deriving it), **not** `field.rs`'s coarse
`FIELD_SCALE`-resolution grid. A different-organism or non-`Plant`
neighbour is a wall, exactly as `diffuse_heat` treats a material boundary.
The reason to reject the field grid specifically: `emergent-world-
architecture.md` §6 and this session's own §6 (explosion debris) both
independently found the same resolution mismatch — a coarse `FIELD_SCALE
= 8` block would smear a one-cell branch's resource level into seven
neighbouring cells that have nothing to do with it. `fire.rs`'s per-cell
diffusion has no such mismatch, because it operates at the CA grid's own
resolution.

**What real transport actually is, and why per-cell diffusion is a named
simplification of it rather than an approximation nobody checked.** The
mechanism plants actually use for sugar transport is the **Münch
pressure-flow hypothesis** (Münch 1930, *Die Stoffbewegungen in der
Pflanze*), confirmed with direct pressure measurements by Knoblauch et al.
(2016), *"Testing the Münch hypothesis of long distance phloem transport
in plants,"* eLife 5:e15341
([elifesciences.org/articles/15341](https://elifesciences.org/articles/15341)):
source (photosynthesizing) tissue actively loads sugar into sieve tubes,
osmotically drawing water in and raising local turgor pressure; sink
(growing/storage) tissue unloads sugar and drops local turgor. Knoblauch
et al. measured source turgor at ~1.08 MPa against sink turgor at ~0.59
MPa — a real, physical pressure differential, not a metaphor — and flow
follows a Hagen-Poiseuille-style *F = (P_source − P_sink) / R* through a
resistance network. See also De Schepper et al. (2013), *"Phloem
transport: a review of mechanisms and controls,"* J. Exp. Bot. 64(16):
4839-4850
([academic.oup.com/jxb/article/64/16/4839/593231](https://academic.oup.com/jxb/article/64/16/4839/593231)).

**The precise claim this report is making:** diffusion gets directionality
right for the same reason the real mechanism does — resource flows from
where it's produced/abundant to where it's consumed/scarce — but it drops
the actual physics entirely. There is no pressure state, no resistance
network, no distinction between "the pipe is narrow so flow is slow" and
"the gradient is shallow so flow is slow." `SecondaryThicken` (§4) growing
a wider trunk under more downstream leaf load is the one place this
engine's model gestures at the missing physics (a real xylem vessel's
conductance does scale with its cross-section), without ever computing a
resistance or a pressure.

**The tier deliberately not attempted, cited so it's a decision and not an
oversight.** Two functional-structural plant models (FSPMs) solve the real
coupled system on an explicit architecture:

- **L-PEACH** — Allen, Prusinkiewicz & DeJong (2005), *"Using L-systems for
  modeling source-sink interactions, architecture and physiology of
  growing trees: the L-PEACH model,"* New Phytologist 166(3): 869-880
  (PubMed record:
  [pubmed.ncbi.nlm.nih.gov/15869648](https://pubmed.ncbi.nlm.nih.gov/15869648/)).
  Grows the tree's explicit branching structure and, every step, solves a
  system of differential equations for carbohydrate flow across that same
  structure — an electric-circuit analogy where each organ is a node and
  each internode a resistive pathway, i.e. Münch's mechanism laid directly
  onto explicit per-branch geometry, with local growth gated by locally
  available carbohydrate.
- **MuSCA** — Reyes, Pallas, Pradal et al. (2020), *"MuSCA: a multi-scale
  source-sink carbon allocation model to explore carbon allocation in
  plants,"* Annals of Botany 126(4): 571-585
  ([academic.oup.com/aob/article/126/4/571/5603521](https://academic.oup.com/aob/article/126/4/571/5603521)).
  Runs the same source-sink computation over the same tree at any of five
  chosen topological scales, quantifying how allocation results and
  compute cost change with resolution — coarsening cut compute by up to
  four orders of magnitude in their own measurements.

This engine fixes one scale (per-cell) and one mechanism (diffusion, no
pressure state) — the coarsest point on the spectrum both of those papers
exist to characterize, chosen for the same reason `plant.rs`'s existing
module doc gives for every other simplification in it: real-time,
interactive, dozens-to-hundreds of simultaneous organisms, not an offline
horticultural solve for one tree.

---

## 4. Secondary thickening: the pipe model, precisely, including its own documented limits

**Shinozaki's pipe model theory** — Shinozaki, Yoda, Hozumi & Kira (1964),
*"A quantitative analysis of plant form — the pipe model theory,"*
Japanese Journal of Ecology, a two-part paper: part I, pages 97-105
("basic analyses"), and part II, pages 133-139 ("further evidence of the
theory and its application in forest ecology") — verified at
[jstage.jst.go.jp/article/seitai/14/4/14_KJ00001775220/_article](https://www.jstage.jst.go.jp/article/seitai/14/4/14_KJ00001775220/_article),
which is part II; no independently verified link for part I specifically
was found, so cite part II's DOI record when tracking this down rather
than assuming the same URL pattern resolves to part I.
**The precise claim:** treating a plant as a bundle of unit-cross-section
pipes each terminating in a fixed leaf quantity, the leaf mass/area
supported above any point is proportional to the sapwood cross-sectional
area at that point, with a proportionality constant (Shinozaki's "specific
pipe length," *L*) taken as constant within one tree at one time.

**The limitation this report is not permitted to omit**, since it directly
bears on whether a single global ratio constant is faithful to the source:
Lehnebach, Beyer, Letort & Heuret (2018), *"The pipe model theory half a
century on: a review,"* Annals of Botany 121(5): 773-795
([pmc.ncbi.nlm.nih.gov/articles/PMC5906905](https://pmc.ncbi.nlm.nih.gov/articles/PMC5906905/)),
states plainly that *L*'s constancy "should not be considered as merely
species-specific" — real trees range continuously from fully-sectored to
fully-integrated vascular systems (hydraulic sectoriality, a separate
axis of variation the review discusses alongside, not derived from, the
leaf-to-sapwood ratio), that ratio is measurably non-linear in many
species, it shifts with the plant's own age, and Shinozaki's original
datasets did not even account for branch order. **Implementation
consequence:** `SecondaryThicken { pipe_ratio }`'s ratio is faithful to
the real theory precisely by being a per-species *parameter*, not a
universal constant baked into the behavior — `tree.ron` and any future
woody species each own their own `pipe_ratio`, and no code should ever
assume one number applies across species. That the real relationship is
tree-local and only roughly stable over short timescales is the theory's
own documented limitation, not a corner this engine's translation of it
cuts.

**Translation to the engine:** `SecondaryThicken`, on a `MatureBody` cell,
every `THICKEN_CHECK_INTERVAL` (60 frames) counts downstream `Leaf` cells
of the same `organism_id` through connected `Plant` neighbours — a bounded
flood fill (`MAX_THICKEN_SCAN_CELLS` = 2000, the same order-of-magnitude
cap `structural.rs`'s own worst-case neighbourhood traversal already
implies is safe per reactive check), growing sideways into adjacent
`Empty`/displaceable cells once `leaf_count / current_width > pipe_ratio`.

---

## 5. Connectivity: correcting a premise, then the shared primitive

**A correction to the plan text this report was commissioned against.**
The plan called for "factoring `structural.rs`'s BFS-from-anchors into a
generic primitive." Read against the actual code, `structural.rs` does
not run a BFS at all — it is an **incremental local relaxation**: each
`Solid`/`Plant` cell's own `aux` caches its last-known distance to the
nearest anchor, and `structural::tick` recomputes one cell's distance as
`min(neighbour distances) + 1` from its 4-neighbours' *currently cached*
values, rescheduling only the neighbours whose own answer might now be
stale. This is closer to a distributed Bellman-Ford relaxation than a
graph search — deliberately, since it is what lets a single disturbance's
cost stay local to the affected neighbourhood rather than re-walking the
whole structure every time. There is no full-graph BFS anywhere in the
current codebase to extract.

**Why this correction matters for the organism substrate.** §2 already
established that `Plant` cells cannot keep the aux-cached half of this
strategy — their `aux` is spent on cell-type-plus-resource. So the
"shared primitive" the plan asked for cannot literally be "the same
function, reused" (the two callers need different storage strategies for
the reason §2 gives); it has to be a primitive that both `Solid`'s
incremental relaxation *and* the organism substrate's on-demand bounded
search can be built from, sharing the traversal shape (walk `is_body`
neighbours, respect a `same_group` boundary) without sharing the caching
strategy.

**The actual shared primitive:**

```
fn reachable_from_anchors<S: CellSurface>(
    surface: &S,
    anchors: impl IntoIterator<Item = (i32, i32)>,
    is_body: impl Fn(Cell) -> bool,
    same_group: impl Fn(Cell, Cell) -> bool,
    cap: usize,
) -> HashSet<(i32, i32)>
```

A bounded BFS from a caller-supplied anchor set, walking only cells that
pass `is_body` and share `same_group` with the anchor that reached them,
capped at `cap` cells visited. `structural.rs`'s M17 caller passes bedrock
positions as anchors, `is_body_material`'s existing `Solid | Plant` check,
and a `same_group` that accepts everything (structural integrity for
inert terrain has no organism boundary to respect) — used only as a
*verification* pass if one is ever wanted, since the incremental relaxation
remains the actual per-frame mechanism for `Solid`. The organism substrate
calls it directly, every disturbance, with a tree's own `RootTip`
positions as anchors, `MaterialKind::Plant` as `is_body`, and `organism_id`
equality as `same_group` — this *is* the per-frame mechanism for `Plant`,
not just a verification pass, per §2's decision. `SecondaryThicken`'s own
downstream-leaf-count flood fill (§4) is the same primitive again, with
its own `MAX_THICKEN_SCAN_CELLS` as `cap` and a tally of how many visited
cells are `Leaf`-typed kept alongside the traversal — a counting variant,
not a stop-at-the-first-leaf one; the primitive itself only exposes a
numeric cap, and "terminal condition" above means what the *caller* does
with cells it reaches, not a change to the primitive's own stopping rule.

**One primitive, three callers, three different roles** — verification
tool, primary mechanism, and a resource-counting variant — which is
exactly the "build once, call by many systems" pattern `emergent-world-
architecture.md` §0 already identifies as the actual payoff of the
thin-organism architecture, demonstrated here for a mechanism that isn't
a field channel.

---

## 6. Organism id lifecycle: closing issue #8 for real

`Reports/pixel-physics-issues.md` issue #8 documents `World::trees: Vec
<TreeState>`'s actual bug: it only ever grows, so a tree whose every tip
and root has died (or whose wood has since burned to ash) leaks its
`TreeState` — attractor list and all — for the process lifetime, currently
bounded only by how many times a person presses a key. Issue #8's own
"Direction" already specifies the fix: generational indices (`{ index,
generation }`) with a free list, so a slot can be reused while a stale
`ActiveSite` referring to the old generation resolves to `None` and drops
itself rather than panicking or resurrecting a dead organism's state.

**Built here, not deferred again**, because the organism substrate makes
the leak *guaranteed to matter*: moss and worms already reseed themselves
in normal play far more often than a player replants a tree by hand, and
this session's own §8 implementation plan requires ants (§12, a later
overnight section) to spawn far more short-lived organisms than trees
ever did.

- `organism_id` assigned from the free list when a `Seed` cell germinates
  (a `Germinate` behavior firing, §1's species data), generation bumped
  every reuse.
- A new cell inherits its parent's `organism_id` verbatim when `Divide`
  succeeds, written as the cell is placed — no separate propagation pass,
  the same "write it where it's decided, not in a follow-up sweep"
  discipline `structural.rs`'s own reactive scheduling already follows.
- Reclaimed when §5's connectivity primitive, run from that organism's own
  remaining anchors, finds zero reachable `GrowingTip`/`Leaf`/`RootTip`
  cells — the generalization of `reclaim_if_tree_is_fully_dead`'s existing
  "every tip and root reports `alive: false`" check, now driven by actual
  cell-grid reachability instead of a per-tip boolean, which is what makes
  it correct for a burned-through trunk that severs a live canopy from a
  live root system without killing either side's own tips directly.

---

## 7. Retrofit order, and what stays out of scope here

Library and scaffolding first, then **moss** (one cell type, no transport
graph, no thickening, no roots — the simplest possible smoke test that the
generic scheduler dispatch and species-data loading actually work end to
end, and `Divide`'s discrete grid-candidate mode is all moss needs, so
this step alone says nothing about whether the tree step's continuous
mode actually shares real code with it — see §1's caveat), then **trees**
(`Divide`'s space-colonization mode, `SecondaryThicken`, the multi-type
transport graph, phototropism/hydrotropism/wind-lean ported from
`tree_tip_tick`'s existing formula rather than rederived — **the step
where §1's `Divide` risk gets resolved one way or the other**, by
implementation rather than by more design), then the **worm** (`Locomote`
instead of `Divide`, confirming the behavior library generalizes past
rooted organisms — a worm is a species with one cell type and no
`Photosynthesize`/`Absorb` behavior at all, which is the real test of
whether the library composes or was secretly tree-shaped). Delete
`TreeState`/`CreatureState` and their dedicated tick functions once each
replacement's own tests are green, not left running side by side.

**Explicitly out of scope for this pass**, named so a future reader
doesn't wonder whether they were missed: cytokinin as a second diffusing
signal (already deferred by `plant.rs`'s own module doc, unaffected by
this rewrite); Palubicki-style shadow-voxel light competition between
branches (same); a resistance-network transport solve in the L-PEACH/MuSCA
sense (§3's whole point); per-species custom rendering beyond palette
colour. None of these are precluded by the design here — the species/
behavior split is exactly the seam a future pass would extend, not
something this rewrite would need to unpick.
