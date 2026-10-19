# The dig rung for the stage 2 brain: design (rev 2.7)

Ladder lane, 2026-10-18. This is rev 2.7 (10-19 00:17): at step 4 the blind control reads against the start, not against 0, because a shaft makes some climbing free (step 4 bar). Rev 2.6 (23:18): from step 4, S1-S3 pay by how deep the cut was, and step 4's bar counts loads the same way. The gated reruns found the trained diggers widening a pit one or two rows deep beside the shaft (section 3 under step 4, section 4). Rev 2.5 (22:05): the kept colony leaves the step 4-8 training suite and is scored once after training, on enough worlds that its floor bites (PR 723 review, S1). Rev 2.4 (20:20): after step 4's first runs gained nothing, only cuts below the surface pay S1-S3 from step 4, and kept arenas become pass/fail gates (change table, and section 3 under step 4). Rev 2.3 (16:11): step 4's bed, shaft order, S4 rule and bar are pinned down for the brain lane's PR 3 (change table). Rev 2.2 (16:08) corrected one line in section 7: the ref − 2 SE floor is for the kept colony only. Rev 2.1 was 08:30, with step 3's bar added 08:45. Rev 2 (07:45) was revised against the brain lane's adversarial review, `review-dig-rung-design.md` (verdict: build with changes). Rev 2.1 changes only the soil gate (section 5, and the lines that name it), after the independent brain review `review-brain-architecture.md` showed rev 2's gate cannot switch putting-down on. Rev 1 (06:50) and rev 2 are in the scratchpad history. Built on:
- `dig-from-scratch.md` (rungs, measured hand-rule table, dead ends);
- `dig-layouts.md` (bed generator);
- `next-rung-forage-dig.md` (suite score, controls);
- `stage2/near/trace-trained-ant.md` (the place trap);
- `stage2/colony/README.md` (brain cost).

Code was read at main `5f80a32c`. **Nothing here is measured unless it says so**; numbers marked *start* are first guesses to tune by tracing.

**What changed from rev 1.** "R*n*" is the review's finding *n*. All 17 findings are accepted, and none is disputed.

| Section | Change | Finding |
|---|---|---|
| 5 | The soil gate moves from memory rows to output-dead scorer units: no lag after a cut, and no trained memory overwritten. The memory truth table is withdrawn. | R1, R2, R16 |
| 5 (rev 2.1) | The gate moves again, to two hand-laid hidden units in the job head. The scorer route can veto a put-down but not switch one on: on every trained start the Drop target score reads 0.95-0.99. | brain review E5, E7, E8 |
| 3, 9 | `nest_shaft` and `door_loose` set between `bed.build_counted()` and `apply`. | R4 (and H0) |
| 1, 6, 9 | A mind has no depth sense yet. `f::SPOIL` includes lining, and `f::CUT` is never written. Both get fixed in PR 2, before step 4. | R3, R5 |
| 1, 9 | The mind's soil put-down needs real ground, not "2 of 3 cells not empty". | R6 |
| 4 | Every stage requires S1. S4 is paid per row climbed. The founding cut counts as "the hole". The door is the home seed cells. | R7-R10, R17 |
| 3, 11 | Budgeted energy for steps 1-8, replacing `never_starve`. The kept colony gets Dig switched on and is scored on heads. Dead links are zeroed at entry. | R11-R13 |
| 6 | The empty-trip cue is dropped. The forage-or-dig choice is trained in the colony, with hunger as the cue. | R14 |
| 4 | Chambers need a per-chamber census and a real joining passage. The 4× width cap is Scott's number. | R15 |
| 7 (rev 2.2) | The ref − 2 SE floor applies to the kept colony (scarce food with Dig on) only. Fetch and trail keep their share floors, `f < forget × ref`, because the Suite doc records the 2 SE floor as rejected on trail. | brain lane |
| 3, 4 (rev 2.3) | Step 4: "generator from 8 rows" means only the generator, whose lens already starts 8 rows down. Shafts 4, 8, 12 run in order. S4 needs S3, with rows paid capped at the cut's depth. Step 4 gets its own bar. | brain lane's four reads, 16:07 |
| 3, 4 (rev 2.4) | From step 4, S1-S3 pay only for cuts below the original surface; top-row cuts pay nothing and are printed. Kept arenas (fetch, trail, scarce-dig) become pass/fail gates: above the floor they count 1, below it the existing ×0.1. | step 4 readout, 20:15 (`stage2/dig/step4-sh4-README.md`) |
| 3, 4, 7 (rev 2.5) | The kept colony with Dig is out of the training suite for steps 4-8. It is scored once after training on 24 sealed mirrored pairs, and failing its floor fails the step. In training its floor sat below 0 on two of four starts, so the gate could never fire. A 2-row scrape is watched, with the response fixed in advance. | review-pr723-8cd4b7db.md S1, N2 |
| 3, 4 (rev 2.6) | From step 4, S1-S3 pay × min(1, depth / target depth). The target is the arena's shaft (4, 8, 12) at step 4 and 12 at steps 5-8. Each stage's 10-pellet cap takes its 10 deepest pellets. Step 4's bar counts a load as min(1, rows paid / shaft), so at shaft 4 a pellet from a 2-row pit counts half. | gated reruns, 23:14 (`stage2/dig/step4-gate-README.md`); review N2 |
| 3 (rev 2.7) | Step 4's blind control: the blinded hold-best scores at or below the start on the same sealed worlds, instead of about 0. A blinded ant gets about 1 deep pellet a world up the shaft, worth 6-9.5 under depth pay. | brain lane, 10-19 00:15 |

## 0. What Scott has settled (thread, 06:16-06:47)

- **The goal:** new-brain ants on bare ground dig their own nest, with no pre-dug mouth and no world help. That means one open door, a shaft, rooms under a real roof, and soil carried out and heaped away from the door. The same brain still forages and trails.
- **The 12 steps (section 3) and the per-pellet score (section 4):** "this sounds good" (06:25).
- **Chambers:** use the existing chamber tools (06:25), with no cap on their number (06:26).
- **Door clearing off from step 1:** my 06:29 default. H0 shows it is fair (section 3).
- **One put-down action, or two:** a card is open (06:33). I recommended one action, gated on what the ant carries, which was Scott's own idea. Work proceeds on that.
  - Scott asked why the action part has no hidden layer of its own.
  - Answer: the rev3 design made "hungry, so eat" one weight on purpose, and never weighed an XOR case. Hidden units in the job head are now the default (section 5, rev 2.1), as the independent brain review recommends.
- **An independent review of the whole brain architecture** is running at Scott's ask (06:47): `review-brain-architecture.md`. If it changes the job head, section 5 follows it.

## 1. What the new brain can and cannot do today (main `5f80a32c`)

