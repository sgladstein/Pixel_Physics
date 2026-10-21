# Review: teacher design 2026-10-21, through biology and the game

Reviewer lens: is the biology right and correctly applied; does the soil planner repeat the project's recorded failures; will the result look and feel right in this engine. Doc reviewed: `Reports/brain-training/teacher-training/teacher-design-2026-10-21.md` (cited as "the doc", sections as §). Sources were checked in code at this branch and against papers where I could reach them.

**Verification tags**: [FT] read in full text (PubMed Central); [ABS] read in the abstract; [GK] my general knowledge, not checked in this session; [CODE] read in this repo.

**Verdict in one paragraph.** Most of the biology is right, and the instinct is right too: copy the local rules real ants use rather than an arbitrary "ideal nest". But the soil planner is built on a grain physics this engine does not have. Placed spoil never slides here, so the 45° geometry, the trial drop and the Dorymyrmex "rolls down the outer slope" analogy all fail. And a full ring round the door is, in a 2D slice, a wall between the nest and the food. That wall is what the record says made ants dig through the mound in the first place. Both problems are fixable cheaply: use a one-sided crescent away from the food, and value put-downs by geometry instead of by a trial drop. Section 4.4 also misses four dead-end entries that map one-to-one onto mechanisms this doc proposes. On blueprints, the realistic and the learnable choice is the same one: the Aphaenogaster staircase, sized to the colony, with chambers kept at about 5-6 cells so they read as rooms rather than cracks.

---

## BLOCKER

### B1. The mound geometry assumes spoil slides at 45°. In this engine placed spoil does not slide at all.

**Claim** (§4.2): "a cell automaton with a 1:1 diagonal slide rests at about 45° … r_c ≥ half the mouth's width + 2 + h". §4.3: "the planner puts the pellet down in a *copy* of the world and runs the grain physics for the settle … A drop that slides inside the scoring zone or into the hole is worth nothing". The render does the same (`render_blueprints.py` docstring: "a 45-degree slope (a 1:1 diagonal slide)").

**Why it is wrong.** `spoil` is `self_supporting: true` and `needs_footing: true` (`assets/materials/spoil.ron:80-83`) [CODE]. In `update_powder`, a `self_supporting` cell takes the footing/crumb branch and then returns before any movement rule (`src/sim/update.rs:1113-1280`, `return wet_changed`) [CODE]. A pellet with ground under it stands forever, at any slope, including a vertical face. It changes only when something removes what it stands on. Then it slumps into loose `soil` (`friction_angle: 33.0`, `assets/materials/soil.ron:10`), and that soil does run. The spoil file says it outright: "A heap of this still stands as a heap; only the parts with nothing under them come down." Four consequences follow.

1. **The trial drop computes a near-constant.** The mind's put-down already requires real ground under the pellet (`dig-rung-design.md` §1, R6). So "does it stay where I put it" is answered "yes" before the trial runs. The ~1 CPU-minute per world in §4.3 buys nothing.
2. **The Dorymyrmex mechanism has no counterpart here.** Real loads that miss roll *outward* and self-correct the pile ([ABS], Tofilski & Ratnieks 2005, verified: 73.0-81.9% stay, the rest roll down the outer slope). Here nothing rolls, so a wrongly placed pellet stays wrong.
3. **An imperfect copy builds walls and towers, not slopes.** Physics will not round off what the brain gets slightly wrong. The record has this twice: the spoil lattice (`Reports/dead-ends.md:1101`, "does not stop a colony stacking spoil on its own spoil into a lattice") and the food tower (`:1199`, "`crumbs` do not slide, so every delivery can raise home a cell"). The slope has to be *taught*. The engine will not give it to you.
4. **Spill-back has a different source in this engine.** Soil gets from the mound into the nest by undermining. A cut into or under the mound unfoots the spoil above it. That spoil slumps into loose soil, and loose soil runs out at 33°, not 45°: a runout of about 1.54 h, not h.

