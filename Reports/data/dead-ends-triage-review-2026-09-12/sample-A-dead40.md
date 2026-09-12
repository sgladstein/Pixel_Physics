# 40 entries from Reports/dead-ends.md (labels withheld)

## E01  (section: plants, register line 769)

- **Reports/tree-rewrite-design.md §0/§1 (deletion target and what replaces it)** - The continuous-position space-colonization tree implementation (Tip/RootTip/TreeState with private float-position state, tree_tip_tick/root_tip_tick) is superseded by the per-cell Grow behaviour dispatched from real grid cells on the M16 schedule. Nothing may reintroduce private float-position organism state.
  *Re-test when:* Settled repo direction (design-philosophy commits to cell-typed organism modelling over TreeState-style structs).

## E02  (section: other, register line 1501)

- **PLAN-log.md 'Overnight run, section 8', organism.rs bullet (Behavior enum)** - Encoding Behavior variants as newtypes wrapping separate structs was rejected on first attempt: RON's syntax for it requires an awkward doubled Divide(Divide(...)), caught by a failing embedded-species-parse test. Struct-shaped enum variants are used instead.
  *Re-test when:* Tied to RON's serialization syntax.

## E03  (section: weather, register line 1222)

- **src/sim/weather.rs fn gust (doc: 'The mistake this is shaped around')** - A steady global wind term was built, measured, and reverted: a uniform velocity in a bounded world pushes air into the walls, creating divergence, pressure, and more velocity, so field::is_converged never returns true again — settled-field cost went from 0.0002 ms to a permanent 3.55 ms on every scene and six field tests failed. Recorded as do-not-retry; gusts (bounded impulses that disperse) are the replacement, guarded by the gust-settling test.
  *Re-test when:* Holds while the world is closed and field sleeping is convergence-based; a gust fired every frame is the same term wearing a different hat.

## E04  (section: plants, register line 869)

- **src/sim/plant.rs fn thicken side-choice comment** - A flat [(-1,0),(1,0)] side order with a break made every thickening cell choose left when both sides were open, so trunks fattened entirely to one side (owner-reported as 'kind of weird'). Same class as commit 68371d7's liquid body shedding bug: a two-option loop with a break is a directional bias unless something breaks the tie. Coin flip from the organism's own RNG stream now.
  *Re-test when:* Unconditional — named as a bug class; check any new two-option loop with a break.

## E05  (section: field, register line 1119)

- **Reports/creature-direction.md §13d Corrections (decay/interval)** - DECAY_RHO in the literature band with PHEROMONE_INTERVAL=4 kills trails before ants can use them: the forced strict-decrease LUT caps unreinforced trail life at 255 passes ≈ ~1,000 frames against a ~2,200-frame round trip. Retuned to interval 12, rho 0.03 — rho wants to sit below the literature band, and the interval is the load-bearing knob.
  *Re-test when:* Tied to the u8 LUT's forced-decrease floor and current round-trip distances.

## E06  (section: powders, register line 203)

- **src/sim/update.rs fn roll_along_slope doc** - The old one-angle repose model was superseded: a single angle implies settled piles relevel to that one slope, where real avalanches have hysteresis (harder to start than to keep going). Replaced by two-angle repose — lenient roll reach while flowing, stricter stability reach at rest.

## E07  (section: rendering, register line 1292)

- **src/render.rs const UNDERGROUND / sky-to-dark ramp doc** - A one-row transition from open daylight to full underground was shipped and read as 'way too intense': a room went from sky to near-black across a single cell the instant it was enclosed, and every cave mouth was a flat black cutout. Replaced by a 24-cell ramp (three field blocks), set by eye — shallower reads as a black slot, deeper stops caves reading dark.
  *Re-test when:* The ramp must stay a pure function of position so a renderer with history and a fresh one agree (the dirty-rect pixel-identity requirement that sank the stateful skyline).