| | Where | State |
|---|---|---|
| Dig acts | `mind.rs:565` `acts()`, `:1095` `act` | No. **Built on the brain lane's branch** (07:24, not pushed), with the jaw bill. |
| Soil put-down acts | `mind.rs:1209` | No. `put_down` returns at `:1216` before any spoil check. **Built on the brain lane's branch**: the spoil branch first, real ground only, a dump log, and dug and dumped counts. |
| Footing for a mind's soil drop | `options.rs:362` | `spoil_footing_drop()` is `SPOIL_FOOTING=ground`, which ships **off** (`creature.rs:16976`). So "footing" means only 2 of the 3 cells below are not empty: a body or a hole's diagonals pass. R6. **Fixed on the branch:** ground for minds. |
| Depth | `mind.rs:802` `c::DEPTH`, `options.rs:463` `f::DEPTH` | **Always 0 for a mind.** Both read `last_out_row`. Its only writer is `depth_slowed` (`creature.rs:13610`), on the game ant's path, behind `DEPTH_SLOW` (off). R3. |
| `f::SPOIL` | `options.rs:425-428` | Spoil **or packed soil**. `line_burrow` packs every neighbour of a cut, so the ant's own face reads as `f::SPOIL`. R5. |
| `f::CUT` | `options.rs:122` | Declared and never written. R5. |
| Senses that work | `mind.rs:67-101`, `options.rs:98-150` | Context: `c::SPOIL`, `c::CROP`, `c::AT_NEST`, `c::ROOFED`, `c::HOME_DIST`, `c::SINCE_NEST`. Option features: `f::UP`, `f::ROOFED`, `f::HEADROOM`, `f::DROP_BELOW`, `f::STAND`, `f::HOME`, `f::HARD` (spoil and packed soil both 0.95, soil 0.8). |
| The founding shaft and doorway | `Scenario::build` (`lab/scenario.rs:276-280`), then `dig_founding_shaft` (`creature.rs:6803`) | Cut **during** `sc.build()`. `nest_shaft` and `door_loose` must be set before `apply` (R4, H0). |
| The door on bare ground | `paint_nest_patch_with`, `NEST_DOOR_SHIPPED` | Founding always paints a 5-column nest pad (`nest`, 6.0, which no jaw can cut). Home (`step_nest_dug`, `world.rs:9431`) seeds from the two open columns just past the pad, and is rebuilt every 256 frames (`world.rs:1575`). R10. |
| Verbs in a suite | `challenges.rs:1671-1679` | **Each arena cuts the mind to its own verbs.** Scarce has no Dig. Rev 1's "bans are the union" was wrong for minds (R12). |
| The trained start | the colony starts | No memory row is free. The Dig job row has 14-15 live links. **About a third of its links can never move a decision** in the arenas trained so far (R11). |

## 2. Every mechanism proposed here, against `Reports/dead-ends.md`

Rev 1's grep is unchanged: spoil, pellet, carry out, haul, door, pit, curriculum, step reward, shaping, tamp, mound, founding/shaft, jaw, dig cost. The review added entries rev 1 missed (marked *R*).

| Proposed here | Entry (line) | What it found | What this design does |
|---|---|---|---|
| Where a pellet goes is the brain's choice, not a rule in `act` | 1093-1094 | "First empty neighbour" refilled the tunnels. "Only under clear sky" sent every pellet down the shaft. The owner's ruling: placement belongs to the genome, as a brain output or sensor. | The put-down has no policy. Physics stays: real ground under the pellet (1099; R6). The gate is genome weights a lineage can lose. |
| A gate on "in the open" | 1101; 1103 | A perfect cover gate gave 0.49× the room: "an ant that cannot put its pellet down stops digging". Walked carry-out made a fifth of the nest. | The gate reads at-nest and distance from home, and is soft. Print the held-time share and digs per ant. Expect smaller nests than the game ant's lift gives. |
| Put soil down off the nest | 1206 | A lever **only with `SPOIL_FOOTING=ground`** (built share 3.1% → 8.4%). Without it, 64% of pellets lost their footing anyway. | Now met: minds put soil down on ground only (R6). |
| `SurfaceCurvature` on the drop; cover weighting on digs | 1208; 1145 *R* | No effect. | Not used. D1's `f::ROOFED` preference is a start only, and it can't fire until depth exists (R3). |
| Aiming haulers at the door | 1205, 1216, 1218, 1219 | At 200 ants the limit is traffic in the one-cell shaft. | Learned walking, scored. Step 9: print queueing at the shaft foot. |
| Keep the door open | 1241, 1239, 1220; 1221 *R* | The plug is tamped soil, and the rim falls in. Food set down at home was the largest thing shutting the lab door. | Step 6 scores the door open, whatever closes it. H0: loose soil at the rim plugs the foot within about 500 frames. Steps 9-12 print food at the door (R17). |
| No roof rule, heap cue or dig-down in the mind | 1225 | Not what kept the lab nest small. | The mind's Dig has no vetoes. |
| Hunger and digging | 1209; 1232 *R* | A hunger gate made a colony stop building whenever it was hungry. Stopping foragers from digging **halved food home**, because their cuts are the passages through the mound. | No hand gate. The forage-or-dig choice is trained in the colony with hunger as a soft cue (section 6). Don't train foragers out of digging. |
| Dig where others dug | 1272 | Failed. Scott's frustration with this is in memory. | Not proposed. |
| Home that grows | 1196, 1197 | Diggers starve in a deep home; mound homes built towers. | Score against the door seed cells, not home (R10). |
| Rooms from crowding; flat rooms | 1925-1928; 1284-1285 *R* | Crowding fired but built no rooms. `FLAT_ROOM` is "the only rule so far that made separate brood chambers". | Chambers are scored, not wired. 1284 is the evidence base for steps 10-11. |
| A sensor fed by one source | 1105 (second entry) *R* | A general-named sense with one source goes wrong when a second source appears. "Nothing in a genome can compensate for an input that is constant". | Minds' `c::DEPTH` and `f::DEPTH` are exactly this (R3). Fix them, and zero their drifted weights. |
| Dead weights | 1298 *R* | "Inert in what they steer, not in what they cost". One step on one such cut tripped home 3.5×. | Zero them at entry (R11, section 11). |
| Step scores; a verb back on at its own job constant | 2100-2101 *R* | The forage step score was the fix. A reopened verb took a fifth of all decisions, hence `QUIET`. | The per-pellet score follows that precedent. Dig gets a stated job start (R11). |
| Curriculum, step rewards for digging | none | "curriculum" has 0 hits. | New ground. |

## 3. The steps, and the arena for each

**Settings are `World` fields set by the arena, never env switches.**
- Set **between `sc.bed.build_counted()` and `sc.apply(...)`**: `nest_shaft` (`world.rs:4479`) and `door_loose` (`:4446`). The founding cut happens inside `apply` (R4, as H0 did it: `probes/dig-door/door-h0.diff`).
- Set after the build: `door_column` (`:4449`), `home_blind` (`:4404`), `births_paused` (`:4620`).
- Guard: a step-7 world has `nest_sites[0].shaft == None` and no dug cells at frame 0; a step-1 world has a 2-row shaft. It must go red when the field is set after `apply`.

