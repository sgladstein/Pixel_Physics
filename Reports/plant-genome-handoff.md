# Handoff: the genome session

The brief below was written by the session that audited the genome (the same
line of sessions that built the water economy), to be pasted cold as the
opening prompt of the genome session. It is preserved here verbatim so it
survives outside the plan file. The session that picked it up produced
`Reports/plant-genome-design.md`; read that for what was actually found and
proposed — including the places where this brief turned out to be ahead of
the tree (the live-state appearance half had not landed at audit time, and
roots turned out to have *zero* heritable variation today, not merely shared
variation).

---

Continue plant work on `plant-substrate-v2` (worktree
`.claude/worktrees/plant-v2`). Work in your own worktree, not the shared
checkout. READ FIRST, in this order: `Reports/plant-genome-handoff.md` (this
brief), `CLAUDE.md` — especially "ask which pixels a lever moves" and the
stale-harness entry — `Reports/plant-appearance-design.md` §7, and
`PLAN.md`'s settled decisions on appearance-as-readout.

The task: decide how many heritable loci a tree should have, and what fills
them — then land it once. Slots are positional forever; renumbering rewrites
every genome ever measured, so nothing is edited until the owner signs off
on a slot map.

This is a re-mapping question, not an append. The audit that produced this
brief found eleven loci already, three of which buy nothing:

- 6 continuous (`OrganismState::genotype_draws`, jittered by each species'
  `Behavior::Grow::genotype_variance`): 0 branch_chance, 1 upward_weight,
  2 plastochron, 3 turgor_per_cell, 4 pipe_ratio, 5 light_weight.
  `assets/species/tree.ron`'s variance vector is
  `(0.5, 0.0, 0.4, 0.18, 0.7, 0.0)` — slots 1 and 5 are zero-variance, so
  two of six do not vary at all in that species. Check the other two
  species before concluding they are dead everywhere.
- 5 discrete (`OrganismState::alleles`, inherited whole, mutated by
  jumping — `organism::DISCRETE_LOCI`): `LOCUS_BRANCH_ANGLE`,
  `LOCUS_INTERNODE`, `LOCUS_FOLIAGE`, `LOCUS_SYMPODIAL`, `LOCUS_TROPISM`.
  `LOCUS_FOLIAGE` is colour — already heritable and mutable, and the one
  locus with no mechanical consequence at all.
- Roots have no genome. `Grow` is one behaviour shared by `GrowingTip` and
  `RootTip`, so a root's `branch_chance` reads slot 0 — the same draw as
  the shoot's (`plant.rs:1074`). Root and shoot cannot diverge within an
  individual, so no amount of selection produces a deep-rooted morph. That
  is the gap the owner's "roots genetically varied with real effects"
  names.

Deliverable: `Reports/plant-genome-design.md`, then the owner's decision,
then one edit. Every candidate locus must pass three tests:

1. Does it change what a cell does, or only what a cell is labelled? Three
   architectural levers (sympody, tropism, acrotony) fired perfectly —
   counters printed beside the sheets — and moved no silhouette, because
   all three only relabel cells. That cost a whole phase; see `CLAUDE.md`.
2. Is there a measurable outcome it trades against? A locus with no
   trade-off is a free parameter and selection has nothing to act on. This
   is why the session is after the water economy: water, root uptake and
   (later) anchorage are the outcomes that make root and leaf traits
   selectable at all. Name the measurement for each locus, and check the
   probe can already make it.
3. Continuous or discrete? A continuous genome smears a population into a
   Gaussian cloud by construction — there is no setting of it that yields
   two clumps. Discrete loci are what make clusters. The split matters more
   than the count.

Candidates to weigh (each named with the engine hook it already has, so
none of them is speculative):

- Wood density — bark tone ←→ stem strength, cell weight, carbon per cell.
  The pioneer-vs-shade-tolerant axis, the best-studied trade-off in tree
  ecology, and the natural hook if wood ever becomes craftable or sellable.
  It is also directly the quantity goal 2's root-plate-vs-stem comparison
  needs.
- Leaf construction economics — photosynthetic rate ←→ water demand,
  construction cost, lifespan. Dark expensive leaves win under shade; pale
  cheap ones win bright and dry. Fully exercisable against the water
  economy.
- Stomatal closure point — drought tolerance ←→ growth rate. Reads
  `water_status` directly.
- Root traits — penetration force, root branch chance, hydrotropic gain,
  root:shoot allocation bias. The owner's stated goal.
- Seed size ←→ number, decay/fire resistance (hooks into `flammability`
  and `decay.rs`).
- Re-purposing the two zero-variance continuous slots and the cosmetic
  `LOCUS_FOLIAGE`, rather than only adding beside them.

Then, in the same session, the trait-derived half of appearance. The
live-state half (drought pallor, bark darkening with thickening) already
landed; this adds bark tone from wood density and foliage tone from leaf
economics. Keep `plant-appearance-design.md` §7's constraint: a derived
colour picks a band and modulates within it, because a continuous hue over
four channels converges on mud across a stand — which is the exact
complaint the appearance work started from.

Verification. `cargo test --lib` and
`cargo clippy --all-targets -- -D warnings`; a paired comparison showing
each new locus moves an outcome (a locus that moves nothing should not have
been given a slot — an exactly-zero delta means suspect the condition is
degenerate before concluding the lever is dead); and a re-run of
`scripts/megastudy.sh` after the re-map, since the previous run's
trait→outcome regressions are against the old slot map.
`cargo build --release --examples` before launching it — a stale binary
silently ignoring `worldseed=` is what turned the last 3.5-hour study into
three populations wearing 24 logs.

Record the final slot map in `Reports/plant-genome-design.md` and in
`PLAN.md`, and state in both that slots are positional forever.