## E08  (section: creatures, register line 924)

- **`src/sim/creature.rs` `is_partable` / `organism::Parted` on woody tissue -- letting a body *occupy* a trunk the way it occupies foliage; measured 2026-09-06** - **It works for movement and it kills plants, and the reproduction is a test that is green on `main`.** Reusing the parting machinery for `wood` takes tissue blocks on `scene=colony` seed 1 from **776 to 17** (blocked steps 18.5% -> 5.0%, moves 7,471 -> 11,422) -- and finishes with the lab bed empty: `lab::tests::copies_carry_what_was_planted_and_still_diverge` ends at `plant_cells 0` in **all three copies** with it on and passes with it off. The mechanism is that ownership is resolved through the **grid**: `plant::is_structural_anchor` opens `if cell.organism_id() != organism_id { return false }`, so a parted cell holds the animal, stops counting as an anchor, and a seedling whose single base stem an ant is standing in becomes an unanchored plant. Foliage never showed it because a leaf is never an anchor. **The repair is not that one line** -- several per-organism passes resolve a plant's own cells through the grid and each would need to answer *"this is still mine, an animal is merely standing in it"*, which wants the parted cell reachable from the plant rather than only from the animal holding it. Superseded by `organism::Crossing`, which never occupies the cell and so has none of this. *Re-test when:* never as a route to walking through wood -- crossing is strictly better and cheaper. The entry survives because the **grid-resolved-ownership** trap is general: any future mechanism that puts a non-plant cell where a plant's own cell was inherits it, and the first symptom is plants dying rather than anything about the mechanism.
  *Kept from the same attempt, because it was measured and is on by default:* a parted cell stays in its plant's `cells` list so the connectivity graph is not cut while a body stands in it -- without it seed 1 severed **7,845 cells against 1,773** and snapped **28 against 10**; with it, snapped is **2**. `PART_KEEP_GRAPH=0` is the control.

## E09  (section: destruction, register line 468)

- **Reports/explosion-mechanics-diagnosis.md §2/§3 (round 1) + round 3 "Result"** - Converting every cleared cell to debris in a single frame was measured and rejected: 2,513 particles for a radius-20 blast, 86% landed after one frame, refilling their own crater (43% on the first frame, fully gone by f+30); the naive full-debris prototype peaked at 7,213 in flight. Staged spawning across the blast duration plus `Tuning::debris_fraction` 0.4 gives an 810 peak, and fewer, longer-lived, faster debris is both better-looking and cheaper.
  *Re-test when:* Unconditional for look and cost; particle counts during a blast are the number to watch if debris volumes ever grow.

## E10  (section: destruction, register line 448)

- **README.md 'M15 status' — 'Rebuilt after a diagnosis pass' paragraph (see Reports/explosion-mechanics-diagnosis.md)** - The original M15 explosion: resolved in a single frame, reused the debug force-ignite tool for the fireball (setting stone alight regardless of flammability), and had no debris pierce — so a buried charge threw nothing at all. Rebuilt to expand over Tuning::duration frames, write real CA temperature for the fireball, and give debris Particle::pierce.
  *Re-test when:* Measured in the diagnosis report; the rebuilt shape is the bar any replacement must meet.

## E11  (section: structural, register line 349)

- **src/sim/load.rs fn bearing_moment body, contact-width comment** - Sizing the granular footing from only the cells whose own underside touches grain was tried first and is wrong twice over: a pile is lumpy, so a slab on its own rubble read as several knife-edge footings and was dismantled one cell at a time — 48,109 cells lost to one dig on scene=worldcrack preset=flat versus 894 for the bug it was meant to fix — and it judges an instantaneous patchy contact the conforming grain erases within frames. The footing is the piece's run of rock with anything beneath it.

## E12  (section: field, register line 1113)