| Step | Ants | Bed | `nest_shaft` | `door_column` | Energy | New thing scored |
|---|---|---|---|---|---|---|
| 1 Cut | 1 | `DigBed`, flat | 2 | drop on, clear off | budgeted | S1 |
| 2 Let go | 1 | same | 2 | same | budgeted | S2 |
| 3 Away from the hole | 1 | same | 2 | same | budgeted | S3 |
| 4 Up and out | 1 | generator: depth 24-52, lens, raised step; mirrored | 4, then 8, then 12: three sub-runs in order, each from the last one's hold-out best | same | budgeted | S4 per row (needs PR 2: depth sense) |
| 5 At the door | 1 | generator | 12 | same | budgeted | only cuts joined to the door, by flood at the bell |
| 6 Door open | 1 | generator | 12 | off | budgeted | × D |
| 7 Bare ground | 1 | generator | 0 | off | budgeted | the same; door = home seed cells |
| 8 A roof | 1 | generator | 0 | off | budgeted | 32 per room cell |
| 9 Twelve ants | 12 | generator | 0 | off | real, food drip 10-30 cols | colony total × founders kept |
| 10 Chambers | 12, then 24 | soil 40-52 | 0 | off | real | spec-chamber cells 64 |
| 11 More chambers | 24 | same | 0 | off | real | +1,024 per extra spec chamber |
| 12 Scarce food | 12 | Scarce bed with soil | 0 | off | real, births on | dig × forage, against the start |

- **Budgeted energy (R13), steps 1-8.** It replaces `never_starve`'s 1e9.
  - Use the same rescale to a finite budget E per step: about 1.3× the start genome's measured burn over the run.
  - A lineage that grows links or digs wastefully runs dry before the bell and loses its last pellets.
  - Energy falls the way an unfed founder's does, so the pinned 0.5 arena cue (C9) is gone.
- **The kept colony from step 4 (R12).** A Scarce variant with Dig switched on, scored on heads only (`steps=off`).
  - Its forget floor is ref − 2 SE, not `f < forget × ref`.
  - References are calibrated on the **unmodified** start, with a pre-flight on the modified one, so harm the gate does is not taken as the baseline.
  - Cost (inferred): it will dominate generation time.
- **Door clearing off from step 1: fair (H0, 07:09).**
  - A rest-only mind ant, budget-free (fed), 20k frames, depths 2/6/12, seeds 1-4: the door was open on 41 of 41 checks in all 24 runs, and nothing filled. `column_cleared` stayed 0 even with clearing on. A shaft filled by hand read shut on all 41.
  - What plugs it is loose soil beside the mouth. A 4×6 pile just west of it shut the door by frame 500 on 4 of 4 seeds, and the door never reopened. It shuts at the **foot**: soil falls into the chamber and rises 2-5 cells. Keep `drop` on until step 6.
- **Step 9 waits** until the colony stage clears a founders-kept bar, or it is a survival score (R17).
- **Run length:** 3× the hand digger's median time to the step's top stage, at least 4,000 frames. Two horizons, 1× and 2×.
- **Moving on:** the step's top stage on every hold-out in at least 3 of 4 seeds, worse side. Blind and rest stay low, fetch and trail hold, Scott sees the picture (and a GIF for steps 4, 6 and 7). Then start from that step's best. The early stop on main applies (patience 30, never before generation 40).
- **Measured on PR 720 (brain lane, 08:32; `stage2/dig/README.md`): steps 1-2 test nothing once Dig and Drop are on.** A blank mind cuts original ground and puts it down on 12 of 12 hold-outs (S1 13.5, S2 13.3 a layout), so training starts at step 3.
- **Step 3 needs a count and a share, because "top stage on every hold-out" is already met by chance.** Trained starts that only wander while laden put 1-3 pellets away on every layout (c33-trim-k450: away 1.75 a layout, at most 3; 10-30 fetch winner 1.92; away share 0.27-0.39; blank 0.08, share 0.01).
- **Step 3's bar (provisional, until the hand digger is measured).** Hold-out, 12 layouts, worse side, in at least 3 of 4 seeds:
  - **at least 5 pellets put down away on at least 11 of 12 layouts.** No start reaches 5 on any layout. Blank's 13.5 original-ground cuts a run show one ant can cut that many in the time;
  - **away share at least 0.75** of original-ground pellets put down, pooled over the 12 layouts (starts 0.27-0.39);
  - **pellets put back in the hole at most 0.5 a layout, as a mean over the 12 layouts** (starts 1.0-1.1, blank 7.75, also means). It is not a limit on every layout;
  - **falls are printed, not gated, at step 3** (ruled 10:25, after round 5). The score already charges each fall -4, the same as a pellet. My reason at the time, that the ant's heaps spread toward the trench, was wrong. The falls land in the founding chamber (see the step 3 result below), so **from step 4 they are gated**;
  - the ant alive with energy left on every layout, fetch and trail inside their kept floors, and Scott sees the picture.
  - **Then replace the 5** with half the hand digger's median away count, once preset `digger` exists. Keep the share.
- **Step 3 result (11:25, four gated seeds; `stage2/dig/README.md`): NOT passed under the 3-of-4 rule. Moving on anyway (ladder ruling 11:30).**
  - Seeds 71 and 72 meet the bar on both hold-out and sealed worlds. Seeds 73 and 74 miss only on pellets put back in the hole (hold-out 0.75 / 0.83, sealed 0.83 / 0.67), and 74 also on share (0.73). Away >= 5 holds on 47 of 48 hold-out worlds.
  - 73 and 74 never beat the untrained gated start on pellets (0.83), and both stopped flat early. That reads as a missing sense, not missing training.
  - **Checked (brain lane, 11:40; "Where the pellets and the falls go back in", `probes/round5/refills/`).** All 40 pellets put back, on 72 worlds, land in a one-row surface scrape 12-53 cols from the door; none land in the shaft. 26 of the 40 are within 2 cells of the scrape's outer end. Without the gate they went back at the door. **The ant stands on the surface beside the scrape when it puts down, so its own depth reads 0.**
  - So step 4 starts from seed72 once PR 721 and PR 2 land, with two changes at entry:
    - PR 2 writes `f::CUT` on the target cell (dug this run, or part of the founding cut), and a veto unit on the scorer pushes down Drop targets with `f::CUT` = 1. This is a veto through `best`, the one thing the scorer route can do.
    - ~~h0 also reads the ant's own depth~~ (dropped 14:05: it blocks food put down under cover in the chamber; the Drop job's `best` read does this job instead, section 9 item 5).
  - The truth table gains the row "soil, standing beside a 1-row scrape 12-53 cols out: hold". The pellet line carries into step 4 unchanged (mean <= 0.5).
  - **Falls are the nest filling, not heaps spreading (my reading was wrong).** 62 of 70 falls land in the founding chamber (38) or in the cell cut under its floor (24), about 1 chamber cell a run out of about 18; ungated, there were 2-4 falls in 12 worlds. So **step 4's bar adds: soil fallen into the founding chamber or shaft, mean <= 0.5 cells a run** (ungated 0.17-0.33, gated 0.83-1.00). From step 4, print chamber fill at the bell and the source of each fall (a put-down from `dump_log`, or a cut). **Traced (brain lane, 11:45; "What drops soil into the founding chamber", `probes/round5/falls/`):** none of 83 grains came from a put-down, and none from the first cut. 66 came from surface cuts, 65 of them the same rim cell, two columns right of the shaft: cutting it drops the shaft's shoulder into the chamber 3 frames later. 15 came from roof-row cuts. The crumb rule (`update.rs` ~1220, with its "nothing beneath" gate, dead-ends.md:1095) holds a worked cell over air only while 3+ of its 8 neighbours are non-empty, and the founding cut's shoulders sit at exactly 3. Nothing the ant senses separates that cell from any other rim cell.
    - Moving Dig's start off `AT_NEST` is not the fix. On `ROOFED`, falls drop to 0.17/0.42, but cuts halve, worlds with 5+ away go from 11-12/12 to 0/12, and pellets back rise to 1.50. So Dig at the nest stays.
    - A thicker founding cut would only hide the problem in the arena: every gallery roof edge the ant cuts later has the same shape.
    - **The fix goes in PR 2 (item 5).**
