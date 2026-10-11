# Adversarial review: scent physics and the gradient check

Reviewer: physics lane, 2026-10-06. Target: `needs-ant/design-doc-snapshot-2026-10-06.md`,
section *Scents and fields in depth* and every gradient claim elsewhere. Repo read at main 3ba1e7bd.
Labels: MEASURED (I ran it; command given), CODE-READ (file:line), INFERRED.
Scripts are copies in my scratch dir
(`/tmp/claude-0/-home-claude-Pixel-Physics/c8162b93-2657-592f-b840-597b957aa530/scratchpad/`),
importing an unmodified copy of `gradcheck.py`.

## Findings (appended as found)

### F1. BLOCKING. "Rooms are flat" comes from where the check puts the sink, not from the nest
**Hits:** Summary bullet 3; *Would a nest gas have a gradient?* (table row 1, "Sealed walls give the right shape and almost no slope in rooms", "This is physics, not our numbers"); *What this means* 2 and 6; *The steering law* ("Rooms have none").
**Evidence (MEASURED, `python3 -I expA.py <scratch> expA.jsonl`; 11 maps, 6-cell sensor, 1%; base reproduces the doc: span 6.0% [2.9-8.8], readable 9.1% [3.3-16.3]).**
The check's sink is "an open cell above the ground with nothing over it". So every covered passage in the spoil mound is inside the gas domain, between the nest and the sink. That mound holds 198-379 ant cells against 15-59 in the nest (results.jsonl). Change that one choice, counting every open cell above the ground line as outside, and the conclusion inverts:

| variant (medians over maps with a way out) | maps open | room spans | reads >=1% (all nest) | room cells | >=1% and points out |
|---|---|---|---|---|---|
| base (doc) | 7/11 | 6.0% [2.9-8.8] | 9.1% [3.3-16.3] | 8.5% | 8.5% |
| sink = every open cell above ground | **11/11** | **63% [53-68]** | **74% [64-86]** | **86% [73-95]** | **72% [61-82]** |
| no sources in the mound | 7/11 | 10.0% | 13.1% | 14.1% | 12.9% |
| brood weight 1 / brood only / ants only | 7/11 | 9.1 / 12.3 / 2.2% | 12.2 / 15.4 / 3.4% | | |
| one map instead of 11 | 7/11 | 5.6% | 8.4% | | |
| 8-neighbour exchange (diag 0.25 / 0.5) | **11/11** | 12.3 / 13.3% | 14.3 / 14.3% | | |
| reader: 0.5%, or log-ratio at 1% (identical) | 7/11 | | 14.5% | | |
| sensor 3 cells / 10 cells | 7/11 | | 2.4% / 22.5% | | |
| plant tissue treated as open air | 7/11 | identical to base | | | |

All four "corner gap" maps open when the mound counts as outside, so every one of those gaps is above the ground line, in the mound.
**INFERRED:** whether a nest gas has a room plateau here depends on how ventilated the mound is. The check assumed that rather than measured it, and no game rule defines it, because the engine has no gas and no "outside". Cox & Blanchard's plateau needs a long entrance tunnel. Here the long, thin "tunnel" is mostly the mound, which changes every few hundred frames (F3).
**Smallest fix:** make the row conditional: "with the sink at the mound surface rooms are flat (9% readable); with the sink at the door they read on 74%". Add the sink-location sweep to the gradient check. Make "where is outside" a line on the nest-air physics card. Drop "this is physics, not our numbers" until the sink is decided.

