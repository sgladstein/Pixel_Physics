# Implementation handoff: finishing the genome re-map

For the implementation session. **Every design decision is already made
and signed off** — `plant-genome-design.md` (§5 map, §9 calls) is the
contract and nothing here re-opens it. Your job is to finish a parked
draft, verify it the way this repo verifies things, and stop at the held
gates. If something surprising forces a judgment call the sections below
do not cover, stop and report rather than deciding.

Work **only** in `.claude/worktrees/plant-genome` (branch `plant-genome`).
Never touch `.claude/worktrees/plant-v2` or the `plant-substrate-v2`
branch — another session owns them. `CLAUDE.md` applies in full; the
traps most likely to bite this specific work are restated in §5.

**STATUS, completion session (2026-08-18): §3 done bar one test, §4
stopped at its first row.** The draft compiles (the missing import was
its only break), `cargo test --lib` is green at 485 and clippy is clean,
six of the seven §3 tests are in, and the stand-level sanity bar is set
fresh. The seventh — `root_and_shoot_branching_read_different_slots` —
**is not written, because its premise is false**: slot 1's consumer is
behind a carbon gate the root economy clears twice in twelve thousand
frames, so `root_cells` is bit-identical at every draw. Measured,
instrumented and written up in `plant-genome-design.md` §8a, with the
reproduction kept as `plant::tests::print_root_branch_slot_pairing`. The
remaining §4 rows are held on the owner's call, per this file's own "stop
and report rather than deciding". §2's gates are all still respected.

## 1. State

- Branch `plant-genome`, based on `plant-substrate-v2`@`8c19439`. Two doc
  commits (`164cc85`, `849b553`) plus a **draft commit of the full code
  edit** (the commit whose message says it does not compile — see
  `git log`). The draft covers: `src/sim/organism.rs` (constants, loci,
  allele tables, `bark_band_for_density`, new `OrganismState` fields,
  `stomatal_reserve` on species), `src/sim/world.rs` (init +
  `desiccation_at`), `src/sim/plant.rs` (all consumer wiring),
  `src/sim/structural.rs` (density × reach), all three species `.ron`
  (9-slot vectors, root vectors, `stomatal_reserve: 0.2`),
  `examples/plant_probe.rs` (9-column table, per-slot variance from the
  correct vector, allele census, per-species variance fix).
- **Known compile error, deliberately left:** `structural.rs` uses
  `organism::WOOD_DENSITY_ALLELES` / `LOCUS_WOOD_DENSITY` without an
  import (E0433 ×3 around `structural.rs:409–418`). Match the file's
  existing `use` style (check its header; likely `use super::organism;`
  or fully-qualified `crate::sim::organism::…` to match neighbours).
  There may be further small breaks behind it — the check was
  interrupted at the first errors.

## 2. Held gates — do NOT do these until the owner relays the water session's review

The water-economy session (in `plant-v2`) is reviewing the plan and
recommending sequencing (`plant-genome-review-request.md`). Until its
verdict arrives through the owner:

- **No merge** into `plant-substrate-v2` (you cannot move that branch
  anyway — it is checked out in their worktree).
- **No `PLAN.md` edit** (contested file; lands with the merge, quickly).
- **No `wiki/plants.md` edit** (their page; same moment).
- **No megastudy launch.**
- If their verdict asks for changes, apply them here first.

Everything in §3–§4 is in-worktree and safe to do now.

## 3. Steps, in order

1. `git -C <main checkout> fetch`-free sanity: check whether
   `plant-substrate-v2` moved past `8c19439`
   (`git log --oneline plant-substrate-v2 -1` from this worktree — same
   repo). If it moved, `git rebase plant-substrate-v2` **before** any
   further work. Expected conflict: `examples/plant_probe.rs` (they hold
   uncommitted probe changes that may land). Resolution rule: their
   changes are about water/scene readouts, ours rewrite the genotype
   table block and add the allele census — re-apply ours by intent on
   top of theirs; if the same lines genuinely collide, theirs win
   structurally and ours are re-inserted after.