- **Step 4's bar (rev 2.3, provisional, until the hand digger is measured).** At each shaft, 4 then 8 then 12: hold-out, 12 layouts, worse side, in at least 3 of 4 seeds:
  - **at least 5 loads a world on at least 11 of 12 layouts**, and **at least 0.75 of the depth-weighted cutting carried out**, pooled over the 12 layouts. **A load is counted by depth** (rev 2.6): each pellet earning S4 counts min(1, rows paid / shaft). So a pellet cut at the shaft's depth and carried all the way up counts 1, and one from a 2-row pit at shaft 4 counts half. The share is Σ min(1, rows paid / shaft) over S4 pellets ÷ Σ min(1, depth / shaft) over cuts below the surface. Until rev 2.6 the bar counted any pellet earning S4, so a shallow pit could meet it;
  - pellets put back in the hole at most 0.5 a layout, and soil fallen into the founding chamber or shaft at most 0.5 cells a run, both as means over the 12;
  - the ant alive with energy left on every layout; fetch and trail inside their share floors; Scott sees the picture and a GIF;
  - **blind stays low, read against the start** (rev 2.7): the hold-best, blinded, scores at or below the start on the same sealed worlds. The share test in section 7's table still applies. **Why not about 0:** a shaft makes some climbing free. A blinded ant cuts about 1 deep pellet a world at the shaft foot and carries it 4-5 rows, worth 6-9.5 points under depth pay. So a gain that blinding does not take away is geometry, not reading. **Measured (brain lane, 00:15; the gated reruns' genomes re-paid by depth, 12 sealed worlds):** blinded hold-best 6.0 / 9.5 / 9.0 / 9.4 against starts 12.4 / 10.1 / 11.3 / 12.4 (s81-s84), so all four pass. The starts sit only 1-3 points above blind, because nearly all a start's pay is its 1-2 deep pellets;
  - **the kept colony with Dig, scored once after training** (rev 2.5): the champion on 24 sealed mirrored pairs, beside the start on the same pairs, at or above the start's mean − 2 SE of those 24. A champion under it fails the step.
  - **Score the start (seed72 after PR 2) on all three shafts at generation 0 first.** If it already meets the bar at a shaft, the bar measures nothing there: raise the 5 and the share above the start before training. **Measured (brain lane, 16:22; worse side, 12 hold-outs):** S4 pellets 2.2 / 1.8 / 2.0 a world at shafts 4 / 8 / 12, rows climbed 5.3 / 7.3 / 7.3 (world totals: per S4 pellet 2.4 / 4.1 / 3.7, corrected 23:14); seeds 71, 73 and 74 read 1.5-2.5. All far under 5, so the bar stands. Pellets back at most 0.17 and chamber fill at most 0.33 at every shaft. A "deep cuts" print is being added so the 0.75 share can be read. Replace the 5 with half the hand digger's median, once preset `digger` exists.
  - At shafts 8 and 12, **print** the shallower shafts' hold-out lines as well (not gated), so a lineage that forgets the shallow climb shows.
  - **Scout (brain lane, 18:16; local, pop 24, shaft 4, early stop at 40 gens):** hold-out best 1.007 at gen 20 against 0.995. S4 pellets a world 2.17 -> 2.50 at shaft 4, 1.83 -> 3.17 at 8, 2.00 -> 2.00 at 12: inside the layout spread, far under 5. S3 rose 8.8 -> 10.7, past the cap of 10, so part of the gain bought nothing. **Not a verdict on the design:** pop 24 is a quarter of a real run. The pop-100 run reads the deep-cuts print first: if most S3 pellets are surface cuts, the ant is quarrying the rim instead of climbing, and step 4 then pays S1-S3 only for cuts below the original surface.
  - **The cap limits points, not eligibility.** A pellet past the 10-pellet S3 cap still earns its S4 if it was put down away.