- **README.md 'The coarse field grid' — 'LIGHT_DECAY retuned from 0.85 to 0.997' paragraph** - LIGHT_DECAY at 0.85: the steep decay effectively required every plant to sit within a couple of field rows (~20 world cells) of open sky once Germinate made light-gated growth real; retuned to 0.997 (~75 cells of useful depth) on owner request for real outdoor sunlight depth.
  *Re-test when:* Owner requirement of realistic light depth. Recorded cost of the new value: convergence to a static sky amplitude takes roughly 100x longer, so the field spends more frames awake near day/night peaks — re-open only if that cost becomes a problem.

## E13  (section: destruction, register line 411)

- **`assets/materials/log.ron` `colors` — a wide palette spread on the *piece* tier** - `log` shipped its first draft with a 44-unit spread across its four colours, copied from the convention every other debris material uses (`deadwood` (64,43,26)-(96,66,40), `litter` (132,96,44)-(182,146,78)). Wrong, and the reason generalises: a wide spread makes a field of cells draw randomly across a range, which is *speckle* — exactly right for grains and exactly what destroys the one thing a piece tier has to show, the **shape** of the fragment. The settled `scene=fell` pile read as one undifferentiated mound with 572 cells of log invisible inside it. Narrowed to 18 units and shifted to a grey timber, off `litter`'s gold and `deadwood`'s dark brown.
  *Re-test when:* Never as stated for a piece tier; the entry is here because the pull toward "match the other debris palettes" is strong and the convention is right for everything except this. It reverses cleanly for anything that *is* granular.

## E14  (section: liquids, register line 77)

- **PLAN-log.md 'Overnight run, section 4', first 'real bugs found' bullet** - Transferring the whole fill difference horizontally (to 'reach equality faster') overshoots past equality — 500/300 becomes 300/500 — and the alternating scan direction flips it back every frame forever (active_chunk_count still nonzero at 24,000 frames). Transfer half the difference.

## E15  (section: field, register line 1123)

- **Reports/creature-direction.md §5b The pass (build_decay_lut) / §10 P-13** - Plain multiplicative decay on a u8 plane fix-points above zero (rounding), leaving permanent ghost trails — the same quantization failure the canopy-density 4-bit packing already demonstrated in this codebase. Decay must go through a LUT with a forced strict-decrease step, asserted at startup.
  *Re-test when:* Holds for any quantized (u8/4-bit) decaying channel.

## E16  (section: plants, register line 919)

- **`examples/selection_arena.rs` `moisture=` / `assets/species/*.ron` `Germinate.soil_water_threshold`; measured 2026-08-30** - **Drying the bed to bring root architecture under selection was predicted in advance, built and refuted.** `plant-selection-teeth-2026-08-29.md` §4b reasoned that `norootbranch`'s null was down to water not being scarce, and that a bed at the wilting point would fix it. Measured over three 18-seed mirrored runs: **50.8%, 8 of 18, p=0.49** in a bed with 5.5x less plant-available water and **50.8%, 8 of 18, p=0.93** in one where water genuinely limits (status 0.678), against 49.7%, 10 of 18, p=0.86 in the wet bed -- paired over shared seeds the beds differ by **+0.0 points**. Not an inert world: `nobranch` loses 13 points on 12 of 12 seeds and 10 points on 18 of 18 in those same dry beds. **A bed cannot be dried into drought at all**, and the reason is a pair of constants nobody set against each other: `Germinate`'s `soil_water_threshold` floors a usable bed at moisture 246 for `herb` and 290 for `tree`, while availability has to fall to 191 and 234 respectively before either species' uptake becomes limiting. Below the floor you get an empty bed rather than a scarce one -- `tree` at moisture 260 goes 6 organisms to zero. The full account, including the lever that *does* produce drought (rooting volume: `soil=4` takes water status 1.000 -> 0.678 at the same moisture), is `Reports/plant-water-scarcity-2026-08-30.md`.
  *Re-test when:* The rejection is of *soil moisture as the lever*, not of water as a selective pressure. It reopens the moment either constant moves -- lower `soil_water_threshold`, or raise the uptake price (`Absorb.rate`, `SOIL_UPTAKE_PER_TICK`) -- since those are what close the band between "a seed will start" and "water limits growth". It does **not** reopen by running the same bed longer or on more seeds. And note the deeper reason the arm was flat even in the real drought: uptake is `rate x available` per *wet neighbour*, so what earns is contact with wet soil, and in a drawn-down bed the bed sets that -- the handicap removed 23% of root cells and **3%** of uptake surface. Any change that makes contact scale with branching (a per-cell depletion zone) invalidates that half.