2. Fix the import (§1); `cargo check --all-targets` until clean. Likely
   follow-ons: array-width literals (`[0.0; 6]`) anywhere the grep
   `\[0\.0; 6\]|LOCUS_FOLIAGE` still hits; a borrow/scope issue in
   `plant_probe.rs` around `scene.species` if `scene` was consumed
   (compiler will say; fix by cloning the species string early).
3. Tests — add these, names and assertions as given (they are the guard
   set the design doc promises; see §4 for what each protects):
   a. Extract the settle arithmetic in `organism_upkeep` into a pure
      `fn settle_water(stock, capacity, demand, reserve) -> (drawn,
      status, desiccation)` called by the inline block (no behaviour
      change), then unit-test: **reserve = 0 ⇒ desiccation ==
      1 − status exactly** over a grid of stock/demand values, and
      desiccation ≤ 1 − status + ε always. This identity is what keeps
      the water session's `drought_death: 0.003` tuning valid — if it
      ever fails, the seam in design doc §4.3 has been broken.
   b. `bark_band_tracks_the_density_allele`: pure fn —
      bands `(first: 0, count: 2)` → alleles [0,1,2] map to bands
      [0,0,1]; `count: 0` → 0.
   c. `penetration_cost_scales_with_resistance`: `penetration_cost_mult`
      over soil (=1.0), sand (=1.75), gravel (=4.375), empty air (=1.0),
      stone (=1.0).
   d. `a_bred_seedling_starts_with_its_parents_stake`: soil-bed scene
      (copy `plant_tree_on_ground`'s walled bed), hand-built Seed cell +
      organism with `inherited: true` and `endowment: 0.3`; run until
      the seed becomes a `GrowingTip`; assert the shoot cell's carbon
      ≈ 0.3 on the germination tick.
   e. `root_and_shoot_branching_read_different_slots`: two runs of the
      same scene, identical genomes except `genotype_draws[1]` = +1.0
      vs −1.0 (set via `inherited: true` before germination so
      `seed_genotype` does not redraw); ~12,000 frames; assert
      `root_cells` orders with the draw and `shoot_cells` stays within
      spread. If this flakes across seeds, do not ratchet margins —
      report it.
   f. `dense_wood_holds_a_longer_loaded_branch`: copy the existing
      organism cantilever test's scene (it pins its own
      `max_cantilever_reach`); same beam, `alleles[LOCUS_WOOD_DENSITY]`
      = 0 vs 2; pick a reach between 0.75× and 1.35× of the pinned
      value; assert the pioneer beam fails and the dense one stands.
   g. `expensive_leaves_demand_more_water`: one scene, two plants,
      economy allele 0 vs 1 (inherited); after settling, per-leaf
      `water_demand` ratio ≈ 1.5/0.7 within tolerance.
4. `cargo test --lib` green, then
   `cargo clippy --all-targets -- -D warnings` clean. Expect clippy to
   police the new closures/maps — fix style, never silence.
5. In-worktree verification (recipes in §4). Rebuild release examples
   first: `cargo build --release --examples` — **and check
   `plant_probe`'s first echoed line names your `worldseed=`** before
   trusting any run.
6. Independent review before the significant commit (repo convention):
   run the code-review skill over the working diff; fix what it finds.
7. Commit on this branch, message carrying the §4 numbers (before/after
   per paired run, what was tried and rejected). Explicit paths only —
   never `git add -A`.
8. Update `plant-genome-design.md` §8 with the measured results and
   update the memory file if you have access to one. Then stop at the §2
   gates and report.

## 4. Verification recipes, per locus

General method: **paired comparison, both runs in the same session on
the same machine; rebuild between any `.ron` or table pin
(`include_str!`)**. An exactly-zero delta means suspect the condition is
degenerate (is the knob actually in the binary? does the state it keys
on ever occur?) before concluding the lever is dead.