- **Step 4 readout (brain lane, 20:15; `stage2/dig/step4-sh4-README.md`): four pop-100 runs at shaft 4 gained nothing.** Soil score 62.7 / 67.5 / 68.5 / 67.7 -> 61.5 / 67.5 / 56.0 / 62.2; S4 a world 2.17 / 2.17 / 1.50 / 2.00 -> 2.33 / 2.17 / 1.83 / 1.67 against the bar of 5 (re-scored on PR 722's head, 12 hold-out pairs, worse side). Two causes:
  - **It is rim quarrying.** About 4 cuts in 5 are in the top row: 12-13.5 cuts a world, only 2-2.8 deep, and the diggers stop just past the S1-S3 cap of 10. S4 cannot exceed the deep cuts. This is the trap `dead-ends.md:1142` found in the game ant's dig roll: a score for breaking *original* ground pays for scraping a flat surface. **So from step 4, S1-S3 pay only for cuts below the original surface** (S1 row, rev 2.4). A deep pellet put down away then pays 7 plus its S4, and a rim pellet 0. On bare ground (step 7) the first cut of a new hole pays nothing, and the cut beneath it pays in full.
  - **The kept colony's luck carried selection.** The fitness is the geometric mean of four arena ratios. Scarce-dig is noisy for one genome: s81's calibration reads a reference of 2.51 with a spread of 3.04 over 6 units. On the hold-out its ratio went from 0.44 (the start) to 1.61 (s81's champion), against about 0.015 in fitness for one more S4 pellet (corrected 20:28: the brain lane first gave 0.44-1.61 as one genome's spread). s81's champion gave up soil 62.7 -> 41.3 for a scarce-dig ratio of 1.61. **So from step 4, every kept arena is a pass/fail gate:** it counts 1 above its floor and the existing ×0.1 below it. Its ratio never enters the product. Clamping at min(ratio, 1) was the other option, but it keeps the downside noise: with a spread larger than the reference itself, a ratio below 1 from luck alone costs many pellets' worth. The forage-or-dig trade-off is trained where it is the task, in dig steps 9-11, not through a kept arena here.
  - **The colony gate could not fire on two seeds (PR 723 review, S1; 22:02).** Its floor is ref − 2·sd/√pairs, and with 4 training pairs it sat at −0.53 (seed 71) and −1.00 (seed 73). Colony heads never go below 0, so the costliest arena in the suite did nothing there. In training, lifting the floor above 0 on every seed needs at least 9 pairs a generation, and lifting it to half the reference about 34: 2-8 times the arena's cost, which already dominates a generation. **So from rev 2.5 it leaves the step 4-8 training suite** and is scored once after training on 24 sealed pairs, where every start's floor is positive: seed 71 2.51 − 2 × 3.04/√24 = 1.27, seed 73 2.21 − 2 × 3.22/√24 = 0.90. Steps 4-8 are one-ant arenas, so the harm the colony guards against (digging when the colony needs food) is trained where the colony is the arena, in dig steps 9-11. If a champion fails the after-training check, the gate comes back for that step as a check on each new hold-out best, not on the whole population.
  - **A 2-row scrape can still fill the cap (review N2).** A cut one row under the top pays in full, so scraping 12 columns two rows deep scores 82, against 70 for the old 1-row scrape and 166 for twelve 8-row pellets. Deep digging still wins, but the trap can come back one row down. **Watch `up_rows` per S4 pellet in the reruns** (2.4-3.1 at gen 0; the 5.0-5.7 first written here was the world total, corrected by the brain lane 23:14). **Fixed in advance:** if it falls to about 2 or below in training, S1-S3 are scaled by depth, × min(1, depth / shaft depth). **Triggered (23:14):** s83's hold-best climbs 1.89 rows a pellet on sealed worlds, and its GIF shows a pit one or two rows deep beside the shaft. The scaling is rev 2.6, in section 4.
  - **The blinded control now sits closer to the start (review N5):** 0.36-0.68 of the start's soil score at gen 0, against about 0.2-0.3 before the rim rule. With 1-3 deep pellets a world this has little power at gen 0; read it again after training.
  - **Guards for the rerun:** 12 top-row pellets put away score 0 at step 4, and 12 deep ones pay S1-S3 for 10 and S4 for all 12; a genome above every floor scores exactly its soil score; one below a floor scores ×0.1. Print the share of hold-out bests vetoed by each floor, because a gate that vetoes often is a noise source of its own.
- **Gated rerun readout (brain lane, 23:14; `stage2/dig/step4-gate-README.md`): two of four seeds dig more from below the surface, as a shallow pit.** Sealed worlds, worse side of 12:
  - s83's hold-best went from 1.67 to 4.50 S4 pellets a world and s82's from 1.00 to 2.67; s81 and s84 stayed flat.
  - Rows climbed per S4 pellet fell, for s83 from 2.90 to 1.89: the pit beside the shaft. That met the N2 trigger above, and rev 2.6 is the response. Under rev 2.6's bar, s83's 4.5 pellets count about 2 loads (inferred from the mean rows; the brain lane prints the exact count).
  - s83's fetch sits at its floor, 19.8 against 24.3.
  - **Re-paid by depth (brain lane, 10-19 00:15; same genomes and behaviour, 12 sealed worlds):** s83's hold-best scores 21.75 against its start's 11.25 (1.93×, was 2.59×) and s82's 12.67 against 10.08 (1.26×, was 2.01×). s81 and s84 stay flat. Most of s83's remaining lead is S4 itself, 4.5 pellets × 1.89 rows.
  - About 9 children in 10 fall under the fetch floor. The cause is the mutation (grow 1 adds about 70 random links a child), not the gate. The ungated runs' median was as low, and hold-out champions are rarely vetoed.
  - On seeds 82 and 83 a champion beat the start's hold-out for 12-25 gens and was then lost.
  - **Next batch:** soil + fetch + trail, 16 layouts a gen, grow 0.1 and sigma 0.3, rev 2.6 scoring, and the colony scored after training (rev 2.5).
  - **Fix-plan step 7, trigger agreed in advance:** a champion beats the start's hold-out for 10 or more gens and the run then falls back to the start's level. Then the hold-out best goes back into each gen, and this bar moves to sealed worlds.
- **Step 4 beds (rev 2.3).** Two checks on the generator, both from `dig-layouts.md`:
  - **A mirrored pair with no lens and no step is the same bed twice** (its finding 1). **Already handled (brain lane, 16:22):** `DigBed` forces the raised step on any bed that drew no lens.
  - **The generator was tried with a 6-row shaft.** **Already handled (brain lane, 16:22):** the lens starts 4+ columns from the nest and the chamber spans ±3, so at most it walls the chamber's side. The brain lane adds a guard over 64 seeds × both sides at shaft 12 (the cut's full 12 + 2 rows) to keep it true. **Guard result (18:16):** a 12-row shaft is whole on all 128 beds; stone first bites at 16 rows, and from 24 rows a bed can fail to found an ant. So shafts up to 12 are safe on this generator.
  - Trail: `colp30-on-seed52` scores 0.752 before and after `enter_rung`, with byte-identical hold-out lines, so entry costs no trail. The round-4 runs simply gained more trail in training.
- **Choices the brain lane made where rev 2 was silent, all agreed:**
  - refill is charged only from step 3;
  - steps 1-3 use a flat bed, not mirrored, with each layout on its own dice (a flat mirrored pair is the same bed twice). Mirrored pairs come back with the generator beds from step 4;
  - the hole is original-ground cells cut this run plus the founding cut, and re-cut tailings are not hole cells;
  - `SOIL_BUDGET` is 1,200 J (starts burn 800-1,096 J). Print how many layouts end with under 10% left. If the champion does, the budget and not the skill is the limit, so go back to about 1.3 times the start's burn.

## 4. The score

### Per pellet: the soil trip (steps 1-5)

Read from `cut_log` and the new `dump_log` (x, y, frame, carrier).

**A pellet earns nothing at any stage unless its cut earned S1 (R7).** This shuts out the tailings shuffler.