## E17  (section: plants, register line 863)

- **src/sim/plant.rs fn organism_tick, turgor gate comment (turgor_taper)** - A hard step cutoff at the turgor height bound (no taper) shipped and was superseded: every lineage ran full speed to one row and stopped, piling growth under the bound, and all eight measured trees capped with a flat horizontal plate. Lockhart's (P−Y)-proportional taper makes the last stretch stochastic so crowns fade over a band.
  *Re-test when:* Unconditional — the step form was rejected on visual measurement (eight trees, all flat-topped); any replacement must keep termination graded.

## E18  (section: destruction, register line 420)

- **`src/sim/load.rs` `evaluate_within`'s bearing clamp, reinstated for `log` by removing its `capacity != i64::MAX` guard** - Tried as the fix for `open-bugs-handoff.md` §Q ("the long skiny vertical pieces should fall over"), on a correct source reading: `bearing_moment` *is* a tipping test, and the opt-out guard is what stops it reaching a landed log. **Giving logs the tipping test does not make them lie down -- it crushes them.** Measured on the identical cut: lying/upright/square 3/8/2 -> **1/11/1**, `log` 833 -> 716, `deadwood` 553 -> 645, i.e. *more* pieces standing, because the ones it condemned were the ones with a footing and the needles survived. The load model's verdicts are *holds* and *fails*, and failing means `breaks_into`: convert where you stand. So a bearing test that correctly finds an over-eccentric piece can only say so by turning it into grit.
  *Re-test when:* Only alongside a `topple` outcome -- an unbalanced piece re-promoted as a body and allowed to rotate. The predicate is right and the consequence does not exist; rotation and the tipping test are one change, and the test is worthless before the rotation.

## E19  (section: plants, register line 685)

- **Reports/plant-species-authoring.md §6 'Nothing creates frontier except the bud bank' (bud siting paragraph)** - Flushing the brightest bud builds a flat cap, because the brightest bud is always on top of whatever the plant has already built. Siting was changed to light per unit crowding, which moves flushes to buds with room.
  *Re-test when:* Holds while light is per-column sky visibility (top cells always brightest).

## E20  (section: creatures, register line 1028)

- **src/sim/creature.rs fn choose_weighted (doc)** - Deterministic selection (Iterator::min_by/argmax) shipped here before and is a published failure (Reports/stigmergy-research.md §2, P-10): the noise is load-bearing, and removing it removes exploration invisibly — agents still move, they just stop finding anything. min_by additionally returns the first minimum on a tie, so a worm whose neighbours read equal always fled west. Never replace the squared-weight sample with an argmax.
  *Re-test when:* Holds for any trail-laying or gradient-following decision; the exploration floor k is part of the mechanism.

## E21  (section: other, register line 1761)

