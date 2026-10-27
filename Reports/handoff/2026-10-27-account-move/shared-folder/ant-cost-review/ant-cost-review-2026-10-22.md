# The new ant's frame: what each rule does, and what it costs

*Fable review lane "Ant cost review", 2026-10-22. Scratch branch `claude/project-thread-oo9r0d` (commit 0efe4017: counters and two switches, never for main). Method as the soil-water review: count how often each costly rule runs and how often its answer changes anything, then split the cost of the code from the cost of the rule. Scene: the Creature performance lane's benchmark, `needscost seed=1 at=40000 handat=40000 arms=mind mind=v2-C3`, 408 ants, `RAYON_NUM_THREADS=1`, on main 47372a09 (after PRs #774, #785, #788). Owner's rules applied: one physics for every world, one brain for every ant, no PR from this lane.*

## The one-paragraph version

Unlike the soil pass, the new ant's cost is mostly **the rule as designed, running at its design rate**: every ant decides every 6 frames, every decision lists ~10.7 options at ~7.7 cells and describes each with 45 features, and the trained genome v2-C3 reads **every one** of those features and every verb. The option list is never the same twice (37 of 96,345 decisions repeated the previous list bit for bit), ants never rest (0.4% of picks), and no verb or feature is dead, so none of the "skip it" levers exist. Two things are degenerate and one is mispriced. Degenerate: the per-option **warmth** read is exactly 0 on 84-89% of options and under 0.01 on 99% (5.0% of the frame); the **scent-group regrouping** ran 2,400 times and minted nothing (1.9%). Mispriced: the **egg-site walk** runs on 65-89% of ant ticks because most ants are above the egg bar all the time, 70-80% of those walks find no home cell within reach, and each such walk still reads 121 cells to learn that: 12.7% of the frame for 1.1% hits. That one is **code, not rule**: a per-chunk count of home cells answers "none here" in four lookups, bit-identically, for about -9% of the frame. The rule change I timed for the same cost (wait 24 frames after a failed walk, -10.3% instructions) moves births by +48-60% on two seeds inside a 4.5x seed-to-seed spread, so it is a labbench question and the exact fix should go first. Beyond that the remaining code-side items are allocation churn (~6%) and a few repeated reads (~2%); the rule-side items are warmth (-5%), the scorer's summation order (-4 to -6%, more in the training arenas), and the only lever on the whole 84% creature share, the ant's own tick cadence, which is its speed and I do not recommend touching.

## Where the frame goes (callgrind, 600 frames, 6.97 G instructions = 11.6 M a frame)

The creature pass is 84% of the frame; the rest of the world (CA sweep 6%, pheromone 2.6%, field 2.1%, soil 1.4%, begin/end step 3%) is 16%.

