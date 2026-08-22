# Night-session handoff: Phase 1 done, Phase 2 built, one lean, one study running

Written for the session that picks this up cold. State as of the last
commit on `plant-substrate-v2` (worktree `.claude/worktrees/plant-v2`):
every suite green (477 lib / 8 bin / 17 worldgen / integration), clippy
clean, and a multi-hour study **currently running detached** — read §6
before touching the tree.

The plan of record is `C:\Users\Scott\.claude\plans\did-we-answer-all-abstract-rocket.md`;
the two review reports (`tree-architecture-variety-review*.md`) are the
design source. This file is what happened since, and what is next.

## 1. What landed tonight (chronological, all committed)

| commit | what |
|---|---|
| `671ab81` | **P1.4**: crowding divides instead of subtracting — the collapse cliff was score arithmetic and is gone (sweep in tree.ron: mass flat 33–36k across weights 6–30, smallest tree 2,811 where the old form's was 26). The tip cap became real: `organism_active_tip_count` counts the cell list (the heap scan missed in-flight dispatch — handoff §3's tripwire fired at 19 vs 14, exactly as it predicted), and `break_buds` now respects `max_active_tips` (it was a second, ungated creator of frontier). |
| `c8491fb` | **P1.5**: one currency — the node (`L_node = MAX_LIGHT × leaf_cluster`). `INCOME_PER_NODE 0.08`, `pipe_ratio 5.5` (nodes per cell of stem width), crowding in fresh-deposit units (`crowding_weight 30`). All conversions of the old operating point, not re-tunes. **Acceptance: `leaf_cluster` 5→1 now moves mean size 7% (33,785→31,467); it used to quintuple income and fuse the stand.** |
| Phase 2 commit | The three discrete levers as species data + counters + two species (§2, §3). |

Earlier the same night (see `git log`): P0 complete (merge with master's
worldgen/destruction/sky line and its two semantic reconciliations;
`break_buds` payment fix; `relocated_seed` on the cell list;
position-keyed genotype with the `a_tree_eventually_stops_growing`
refit), P1.1–P1.3 (per-column Beer-Lambert light; the stale one-block
offset removed — crowns separated by self-shading alone; phase-free
noon-equivalent economy; graded shade abscission ON at 0.03 → bare boles,
wood:leaf ~7:1).

## 2. The owner's playtest directives, and where each stands

1. **"Way more variability — they still all look the same."** Phase 2 is
   the answer and is BUILT: `sympodial` (ByOrder<bool>), `tropism`
   (ByOrder<Orthotropic|Plagiotropic>), `acrotony` (signed scalar on
   BudBreak), all species data, all counter-instrumented
   (`OrganismState::sympodial_forks` / `plagiotropic_steps`, printed by
   `plant_probe`). Three species: `tree` (control, counters 0/0 ✓),
   `shrub` (**works, and looks it** — hedge of low mounded multi-stem
   bushes, 46–186 forks each, `target/filmstrips/p2-shrub.png`),
   `conifer` (**known-wrong by eye**, §4).
2. **"Roots regressed, want them meaningful + genetically varied with
   real effects."** NOT touched tonight beyond what already exists.
   Queue: (a) look-first probe — root depth histogram + root-crop sheet
   (the grove sheets show roots as pale fences hugging the surface;
   suspects: `heading_inertia: 0.75` locking horizontal runs once
   started, and the moisture field being laterally uniform at the
   surface); (b) genome slots for root traits (`penetration_force`,
   root `branch_chance`, hydrotropic gain — slots 6+; the draws array is
   `GENOTYPE_TRAITS = 6`, widen it and the variance array together,
   slots are positional forever); (c) the *real-effect* coupling already
   exists to build on — roots feed water (soil aux → Absorb) and anchor
   (structural), so deeper/wider roots honestly buy drought resilience
   and (after §5) stability. No pre-programmed effects needed.