**Change.**
- Drop the trial *drop*. Value a put-down by geometry: outward of the crest, on the far side from the hole, distance from the inner toe.
- Spend the planner's privileged computation on a trial *cut* instead: does cutting this mound or rim cell unfoot spoil whose slump reaches the hole? That is the delayed hazard no ant can see. `ROOF_DROP` sees only the first cell one cut ahead, not the cascade.
- Use 33° for the safety margin: r_c ≥ half-mouth + 2 + 1.54 h.
- Put one fork to the owner and record its cost: should a freshly placed surface pellet settle a cell or two before it packs? It would make Dorymyrmex-style rolling real, and it is visible feedback (S2 below). The record against loose pellets near galleries (`dead-ends.md:1101`, roofed void 2/1 against 54/58) is the cost to weigh.

### B2. A full ring in a 2D slice is a wall across the only way to the food. That is the failure the owner described.

**Claim** (§4.2): "both sides of the mouth evenly (or one side, if the owner picks the crescent)". §4.4: "In the solo dig arena there is no food to reach, so the planner labels mound cuts 'no'."

**Why it is wrong.** In 3D, foragers walk round or over a thin disc. In a vertical slice, every forager must cross both ring flanks or cut through them. The record shows the ants did exactly that, and that forbidding it killed colonies:
- `dead-ends.md:1233` (`MOUND_DIG`): "the mound between the door and the food heap is hollow and full of ants; on, it is solid … ants shut in it starve". Colony mean fell from 268-275 to 134-198.
- `:1234` (`FORAGER_NODIG`): "foragers' cuts are the passages through the mound's food side". Food home halved.
- `:1230`: "spoil heaped as a mound (`PIXEL_PHYSICS_SPOIL_RING=1,2`)" was the *worst* door-use arm measured: 30 of 72 samples against 54 shipped. The doc does not cite it.

So "ants dug into the mound" was not a bug in the ants. It was the cheapest route through a wall the colony had built across its own road.

Two more problems:
- **The ring buries rung 9's food.** Rung 9 drips food 10-30 columns from the door (`dig-rung-design.md` §3 table). The doc's own mound reaches 34-86 cells from the mouth (§5.1).
- **The solo "no" label is the rejected rule in another form.** A solo lesson "never cut placed spoil" goes into the copy loss. The doc's own C5 says the ant cannot tell a passage from a scrape locally, so that lesson will spread into the colony, where `MOUND_DIG` measured it starving ants. The re-test condition the record names is "a rule that can tell a passage from a scrape … *or the colony has another way to its food*". A teacher that can tell them apart does not help if the student cannot.

**Change.**
- **Default to a one-sided crescent on the side away from food and trail.** It is real biology: Trachymyrmex septentrionalis builds a one-sided crescent arc of about 75°, rebuilt at the same orientation and turned downslope [ABS]; Pheidole oxyops piles a crescent about 13 cm out [ABS, verified]. In 2D it leaves the food side clear, so it meets `MOUND_DIG`'s second re-test condition by construction.
- Show the owner ring against crescent as a paired render with the forager route drawn on. "A ring is ideal" was said without the 2D traffic cost in view.
- Take mound cuts out of the copy loss in solo arenas. Abstain, so the lesson is the brain's own odds. Do not label them "no".

---

## MAJOR

### M1. §4.4 reads three records right and misses five that bear directly on this plan.

**Claim** (§4.4): the three-row table.

**What it gets right.** The `SPOIL_RING`/`SPOIL_CREST`/`DOOR_COLLAR` entry (`dead-ends.md:1222`) does name the crater's loose rim as the largest source of fill. `MOUND_DIG`'s passage finding is quoted correctly.

