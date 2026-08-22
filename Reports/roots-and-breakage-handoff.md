# Roots that matter, and trees that break

Written to be picked up cold. Two owner-set goals, both of which turn out to
depend on the same missing quantity — **how much root a plant has, and where
it is** — which is why they are one report and should probably be one branch.

State: `plant-substrate-v2`, all suites green (478 lib / 8 bin / 2
determinism / 17 worldgen), clippy clean under `-D warnings`.

---

## Goal 1 — make roots matter

### The finding: roots are currently optional

Verified by reading, not inferred:

1. **`Absorb` credits the same pool `Photosynthesize` fills.** Both add to
   `resource`. Water and carbon are one currency, so a root does not supply a
   distinct thing the plant needs — it adds more of what leaves already make.
2. **`transpire` costs the plant nothing.** It decrements soil moisture in
   neighbouring cells and never touches the plant's own `resource`.
3. **And in the canopy it does nothing at all.** It early-returns unless a
   neighbour has `water_capacity > 0`, which no canopy cell ever has.

So a rootless plant runs no deficit. It grows on light alone, and a tree
standing in another tree's crown is fully viable — which is why the canopy
filled with epiphytes (measured: **430 of 487 organisms rooted above the
soil**, 410 of them more than 25 rows up).

That was stopped with a guard — a seed no longer germinates resting on
`MaterialKind::Plant` — and the guard is a **symptom fix**. The owner's
framing is the right one: *a canopy plant cannot have roots, so it should
starve rather than be forbidden.*

### The mechanism to build

**Charge the plant for transpiration**, so root uptake has something to
replace.

- Foliage costs `resource` per organism tick (transpirational demand).
- `Absorb` already credits it from soil.
- `organism::diffuse_resource` already moves it around the plant.

Then a plant with no soil contact runs a net drain and dies on its own, deep
roots buy drought resilience honestly, and the root-genome traits the night
handoff queued (`penetration_force`, root `branch_chance`, hydrotropic gain)
finally have something to be selected *on*. Today they buy nothing
measurable, which is why they have never been worth adding.

### What it costs, stated up front

**This is an economy change and re-tuning is part of it, not scope creep.**
Adding a per-leaf cost changes what `INCOME_PER_NODE`, `Grow.cost` and the
bud-break gate mean. Expect to sweep, and expect stand mass to fall before
it is re-balanced.

Sequence that avoids tuning twice:

1. Land the cost with a **counter** for water balance per organism (uptake,
   demand, deficit) — the deficit is the quantity, and a *sum* separates
   more cleanly than a count of starving cells.
2. Re-derive the economy constants against the standard probe.
3. Only then judge shape and colour, which this session left in a good place
   and which a hungry economy will move.

### How to know it worked

- **Epiphyte count goes to zero without the germination guard.** Temporarily
  revert `plant.rs`'s `MaterialKind::Plant` check in the `Germinate` arm and
  confirm the economy kills canopy seedlings by itself. If it does, the guard
  is redundant and should be deleted rather than kept "just in case" — a
  superseded mechanism whose tests keep passing is a documented trap here.
- **A paired dry/wet scene separates.** Same genome, deep soil vs thin soil
  over stone; the deep-soil stand should out-grow the thin one, and today it
  would not.

---

## Goal 2 — trees that break, rip and topple

Owner's words: *chop branches off, cut the tree down, a rock or a storm
knocks a weak-rooted tree over or tips it, or if the roots are strong enough
the trunk breaks instead.*

That last clause is the design in one line: **the failure mode is decided by
which is weaker, the root plate or the stem.** Both need to be real
quantities before any of it can be judged.

### What already exists

- `MatureBody` and `DormantBud` carry `StructuralAnchor`.
- `wood.ron`: `max_unsupported_span: 8`, `breaks_into: "deadwood"`.
  `leaf.ron`: span `1` — "a leaf holds up nothing but itself".
- `structural.rs` routes organism-owned cells to
  `organism_structural_tick`, which is separate from the `Solid` path
  because `Cell::aux` holds the cell-type tag for organisms and cannot also
  hold an anchor distance.

### Two defects in the one function this all depends on

`organism_is_supported(world, x, y, organism_id, max_span)`:

1. **It is a hop-bounded BFS from the checked cell**, looking for a cell
   touching solid ground within `max_span` hops. `wood`'s span is 8 and a
   mature tree is ~150 cells tall, so any check fired in the crown reads
   "unsupported" and converts healthy canopy to deadwood. This is already in
   `CLAUDE.md` as a landmine — abscission scheduling one collapsed every
   shedding sweep, 772 cells against 20,213 from that single line — and it
   is why growth deliberately schedules no structural checks.
2. **It traverses `NEIGHBOURS_4` while `Grow` places cells at 8.** So the
   support search sees a connected tree as disconnected fragments. This is
   `CLAUDE.md`'s "a traversal must use the same neighbourhood the writer
   used" rule, violated in the function goal 2 is built on. *Found while
   writing this report and not yet fixed.*