3. **"Damaged trunk leaves the crown floating and growing; weak roots
   should topple."** Root cause already diagnosed and measured this
   session: `organism_is_supported` is a **hop-bounded BFS from the
   checked cell**, so (a) checks fired mid-crown amputate healthy canopy
   (26x outcome difference, found via abscission), and (b) a crown
   *severed* far from the cut can pass or never get checked — the
   floating crown. One fix serves both: compute support **from the
   anchors outward** once per organism per tick (a BFS from RootTip
   cells over the cell list — the same shape `accumulate_support`
   already runs; cells not reached are unsupported, however far away),
   replacing the per-cell bounded search. Then damage semantics become
   real, and "weak roots topple" becomes expressible (an organism whose
   anchor set is too small for its `supported_load` fails at the
   collar). Owner does not care whether this or roots goes first.
4. **"Slow growth at night"** — NEW DIRECTIVE, refining P1.3. Keep
   *decisions* phase-free (that fixed the nightly extinctions and the
   noon/night tip swing — do not undo it), but scale *income* by
   daylight: in `allocate_to_frontier` (and the two `Photosynthesize`
   credit sites), multiply by something like
   `0.25 + 0.75 * field::daylight_fraction(world.frame)` — visible
   day/night growth rhythm, night at a quarter speed rather than zero
   (zero re-creates starvation-retirement overnight; the floor is a
   design knob to sweep). `daylight_fraction` is already pub. Expect a
   stand-size drop (~40%?) since daily income falls; that is wanted —
   the P1.3 commit records the economy "runs hot until re-tuned". Verify
   the tripwire test and `a_tree_eventually_stops_growing` (property-
   based, survives), and re-run the standard probe for the new baseline.

## 3. The lever mechanics, for whoever touches them

- Sympody: fork block in `Grow` — on a sympodial tier's fork, the
  *primary* child is re-labelled a lateral too (order+1, heading =
  its own step). Between forks the axis continues unchanged; that is
  correct Leeuwenberg behaviour (straight modules between forks).
- Plagiotropy: the tier's reference in the score is
  `(±0.912, 0.410)` — outward with a spruce droop — side from the
  axis's own `heading.0` with a ±0.05 dead zone resolved by the
  per-(organism,cell,frame) rng stream. `upward_weight` weights this
  reference, so for plagiotropic tiers it is an *outward* weight.
- Acrotony: bud score × `(1 + acrotony · (elevation − 0.5)).max(0.05)`,
  elevation 0 at `collar_y` to 1 at `shoot_top_y` (new field, refreshed
  in the upkeep walk beside the collar).
- Flushed buds now launch with **no heading** (fall back to
  away-from-supply). They used to inherit the retired stem node's
  near-vertical heading, which fed the conifer lean (§4) and pointed
  every regrown lateral up the trunk's own line.

## 4. OPEN BUG — the conifer stand leans rightward in unison

`target/filmstrips/p2-conifer.png`. Tiers run (plag counters
1,797–2,750/plant), trees are correctly taller and narrower than
`tree`, but every individual sweeps up-and-right in long arcs; no
upright bole + horizontal tiers.

Ruled out, in order, each by measurement or code-read:
1. **Wind**: `PREVAILING_DRIFT` is gas-movement-only; the field carries
   no standing velocity in this scene; `wind_lean_dir` reads (0,0).
2. **Side tie-break**: `heading.0 < 0.0` sent sign-of-epsilon (and
   exactly-vertical) headings uniformly right. Fixed (dead zone + rng
   coin) — lean unchanged.
3. **Bud-flush heading inheritance** (§3 last bullet). Fixed — lean
   unchanged. (Both fixes are correct regardless and stay.)

