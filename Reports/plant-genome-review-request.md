# Review request: the genome slot-map landing

Written by the genome session for the water-economy session to review; the
owner relays it. A copy of this text is what gets pasted to that session.

---

You are the session that built the water economy in
`.claude/worktrees/plant-v2` (`8c19439`). The owner has asked you to review
the genome session's plan and recommend **when its landing should happen
relative to your own remaining work**. Do not check out or build anything
outside your worktree — read via `git show`.

READ, in order:

1. `git show plant-genome:Reports/plant-genome-design.md` — the proposal:
   §5 slot map, §4 per-locus cases, §10 delta-check against your `8c19439`.
2. `git show plant-genome:Reports/plant-genome-handoff.md` — the brief it
   answered.
3. The implementation decisions below, which go past the doc.

ALREADY DECIDED by the owner (2026-08-18) — review for defects, do not
relitigate: **Map A** (re-purpose dead slots 1/5 as root branch chance /
root tropism gain; `GENOTYPE_TRAITS` 6→9); **`LOCUS_FOLIAGE` re-keyed to
leaf economics** (2 alleles; band = allele exactly as today, plus
rate/transpiration multipliers ×[1.2, 0.85] / ×[1.5, 0.7]); **penetration
force in**, with resistance-scaled root `Grow` cost as its bill; **seed
strategy deferred**, but its plumbing lands now (endowment on
`OrganismState`; `Reproduce.seed_cost` becomes the seedling's starting
stake at germination instead of vanishing).

Implementation decisions (a full draft of the edit is parked on branch
`plant-genome` on your tip — committed but flagged as not compiling and
unverified; `plant-genome-implementation-handoff.md` is its completion
plan):

- Root slots (1/5/8) read the **RootTip `Grow`'s own variance vector** —
  non-zero for the first time; slots 4/6/7 borrow the shoot vector per
  your `pipe_variance` "one plant, one genotype" precedent.
- Slot 7 (stomatal closure): new species scalar `stomatal_reserve`
  (serde default 0 = bit-identical to your engine), openness ramps
  `stock/capacity` through `reserve × genotype(7)`; **`drought_death`
  re-keys from `1 − water_status` to a new `water_desiccation` = the
  open-stomata shortfall** — provably identical to `1 − status` whenever
  reserve is 0, so your 0.003 tuning is untouched for every species until
  the `.ron` opt-in (0.2 first pass) rides along. **Check this identity
  claim against your settle — you wrote it** (`plant.rs:3422` at your
  tip).
- Penetration bill: cost multiplier `max(1, resistance/0.8)` on the
  entered `Powder`, applied per candidate (hard ground a root cannot
  afford this tick drops out of its candidate set — poor roots prefer
  soft ground) and again at payment for the chosen target.
- Density: the allele multiplies `max_cantilever_reach` per individual
  (u16-saturating so infinite-span materials stay unbounded) and
  shoot+root `Grow.cost`; `thicken()` pays no carbon today, so the price
  binds on extension only — known and recorded, not hidden.
- First generation: leaf-econ allele comes from the existing stream-64
  band pick; the density allele from stream 65 (which used to pick the
  bark band directly) with bark deriving from it — day-one stand variety
  is preserved on both axes; angle/internode still start authored-mid.

YOUR DELIVERABLE — a short report the owner can relay:

1. Defects or objections in any of the above, with file:line into
   `8c19439` where applicable.
2. Sequencing around the probe: you hold uncommitted changes to
   `examples/plant_probe.rs` and `examples/common/mod.rs`; the genome
   edit also rewrites the probe's genotype table (9 named columns,
   per-slot variance sourced from the correct vector, an allele-frequency
   census, and a fix for the table printing tree's variance under any
   `species=`). Should the genome session rebase over your next commit
   before touching the probe, or land first and leave you the rebase?
   What do your probe changes do?
3. Where does the genome landing sit relative to your remaining queue
   (economy re-tunes, sweeps, anything in upkeep/allocation)? The re-map
   re-baselines trait→outcome comparability, so any sweep of yours should
   fall cleanly before or after it, never straddle it.
4. The megastudy: one combined re-run after both streams land
   (recommended — 3.5 h, and it should follow appearance-affecting work),
   or do you need an interim run first?
5. `PLAN.md` and `wiki/plants.md`: do you have pending edits? The genome
   landing appends the final slot map to `PLAN.md` and updates
   `wiki/plants.md` (your page) with heritable morphs and
   colour-as-readout.

Constraints from the genome side: it will not touch `plant-v2` or the
`plant-substrate-v2` branch; it builds and verifies on `plant-genome`
only, and holds merge, `PLAN.md`, `wiki`, and the megastudy until your
recommendation comes back through the owner.