| lever | control | quantity (probe prints it) | expected |
|---|---|---|---|
| root branch (slot 1) | root vector `0.5 → 0.0` at slot 1, rebuilt | spread of `root_cells` across a 16-plant stand | width 0 collapses the spread; 0.5 widens it |
| root tropism (slot 5) | same, slot 5 | root depth histogram (the `ed28d16` readout) | high draws deeper/narrower, low draws shallower/wider |
| allocation bias (slot 6) | shoot vector slot 6 `0.4 → 0.0` | root:shoot ratio spread | same shape as above |
| stomatal (slot 7) | `stomatal_reserve` `0.2 → 0.0` all species, rebuilt | foliage retention + cells on a drying scene vs a wet one | reserve buys retention in dry, costs growth in dry-but-surviving; **wet-scene deltas should be ~0** |
| penetration (slot 8) | needs a sand-bank scene (build one: soil bed with a sand stratum, walled) | rootwood cells inside sand, per plant vs slot-8 draw | only high draws enter sand; all pay more carbon per sand cell |
| leaf economy | pin `LEAF_RATE/TRANSPIRATION_ALLELES` to all-acquisitive vs all-conservative, rebuilt | cells + water block, wet scene vs thin-dry scene | acquisitive wins wet, conservative wins dry — the crossover is the point |
| wood density | pin `WOOD_DENSITY_ALLELES` all-0.75 vs all-1.35, rebuilt | cells at 30k (growth) AND deadwood conversions under load | cheap grows faster, dense survives load — both directions must show |
| endowment | `seed_cost` sweep 0.15/0.3/0.6 (species-level), rebuilt per point | established count per seeds-set | this measures the §4.8 response curve — record it either way |
| bark/foliage readout | one filmstrip sheet per pinned extreme | the **band counters printed beside the sheet** (a recolour is invisible at sheet zoom) plus the eye | dense = distinct bark band; economy alleles = the two foliage bands |

Do not expect the stand to match any pre-genome baseline: first
generations now draw mixed density/economy alleles by design. The
stand-level sanity bar is fresh: standard probe (8 trees / 30k frames)
still establishes a breeding stand in the gross range of the water
session's baseline — set the number from your own baseline run first,
same session.

## 5. Traps, restated for this change

- The two `Grow` payment sites are `step_cost` / `branch_step_cost`
  (penetration-priced), not `cost`. Do not "simplify" them back.
- The candidate-loop affordability filter deliberately does **not** set
  `found_candidate`; an emptied set banks staleness, which is the
  designed retirement path. Do not mark it as a found candidate.
- `water_desiccation` vs `water_status`: earning reads status, shedding
  reads desiccation. Anything that couples them re-opens the §4.3
  trade-inversion.
- Stream 65 now draws the density allele (bark derives). Do not add a
  new stream for it — first-gen bark variety rides on this reuse.
- The u16 saturation on density × `max_cantilever_reach` keeps moss's
  infinite span infinite. Keep it.
- `LOCUS_ALLELES[0]` is 2 now; any code assuming 6 foliage alleles is
  wrong, not the constant.
- Species `.ron` are `include_str!` — every sweep point is a rebuild,
  and identical outputs across settings mean the knob never connected.
- `cargo test --lib` (not bare `cargo test`) while any app instance is
  open; `rm -rf target/debug/incremental` for bogus LNK2019 errors.
- Two drivers: anything judged by eye should come from the parallel
  driver at least once (`filmstrip` default path does).
- Commit messages carry the measurement. Reverts keep the knowledge.

## 6. Definition of done (for this branch, pre-gate)

Compiles; suite + clippy green; the seven §3 tests in and passing; every
§4 row measured with its numbers recorded in the design doc §8; sheets
looked at (not only measured); review run; committed. Merge, `PLAN.md`,
`wiki/plants.md`, megastudy: **held** per §2.