- **`assets/species/ant.ron` hidden units 2/3 (the channel-B food-route gate), re-weighted off saturation the same way units 0/1 were; measured 2026-09-09** - **The repair is correct, it does exactly what its arithmetic promises, and it makes the animal decisively worse.** `open-bugs-handoff.md` §Z7's fault is real for both mirrored pairs: parked at a gate sum of +30 the pair moves `P(move)` by 0.004 against a walk drive of 2.0. Moving the open state to +0.5 raises that to 0.540 at the gradient a laid trail actually delivers -- 135x, at an unchanged shut-pair leak of 0.009 -- and a laid channel-B trail then moves a colony's near-target ant-ticks from **595 = 595 exactly** to 13,985 against 0, three seeds of three. **And `creature_arena arm=same mirror=on seeds=6 frames=24000` reads 25.0% for the re-gated arm, 0 of 6 seeds above 50%**, against a *zeroed-brain* control of 15.4% and a position-confound control (`mirror=off`) of 41.2%. Split by pair, the same race reads **63.6% for the homing pair alone (5 of 6 seeds up, and that half shipped) and 47.6% for the food pair alone**, so the two halves are strongly non-additive. **The mechanism is what generalises**: a colony whose laden ants walk home on channel A and whose empty ants search at random is a central-place forager -- directed return, *undirected* search -- and giving the empty ants channel B turns the search directed toward patches the colony has already eaten, because channel B is laid by ants that already found food. `labforage` shows it directly: `unvisited` 57% -> 74% of the standing larder, the 16-48 distance band eaten out (284 cells standing -> 49) while the >128 band goes untouched (1,285 -> 1,514), and the highest ant head falling from 29 rows to 20. Neither a softer gate (open state +2.0, a quarter of the response: 40.0 survivors against the full fix's 40.8 and shipped's 45.8 over six seeds) nor a clumpier larder rescues it -- `plant=tree` narrows the survivor gap to 44.3 against 47.0 and `plant=conifer` to 31.3 against 31.3, neither reversing.
  *Re-test when:* The rejection depends on **what a food trail is worth in this bed**, not on the gate. Two things would change it and both are now measurable without a rebuild: a larder patchy enough that recruiting to a found patch beats searching (sweep `labforage plant=` / `creature_arena plant=`, both added 2026-09-09), and a channel-B decay fast enough that a trail stops recruiting once its patch is eaten (`labforage bdecay=`, `Pheromones::set_channel_rho`, added the same day -- the shipped `DECAY_RHO` is 0.03 for both planes and nobody had measured whether that is right for B). **One point of the decay sweep is already spent and it went the wrong way**: `bdecay=0.15`, five times shipped, with the food pair re-gated, reads 30 survivors and 45.1k intake on seed 1 against 33 / 49.6k at shipped decay and 43 / 62.4k for the shipped ant. One seed settles nothing, but the obvious first setting is not the answer. Do not retry the weight change alone; it is not the variable.

## E22  (section: destruction, register line 478)

- **Reports/explosion-mechanics-diagnosis.md round 3 "Two bugs found while building it"** - Heat-only ignition — writing cell temperature and relying on `fire::try_ignite`'s temperature path — cannot light anything: no shipped material has a finite `ignition_temperature` (every material file leaves the "never" default; oil's comment says so). A fireball that only wrote heat would have looked right and started no fires, so `scorch` also rolls `flammability` directly, the same property neighbour-contact ignition rolls.
  *Re-test when:* Holds until materials ship finite `ignition_temperature` values; `oil_beside_a_blast_ignites_but_stone_does_not` pins both halves.

## E23  (section: worldgen, register line 1381)

- **`src/worldgen/passes.rs` `strata_rock`, marker beds drawn on the *unit* stream; measured 2026-08-29** - Drawing the ironstone rib and the basalt sill from the same cumulative selection as the ordinary rocks makes a marker inherit the unit's thickness, and a marker bed's whole value is that it is thin -- the first rendering put a **60-cell rust formation** through the section, which reads as a different world rather than as a bed you can follow. Markers are now drawn per *bed* on their own stream (`Purpose::RockMarker`), 3.0% ironstone and 1.5% sill, which at `strata_thickness` 8-12 is a rib roughly every 30 beds.
  *Re-test when:* Holds while a rock unit is more than one bed thick. The rejection is about granularity, not about the probabilities.