| part of the mind ant's tick | share of frame | per call | calls per frame | rule or code |
|---|---|---|---|---|
| option build (`options::build_with`) | 36.8% | 53 k instr a decision | 80 decisions | mostly rule |
| ... of which describing targets (45 features) | ~24% | 4.4 k a target, 7.7 targets | | rule |
| ... of which warmth: `field_at_bilinear` per option | 5.0% | 940 | 623 | **degenerate** (see census) |
| ... of which roofed: 20-row scan per target | 4.2% | | | rule (differs from the head's on 4-6% of options) |
| ... of which walk legality (8 landings: body_after_step, placeable, foothold) | 4.2% | 6 k a decision | | rule |
| egg-site walk (`brood::pile_site`, inside `settle`) | 12.7% | 27.7 k | 53 (63% of ticks) | **code** |
| scorer loop (`z += w * feats[i]`, 184 live weights) | 8.2% | 12 k a decision | | rule |
| allocation churn (malloc, free, realloc; memcpy of 196-byte `Opt`s on top: 2.2%) | ~6% | | ~20 allocations a decision | code |
| the walk act (`commit_step`, `relocate_chain`) | 4.7% | | 52 | rule |
| fall check (`fall_now_if_unsupported`, two Vec clones + `touches_ground`) | 2.9% | 4 k a tick | 84 | code 0.5% / rule |
| context (`mind::context`) | 2.4% | 3.5 k | 80 | `adjacent_nest` twice a tick (1.2%), `under_cover` at the head (0.7%) |
| scent regrouping (`regroup_by_scent`) | 1.9% | 217 k a frame | 1 | rule, cadence |
| `settle` less the egg walk (digest, bill, bud check) | 1.5% | | | |

## The census (`needscost census=1`, 1,200 frames, seeds 1 and 2)

Hashes with the counters compiled in equal the base build's: the counters change nothing.

| what | seed 1 | seed 2 |
|---|---|---|
| mind ticks a frame / decisions | 84.5 / 96,345 | 74.8 / 84,117 |
| options a decision / targets described | 10.7 / 7.7 | 10.8 / 7.8 |
| **option list bit-identical to the ant's last** | 37 (0.04%) | 25 (0.03%) |
| ... identical ignoring scent and warmth | 0.5% | 0.4% |
| head unmoved since last decision | 34.6% | 33.6% |
| same verb as last / same verb and target | 59% / 4.8% | 60% / 4.7% |
| picks: walk / feed / turn / rest / dig / drop / eat / pick up | 64% / 34% / 1.0% / 0.4% / 0.16% / 0.12% / 0.10% / 0.07% | 65% / 33% / 1.0% / 0.4% / 0.13% / 0.11% / 0.15% / 0.10% |
| verbs on in v2-C3 / options for off verbs | all eight that act / 0 | same |
| features with a live weight in v2-C3 | 45 of 45 (184 of 450 scorer weights, 10 of 16 units) | same |
| warmth exactly 0 / under 0.01 | 84.2% / 99.2% of options | 88.6% / 98.5% |
| roofed differs from the head's | 5.7% of options, 19% of decisions | 3.8%, 14% |
| all nine scent reads zero | 0 options | 0 |
| regroup calls / past the fast path / minted | 1,200 / 1,200 / 0 | 1,200 / 1,200 / 0 |
| ticks above the egg bar (walk runs) | 65.1% | 88.9% |
| ... no home cell in the 9x9 | 79.8% | 70.6% |
| ... site found (egg laid) | 1.1% | 0.2% |
| ... of sites found, layer not beside home | 61% | 27% |
| ... tries from the same cell as the ant's last try | 34% | 32% |
| at nest / flipped since last decision | 21.5% / 2.2% | 15.9% / 2.3% |
| falls (ticks spent falling) | 5.0% | 6.3% |

Three negatives worth keeping: an option cache cannot pay (the list repeats 0.04% of the time even with the ant standing still a third of the time, because neighbours move and scent and hunger change under it); nothing the genome reads is dead, so lazy features and off-verb skips are empty for v2-C3; and "rest holds for k ticks" has nothing to hold, since the ant rests 0.4% of the time. The scent is never zero anywhere an ant stands in this colony, so a "no scent here" skip is empty too.

## A. Bit-identical, for the Creature performance lane (ranked by gain)

1. **The egg walk's "no home in reach" answer from a per-chunk count, not a 121-cell fill.** ~-9% of the frame. 70-80% of `pile_site` calls return at `any_home()`, after `HomeLens::new` has filled an 11x11 memo and summed 3x3 nest neighbourhoods over 13x13. Keep a count of home cells (nest material, and `nest_dug` membership) per chunk, maintained where `set`/`set_world` already classify the cell (lab-world's file; the `has_liquid` flag is the precedent) and where `nest_dug` is written; the 11x11 square touches at most four chunks, so a zero sum skips the lens entirely. Exact: the lens's `any_home` is true only if some cell of the square is home, and every home cell is counted. Without touching chunk.rs: scan the 121 cells for any nest or dug cell before building the sums, worth about half (the fill's `World::get`s are 123 M of the 887 M; the index loops and sums are the rest).
2. **Allocation churn, ~5-6%.** Per decision: `CellMemo::new` (13x41 buffer), `described` (24 x 196 B), `scores`, `present`, `shifted` x2, `targets` in `decide_planned`, eight landing `Vec`s from `body_after_step` (384 k a 600 frames), `fall_now_if_unsupported`'s `chain.clone()` and `segment_groups.clone()` every tick (50 k), and `pile_site`'s `seen`, `frontier`/`next`, and the lens's three Vecs (32 k calls). Thread-local scratch buffers, as `SCRATCH` already is for the options; nothing is reordered. malloc+free+realloc are 431 M of 6.97 G; memcpy another 154 M, of which the `Opt` copies in `push` and `opts[d.pick]` are inherent.
3. **Warmth once per field block** (the lane's in-flight corner read, -3%): the ceiling is 5.0%; the rule change in B2 takes the rest.
4. **`adjacent_nest` twice a tick** (once in `mind::tick` for `at_nest`, once in `context` for `c::AT_NEST`): compute once, -0.6%. `under_cover` at the head in `context` (52 M) is a column the memo already holds: -0.5% if the memo outlives the build.
5. **Roofed as a column scan.** Targets lie in 3-4 columns; one scan of 22 rows per column with a prefix "ground above" count answers every target, 66 memo reads against 154. The lane's per-cell memo measured nothing, so this is a 1-2% ceiling, not a promise.
6. **Small exact ones**: the minds' `HashMap<OrganismId, Mind>` is SipHash and is hit three times a tick (`has`, `get`, `get_mut`): Fx, -0.3%. `id_of("spoil"/"packedsoil"/"corpse")` three times a decision (18 M): cache on `Minds`.
7. **Not worth it**: regroup cadence (1.9% in total and a rule change), the option cache (0.04% hit), unread-feature skipping (none unread), off-verb skipping (none off).

## B. Rule changes, for Scotttt (each with what the ant gives up, the gate, and the number)

The Teacher lane is mid-run with v2-C3's line. Any of these changes the trajectory of the colony it is selecting in; the honest interaction is that a run started before the change and continued after it is two experiments. B2 and B3 are small enough that a genome trained before them should carry over; B1 changes how fast the colony breeds.

1. **After a failed egg walk, the ant waits before looking again.** Prototyped as `SCRATCH_EGG_RETRY=24` (frames; four ticks), switched on at the window so the pre-run colony is the shipped one. Walks 65,959 -> 14,222 (seed 1) and 79,771 -> 16,052 (seed 2), -78%; whole frame **6.97 G -> 6.25 G instructions, -10.3%**, same ant count in the window (408 -> 428 against 408 -> 430). At 48 frames: walks -87%, native 1.70 -> 1.45 ms and 1.62 -> 1.40 ms a frame. What the ant gives up: a site that opens within 24 frames of its last failure goes to another ant. Births in the 1,200-frame window moved, and not the way a delay suggests: 732 -> 1,177 (seed 1) and 160 -> 236 (seed 2), inside a seed-to-seed spread of 4.5x, so the sign is unknown. Gate: `scripts/labbench.py`, 12 seeds paired, births, starved, underground, colonies died out; plus a short in-run training run. **My recommendation: do not take it yet.** A1 removes most of the same cost bit-identically; take B1 only if A1 lands short, and then judge it on births. The finding behind both: 65-89% of ant ticks are above the egg bar, so the site, not the energy, is the lay gate, and every rich ant pays the search every tick.
2. **Warmth at the head only.** Drop `f::WARMTH` per option and read the temperature once a decision as a context input. -5% of the frame (-2% beyond the lane's exact fix). What the ant gives up: a per-target temperature difference that is exactly 0 on 84-89% of its options and under 0.01 on 99%; v2-C3 has live weights on it, whose effect at those magnitudes is under 0.01 before the sharpness. Gate: hashes differ by construction; labbench 12 seeds and a short in-run run; the trained weights on the column are lost, not translated.
3. **Let the scorer sum in a fixed dense order.** The inner loop is 8.2% of the frame (and the mind is 45% of the trail arena, 43% of the in-run colony). At 184 live weights of 450 the sparse `(feature, weight)` walk is slower than a 45-wide dense dot product with SIMD would be, but a dense sum adds in a different order, so it is not bit-identical: decisions change only where a draw sits within a rounding error of a boundary. -4 to -6% here, more in training. Gate: training stays deterministic (same build, same order); labbench; one trail hold-out scored before and after.
4. **The ant's tick cadence** (`tick_interval` 6, 4.8 with pace): 84% of the frame scales with it and nothing else does. It is the ant's speed against the world's clocks (pheromone every 12 frames, plants, water), and it is species physics shared with the game ant. Named because it is the only lever on the whole creature share; not recommended.
5. **Not recommended, measured**: the egg walk only beside home (27-61% of eggs are laid by ants two to four steps from a home cell, through the crowd); regroup every N frames (1.9% and it moves when a split is named).

One behavioural note for the Teacher lane rather than a cost: v2-C3's colony picks **Feed on a third of its decisions** (32,938 of 96,345), passing energy between adults and to larvae; walk is 64% and everything else under 1.1%.

## Reproducing

```
git fetch origin claude/project-thread-oo9r0d        # 0efe4017, counters only; never for main
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --example needscost
RAYON_NUM_THREADS=1 target/release/examples/needscost seed=1 at=40000 handat=40000 window=1200 reps=1 arms=mind mind=/mnt/project-files/perf/genomes/v2-C3.txt census=1
SCRATCH_FROM=40000 SCRATCH_EGG_RETRY=24 ... census=1           # rule B1
RAYON_NUM_THREADS=1 valgrind --tool=callgrind --instr-atstart=no ... window=600 cg=1   # the profile, ~8 min
```

Files beside this one: `census-seed1.txt`, `census-seed2.txt`, `retry-s*-k*.txt`, `callgrind-main-47372a09-top.txt`, `callgrind-retry24-top.txt`.