Three theories down = stop theorizing (CLAUDE.md). **Next step is
instrumentation**: add left/right side counters to the plagiotropic
reference (per organism, printed beside forks/plag), and a
per-branch-order breakdown if needed. Candidate remaining suspects, to
be *tested not assumed*: asymmetric candidate geometry when the trunk
occupies one neighbour (a right-leaving lateral's candidate set differs
from a left-leaving one's relative to NEIGHBOURS_8 iteration only via
scores — check the weighted-pick math for a subtle first-element bias
when `pick` lands exactly on a boundary; `chosen = candidates[0]`
initialisation!), and `cross_section_axis`/`thicken` sidedness feeding
crowding asymmetrically. NOTE `let mut chosen = candidates[0];` — if
float subtraction leaves `pick` ≥ all remaining scores (sum rounding),
the loop falls through with `chosen = candidates[0]` = **NEIGHBOURS_8's
first entry = up-LEFT diagonal**... which would bias *left*, not right —
but check the actual reachable case rather than trusting this note.
The megastudy includes conifer across 8 world seeds precisely so the
fix session can ask: does the lean side ever vary (per seed, per
individual)? If literally never, the bias is deterministic structure,
not accumulated drift.

## 5. Support-from-anchors design sketch (for directive 3)

Replace `organism_is_supported`'s per-cell bounded BFS with a per-tick
whole-organism pass (mirror `accumulate_support`): BFS from all
RootTip/anchored cells over `state.cells`; mark reached. Store the
unreached set (or a per-cell `supported` bit in `OrganismCell`).
`organism_structural_tick` then just reads the bit — O(1) per check, no
span bound, no amputation, no floating crowns. Severed regions convert
to deadwood over their next checks (schedule checks for unreached cells
when the set changes). Cost: one O(N) BFS per organism per tick, the
same shape as three passes already running. This unblocks honest damage
work (the review-cut experiment's "topped trees do nothing" is
double-contaminated: mid-crown checks amputated the remnant AND
`break_buds`' recovery gate is backwards — both must be fixed before
damage numbers mean anything).

## 6. The megastudy (RUNNING — check before rebuilding!)

`scripts/megastudy.sh`, launched detached ~07:39; writes to
`target/megastudy/` (MANIFEST.txt, `<species>-seed<N>.log`, sheets).
3 species × 8 world seeds × 16 plants × 45,000 frames ≈ 3–4 hours.
**A `cargo build` while it runs may block on the exe lock or swap the
binary between runs — check `MANIFEST.txt` for "complete" before
building anything in this worktree.** Resumable: re-running the script
skips finished logs.

Analysis guide: each log has per-tree sizes/heights/thickness, the
genotype table (draw multipliers per trait), architecture counters, and
the leaf-light histogram. Questions it was designed to answer: trait →
outcome response across 128 individuals/species (regress like the old
1,024-genome studies, now per species); cross-species separation
(height/width/mass distributions should be disjoint or nearly);
establishment failure rate and its spatial pattern at 16-tree density;
and the conifer lean's seed-dependence (§4). Worth writing up as
`Reports/genetic-variability-study.md` with the sheets.

## 7. Tonight's do-not-relitigate additions

- Crowding divides; the cliff was subtract-then-filter (guard test
  proves the old form fails).
- The tip cap counts the cell list; `break_buds` respects it.
- Node units end the treadmill; the 7% cluster-invariance run is the
  evidence. Constants are conversions of the old operating point.
- `SOIL_CAPILLARY_REST = SOIL_SATURATED − SOIL_FIELD_CAPACITY` is
  *derived* (the two-cell pump argument in its doc) — do not tune it.
- Shade abscission is a graded pressure; thresholds also work (20,044 vs
  20,213 measured) — graded won on separation/transients; the
  "collapses at any setting" sweeps were the structural-check confound.
- Mid-organism structural checks amputate (until §5 lands).
- Sweeps: rebuild per point (`include_str!`); prove pattern edits touch
  one line (tree.ron has two `crowding_weight`s); never edit a file a
  running sweep owns.
- Genotypes key on (world seed, germination spot); draws cached on
  `OrganismState`; variance stays live; slots positional forever.

## 8. Standing state numbers (for pairing)

Standard probe (8 trees / 30k frames, world seed default): tree 33,785
cells / heights 106–172 / fused run 63; conifer 45,346 / 144–201 / 51;
shrub 20,127 / 50–80 / 79 (a hedge — spacing-fused deliberately). All
three sheets in `target/filmstrips/p2-*.png`. Night-growth (directive 4)
will move all of these; measure paired before/after.