## E24  (section: liquids, register line 165)

- **src/sim/update.rs test a_spreading_front_does_not_shed_a_comb_of_detached_ledges (candidate 3)** - Shrinking LIQUID_LATERAL_REACH to reduce shed films is a pure trade against levelling speed with no path to zero films. Rejected.
  *Re-test when:* Re-test only if the settle-drop landing rule changes; reach itself trades levelling speed, not the artifact.

## E25  (section: field, register line 1151)

- **src/sim/field.rs fn step, next-map construction ('Merging solved tiles into the live map instead was tried and reverted')** - Merging solved tiles into the live map (to remove the clone per sleeping tile) was tried and reverted: it forced apply_sky to run after convergence was judged, so its light writes landed in old but never in next, every touched tile read as changed forever, nothing converged, and a four-cell impulse went from 5.2 ms back to 47 ms at 2048x1280. Whatever writes a channel must write it before the comparison that decides whether the tile is done.
  *Re-test when:* Removing the clone is legitimate but requires apply_sky to write into the solved subset while walking the full column — a second mechanism, not a reordering.

## E26  (section: destruction, register line 540)

- **src/sim/player.rs fn face_toward (doc)** - Aiming a dig by clamping the cursor onto the reach circle looks right and was wrong: clicking deep inside a massif put the bite behind the rock face, carving a sealed pocket whose spoil had nowhere to go, so stone turned to rubble in place — on screen, a dig that does nothing (rubble and stone are deliberately near the same grey). Replaced by a ray to the first blocking cell, which digs the near face like a pickaxe and guarantees open space behind the bore.
  *Re-test when:* Holds while spoil must be displaced rather than deleted; any aim that can land behind a sealed face re-creates it.

## E27  (section: structural, register line 344)

- **src/sim/load.rs fn arch_span body, comment on the cover probe** - Testing only the cell immediately beneath the roof for the opening was tried and was far too narrow to matter: the clamp reached one row while overload failures happened in the rock above it, and the A/B came back identical on two of three bore sizes. The whole roof beam arches, so a bounded downward cover probe is required.

## E28  (section: plants, register line 879)

- **src/sim/plant.rs stale-limit retirement comment ('Covers RootTip as well…') and test a_root_tip_that_ages_out_retires_instead_of_becoming_a_phantom** - Staleness retirement covering only GrowingTip left an aged-out RootTip matching no branch: never rescheduled, never retired, skipped by upkeep — a phantom still counted in root_cells and against max_active_tips, tightening the very allometry ratio that blocked it. Found by independent review.
  *Re-test when:* Any new frontier cell type must be covered by the stale-limit retirement branch or it becomes a phantom the same way.

## E29  (section: other, register line 1745)

- **`src/sim/creature.rs` `backed_out_body`, freeing a stuck long body by walking it *backwards* (the tail leads into a free cell, every segment inherits the position of the one behind it, roles stay on their cells); built, measured against the flip and rejected 2026-09-10** - **A backward step does not change which end the head is, so the animal is boxed again on the very next tick and the rule has to fire over and over without ever resolving anything.** Measured on the one-wide tunnel scene, one binary, `PIXEL_PHYSICS_REVERSE=back` against `flip`: backing out fires **809** times and still leaves the two-wide body refused on **49.0%** of its steps, where flipping fires **136** times and leaves it on **7.6%** (a two-cell ant's bar on that scene is 5.1%). It also needs a tail-target chooser -- a second steering decision with no brain behind it -- which the flip does not, because a flip places no cell anywhere new. And when a backing animal finally clears the tunnel mouth the head is the last thing out, still facing back down the passage it just left, so it walks straight back in. Replaced by the reversal in place (the owner's own proposal): the segment order swaps end for end, no cell moves, and the animal walks out forwards. Full tables in `Reports/creature-articulated-body-2026-09-09.md` §13d. *Re-test when:* a body plan appears whose head and tail are not interchangeable -- a flip mirrors the animal, so a species where that reads wrong (a long tail, a head that is visibly a head at play zoom) may prefer to pay backing-out's cost. **And carry the measurement error rather than the result**: the first comparison gated both arms on the flip's own delivery test (*the reversed body must be able to walk*) and measured backing out at 8 reversals against 2,371 refusals, which reads as "backing out never works" and is a fact about the gate -- one backward step cannot unbox a head. Two rules that resolve a state at different rates cannot share one delivery test.