**Neither is a tuning problem and neither should be worked around.** Any
damage result taken before both are fixed is contaminated — the night
handoff already says to treat Phase 3 damage numbers that way.

### The designed replacement, already sketched

`Reports/plant-night-session-handoff.md` §5: compute support **from the
anchors outward**, once per organism per tick — a BFS from `RootTip` and
ground-touching cells over `state.cells` (the same shape `accumulate_support`
already runs), marking reached cells. `organism_structural_tick` then reads a
bit: O(1) per check, no span bound, no amputation, and a severed crown is
unreached however far away it is.

This one change unblocks all of it: floating crowns after trunk damage,
"weak roots topple", and honest damage measurement.

### What each owner verb needs on top of that

| verb | needs |
|---|---|
| **chop a branch** | the severed region is unreached by the anchor BFS → detaches. Then it must *fall as debris*, not vanish or convert in place — `breaks_into: deadwood` exists; whether it drops as loose cells or as a body via `rigid.rs` is the open call |
| **cut the tree down** | same mechanism at the collar, but the whole shoot detaches at once. This is the case that most wants a body rather than per-cell conversion — a felled trunk that dissolves into grit would fail the ethos test |
| **a rock knocks it over** | an *impulse*, not an erasure. `CLAUDE.md`'s destruction ethos is explicit that removing support delivers no load and nothing ever failed from being *hit*. Check what `load.rs` can already deliver to a `Plant` cell |
| **weak roots topple** | root **anchorage** as a number: anchor cell count, depth, and spread, against the shoot's overturning moment. `OrganismState` already tracks `root_cells` and `shoot_cells` |
| **strong roots → trunk breaks instead** | compare the two failure thresholds and take the smaller. This is the "graded outcome beats a binary one" case: the same storm should tip one tree and snap another |

### Where the two goals meet

**Root mass and depth become one quantity with two consequences** — water
uptake and anchorage. That is the coupling that makes root traits worth
having, and it is what lets a storm sort a population: shallow-rooted
individuals blow over, deep-rooted ones do not, and the survivors' offspring
inherit it. Goal 1 and goal 2 share a denominator.

Also note this gives **selection a second axis**, which the clustering work
needs. Today the only pressure is light.

---

## Carried-forward to-do, ranked

1. **Support-from-anchors** (§ above). Prerequisite for everything in goal 2,
   and fixes the `NEIGHBOURS_4` bug in the same edit.
2. **Transpiration cost + economy re-tune** (goal 1).
3. **Delete the germination guard** if goal 1 makes it redundant — verify,
   don't assume.
4. **Root genome traits** — `penetration_force`, root `branch_chance`,
   hydrotropic gain. Genotype slots are **positional forever**; widen
   `GENOTYPE_TRAITS` once, for these and for the colour locus together, not
   twice.
5. **Re-run the megastudy.** It is invalid as it stands (see
   `genetic-variability-study.md`): a stale binary meant all 8 world seeds
   produced byte-identical logs, so it is 3 populations, not 24. Rebuild
   first and check the echoed parameter line in the first log.
6. **The conifer lean** — apparently fixed by the internode straightness
   budget, on picture evidence only. Add the left/right departure counter and
   confirm before closing.
7. **`free_organism` / seed-bank leak.** The seed bank only grows (455
   standing at 60,000 frames); every seed is an `OrganismState` that lives
   forever. Fine now, a leak at M10 scale.
8. **`wiki/plants.md` does not exist.** Every other subsystem has a page.

## Claims from the last session that are now known wrong

Recorded because they are in commit messages and reports, and someone will
read them:

- **"A closed canopy forest, ecologically correct crown recession."** No — it
  was 430 of 487 plants standing on each other.
- **"Clustering, not a smear" (3 morphs of 32).** The number is real; the
  population it was measured over was produced by the epiphyte bug, so the
  conclusion does not follow. Re-measure.
- **"Competition selects for convergence"** as the explanation for the leggy
  brown look. The cause was epiphytes stacking.
- Every crown-profile figure quoted before the metric fix
  (`2e54689`) is void — the bands spanned canopy-top to *root tip*, so the
  bottom one or two were underground.

## Method notes this session paid for

- **Judge plant shape at noon** (frame 28800, or any multiple of 3600), and
  **at the same zoom as whatever you are comparing against**. A night render
  produced a confidently wrong "there is no trunk"; comparing 4× work against
  1× history produced a confidently wrong "the foliage is distributed".
- **A constant that was correct when it landed can go on being paid long
  after the thing it bought is provided elsewhere.** `shade_death` cost 4.8×
  the foliage to separate crowns that four later mechanisms separate for
  free; re-sweeping it tripled foliage at zero cost to fusion. When
  mechanisms accumulate in one area, re-sweep the oldest.
- **A value identical across cases that should differ is the cheapest
  possible sanity check**, and it fired three times: the stale-binary
  megastudy, the saturating `moisture_threshold`, and the crown-profile
  metric's constant `0` in its last band.
- **Counters cannot see what they were not asked.** Generation counts, morph
  histograms and seed totals were all healthy while most of the population
  was rooted in mid-air. The owner found it by looking at a time lapse.