**What it misses:**
- **`:1222` itself**: "A ring at mean 21 columns out lost 2 of 4 colonies". Carrying further out has a price. The planner pushes the crest outward as h grows (§4.2), so that is the direction it pays. The same entry says the collar failed "because … the heap reaches the mouth from above and through it". The heap *did* reach the mouth. Its re-test condition is "the crust holds … and never on four seeds to rank fill".
- **`:1230`**: the mound arm was worst on door use (B2).
- **`:1210`** (`SurfaceCurvature` on `DropSpoil`): "holding a pellet in a pit and dropping it on a rise" moved nothing (30% → 26% below surface, inside the noise). This is the slope-on-put-down idea §3 and §4.5 propose as a new sense, already tried on the old creature. Its re-test condition is to census the curvature reading *at the drop* first.
- **`:1106`** (Khuong deposition attracted to pellets): declined, then re-censused. Once spoil stopped packing, "the marker went from too thin to read to everywhere, and a preference between cells that all carry it is still a preference on one decision in a hundred". This is the `BUILDING` scent §3 proposes.
- **`:1286`** (`FLAT_ROOM`, a room-height cap): separate brood chambers on 4 of 12 runs, then "a halved colony on the resting base". This is the "widen sideways, not up" lesson of §5.4.
- **`:1927-1930`** (`Crowding → Dig`, Toffin's room mechanism): "fires … and does not produce the outcome it was built for". More compact on 16 of 33 seed pairs, a coin flip, and buds never moved. This is §3 way-out 3's "bud when crowding at the wall falls" and §6.2's budding-by-crowding.

**Change.** Add these rows. Each one turns a "new idea" in the doc into "tried, and here is the re-test condition". The re-test conditions are cheap pre-checks, which is a gain for the doc.

### M2. The "2D lab nests" are horizontal sand discs, not vertical sections.

**Claim** (§5.1): "From the notes (Tschinkel's casts …; the 2D lab nests of Toffin, Buhl, Gautrais)". The notes (§2) call them "the closest match to the game". The `round_room_budding` blueprint is drawn from them.

**Why it is wrong.** Gautrais et al. 2014 [FT, read this session]: ants "dig into horizontal moistened and homogeneous disks made of sand". The authors also warn that "3D extensions … should also pay attention to the special role played by gravity". The Toffin 2009 abstract [ABS] says only "two-dimensional"; the same group used the same setup. In a horizontal disc there is no floor, no roof, no spoil to lift, and no up. The round cavity that buds into galleries is a plan view. Drawn in vertical section it becomes a starburst under the door (sheet panel 3), and no field cast looks like that.

**Change.**
- Relabel Toffin/Buhl/Gautrais as "plan-view lab nests". Use them for the colony-level laws (excavated area roughly proportional to workers; meshedness rising with workers), not for section shapes.
- The closest real analogue of this game is the glass-sided vertical enclosure (Halfen & Hasiotis 2010, P. occidentalis [ABS]), plus the shallow Aphaenogaster casts.
- Demote `round_room_budding` as a section blueprint.

### M3. The blueprints are nests of colonies 10-1,000 times larger than the game's.

**Claim** (§5.2): blueprints "fitted to the box … depths are relative". §5.1: "the shape does not change as the nest grows".

**Why it is weak.** Tschinkel's size-free shape holds *within* P. badius, over 1,700 to 8,000 workers [FT]. Across colony sizes the architecture changes kind. Gautrais 2014 Table 1 [FT] compiles field nests:
- 9-300 workers: a single vertical tunnel;
- 368-1,000: stacked chambers on one tunnel;
- meshed networks: thousands of workers and up.

P. badius adds its second shaft series at about 3,200 workers [FT]. The game's colonies are 12-24 ants. The notes' own table puts the blueprints at room for 17-39 ants at legible scale and 9-24 at true scale, and a meshed "garden-ant farm" for 24 ants is not a nest any real colony that size digs.

**Change.** Pick by colony size, and make growth graded:
- 12 ants: shaft plus 2-3 chambers.
- 24 ants: 3-4 chambers.
- Later: a second shaft series. That is the "middle" law 1 asks for, and it is the A. floridana rule [FT]: +1 chamber per 100 workers, scaled.
- This also halves the mound problem (M5). About 300 cells for 12 ants, against 438-991 drawn.

### M4. True-scale "flat lens" chambers will look like cracks, and the progress census cannot see them.

**Claim** (headline, §5.1): "real chambers are flat lenses … 2-4 cells tall in this game". §5.2: progress by `nestcensus::rooms_of` ("7 of its 9 cells open"), "`World::in_room`, `in_chamber`".

**Why it is wrong.**
- **The census.** `rooms_of` marks a cell as room only if 7 of its 3×3 are open (`src/lab/nestcensus.rs:64`, `:195-213`) [CODE].
  - A 2-cell-tall chamber has **zero** room cells: every 3×3 includes a wall row, so at most 6 are open.
  - A 3-tall chamber has one row.
  - A 4-cell-wide passage, horizontal or vertical, scores exactly like a 4-tall chamber.
  - Step 8 pays 32 per room cell, so a true-scale chamber earns nothing and a fat passage earns as a room.
- **The helpers.** `in_room` and `in_chamber` are methods on `SideRoom`/`ShaftFootprint`, the founding cut's rectangles (`src/sim/world.rs:1438`, `:1478`) [CODE]. They are not on `World` and they are not a census of dug rooms, so they cannot measure a blueprint.
- **The look.** At true scale, chamber height (1-1.7 BL) is about the passage width (1-1.3 BL real; 2 BL by the owner's "passage ~4"). In section, a flat chamber and a horizontal tunnel differ only in length and taper. The sheet already shows it: harvester and carpenter chambers read as twigs. The specs file says it too ("5-6 cells tall and 3x+ as wide reads as a real chamber", `nest-blueprint-specs.json` → `legibility.warning`). The doc's 2-4 contradicts its own source.

**Change.**
- Recommend chambers at 5-6 cells tall and at least 3× as wide (15-25 cells), with passages visibly narrower than chambers.
- Make a chamber read by what is in it, not by its height: a flat floor with food or brood on it. That is also what the real nest's purposes are (§6.3).
- Replace `in_room`/`in_chamber` in §5.2 with a per-chamber census that can see a 5-tall lens. `dig-rung-design.md` §3 already specifies one ("each core component as a chamber"). Positive-control it on a hand-cut 5×20 lens before trusting it.
- Correct the headline: the ladder's 8-16 cells (4-8 BL) against real 1-1.7 BL is "about 2.5-8×", not "three to five times". The better point is not height anyway: the ladder chamber is a box, and a real chamber is a lens.

### M5. The mound "conservation problem" is mostly not a realism problem. The doc's numbers for it are a parameter.

**Claim** (§5.1): "every blueprint's mound comes out a flat-topped ring about 8 cells tall reaching 34-86 cells from the mouth, wider than any real disc in body lengths."

**Why it is wrong.**
1. **"About 8 cells tall" is the renderer's cap, not a result.** `render_blueprints.py` docstring, and lines 147-149: `if h <= 8 … break; ro += 2`. The suspiciously tidy 7.6-7.9 across five very different nests is the tell.
2. **"Wider than any real disc" is false by the notes' own numbers.** P. badius discs are 80-5,700 cm² [FT], so 10-85 cm across, 12-106 BL, a radius of 12-106 cells. The mean is 55 BL across, about 55 cells of radius. A mound reaching 34-86 cells from the mouth is inside the real range.

What is unrealistic is the *thickness*: 8 cells against the real ~5 cells (2.5 BL [DER]). What actually costs is traffic (B2) and carry distance (M1).

**What real ants and the game can do, in order of fit:**
- **Size the nest to the colony (M3).** That halves the spoil.
- **Backfill.** The glass-sided P. occidentalis enclosure, the one real *2D* nest, put about 51% of its spoil back as backfill or lining (Halfen & Hasiotis, IUSS 2010 [ABS]; a 1,000-ant enclosure, lining included). The field figure is 2.5% (Tschinkel, Rink & Kwapich 2015 [ABS]). The 2D constraint drove real ants to the same response, so backfill is the realistic answer *specifically here*. Acromyrmex over-dig and refill too (Römer & Roces 2015 [ABS, verified]). A planner can mark dead ends and over-dug chamber margins as legal put-down sites.
- **Weathering.** `spoil.ron` already names it ("Not a decay path, yet … §3b wants dumped spoil to weather back to soil"). It turns the mound into a thin, spreading disc over a session, which is Tschinkel's biomantling disc. Graded, and visible. The cost is that loose soil runs at 33°, so it needs the door kept clear (S4).

**Change.** Correct both statements. State the budget as nest size (by colony) plus backfill share plus the remainder on a crescent.

### M6. The proposed senses: keep two, rework two, and test the probe on the brain's own form.

**Claims** (§3, way-out 4; §4.5).

- **Slope under a put-down cell.**
  - The drop option already carries `UP` (target above or below the head), `DROP_BELOW`, `HOME`, `CUT` (`options.rs:166-199`). "Put down ahead, below the head, away from home" is already the outer flank. So §4.5's prediction may well hold without a new sense.
  - **But the probe must be fitted in the brain's functional form**: each option's own features plus the shared context. Fitting on all options jointly would let it read the step options' `UP` while scoring a drop. A joint fit can pass on information the per-option scorer cannot combine. That is one candidate reason for step 1's "passed the probe, did not copy".
  - If a sense is needed, the best-grounded one is **body pitch** (head row minus tail row on the 2-cell body) and its change since the last decision. That is graviception, and the Pheidole experiment *was* a tilt test: 15° of tilt moved the drop point from 0.53 to 0.46 or 0.63 of the pile width [ABS, verified]. "Height difference across the put-down cell" is less natural for an ant.
  - Run `dead-ends.md:1210`'s pre-check first: census the reading at drops.
- **`BUILDING` scent.** This is Khuong's pheromone [ABS, verified]. That paper's own model says "the lifetime of the pheromone is a highly influential parameter". A permanent mark carries nothing `f::SPOIL` does not already carry, and `:1106` measured that difference on about 1% of decisions. **Only a decaying "fresh pellet" mark adds information**: where the colony is building *now*, which is the active crest. Specify the decay, or drop the sense.
- **`LAST_FOOD`.** Realistic: desert ants hold a food-site vector (Wehner [GK]), and Lasius niger route memory can override trails (Grüter et al. 2011 [GK]). It is a memory, though, not a local sense. And `HOME` is computed from an exact anchor (`options.rs:629-637`), so a `LAST_FOOD` built the same way is error-free path integration. Acceptable, but say so.
- **Crowding for the colony tier** (§6). `CROWD` is animals touching the head, an instantaneous 3×3 count (`options.rs:609-624`). At 12 ants it is usually 0. `:1927` measured that crowding-driven digging "fires" and builds no rooms. A better-grounded colony signal is **encounter rate over time**, a memory that integrates contacts: Gordon's harvester ants allocate tasks by contact rate, and Temnothorax senses a quorum by encounter rate (Pratt 2005) [GK for both]. Probe it before proposing it.

### M7. Khuong's body-size template is about *building* roofs, not *digging* chambers.

**Claim** (§3): "stop the roof at about a body height (Khuong et al. 2016)". §5.4: "the body-size template real ants use".

**Why it is weak.** Khuong 2016 [ABS, verified]: ants "use their body size as a cue to control the height at which they start to build a roof *from existing pillars*". That is deposition. That excavated chamber height is invariant is Tschinkel's observation ("about 1 cm, and does not change with area", P. badius [ABS, verified]), but no paper here gives a digging mechanism for it.

**Change.**
- Cite Tschinkel for the shape and Khuong as an analogy.
- Note that a hand-taught height ceiling is `FLAT_ROOM` (M1) and cost the colony. Gate the lesson on colony outcome, not chamber shape.

---

## MINOR

1. **§5.1 vs the bed.** The blueprints were drawn on 64 rows of soil and reach 41-51 rows deep (measured off the sheet). Steps 10-11 run on "soil 40-52" (`dig-rung-design.md` §3). Refit them to the arena that will run them, or most will hit its floor.
2. **§3 table, "SPOIL".** `c::SPOIL` is "carrying a pellet" (`mind.rs:77`); placed spoil is the option feature `f::SPOIL` (`options.rs:144-148`). Name the feature to avoid a probe on the wrong input.
3. **§5.1, "about half the spoil" (Halfen & Hasiotis).** It is a conference abstract, a 1,000-ant enclosure, and it includes lining. Say so beside the field 2.5%.
4. **§4.2, Pheidole.** The paper tests the crest *hypothesis*. The measured effect is a shift of about 0.07-0.1 of the pile width for a 15° tilt, around a mid-pile mean [ABS]. Real drops are a broad distribution around the crest, not a line (S1).
5. **§4.4, "the brain already senses it (`ROOF_DROP`)".** It reads one cut ahead through `worked_cell_gives_way` (`update.rs:827`). It does not see the cascade when an unfooted spoil cell slumps and unfoots the one above it (B1).
6. **Stage 0 of the gauntlet.** The owner is judging a PIL drawing (flat colours, a hand-sized mound). Render the picked blueprint *cut into a real bed in the engine* (lining, ants walking in it, food on a floor), with the chamber census count in the card's `meta`. That is the look the owner will actually get.
7. **Sheet panel 3** is captioned "one fat room" but shows a small blob with radiating arms. The caption oversells it.

---

## SUGGESTIONS: making it graded and satisfying

- **S1. Target a distribution, not a profile.** Grade a put-down's value by distance outward from the crest, with a broad peak just past it, as the Pheidole data show. Give each ant a preferred side from its `SELF` number. Cataglyphis diggers each keep a constant carry direction ([ABS], Wehner 1970). The result is a lumpy, asymmetric, real-looking heap instead of the trapezoid in the render, which reads as engineered.
- **S2. Make the drop visible.** If the owner takes the settle fork (B1), a fresh pellet that tumbles a cell down the outer flank is exactly the "no visible consequence, not finished" fix. If not, a one-frame dust puff or tint on placement does the same job without physics.
- **S3. Put event counts in every card's `meta`**: pellets placed on the outer flank, the inner flank and the food side; mound cells slumped (and how many reached the hole); door plugs cleared; chambers by census. Use paired seeds, ring against crescent.
- **S4. Door-keeping as a verb.** Dorymyrmex rebuild a removed side of the ring [ABS, verified]; harvesters keep the disc clear [GK]. A planner can label "carry the loose soil off the mouth" for an empty ant at the door. It is solo-teachable and graded (the mouth narrows, then is cleared), and it answers the H0 finding that loose soil at the rim shuts the door within about 500 frames (`dig-rung-design.md` §3).
- **S5. Grow the nest in stages.** Tschinkel's deep chambers begin as a circular niche off the shaft and become lobed as they are enlarged [ABS, verified]. A planner whose frontier is "niche, then lens, then a lobe, then the next chamber down" gives a watchable middle at every moment.
- **S6. Tell the owner that tunnels in the mound are real biology.** P. badius superficial chambers "often lie within the soil excavated … and dumped around the nest entrance" [FT, per the notes], and the fire-ant mound is "a dense matrix of narrow tunnels". Digging into the mound was never the failure. The failure was spill-back and the lost road. `spoil.ron`'s `packs_into` already lines a tunnel driven through tailings.

---

## Things to keep

- **"Change the goal to one local rules generate"** (way-out 3). It is the right answer to the imitation gap, and the Aphaenogaster edge-down/centre-in rule [FT] is the cleanest instance in the literature.
- **The honest prediction in §4.4** that the soil planner fixes where pellets go and not spill-back. Keep it, and add the mound-undermining trial cut (B1) as the part that addresses spill.
- **"Never forbid mound cuts by position"** and the passage/scrape distinction, read correctly from `MOUND_DIG`.
- **Empty ants get no label from the soil planner.** It removes the walk-home spread that broke fetch.
- **Teach outcomes, not actions, for size and stopping** (§6.2), and nothing that reads crowding taught solo.
- **The notes file's verification tags.** They made this review fast. The spot-checks held: P. badius chamber height invariant at ~1 cm and top-heavy; Dorymyrmex 73-82% stay and almost never roll back; Pheidole's tilt shift; Toffin's density effect; Khuong's template and pheromone. The errors are in how the papers were applied, not in what they say.
- **Drawing the mound budget instead of describing it.** It surfaced a real constraint, even though the cap hid its size.

---

## Sources checked this session

- Tschinkel 2004, *J Insect Sci* 4:21 [ABS] — https://doi.org/10.1093/jis/4.1.21
- Toffin et al. 2009, *PNAS* 106:18616 [ABS] — https://doi.org/10.1073/pnas.0902685106
- Gautrais et al. 2014, *PLoS ONE* 9:e109436 [FT] — https://doi.org/10.1371/journal.pone.0109436
- Khuong et al. 2016, *PNAS* 113:1303 [ABS] — https://doi.org/10.1073/pnas.1509829113
- Tofilski & Ratnieks 2005, *J Insect Behav* [ABS] — https://doi.org/10.1007/s10905-005-5608-8
- Tofilski & Ratnieks 2008, *Insectes Soc* [ABS] — https://doi.org/10.1007/s00040-007-0967-0
- Römer & Roces 2015, *Insectes Soc* [ABS] — https://doi.org/10.1007/s00040-015-0419-1

The first four are from PubMed/PMC; the last three came through SciSpace.