| Stage | Points | Counts when |
|---|---|---|
| S1 cut | 1 | The cut cell was original ground: ground at frame 0, never dug before, and not a placed pellet. **From step 4, also below the original surface** (depth >= 1, measured as for S4), so a top-row cut earns nothing at any stage (rev 2.4). **From step 4, S1, S2 and S3 are each paid × min(1, depth / target depth)** (rev 2.6), with depth as S4 reads it. The target is the arena's shaft (4, 8 or 12) at step 4, and 12 at steps 5-8 (provisional until step 7, which has no dug shaft, is built). A top-row cut has depth 0, so it still pays 0: this rule contains rev 2.4's. S4 is unchanged. |
| S2 let go | 2 | Put down on ground. Not still held at the bell, not lost on death. |
| S3 away from the hole | 4 | The put-down cell is at least 3 cells (Chebyshev) from every hole cell **at the bell**, and not inside one. A hole cell is any cell dug this run, plus every open cell below the original surface at frame 0 that is joined to the door: **the founding cut is part of the hole** (R9). "And stays" is left to the refill cost (R17). |
| S4 up and out | 8 × min(1, rows paid / 8) | **Needs S3** (rev 2.3): a pellet put down at the rim is not out. Rows paid = min(rows climbed from the cut to the put-down, the cut's depth), so climbing a heap or a step with a shallow pellet earns nothing extra. **Per row** (R8). Only cuts below the original surface count. Depth is measured from the **lowest** original surface within ±3 columns of the cut, so a step's face is not a pit (R8). Only once the depth sense exists (PR 2). |

- **Caps.** Once a later step is trained, each earlier stage pays for at most 10 pellets per ant. The newest stage is uncapped. **From step 4 (rev 2.6) the 10 are each stage's 10 deepest pellets.** So an extra shallow pellet never pushes out a deep one. The cap counts pellets, not scaled points, so twenty half-depth pellets cannot fill it. Worked at shaft 4: twelve 2-deep pellets put away score 35 + 24 = 59 (94 under rev 2.5); a 2-row scrape of 12 columns scores 17.5 + 12 = 29.5 (82); twelve 4-deep pellets score 70 + 48 = 118 (118).
- **Refill.** At the bell, after the settle, every hole cell (founding cut included, R9) that is ground again costs 4. Print it split by cause: a pellet, or a fall.
- **Step 5, joined to the door.** Scored at the bell by a flood from the door seed cells through dug cells (R10), not by `at_nest` at the cut. `nest_dug` is rebuilt only every 256 frames, so it goes stale behind a fast face.

### Higher steps

- **The door (R10).** `nest_dug`'s seed cells: the open columns just past the painted pad. On bare ground they are found the same way.
- **Step 6, D.** The share of checks (one every 500 frames) where a flood from the door through empty, ant, brood and crumb cells reaches open air (`doorseal.py`'s rule, ported). The whole score is multiplied by D.
  - An open crater keeps D = 1. That is accepted until step 8. Print the pit.
- **Step 8, room.** 32 per cell of `joined_room` gained. **Print room depth** and the count roofed only by lining (`Filled`): a tunnel one row under its own lining counts today (R17).
- **Steps 10-11, chambers (R15).** digbox's `chambers_of` returns aggregates: a count, medians, the widest, and one global passage bore. So the rig needs a **new per-chamber census**:
  - each core component as a chamber, with its box, its cells and the passage that joins it;
  - over `joined_room`'s mask, not digbox's covered void;
  - shared with digbox, so both count the same way.
- **What a chamber needs:**
  - **Spec shape:** box 8-16 tall and at least as wide as tall. The upper width cap (I proposed 4×) has no basis in code. **It is Scott's number, and only needed by step 10.**
  - **Step 10:** room cells inside a spec chamber score 64.
  - **Step 11:** +1,024 per spec chamber after the first, uncapped. A chamber counts only if the passage joining it runs at least 4 non-core cells, which stops a pillar splitting one hole into two "chambers" (R15).
  - Print the structure percentile against digbox's `walkers` null as a check.
- **Step 12.** The suite's product, with worse side per arena.

**Printed, never scored:** ants alive, pellets still held at the bell, digs per ant, the held-time share, pit, mound, `under_fill`, `spoil_below`, and pellets set on a body (R6).

## 5. The place trap, and the start wiring

**The trap.** One put-down verb serves food and soil. The job value (`mind.rs:885-906`) is a weighted sum, and "food at the nest yes, soil at the nest no" is an XOR it cannot hold. Rev 1 proved this, and both reviews checked it. The brain reaches that kind of rule only through squashed units.

**Two routes tried on paper, and why each fails.**
- **Rev 1: memory rows.**
  - Memory updates after the pick, so about 48% of post-cut decisions put the pellet down beside the face (R1; E1 measured 48%).
  - Both trained starts have no free memory row (R2).
- **Rev 2: output-dead scorer units, read through the Drop job's `best`.**
  - The `best` input is pinned near its top: the Drop target score reads 0.974-0.990 in every food and soil state on all three colony starts, and 0.950-0.976 on the forager (architecture review E5). So a scorer unit can veto a job, but can raise it by at most about 0.03 × its `best` weight.
  - Rev 2's g5 (the veto) works. Its g6 ("soil far out, so put down") cannot. Of 896 settings of rev 2's wiring on the forager in the review's toy corridor, none kept food delivery ≥1.10 (start 1.20) together with a soil score ≥0.8 (E7). The best soil row, +0.95, put 36 food cells down away.

**Default (rev 2.1): two hand-laid hidden units in the job head.**
- The job head gets 8 hidden units. Each reads this decision's context (20), memory (12) and a bias, and feeds every job. That is 8 × 33 + 9 × 8 = **336 slots** (genome 2,333 → 2,669).
- All weights are zero at load, so behaviour is unchanged. But `is_mind(len)` refuses any other length, so this needs a **padding loader** (architecture review F9) and an identity guard: every saved champion is byte-identical in fetch and trail with the units at zero.
- At entry, lay rev 1's numbers on units 0 and 1. The units use the plain squash x/(1+|x|), and their outputs are added to the job sum before the job's own squash(sharp·sum), as in the review's toy (`placegate.py:289-291`). The brain lane is building this (08:35), with a loader that zero-pads 2,333-slot files.

| Unit | Inputs | Reads |
|---|---|---|
| h0: soil, and at the nest or under cover | `c::SPOIL` +40, `c::AT_NEST` +20, `c::ROOFED` +20, bias -45 | +0.94 with soil at the nest or under cover; -0.83 with soil in the open away from the nest, **and with food at the nest under cover**; -0.96 to -0.98 otherwise |
| h1: soil, and far from home | `c::SPOIL` +40, `c::HOME_DIST` +40, bias -48 | crosses 0 about 8 cells out (`HOME_DIST` 0.2), +0.8 by about 13; off with no soil |
| Drop job | h0 **-10**, h1 **+6**, the start's own `AT_NEST` kept, offset shifted by **-(-10·h0_off + 6·h1_off)**, about **-3.8** | the shift puts back the units' off level, so food-in-the-open rows match the start. Rev 1's -3 (forager offset -4 to -7) leaves them about +0.8 high |

- **No lag:** the units read this decision's context, so the decision just after a cut sees `SPOIL` = 1 at once.
- **Measured in the review's toy corridor, from generation 0** (E8): food 1.30 against the start's 1.20, soil score +0.95, 1 of 64 pellets put down at the nest, 4 food cells put down away (start 3). That is the best start of every arm tried.
- **The off levels are not flat.** With food, h0 reads -0.83 at the nest under cover (20 + 20 - 45 = -5), -0.96 at the nest in the open and -0.98 away. So food at the nest under cover gets about 1.3 less on its Drop sum than food at the nest in the open. On the forager's numbers that is about +1.4 against +2.7, so both still put down. The architecture review found the same leak in g5, about 8% of the gate's swing in the lab's main delivery state. The toy corridor may not have tested that row, so the food rows of the truth table below must come out within noise of the start's. If the under-cover row comes out low, a small Drop-job `ROOFED` term can offset it.
- **Lay the units by hand; do not leave them to evolve.** In the toy, 8 hidden units at zero left to evolve was the slowest arm (walled corridor: 3 of 6 lineages reached soil 0.8). In 79 toy runs, no weight grew from zero past about 9 (F3), and these gates need 40s.
- **What else it buys.** Any later rung that needs an "and" of this moment's senses (dig at the face and not overhead; go out when empty and the store is low) can lay its gate on units 2-7 (F1).

**The truth table must be re-derived on the real start genome before any training.** It is not the forager's, because the trained start's other job weights also move Drop. These rows must be traced on the start's fetch and dig layouts:

| Carrying | Where | Must |
|---|---|---|
| food | at the nest, open and under cover | put down at the start's rate |
| food | away | hold at the start's rate |
| soil | **the decision after the cut**, at the face | hold (R1's row) |
| soil | at the nest, or under cover | hold |
| soil | in the open, near home | rarely put down |
| soil | in the open, about 8+ cells from home | put down |

- R16 corrected rev 1's draw rates: a sum of -3.5 is drawn about 0.65% as often as a walk, not 4%. The ring is sharp at 7.6-8 cells from the last home contact, which is roughly the pit's rim. Print where put-downs land by distance.
- **Even a perfect gate leaves a floor.** The pick draws a wrong put-down at ≥0.23% of the best verb's rate (review F2). That is fine.
- **The tripwire stays:** soil put down at the nest above 5% of soil put-downs, or food put down away above the start's rate, in two straight generations' champions. Either trips the fallback.
- **The `enter_rung` trigger is Dig's switch == -1 at entry (R11), not "Dig row all zero".** At entry it:
  - zeroes the drifted Dig and `SPOIL` rows (14-15 links in Dig's job; 24 on `c::SPOIL`);
  - gives Dig a stated job start: `AT_NEST` +6, offset -4;
  - lays h0, h1 and the Drop terms, **adding** them to the start's Drop row rather than overwriting it;
  - logs each change.
- **Zero-slot alternative, not the default:** update memory before the pick. With rev 1's memory wiring that gives soil +0.94 and 1 of 64 at the nest (E6). But it moves every saved brain, since each memory then sees its context one decision earlier (toy forager food 1.20 → 1.16). It also still needs two memory rows, which is R2's problem.
- **Fallback:** a separate `Verb::Dump`. 63 more slots, padded mid-row in `M_IN` (R17). In the toy it starts at soil +0.42 and reaches 0.8 in 4-42 generations (walled), behind the hand-laid units.
- **Separately, for every later rung: make `PickUp` act for food, and keep `Eat` for eat-from-crop only** (review F6). It needs 0 slots and a one-time copy of Eat's job row to PickUp's at load. It removes the trained ant's first trap (`stage2/near/trace-trained-ant.md`). That is the brain lane's call, and the dig rung does not depend on it.

## 6. The hand-built digger (preset `digger`, on top of `forager`)

This is each step's pass arm, tuned by tracing every ant. Every weight is within 40.

| Rule | Behaviour | Status |
|---|---|---|
| D1 dig at the face | At the nest with jaws free and crop empty, cut. Never cut overhead; prefer down. **No `f::SPOIL` term** (it reads lining, R5). **Sideways at depth waits for the depth sense** (R3), so steps 1-3 dig down only. | job `AT_NEST` +6, offset -4. Unit `f::UP` -6. |
| D2 carry up and out | Climb at the nest while holding soil. The forager's unit 5 fires for a pellet, because its flag reads the crop only. | as the forager |
| D3 put down away | Section 5's h0 and h1 on the job head's hidden units. | section 5 |
| D4 go back | A latch set by putting soil down, cleared at the nest, pulling home through a second home unit. The memory lag is harmless here, since the latch only needs to hold. | builder |
| D5 down to the face | At the nest without soil, walk down. Unit 5 must also gate on `c::SPOIL`. | builder |
| D6 clear the door (step 6) | Cut the plug overhead when it is thin and the ant is shallow. **It needs depth** (R3), and maybe a "way out open" sense. | after PR 2 |

**The forage-or-dig choice (R14).**
- Rev 1's empty-trip cue is dropped. `since_nest` resets at every home contact, which is where Dig acts. A dig arena starts with `since_nest` at 0. And squash(n/200) flattens out.
- Instead, train the choice where it exists: in the kept colony with Dig switched on (from step 4), with hunger as the soft cue, made honest by the budgeted energy of steps 1-8.
- Don't train foragers out of digging (1232).

## 7. Controls and cheats

| Arm | Answers | Expect |
|---|---|---|
| rest | the floor | 0 |
| blank (Dig and Drop on) | luck | S1-S2 only, capped |
| hand digger | passable | passes |
| put-down-anywhere (h0 and h1 zeroed) | the score sees placement | well below on S3, S4 and refill |
| shaft dumper | dead end 1093 | S1-S2, plus a refill cost |
| **tailings shuffler** (cuts only placed spoil) | R7 | **0 on every stage** |
| cut-and-drop farmer | caps | hits the cap |
| blind (`home_blind`) | is home read? | It also zeroes `f::HOME`, so it can't let go or return, and "below on S3-S4" is met trivially (R17). **Compare the share of put-downs that land away instead.** If blind is at or above the champion there, the control has failed. |
| forage winner, Dig quiet | free digging | low |
| from step 10: a hand-built two-chamber mask, a giant hole, a hole split by a pillar | the per-chamber census | 2; 1 or 0; 1 |

**On every run:** worse side per mirrored pair; sealed hold-outs (70000+) scored once; two horizons; fetch and trail kept inside their share floors (`f < forget × ref`, as today), and the colony from step 4 with its floor at ref − 2 SE; pictures.

**Cheats, and what now closes each** (the review's attack table):

| Cheat | Closed by |
|---|---|
| C1 cut and drop beside the face (the memory lag) | h0 reads this decision's context, so no lag (R1-R2) |
| C2 tailings shuffling | the S1 gate (R7) |
| C3 quarrying the surface, or the face of a step | S4 per row, from the lowest surface within ±3 columns (R8); step 5's flood |
| C4 a wide scrape | never room; the pit is printed |
| C5 pellets beside the door column sliding in | refill includes the founding cut (R9); clear off; `drop` on until 6 |
| C6 holding forever | no S2 |
| C7 dumping down the shaft | S3 and refill include the founding cut (R9) |
| C8 plugging the door | D |
| C9 the energy-0.5 cue | budgeted energy (R13) |
| C10 cells recorded but not cut | the frame-0 read |
| C11 links free when fed | budgeted energy; the colony with Dig from step 4; dead-link zeroing (R11-R13) |
| C12 pellet set on a body or over a hole | ground footing (R6) |
| C13 a pillar splitting one hole into two chambers | a passage of at least 4 non-core cells (R15) |
| C14 a stale home missing honest cuts | flood at the bell (R10) |

## 8. What to print beside the score

- **Nest funnel per ant**, at its high-water mark: cut, let go, away, out, joined, roofed room. Counts, plus "of prev" and "of all".
- **Where every pellet went:** at the nest, under cover, open near home, open far (by distance in cells), on a body (R6), the post-cut decision's put-downs (R1). Refills by cause. **The same table for food**, including food at the door from step 9 (R17).
- **`MindCounts`:** `dug`, `soil_down`, `soil_down_at_nest` (R17).
- **Digging:** digs per ant, held-time share, `dig_energy`, shaft-foot queueing (step 9+).
- **Door:** D, and what fills a shut door.
- **Nest:** room, depth, `under_fill`, pit, mound; chambers per chamber (box, passage); the `walkers` percentile.
- **Brain:** live links, dead links zeroed at entry, tax J a tick, and the budget left at the bell.

## 9. Code changes, in two PRs (R's "smallest first PR")

**PR 1: steps 1-3 trainable.** Neither commit changes the game ant.

1. **Mind**, on the brain lane's branch:
   - Already built: Dig acts (`jaw_can_cut` re-checked, then `cut_cell`, made `pub(super)`, `creature.rs:21616`), with the jaw bill mirroring `creature.rs:8204-8211`. Also built: the soil branch first in `put_down` (`mind.rs:1209`, before its return at `:1216`), real ground (`spoil_site_open(..., true)` in `options.rs:362`, minds only), the `dump_log` and the counts.
   - Still to do: the job head's 8 hidden units with a padding loader (section 5, rev 2.1), `enter_rung` per section 5, and preset `digger` D1-D4 (down only, no `f::SPOIL`).
2. **Rig:**
   - a **new** arena, so today's `Dig` stays byte-identical: minds, `clone_minds`, the budgeted energy, the build split, a flat `DigBed`, and the settle with `rest` minds;
   - `SoilTrip` with the S1 gate, hole including the founding cut, caps and refill;
   - the controls in section 7.
3. **Guards, each watched red once:**
   - the build split (red when the field is set after `apply`);
   - the digger's soil put down at the nest under 5% (red with h0 removed);
   - every saved champion byte-identical in fetch and trail with the hidden units at zero (red with one unit's Drop weight set);
   - the ledger equal to `creature_stats.digs` and `spoil_dumped`;
   - the shuffler at 0 (red without the S1 gate);
   - the farmer at the cap;
   - no soil drop offered on a body (red with footing off);
   - fetch and trail champions byte-identical. That is a tripwire for the shared `put_down` edit only, since Dig is banned there per arena.

**PR 2: before step 4.** It moves trained champions, so it lands on its own.

1. **A depth sense for minds** (R3): rows below the frozen ground datum at the ant's column, the same datum S4 and `joined_room` use. It is right on bare ground too. Zero drifted `c::DEPTH` and `f::DEPTH` weights in any start (25 and 10 live links in col30-s11-seed33).
2. **The `f::SPOIL` split** (R5), slot plan agreed with the brain lane at 11:50:
   - `f::SPOIL` means placed spoil only. `f::CUT` (slot 22) becomes the hole flag of item 5.
   - Two new features go at the end of each unit's block: `LINED` (packed lining) and `ROOF_DROP` (item 5's Dig feature). FEATURES goes 43 → 45, and the genome 2,669 → 2,701. The genome length is the version: a 2,333- or 2,669-slot file is pre-PR-2. That holds only while no two layouts share a length, until the review's F9 named-block format.
   - At upgrade, `mind::pad` does three things:
     - zeroes drifted weights on every input going live (`c::DEPTH` in all five blocks, `f::DEPTH`, `f::CUT`);
     - **copies each unit's `f::SPOIL` weight into `LINED`**, so a lining cell scores exactly what it did;
     - starts `ROOF_DROP` at zero.
   - Old champions are then byte-identical in fetch, trail and scarce by construction. Measured (brain lane, 13:55): byte-identical in fetch, trail, scarce and soil, for all 3 starts. **With the copy removed, the guard goes red in the soil arena only.** Lining is only ever a Dig target, and Dig is banned in fetch and trail, so the guard lives in soil.
3. **Dead-link zeroing at entry** (R11): links on inputs constant in every arena the rung runs, on banned verbs, or into output-dead units. Log the count and the J a tick saved.
4. **The kept colony with Dig** (R12): heads only, floor at ref − 2 SE, calibrated on the unmodified start.
5. **Two hand-laid scorer vetoes, each with a target feature** (step 3's misses, traced 11:40-11:45):
   - **Drop:** `f::CUT` on the target cell (dug this run, or part of the founding cut); a veto unit for Drop targets with it set.
   - **Dig:** a feature for "cutting this drops a roof cell". Flag the target if any of its 8 neighbours is a self-supporting worked cell with nothing beneath it (or that would have nothing beneath it once the target is cut), and whose non-empty count after the cut is under 3. That is the crumb rule read one cut ahead, **including its "nothing beneath" gate** (dead-ends.md:1095: dropping that gate once took the nest with it). Add a veto unit for Dig targets with it set.
   - Both use g5's shape (verb gate plus feature, negative output). A veto through `best` is what the scorer route can do (architecture review E5).
   - **Measured 13:55: the vetoes alone moved nothing**, because no job read `best`. On every step-3 champion, Dig's and Drop's job rows weight `best` at 0. My spec's miss. Every pellet put into the hole came on the decision right after a cut in the open, with every Drop target in the hole. Falls went 0.96 → 0.78 a world, and only where there was another cut to re-pick. **Revised (brain lane's default, agreed 14:05):**
     - **Drop:** the scorer veto plus a `best` read on the Drop job, +6 with offset -6×0.98. Food moves at most ±0.2 on the sum, because Drop `best` reads 0.95-0.99 in every state. "Every target in the hole" sends `best` to about -0.95, so Drop goes off. **The veto must be gated on carrying soil**, because food is put down into the founding chamber and dug chambers, which are hole cells. For example: S_VERB Drop +35 (others -40), `f::CUT` +40, `c::SPOIL` +40, bias about -100. That gives soil on a hole target +15 (on) and food on a hole target -25 (off). Check the food-at-the-nest-under-cover row on fetch and scarce.
     - **Dig:** a context sense `c::ROOF_CUT` (every cut in reach drops a roof cell), read by the Dig job at -20 by hand and trainable. The scorer veto is kept for mixed sets. CONTEXT goes 20 → 21, and the genome 2,701 → 2,749.
     - **Rejected:** filtering roof-dropping cuts out of the options. It would be exact, but the brain could never refuse it or unlearn it.
     - **Dropped:** h0 reading the ant's depth. With food under cover in the founding chamber it puts h0 at +3 (on), which blocks that food put-down.
   - Roof-sense cost: 1-3% of a soil run and 4-6% of a scarce run (paired, 5 reps). It is read only when Dig is on.
   - Guard: in a scripted world, the rim cut fires the feature beforehand and the shoulder falls. With the beneath term removed, it must flag cells that do not fall.
   - Cost: about 64 cell reads a Dig target, on Dig options only. Log it.

**Later:** the door flood and per-chamber census (steps 6 and 10, R10 and R15); the step-9 colony arena on real energy.

## 10. Open questions

1. **Scott:** the chamber width cap, needed by step 10. My 4× has no basis.
2. **The architecture review** (`review-brain-architecture.md`, done 08:10): it moved section 5's gate to job-head hidden units (rev 2.1). Its other findings (relative pick, brain price, PickUp for food, colony-need senses) are for the brain lane and Scott, not this rung.
3. **A "way out is open" sense** for D6 and the door (`dig-from-scratch.md` concern 2).
4. **`DOOR_LOOSE` on bare ground:** it keys on a founding shaft, so step 7's doorway is tamped like any wall. Measure what plugs the door there first.
5. **The budget size per step** (R13): 1.3× the start's burn is a start; re-set it once the colony stage's smaller brains exist.
6. **Food at the door from step 9** (1221): the mind drops food in any empty neighbour, with no `FOOD_DOOR`.

## 11. Paying for brain size and hunger

**The colony stage found the brain tax is causal** (brain lane, 07:24; `stage2/colony/README.md`). The same trimmed ant on 8 hold-outs starved:

| Tax | Starved, of 12 |
|---|---|
| ×0 | 2.25 |
| ×0.25 | 6.5 |
| ×1 | 11.25 |

- **A third of the start's links are dead.** Cutting them leaves fetch and trail byte-identical.
- **Pruned to 450 links**, the ant keeps fetch and loses 4 of 12 instead of 11.
- **Round 3 trains for a small brain** (wire 2e-3, grow 0.2), so the dig start will be well under 1,300 links.

**How the dig rung pays:**
1. **Steps 1-8: budgeted energy** (R13), not `never_starve`. A brain or a dig habit too costly for the budget ends the run early and loses its last pellets.
   - The arithmetic, from R13: 4.44e-4 J per link per tick; idle 0.10 J a tick; a cut 1.5 J. With 1,325 links, 200 J lasts 210-290 ticks. A carry-out is about 20-40 decisions, so 5-15 trips fit in a life.
2. **From step 4: the kept colony with Dig switched on** (R12). Digging when it should forage costs founders, and that is where the forage-or-dig choice gets trained.
3. **At entry: dead links zeroed** (R11). That alone is 0.18-0.20 J a tick in the old starts, twice an idle ant.
4. **Steps 9-12** run on real energy with food.
5. **The blinded oddity** (a scent-cut colony brought home more) applies to section 7's blind arm. A blind arm at or above the champion on the share landing away is a failed control.