## E30  (section: structural, register line 383)

- **src/sim/structural.rs fn tick, judge-on-settle comment** - Running the full subtree judgment on every step of the distance wavefront cost 4,456 ms/frame on scene=strike and 6,556 ms on scene=capped, and it reads the support forest mid-flight (load-model-handoff §5.2's stale-parent hazard). Cells are judged once, on the tick their distance settles or worsens.

## E31  (section: other, register line 1517)

- **Reports/ecological-lod-design.md §1 + §4 'The answer to §7b'** - Advancing off-camera populations with a continuous model (the ODE 'correct population after N frames' framing) was rejected: it produces atto-fox behaviour — a predator population of 10⁻¹⁸ that recovers to carrying capacity. Individuals are frozen at unload, only fields and the patch tier advance, and every population-like quantity is quantized to integers so a patch that decays to 0.4 individuals rounds to zero and stays zero.
  *Re-test when:* Structural to advancing discrete populations; the off-camera world owes plausibility and conservation, not correctness. Corollary that must ride along: frozen means frozen for hunger, age and breeding alike — a creature that ages but does not eat off-camera is the worst of both.

## E32  (section: destruction, register line 571)

- **src/sim/rigid.rs fn mine_swept doc, section 'Why a bore has to be swept and not stamped'** - Driving a tunnel as a row of stamped discs was shipped and failed on sight and in function: the bore's usable height is the scallop between discs, not the disc — the pinch between two radius-7 bites measured 13 cells against the gnome's height of 14, wedging him at the mouth of his own bore; the owner: 'why are you digging tunnels a row of circles instead of a tunnel.' Bites are swept as capsules from the previous bite, giving constant 2r+1 height.

## E33  (section: liquids, register line 93)

- **Reports/liquid-heightfield-design.md §1a fact 2–3 (scoped free-surface variant)** - Scoping the reorder to a literal local free-surface test ('cell above me is not more of the same liquid at full fill') was also tried and reverted: it did not fix the ballooning (~4.8x peak). The approach-level conclusion: no per-cell local heuristic can distinguish the free surface of one connected body from any cell with room nearby once the body's profile is roughened — many interior cells legitimately read as 'at the surface' and collectively over-dilute. Being at a free surface is a whole-body property.
  *Re-test when:* Terminal for per-cell approaches; the heightfield body representation (where the over-diluted state has no encoding) is the recorded answer. §12 B-1/B-2 deliberately share one scene because responsiveness and no-ballooning were in direct tension under every per-cell attempt.

## E34  (section: character, register line 1466)

- **src/sim/player.rs `shake`'s dislodge pass (comment)** - Shaking loose material off every cell of the plant, with no test for whether the cell is in the open. Roots are part of the plant and the soil bed rests on them, so every root read as a laden branch: 1,235 cells "knocked loose" in 43 shakes, nearly all of it soil being stirred round a root system. Gated on the cell having air beside it: 429.
  *Re-test when:* Holds while roots are `climbable` and grow through soil.

## E35  (section: plants, register line 841)

- **src/sim/plant.rs fn break_buds doc, property 2** - A bare rate cap ('one bud per organism per tick') as the bound on frontier creation was considered and rejected: it converts exponential growth into linear growth, which still fills the world. It survives only as a rate limit on top of the supportable-count bound.
  *Re-test when:* Unconditional — a cap bounds rate, never total; the bound must come from income.

## E36  (section: plants, register line 715)

- **Reports/plant-substrate-v2-design.md §4c 'What "too much moisture" actually costs a root'** - Two softer waterlogging responses were explicitly rejected: reduced absorb efficiency has the mechanism backwards (oxygen is the scarce resource in waterlogged soil, not water — a root drinking less has no physical story), and slowed growth makes waterlogging observationally indistinguishable from mild drought, throwing away the asymmetry that makes the mechanic interesting. RootTip necrosis via a duration-gated anoxia counter was chosen.
  *Re-test when:* Grounded in threshold-then-death literature; necrosis also yields the emergent capillary-fringe root boundary the soft options cannot.

## E37  (section: rendering, register line 1276)

- **Reports/open-bugs-handoff.md §4b 'A cell alone in the air drops its column's skyline' (closed)** - Every version of inferring the sky surface from world geometry failed: the last one (topmost cell plus repair within six columns) gave a floating grain 20 rows of cave, treated a 51-wide plank identically, and opened daylight 35 rows into a mountain at a 13-wide shaft. No reach setting fixes it — hill, shaft, roof and grain are the same arrangement of cells, and 'I dug this' versus 'this is a hill' is history, not geometry.
  *Re-test when:* Unconditional for inference; the fix stores history (World::sky_surface recorded once at first frame). Revisit only by storing more history, never by inferring. **Revisited exactly that way and it worked** (Reports/dark-bands-diagnosis.md): the per-column form still could not tell a cave roof from a cliff brow, because that is a *column* question asked of a per-cell fact, so the same answer is now stored per cell (World::freeze_underground_map, one bit, seeded by a genesis flood fill). No inference, no threshold, and every property the column form was chosen for survives. The remaining case — a wide open pit you dug still reading as cave — is not reachable by any better boolean and needs light propagation instead (Reports/prior-art-underground-lighting.md). **Propagation shipped and only half-answered it**: seeded spread measures distance from a lit cell, which cannot tell a hole from a hollow, so a broken-open hilltop still drew black on a 2026-08-29 playtest. Closed by adding a sky-*visibility* term beside it (render.rs SKY_VIEW_RAYS, Reports/sky-light-design.md) — still not a boolean, and still not a threshold on width.

## E38  (section: structural, register line 374)

- **src/sim/structural.rs fn compute_world_distances body, edge-anchoring comment** - A first version guarded the mirror's neighbour reads with a bounds check, treating off-world as nothing — which un-anchored every cell touching the world edge and took six load tests down with it. Off-world must read as bedrock, reproducing the Cell::OUT_OF_BOUNDS convention.

## E39  (section: rendering, register line 1250)

- **A pixel-art upscaler (hqx / xBR / EPX) over the finished frame, as the answer to "the plants look blocky"** - The obvious reading of "upsample it", and it is backwards here. Those filters infer shape from **colour** — which neighbouring pixels happen to match — and this world is their worst case: every material carries a deliberate per-cell shade jitter, so adjacent pixels of one material rarely match and the filter has nothing to latch onto. It would smear the grain, which is the thing that works, and leave the one-cell plant strands alone, which is the thing that does not. Reconstructing from **what the cell is** (material, organism cell type) is information a post-hoc filter cannot recover and the renderer already holds.
  *Re-test when:* Never while the palettes carry grain. `Reports/subpixel-rendering-2026-08-29.md` §4.

## E40  (section: destruction, register line 585)

- **src/sim/structural.rs fn crush_in_place body, single-origin comment** - Scattering several crack origins through the piece, each throwing straight spokes, was rejected by the owner: 'it doesn't really look like a spreading crack from a boulder, it just looks like criss cross irregular lines.' Everything must descend from one origin — a few primaries like a windscreen star, every other stroke a fork — so it reads as a trunk with branches, not scribble.