### F2. BLOCKING. The solved level does not say "how far in"; the door reads like the deepest room, and the level is a census
**Hits:** cue table row "Nest air, its level ... not needed: the level itself says how far in"; *What this means* 2 ("needs no gradient at all"); *Needs* "Idle ants are held inside by the level kinesis"; Principles "Local signals only".
**Evidence (MEASURED, `python3 -I expB.py <scratch>`; the doc's own sealed field, 7 open maps):**
- The level at the nest's door (the nest cell nearest open sky) is **91.7-97.7%** of the nest's highest level. Door to deepest room differs by only 2-8%.
- By superposition, ants in the mound supply **32-65%** of the nest's top level. That is a third to two thirds of the "stuffiness" deep in the nest, made by ants that are outside it.
- For 25 random room cells per map, sources within 6 cells (sensor reach) make **8-17%** (median per map) of that cell's level, and sources within 20 cells make 43-94%. So the level is mostly a resistance-weighted count of bodies the ant cannot sense. Solved as a steady state, it updates instantly at every refresh, where the doc's own stepped gas takes 1.5M frames. That is a global count in a field's clothing, which the "Local signals only" ruling allows only as a labelled stopgap.

**Smallest fix:** strike "the level itself says how far in". State that the solved level is 92-98% of its maximum at the door and is a census by construction, and label it under the global-count ruling with its local replacement named. If depth is wanted, read depth (rows under the ground, or "soil overhead", which `under_cover` already samples to 20 cells).

### F3. MAJOR. Over time the level jumps more than its spatial signal, and the nest's gas connection flips open and shut
**Hits:** cue table "Nest air, its level"; *What this means* 6 ("refreshed on a slow schedule"); Build order slice 4; Genes "Kinesis: its strength and the level where it acts".
**Evidence (MEASURED, `expB.py`; seed 1, all 101 maps 150k-250k, one per 1,000 frames, sources from that map only, which is what a refresh would see):**
- 4-neighbour exchange: **43 of 101** maps have the nest's only gas path through a corner gap, so the nest reads sealed. That is far more prevalent than "4 of 11 maps" suggests.
- With the walk's 8-neighbour connectivity (diagonal 0.5, as *What this means* 6 proposes), none are sealed. The nest's median level still ranges **434-1,490 (3.4x)** over the 100k frames. From one map to the next it changes by a median **16%**, a 90th percentile of **57%** and a maximum of **82%**, with jumps over 25% in **39 of 100** refreshes.
- The spatial signal a level kinesis needs (door against deep room) is 2-8% (F2). So a threshold on the absolute level (the gene "the level where it acts") would switch the whole nest between hold and release together, every few refreshes.

**INFERRED:** both the sources (ants move) and the geometry (the mound's passages open and close, 33-68 cells per 1,000 frames per the check's own `warm.py`) drive the jumps. I did not separate the two.
**Smallest fix:** add "level stability over consecutive maps" to the gradient check's gate. Any kinesis on nest air must be relative (a level compared with the level just walked through), not a threshold on the absolute level.

### F4. BLOCKING. Slice 1's kinesis stand-ins hold the mound, not the nest, at the proposed switch frame
**Hits:** Build order slice 1 ("idle kinesis", gate "time in the dug nest above the shipped walk's"); steering table "Kinesis ... `under_cover` and `Crowding` stand in"; Risks row "Idle ants pile up where the first few stopped"; Kill criteria after slice 1.
**Evidence (MEASURED, `expD.py` part (d); map_f050000 of seeds 1-4; covered = soil within 20 cells above, as `under_cover` does at creature.rs `fn under_cover`, COVER_REACH 20; crowded = >=3 other ant cells within 2):**

| seed @50k | nest ant cells (covered, crowded) | mound ant cells (covered, crowded) | surface |
|---|---|---|---|
| 1 | 28 (1.00, 0.82) | 96 (0.96, 0.95) | 242 |
| 2 | 27 (0.93, 0.59) | 124 (0.97, 0.97) | 249 |
| 3 | 26 (1.00, 0.73) | 107 (0.95, 0.97) | 243 |
| 4 | **0** (no dug nest reachable) | 6 | 416 |

A kinesis on "dark and crowded" at the switch finds about 88-117 qualifying ant cells in the mound against 16-28 in the nest, 4-7 to 1. Stopping raises crowding where it happens, so it locks in where it starts. Seed 4 has no nest to hold anyone at 50k.
**Model of the solved field instead (MEASURED in a mean-field model, `expD.py` (c); INFERRED as a prediction):** I redistributed the nest and mound ants with density proportional to 1/p(level), where p is a steep threshold kinesis (floor 0.02) and the level is re-solved each round. This fills the nest broadly rather than heaping: 88-93% of those ants end inside (from 10-15%). The densest 5% of cells hold 7-8%, the door zone 6-13%, and about half sit within 2 cells of brood, which fills 25-36% of the nest. It works only through the census effect (F2): the top level rises 1.9-2.4x as ants enter. But that field is slice 4, marked optional. Slices 1-3 never have it.
**Smallest fix:** say plainly that the mechanism for designed behaviour 1 is deferred to an optional slice, and that its slice-1 stand-ins favour the mound 4-7 to 1 on these maps. Either move a nest-held level (or plain depth) into slice 1, or pick the switch frame and stand-in so the first idle-cluster check can pass. State seed 4's no-nest case at 50k in the plan.

### F5. MAJOR. "Tunnels and shaft yes" is not what the check measured; tunnels are as flat as rooms
**Hits:** "The slope lives in the shaft and the tunnels"; cue table "Nest air, its slope: tunnels and shaft yes, rooms no"; *What this means* 1 ("nest air in tunnels") and 4; Hunger and Haul rows ("in tunnels, down the nest-air slope"); Idle row ("inward along tunnels by the nest-air slope 0.5").
**Evidence (MEASURED from the check's own `results.jsonl`, sealed air, the 7 open maps):** the non-room cells (the check's own `room` mask: under 70% open in a 7x7) are flat at 1% on **83.8-92.0%** (per map: 91.3, 83.8, 90.6, 85.9, 89.0, 89.6, 92.0%), against 84-98% for room cells. The pictures show where the readable 9% sits: a cross at the shaft's top, under the door (green in `inside-nest_s1_f200000.png`). The idle drive's "inward along tunnels by the nest-air slope" would read nothing on about 9 tunnel cells in 10. Per F1, that is only true with the sink at the mound surface.
**Smallest fix:** replace "tunnels and shaft" with "the top of the shaft, within a few cells of the door". Remove the nest-air-slope terms from the Idle and Hunger rows, or mark them `NestWay`-only until a field passes the check there.

### F6. MAJOR. "Walls ignored (today's planes)" is not what pheromone.rs would do; with today's fade the answer flips at the door
**Hits:** Summary ("today's terrain-blind planes point it away from the door"); table row 3; bullet "Today's planes make it worse ... at the door the field slopes away from the door"; *Four ways* row 1.
**Evidence.**
- **CODE-READ:** `src/sim/pheromone.rs` `step` (665-822) blends 3x3 with `diffuse`, rounds to u16, then applies `decayed` (489): `min(v*factor>>16, v-1)`. There is no sink anywhere, the sky included. Constants: DIFFUSE 0.25 (164), DECAY_RHO 0.03 (204), TRAIL_A_RHO 0.0 (271), LAB_B_DIFFUSE 0.05 (982), PHEROMONE_INTERVAL 12 (110).
- **The check** (`gradcheck.py` `solve`, eps=1, k=0) has no decay and **adds** a zero-sink on every open-sky cell.
- **MEASURED** (`expC.py`): that rule, run on s1@200k and s3@200k with no sky sink and nest-air deposits of 256 raw units a pass per source (11-map averaged positions, ant cell 1, brood 0.25), padded 40 cells:

| plane setting | max level | nest cells reading exactly 0 | reads >=1% | reading points out | at the nest door the field rises |
|---|---|---|---|---|---|
| blend .25, fade .03 | mound (s1) / nest (s3), nest/mound top 0.96 / 1.05 | 7-8% | 100% | 51-53% | **up, out to the mound** |
| lab B: blend .05, fade .03 | nest/mound top 0.99-1.02 | **28-29%** | 96% | 49-50% | **up, out** |
| trail A: blend .25, no fade (forced -1 only) | nest | 0% | 94% | 24-26% | down, in (the doc's picture) |

With a fade, the decay length is about sqrt((0.25/3)/0.03) = 1.7 cells (INFERRED from the constants). The plane is then a blurred map of where bodies are. It peaks where ants crowd, which on today's colony is the mound (F4's table), and it says nothing about the way out: about 50% of readings point out. Only the no-fade setting gives the doc's picture. The settling statement does not hold for a fading plane either. With fade 0.03 a pass, the field's memory is 1/0.03 = 33 passes, about 400 frames. The "492,000 frames" is the no-fade solve.
**Smallest fix:** relabel row 3 "walls ignored, no fade (trail A's setting)". Add the faded variant and state that it peaks where ants crowd (mound on s1) and rises outward at the door. The conclusion "a plane-laid nest air is useless for in versus out" survives under both settings. The specific "points away from the door" does not.

### F7. MINOR (conclusion stands, numbers overstated). Relaxation counts are an artifact of omega = 1.9; the direct solver is cheap
**Hits:** "Simple relaxation took 1,710-5,375 sweeps per refresh even warm-started ... A direct solver is the realistic route; its cost is unmeasured in Rust"; *What this means* 6; open question 4.
**Evidence (MEASURED, `expE.py`).**
- SOR on s1@200k, warm-started from 1,000 frames earlier, to warm.py's own tolerance: **omega 1.9: 3,940 sweeps; 1.97: 1,265; 1.99: 480.** Unpreconditioned CG: 236 iterations (rtol 1e-2), 295 (1e-4).
- warm.py's tolerance, 1% of the field's maximum, is 11-33% of the whole room's 3-9% variation. A field "converged" to it has in-room slopes of either sign.
- **Direct:** sparse LU of the sealed field (n = 900-1,369, nnz(A) 4,122-6,043) with minimum-degree ordering gives nnz(L) **6,364-9,271**, about **0.06-0.10 Mflop** to factor (sum of squared column counts). A solve is about 4*nnz(L), roughly 0.04 Mflop. Natural ordering: 0.40-0.94 Mflop. In scipy, overhead-bound: factor 1.3-2.8 ms, solve 0.04-0.13 ms.
- **INFERRED** for compiled code: well under 1 ms per refresh, including re-ordering, which is needed because 33-68 cells change per 1,000 frames. Cost allows N = 1 frame. At N = 1,000 the amortised cost is under 1 us a frame. The constraint is the worst frame, which pays the whole refresh at once (CLAUDE.md quotes worst-frame timing).
- Speed is not the problem. A field solved to steady state every N frames responds instantly where the gas it models takes about 1.5M frames, which is why it behaves as a census (F2) and jumps (F3).

**Smallest fix:** replace the sweep counts with "SOR at a tuned omega needs about 500 sweeps; a direct solve is about 0.1 Mflop". Choose N for the field's meaning, not its cost: smooth the solved level over a stated window (for example an exponential over 10k-100k frames), which also damps F3's jumps.

### F8. MINOR. The settling sentence mislabels its field
**Hits:** "At today's plane rate the leaky version needs 492,000 frames to settle within 10%".
**Evidence (CODE-READ):** `gradcheck.py` computes `settle_frames` only for `name in ('air_sealed', 'air_plane')`. The leaky `air_p02` is never timed. `settle/results.json` holds one 492000 and one null; the README says "walls ignored 492,000 frames". Per F6, today's planes fade, and a faded plane's memory is about 400 frames.
**Smallest fix:** "the walls-ignored, no-fade version needs 492,000 frames".

### F9. MAJOR. "Down leads out 99.6%" is an ideal float64 follower; an ant with a threshold reaches the sky from almost nowhere
**Hits:** table columns "Down the slope reaches open sky" and "shortens the walk out"; "Sealed walls give the right shape ... Down leads out and agrees with the shortest walk out"; Summary ("points the right way in tunnels").
**Evidence (MEASURED, `expF.py` (2); sealed field, 4 open maps).** I followed the steepest 1-cell descent, stepping only when the drop clears a minimum. Share of nest cells reaching open sky:
- ideal (float64, >1e-15): **99.5-99.8%** (the doc's number);
- a u16 storage step (1/65,536 of the top): 91.1-98.6%;
- an 8-bit step (1/256): **0-9.8%**;
- **a 1% relative threshold, as the doc's own sensor uses: 0-0.7%**.

The "right shape" is a property of the field, not of anything an ant can read with the doc's 1% rule. Together with F5, nest air's slope is followable only near the door.
**Smallest fix:** label those columns "ideal follower (exact values, 1-cell look)", and add the thresholded column beside them.

### F10. MINOR. A quarter to a half of the sealed field's few readable reads look through soil
**Hits:** table column "Sensor reads 1% or more"; gradcheck `sensor_contrast` (the front must be open, but the path to it is never checked).
**Evidence (MEASURED, `expF.py` (3)):** of the (cell, heading) reads at or above 1%, those whose straight line to the 6-cell sample crosses a soil cell: s1@200k 54/174 (31%), s2@100k 38/161 (24%), s3@200k 40/122 (33%), s4@295k 33/59 (56%). With the sink at the door (F1) it is 2-4% (80/2,347; 74/1,776; 122/2,182; 119/3,500), so F1's 74% is not inflated this way. The shipped sensor (6 cells, honesty gate on the sample only, lesson 7) would make the same through-wall reads. On a field solved over open cells, a through-wall read compares two unconnected galleries, so it is not a slope the ant can walk.
**Smallest fix:** add line of sight to the physics card's "where the sensor samples", and report readability with and without it.

### F1 addendum. The swing is continuous, not an extreme-case artifact
**MEASURED (`expH.py`; all 11 maps).** I kept the check's sink and let only the loose spoil *above* the ground line pass gas, at conductance eps. The ground around the rooms stays sealed.

| mound spoil conductance | maps with a way out | room spans | reads >=1% | >=1% and points out |
|---|---|---|---|---|
| 0 (the check) | 7/11 | 6.0% [2.9-8.8] | 9.1% [3.3-16.3] | 8.5% |
| 0.02 | **11/11** | 12.6% [8.9-17.3] | 16.8% [8.7-23.1] | 16.0% |
| 0.1 | 11/11 | 24.2% [18.8-27.2] | 28.2% [15.6-39.2] | 27.4% |
| 0.3 | 11/11 | 33.2% [28.4-35.1] | 46.4% [27.5-61.7] | 43.0% |
| everything above ground is outside | 11/11 | 63% | 74% | 72% |

The doc tests two cases: all soil sealed, and all soil leaking 2% (which points the slope at the roof). It misses the plausible middle: porous spoil in the mound and sealed ground around the rooms. Even 2% there opens every corner-gap map and nearly doubles readability, with no slope into the nest's walls.

### F11. MINOR. The larva row measures a proxy; the shipped kernel works better, but its strength saturates for the lay brake
**Hits:** table row 4 ("Larva scent, fades within 10 cells"); "A short-range scent from a source works"; Lay job ("lays less where larva scent is strong", audit loop L1).
**CODE-READ:** `src/sim/brood.rs:1281` `larva_scent` is not a field read by a sensor. It is a vector summed at the head: need*(dx,dy)/d^2 over a 13x13 box (`NURSE_SCENT_REACH` 6, :1266), larvae only, walls ignored. Strength is len/(len+`NURSE_SCENT_HALF` 0.25) (:1269). The check solved a sealed exponential field with reach 10 and read it with a 6-cell sensor, a different mechanism.
**MEASURED (`expF.py` (1); s1@200k, s3@200k, s4@295k; random subsets of 'b' cells as hungry larvae):** an ideal follower of the shipped vector ends beside a hungry larva from **100%** of cells in reach, at 100%, 20% and 5% hungry (2-24% start there). Its heading points into soil on 0-4%. It survives. But the strength at lay sites (cells touching brood) is **0.97-0.98** with all larvae hungry, **0.88-0.89** at 20% and 0.67-0.78 at 5%. A five-fold change in brood hunger, from 20% to 100%, moves the brake's input by about 0.1.
**Smallest fix:** re-run row 4 on the shipped kernel, and quote these numbers. Give the lay brake an unsaturated level (the summed need, or a half-constant sized to the brood pile) and say so on its physics card.

### F12. MINOR. The physics card is missing the five lines that moved the answer in this review
**Hits:** "Every term carries a physics card before it ships" (who writes, slope measured, threshold against storage step, where the sensor samples, self-writing).
Each of these moved a result above, and none is on the card:
1. **Where outside is**, the sink (F1: 9% to 74%).
2. **The field's time constant against its refresh**, and any smoothing (F3, F7).
3. **Line of sight** from nose to sample (F10: 24-56% of reads).
4. **How a reading becomes a term**: sign, ratio or log. A log-ratio reader at the same 1% reads 14.5% against 9.1% (F1 table), and the doc never says how a 0.2% contrast is scaled against persistence 1.
5. **Dynamic range against the source count.** The lay brake saturates (F11). A plane deposit of 10,240 raw units a pass per source reaches the u16 ceiling (63,568 after fade) on **23%** of nest cells and 8% of mound cells. At 2,560 the peak is 56,697; at 256 it is 5,648 (MEASURED, `expG.py` (b), s1@200k, blend .25, fade .03).

Also on source accounting (CODE-READ how-the-ant-works §2, `Chain(2)`, stack cap 4): the check weights each ant *cell* 1. An ant is two cells, and up to 4 can share a cell that the map shows as one 'a'. So the brood:ant weight actually used is 1:8 per ant, and crowds are undercounted. INFERRED small: brood weight 1 moves readability only from 9.1% to 12.2%.
**Smallest fix:** add the five lines to the card's definition, and say "per ant cell" in the source column.

### F13. MINOR. Wording in the gradient table
"Medians over the 11 maps that had a way out" contradicts row 1's "(7 maps with a way out)". Row 1's 99.6% and 96% are over 7 maps; rows 2 and 3 are over 11. **Fix:** say which n each row uses.

## Claims I tried to break and could not

- **The walk passes corner gaps** (CODE-READ). `usable_headings` (creature.rs:23765) calls `body_after_step` (:25549), which for a chain body is `chain_follow`, then `landing_is_placeable_through_tissue`. That checks only the cells the body lands in, never the two corner cells of a diagonal step. So "face-to-face exchange calls the nest sealed while ants walk out" is right. It is also more common than stated: 43 of 101 seed-1 maps from 150k to 250k (F3). All four sampled gaps lie in the mound, above the ground line (F1).
- **The check's arithmetic.** My independent solver reproduces the doc exactly: readable 9.1% [3.3-16.3], span 6.0% [2.9-8.8].
- **The averaging window does not manufacture flatness:** one map gives 8.4%, eleven give 9.1%.
- **Plant tissue written as '#'** in deeptrace's map (examples/deeptrace.rs `write_map`) does not move anything on these maps: identical results with plants treated as open air.
- **Self-blinding is not the problem for a solved field:** one ant cell's own share of the 6-cell contrast is 0.01-0.04% in rooms, against the field's 0.12-0.31%. In tunnels the two are comparable (0.03-0.07% against 0.05-0.27%), and both are under 1% (`expD.py` (b)).
- **"A direct solver is the realistic route"** holds, and it is cheap: about 0.1 Mflop per refresh (F7).
- **"A short-range scent from a source works"** holds, and holds better with the shipped kernel: 100% (F11).
- **Kinesis heaps (item 5).** In a mean-field model of the solved field there is no door heap: the door zone holds 6-13% and the densest 5% of cells hold 7-8%. Resting ants are enriched near brood by x1.46-1.59, and the enrichment is the same with the brood weight at zero (`expG.py` (a)). So a brood-column heap from brood emission is not predicted. The enrichment comes from geometry, and it gives designed behaviour 2 (worker rooms apart from brood) no help. The heap risk that does show is the mound under the stand-ins (F4).
- **The stepper's positive control** (shaft, 222k against 218k frames): not attacked, accepted.

## What I could not establish

- How much of F3's jumps come from ants moving and how much from the mound's geometry changing. I did not separate them.
- Rust cost: no builds were allowed, so F7's under-1 ms is an estimate from flop counts. The worst-frame effect is unmeasured.
- Kinesis dynamics. F4's model is a mean-field fixed point (density ~ 1/p), not a stochastic run. It cannot show "where the first few stopped" effects, and its threshold (0.5-0.95 of the starting top) is my choice.
- F4's switch-frame census uses the shipped walk's 50k maps, which is the design's own proposal. Where ants would be after some time under the new walk is unknown.
- What deposit rate a plane-laid nest air would use (the doc does not say). F6 is reported at 256 raw units a pass, with saturation at 2,560 and 10,240 in F12.
- Whether mound spoil *should* pass gas. No engine rule defines it, so F1 is a sensitivity, not a correct value.
