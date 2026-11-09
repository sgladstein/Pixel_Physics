# What Jenson, Cherouvim and Jones do that our mould does not (2026-11-09)

**Read this first: I could not open any of the three sources myself.** This session's network blocks `cargocollective.com` (Sage Jenson's post), `ch3.gr` (Georgios Cherouvim's site) and `uwe-repository.worktribe.com` (the free copy of the Jones paper). Every other mirror I tried is blocked too: arxiv, Semantic Scholar, CORE, ResearchGate, CiteSeerX, MIT Press and the Wayback Machine. Fetching the post through a third-party reader proxy was refused as a way round the block, and I did not try other workarounds. To get a real full read, add those three hosts under the cloud environment's *Network access* (Allowed domains) and re-run this brief.

What I *did* read, so you can weigh each claim:

- **Jones 2010: about a dozen passages of the full text.** SciSpace's extraction tool reads the paper's full text and returned quoted passages to ten targeted questions: the model rules, the SA/RA map (its figure 13), network evolution, population and decay, stimuli, chemorepulsion, damage repair, shrinkage, and the discussion. That is real text from the paper, but chosen by a machine and not by me. **Table 1 (the default numbers) did not come back**, so some defaults below are from memory and marked as such.
- **Jones 2011** (*Influences on the formation and evolution…*, Natural Computing) and **Jones & Adamatzky 2012** (*amoeboid movement*): passages extracted the same way. These hold the growth and shrinkage rules and the emergent oscillation.
- **Jenson's post:** search-engine summaries of the page only, which quote the parameter list, the hardware and the credit line. I also read the code and README of Bleuje's open re-implementation of Jenson's later *36 Points* (on GitHub, which is reachable), including its full parameter matrix.
- **Cherouvim:** search summaries of three of his pages (*Slime mold*, *Sugarscape and slime mold*, *Strange Jealousy*).
- **Our code: read directly.** All of `src/mould/mod.rs` (the model, economy and flow), the drawing in `look.rs` (the `dish` function, the pulse, the starving looks), and the doc headers and key functions of `render.rs`, `play.rs`, `level.rs`, `wander.rs`, `organ.rs`, `tune.rs` and `lesson.rs`. Also the design record and the earlier research pass. I did not read every line of the 3,800-line `look.rs` or of `play.rs`. Most of both is HUD and bots.

Marks used below: **[X]** quoted passage from a paper's full text via SciSpace; **[S]** search summary of the page; **[G]** read on GitHub; **[C]** our code, read; **[K]** my own memory of the source, not checked this session.

---

## Summary, in plain words

1. **Our mould is set to the one setting in Jones's map that settles down and stops changing.** We run his "sensor angle 45, turn 45". In his paper that setting closes up its small loops, pulls tight into a few simple paths, and then holds still [X]. That explains three things we have already measured: the lamp's bend springs back within 5 s, a tube left alone moved 0 cells in 60 s (`organ.rs`), and the storm star is hard because the network has no spare loops. Changing one number (sensor angle 22.5) gives his other main setting, a network that keeps branching and rebuilding itself and keeps spare loops [X]. **On screen:** a mould that is visibly busy even when you leave it alone. **In play:** loops survive the storm more often, but networks run longer. This costs no frame time, and it has never been tried here: the probe can already sweep it, but no run is recorded.
2. **Let the player change the mould's temperament locally.** Jones shows that the sensor-to-turn ratio decides "pull tight" against "branch out" [X]. Jenson's later work sets those angles from what each walker senses [G]. A tool (mist, or warmth) that shifts the angles where you paint it is a verb whose result is the mould's own behaviour. It gives a graded answer, and nothing in it is drawn.
3. **Hungry mould should search differently, not just turn grey.** Make a starving walker's sensors reach wider and turn harder. A cut-off branch then *fans out searching*, which you can see without any colour change.
4. **Jones keeps tubes thin by killing walkers where they crowd, and grows only where walkers are moving forward** [X]. We have the growth half of that, loosely, and not the crowding half. Adding it should give thinner, more even tubes that repair a break by growing new mould into it [X]. It should also stop the fat blob round every pile.
5. **The real organism spreads first and prunes later; ours only reaches out.** Jones's "shrinkage" run starts the dish full of mould and removes walkers slowly at random. The sheet shrinks onto the food and leaves a clean network. Remove them faster and it tears into holes [X]. This could be a dish type, or a "let it dry" verb where the drying speed decides between a clean network and a torn one. That is a spread of outcomes, not a yes/no.
6. **How strongly a food smells decides the network's shape.** Faint food gives meeting points in empty space (Steiner trees); strong food gives direct spokes [X, Jones 2011]. That is exactly what *Beat the Ruler* asks for. Today the strength is a fixed constant (`fed_scent`). Tying it to how full a pile is, or making it something the player can change, would let the network's shape follow the food.
7. **A rival mould costs us almost nothing in this model.** Jenson-style species each follow their own trail and avoid the others' [S on derivatives, K on Jenson]. They hold territory and form borders with no new rules.
8. **The pulse could come from the walkers themselves.** Jones & Adamatzky 2012 get oscillations and travelling waves out of nothing but walkers jamming in a crowd. Food makes the beat stronger and light makes it weaker [X]. That would replace both our drawn pulse and the clock-driven pulse in flow mode with one Scott can trust. It is the highest-risk idea here: it was shown in a dense blob, not in tubes.
9. **Cheap looks taken straight from Jenson, all drawn from real simulation data.** Draw the faint trail below the tube line, so you see where the mould is searching. Draw walkers as moving specks, so you see the streaming. Colour by species or by load. Jenson's look comes from a continuous trail field and huge numbers of walkers [S]; ours cuts the trail off at a hard edge.

---

## 1. Sage Jenson, "physarum" (cargocollective.com/sagejenson/physarum)

**Status: not read. Search summaries only, plus his later *36 Points* through an open re-implementation.**

What the summaries confirm [S]:

- **The model is Jones 2010**, described as two layers. An agent-based "data map" (the particles) and a continuum "trail map" (a 2D grid of intensities). Particles deposit onto the trail map and sense from it to steer.
- **Each particle** has a location, a heading and three sensors (front-left, front, front-right).
- **Each step the trail map is diffused** with a 3x3 mean filter and then multiplied by a decay factor.
- **The configurable parameters**, as listed on the page: *sensor distance, sensor size, sensor angle, step size, rotation angle, deposition amount, decay factor.* "Sensor size" and "step size" are the two that our model does not have. Ours samples one cell and steps exactly one cell.
- **Scale:** C++ and GLSL in openFrameworks, with everything on the GPU (particles and trail map). It runs in real time on a GTX 1070, and most examples use **5 to 10 million particles**.
- **Credit:** Jenson says the work was inspired by Georgios Cherouvim's posts, which referenced the Jones paper. The Emergent substack's history gives the chain as Jones's 2009 to 2011 papers, then Cherouvim, then Jenson.
- A Processing forum thread records Jenson saying they only played with the parameters listed in the article. So the look comes from parameter choice and rendering, not from hidden rules.

From memory, unverified [K]: the post is mostly animated images of different parameter sets, showing the range from fine reticulate webs to fat ropes and dot fields. It shows the trail map rendered directly through colour ramps, never the particles, and I believe it shows multi-colour variants. I could not confirm a multi-species section. Treat "species attracted to their own trail and repelled by others" as the convention in derivatives that cite Jenson (Slime50, the Buzing implementation, Sebastian Lague's video) [S], not as something I saw on the page.

**Jenson's *36 Points* (2019 to 2022): what it adds, read through Bleuje's port [G].** `points_basematrix.h` in `Bleuje/interactive-physarum` reproduces the parameter matrix. **Every particle first senses the trail value `s` where it stands, then sets its own parameters from it:**

```
sensor distance = SD0 + SDA * s^SDE
sensor angle    = SA0 + SAA * s^SAE
rotation angle  = RA0 + RAA * s^RAE
move distance   = MD0 + MDA * s^MDE
```

There are 24 named presets ("pure_multiscale", "hex_hole_open", "vertebrata", "star_network", "waves_upturn", "massive_structure", "clear_spaghetti" and others), each a row of these coefficients. So one rule makes thin wanderers in sparse areas and tight bundles in dense ones, at once. Bleuje adds:

- spatial blending between two presets;
- "interaction to move particles" and "to spawn particles";
- particle inertia;
- a deposit that counts particles per pixel, not a fixed amount.

Bleuje's installation has a gamepad button that briefly raises speed and sensing for the whole field, which the earlier research noted as a verb worth copying. In 36 Points, particles also respawn at random places after a while. Bleuje's explanation article notes that in 36 Points the trail acts like translucent dots, so trail values never exceed 1 [S].

**What Jenson does that we do not:**

- a continuous trail field rendered directly, at a very large particle count;
- area sensing ("sensor size");
- fractional step sizes;
- parameters that depend on the sensed value;
- colour as data (species or density).

## 2. Georgios Cherouvim (ch3.gr)

**Status: not read. Three search summaries only.** The page Jenson's post links is `ch3.gr/2018/slime-mold/`, per the `morphogenesis-resources` index [G].

- **"Slime mold"** [S]: "an agent based feedback loop system that simulates the behavior of Physarum polycephalum (slime mold) and the natural process of network optimization. At later iteration it is also combined with additional forces and a video feed to affect its growth and behavior." It is rendered in real time, about **300k particles at 60 fps**, in GLSL compute shaders inside TouchDesigner.
- **"Sugarscape and slime mold"** [S]: a terrain generated with a *sugarscape* (Epstein & Axtell's *Growing Artificial Societies*), "a cellular automata model with agents looking for sugar in the environment", combined with the slime mould as "a multi agent based system, obeying a set of simple rules and forces." In a standard sugarscape, each cell regrows sugar (typically +1 a step) up to its capacity. Agents have *vision* and *metabolism*, and starve when their sugar runs out [S, general Sugarscape sources]. I could not confirm how Cherouvim coupled the two.
- **"Strange Jealousy"** (a music video) [S]: live footage drives "a multi agent based system, obeying a set of simple rules and forces."
- He is an FX/CG lead who teaches Houdini at SVA New York [S].

**What Cherouvim does differently:**

- **external forces** act on the agents alongside the trail sensing;
- **an image or video field** steers growth;
- **food as a landscape that regrows**, not a few fixed piles.

The first is what our lamp push already is. The last two we have not tried.

## 3. Jeff Jones (2010), *Characteristics of pattern formation and evolution in approximations of Physarum transport networks*, Artificial Life 16(2):127-153

**Status: passages of the full text, not a cover-to-cover read.** SciSpace record: <https://scispace.com/papers/characteristics-of-pattern-formation-and-evolution-in-3sr6e5wv1h>. Free copy (blocked here): <https://uwe-repository.worktribe.com/output/980579>.

**The agent [X].**

- Each lattice site holds at most one agent. Every scheduler step, every agent first tries to step forward along its heading. Then the whole population senses.
- **If the step succeeds** (the next site is empty), the agent moves and deposits a constant amount of chemoattractant. **If it fails**, the agent stays put, deposits nothing and picks a random new heading.
- Agents are picked in **random order** in both the motor and the sensory stage, "to avoid long-term bias".
- After each step the trail is diffused by "a pseudoparallel simple mean filter in a 3 x 3 kernel" with "an adjustable decay value".
- There is also a "passive" variant, in which agents sense but do not deposit.
- The parameters that matter most are **SA** (sensor angle), **RA** (rotation angle), **SO** (sensor offset), **p** (population, as a percentage of lattice sites) and the decay.
- The habitat is a grayscale image whose values mark habitable ground, obstacles and food.
- *From memory [K], Table 1's defaults:* SA 22.5°, RA 45°, SO 9 px, sensor width 1 px, step 1 px, deposit 5, decay 0.1 (90% kept), pCD (chance of a random turn) 0, sensitivity threshold 0. If so, our defaults are Jones's except that our SA is 45°.

**The pattern map [X].** Figure 13 varies SA and RA each from 0° to 180° in 22.5° steps, runs every cell of the map for 500 steps, and shows snapshots ("the patterns continue to evolve"). In Jones's words, evolution "may remain disordered (where no patterning was seen), complex (as in RA 45, SA 22.5 with dynamic remodeling), minimizing (as in RA 45, SA 45), or a simplification of the initial patterning". "When both parameters are increased together there is a trend from reticular to labyrinthine to island-like patterns"; "low values show higher connectivity, whereas higher values result in more isolated patterns". By chemoattraction he names *irregular spots, labyrinthine and reticulated* patterns. From Jones 2011 [X]:

- **RA = SA (both 45°):** the network contracts.
- **RA < SA:** contraction increases.
- **RA > SA:** "spontaneous branching", because "the larger rotation angle places particle sensors outside the chemoattractant trail path".

**How networks evolve [X].**

- **SA 45 / RA 45:** after the same complex start, "bifurcation and sprouting of new trails do not persist… lacunae gradually become larger as smaller cycles are closed", ending in "a simple network" that "approximates a regular hexagonal tiling". It is "a self-balancing structure": stable while its parts turn over.
- **SA 22.5 / RA 45:** "constantly changing… does not reach minimal configurations… network length is sacrificed for network connectivity". With fixed boundaries it still condenses, but branching stops it collapsing into a solid sheet.
- **Bigger population:** thicker network edges and fewer lacunae (the holes between paths).
- **Faster decay** (a weaker gradient field): coarser edges. **Very slow decay** (strong gradients): stronger attraction between agents and *less* connectivity.
- **SO is a scale factor.** He shows 3, 9, 15 and 25 px: small SO gives fine structure, large SO gives wide, coarse paths. Jones 2012 adds that "a minimum distance of 3 pixels is required for strong local coupling".

**Single agents [X].** He tracked one agent for 3,000 steps. It hesitates and moves back and forth along the path "strongly indicative of shuttle streaming flow", because of random differences in path strength and collisions between agents.

**Quasi-physical behaviour [X].** The paper reports:

- tension-like effects;
- minimisation, with junctions tending to three-way nodes at 120°, as in Plateau's laws;
- **network repair**. Chemoattraction networks repair a wound "by interdigitating migration… gradually reestablishing the network, but without recovering the original pattern". Chemorepulsion patterns close the wound and restore the original pattern.

**Food and other stimuli [X].**

- Food ("pre-pattern stimuli") is written into the *same* trail layer every step, at strength stimulus value × weight, with the weight "typically 0.01 to 0.1", and then diffused like the trail.
- Stronger coupling to the food tightens the network. It moves from a Steiner tree (meeting points in empty space) toward a minimum spanning tree (direct food-to-food links).
- Jones 2011 quantifies this: projection weight 0.005 gives Steiner trees, and 5 favours direct connections, over 20 runs × 10,000 steps.
- With more food nodes, "cyclic areas are less likely to be closed": the network keeps more connectivity at the cost of length.
- Jones 2011's abstract says the model also approximates relative neighbourhood graphs, convex hulls and concave hulls.

**Chemorepulsion [X].** Agents that turn *away* from the trail make Turing-like patterns: "regular periodic arrays of spots" and stripes. Local patterns spread, compete, and one wins and covers the whole area, regular but still dynamic.

**Shrinkage [X].**

- Start with a solid sheet (p 50 on a 300x300 lattice, 45,000 agents) over the food.
- Remove agents at random, never re-admitted, with probability **0.00025 per agent per step**.
- Slow removal keeps agents evenly spread, so "particle flux can adapt". The sheet "maintains a minimal area as it shrinks", and three runs ended in identical networks.
- **Faster removal tears the sheet into holes.**

**Growth and shrinkage as rules (Jones 2011, [X]).**

- **Grow:** if there are 1 to 10 particles in the 9x9 window round an agent, *and it has just moved successfully*, it divides into an empty adjacent site.
- **Shrink:** if there are more than 20 particles in its 5x5 window, it is removed.
- Both tests run every 3 steps.
- Growth therefore happens at the edges and where food drives movement. Crowding removal "keeps network paths thin".
- An adaptive population "prevents snapping" when tension rises: new particles repair a break.

**Emergent oscillation (Jones & Adamatzky 2012, [X]).**

- Oscillations come from non-oscillating parts. Disordered local oscillating domains compete and lock into regular waves that travel through the blob.
- Lowering `pID` to 0.01 makes the oscillations much stronger. That parameter "restricts particle movement fluidity"; I believe [K] it is the chance a blocked agent re-orients.
- The boundary distorts, and because the blob is incompressible at fixed area, it migrates: amoeboid movement.
- Food increases local oscillation strength, and simulated light (sensitivity multiplied by a weight below 1) decreases it.
- A band of light through the middle cleaves a blob in two, and two blobs that come close fuse.
- The collective must be large enough for oscillation to start. Small SO fragments it.

**Briefly, related Jones work.** *From Pattern Formation to Material Computation* (Springer 2015) [K] collects all of the above and adds:

- growth from a single inoculum;
- food that is "engulfed": its stimulus is suppressed once covered;
- travelling-wave transport of objects;
- a long catalogue of ways to steer the model (*Parallel Processing Letters* 2015, per the earlier pass).

*Towards Physarum Engines* (Tsuda, Jones & Adamatzky 2012) uses those oscillating domains to push objects along [X, abstract].

---

## 4. Our model against theirs

| Mechanism | Jones 2010/11/12 | Jenson | Cherouvim | Ours (`src/mould/`) |
|---|---|---|---|---|
| Sensor angle SA | Swept 0-180°. Default 22.5° [K]. 45 = minimise | Per preset; in 36 Points, a function of sensed trail | Not known | **45°**, fixed (`Params::sensor_angle`). Not on the T panel |
| Rotation RA | Swept 0-180°. 45 typical | Same | Not known | 45°, fixed |
| Sensor offset SO | Scale: 3/9/15/25 shown; ≥3 for cohesion | Per preset / sensed | Not known | 9 → tubes about 12 cells wide (`TUBE_WIDTH`) |
| Sensor width | 1 px [K] | "sensor size" is a parameter | | Point sample |
| Step | 1 px | "step size"; in 36 Points, a function of sensed trail | | 1 cell |
| Random turn pCD | 0 by default [K] | | | None |
| Deposit | Constant, only on a successful move | Per preset; Bleuje counts particles per pixel | | 5, only on a successful move ✓ |
| Diffuse / decay | 3x3 mean + decay | 3x3 mean + decay | | 3x3 mean, keep 0.90, no-flux at walls ✓ |
| Update order | Random, motor and sensory stages separate | GPU-parallel | GPU | Shuffled, sense+move in one pass |
| Population | p% of lattice (3% to 50% shown) | 5-10 M particles | 300k | 400 to about 2,200 on about 30k agar cells: **p ≈ 1% to 7%** |
| Boundary | Periodic or fixed; fixed → condensation | Wrap (usual) | | Fixed rim (ellipse) |
| Food | Written into the trail layer, weight 0.01-0.1; weight sets Steiner vs MST | | Sugarscape terrain (regrowing) | Separate **scent** field (keep 0.97), 20/cell/step, 15% once eaten (`fed_scent`); fixed piles |
| Repellent | Chemorepulsion = spots, stripes. Light = sensitivity × w<1 | Other species' trail (derivatives) | Forces, video | Salt is a wall; the lamp **pushes** trail and walkers (sensor repulsion is a recorded dead-end) |
| Growth | Every 3 steps: 9x9 holds 1-10 *and moved* → divide | Respawn at random | | Chance 0.03/step if fed and 5x5 ≤ 7; paid from food. No "moved" condition |
| Removal | 5x5 > 20 → removed (crowding); random removal for shrinkage | Respawn | | Starvation only (`starve`, `hungry`); no crowding death |
| Oscillation | Emergent from jamming (2012) | | | A global sine clock (`Dish::beat`) in flow mode; a drawn band (`look.rs::pulse`) otherwise |
| Memory | None (trail only) | | | `wall` field (flow level 3) |
| Rendering | Trail map / particle positions, grayscale | Continuous trail map through colour ramps, huge N | Real-time particles | Trail cut at 0.5×`tube`, log ramp, emboss and glow; walkers never drawn; fed/starving palette |

**Where we sit on Jones's map.** SA = RA = 45°, SO 9, low population (p about 5%), fixed boundary. That is **the minimising, self-balancing corner**: small loops close, the network pulls toward a few straight paths, and then it holds still. Our own record shows that corner's signature three times over:

- The organ probe measured **0 cells of tube-length change over 60 s** untouched. A lamp bend of +79 to +245 ms "is gone within 5 s of letting go on 8 of 8 seeds" (`organ.rs` header).
- The toughness star is the hardest star. Minimising throws away the redundant loop that would survive a cut, and the storm splits trees (`level.rs`, *No storm*).
- Thrift (length ≤ 1.15× the shortest) is reachable, because the mould minimises by itself.

**What moving each lever should do, by Jones's words** (none of this is measured on our dish yet):

| Move | Expected pattern | On screen and in play |
|---|---|---|
| SA 45 → 22.5 (RA 45) | "Dynamic remodelling": constant branching, loops kept, never minimal | A busier, twitchier network; more loops, so more storm survival; worse thrift; the lamp's bends may stick a little, because nothing is pulling tight |
| RA 45 → 22.5 (SA 45) | RA < SA: "contraction increases" | Tighter, straighter trunks, quicker to snap back; a "taut" mould |
| Both up (67.5/67.5 and beyond) | Reticular → labyrinthine → islands | Wormy labyrinths, then isolated blobs; probably unplayable past 67.5 |
| SO 9 → 5 | Finer network, thinner paths | More, thinner tubes from the same walkers; every width-calibrated constant moves |
| SO 9 → 15 | Coarser, wider paths | Fewer, fatter ropes |
| More walkers (p up) | Thicker edges, fewer holes | Fuller look; costs sim time linearly |
| Slower decay (keep 0.9 → 0.95) | Stronger attraction, *less* connectivity | Clumpier |
| Food weight down (once eaten) | Steiner points (meetings in empty space) | Exactly *Beat the Ruler*'s goal |
| Food weight up | Direct spokes, MST-like | Straight food-to-food lines |

The probe can already sweep all of these (`mouldprobe sensor_angle=… rotate_angle=… sensor_offset=…`). The design record has no such sweep.

**Where the earlier research pass (`mould-game-brainstorm-2026-11-08/research.md`) went shallow.** It summarised biology well, but:

- **It never placed our settings on Jones's map**, or suggested the one-number change that Jones says flips the behaviour. It treated Jones as "what we run" and moved on to other models.
- **It read no text from any source.** Every [V] was a search summary or an abstract. It reported the paper's abstract ("tension-like effects, network minimisation, and repair") and nothing inside it: no SA/RA results, no growth or shrink rules, no food-weight result, no shrinkage experiment, no chemorepulsion.
- **Its "visible heartbeat" (#2) was drawn-only** (brightness = sin(t − d/λ) on BFS distance). It was built and then rejected by Scott ("everything we're seeing should be based on real pixel simulations"). Jones & Adamatzky 2012, which it cited in one line for exactly this behaviour emerging from the agents, was not followed up.
- **It concluded that art versions get their beauty "from particle count and colour ramps, not new rules"**, then proposed no rendering change. It also missed Jenson's actual new rule: parameters as a function of the sensed value.
- **It did not look at Cherouvim at all.**
- **Most of its ranked borrows have since been built** (wall memory, the pulse, light-as-upkeep was proposed). So the next layer of depth has to come from the model's own dynamics, which it skipped.

---

## 5. Improvements to what we already have

Ranked by value against risk. "Frame" is the risk to the frame budget: currently mean 3.3 ms (old rule) or 5.0 ms (flow), and worst 8.8 to 17 ms (design record). "Rules" are Scott's: **sim** (on screen = simulated), **fun** (fun over fidelity), **spread** (graded outcome), **verb** (player action with a visible result).

1. **Put SA, RA, SO (and a new pCD) on the T tuning panel, and sweep SA 22.5 against 45 as a paired A/B with pictures.**
   - *Where:* `tune.rs` rows, `Params` (pCD is new: one random-turn roll in `move_walkers`).
   - *Frame:* none.
   - *Serves:* fun, spread.
   - *Note:* the constants calibrated at 45/45 must be re-derived if the default moves (CLAUDE.md's *Fixing a bug often exposes a constant that was compensating*): `TUBE_WIDTH` 12, `OATS_PULL` 28 and `OATS_MAYBE` 36 (measured pull bands), `THRIFT_BAR` 1.15, every level's star bars, and `STORM_CUTS` (whose dead-end says re-test "if the network gets denser").
   - *Cheapest high-value experiment in this list.*
2. **Jones's crowding removal** (5x5 > N → the walker dies and its food is refunded to the walkers near it).
   - *Where:* `economy_flow` / `economy_by_piece`.
   - *Frame:* small (a 25-cell count per walker every 3 steps, like the existing `crowd` test).
   - *Serves:* sim, fun.
   - *Expected:* thinner, more even tubes; no fat blob round piles; mould redistributed rather than stacked. This touches the economy (fewer walkers per pile), so `table` and `eat` need re-tuning.
3. **Growth only after a successful move** (Jones 2011), and a sparse-window test like his 9x9 holding 1 to 10, in place of our "5x5 ≤ 7".
   - *Where:* the births loop in both economies; needs a per-walker `moved` bit set in `move_walkers`.
   - *Frame:* none.
   - *Serves:* sim, spread.
   - *Expected:* growth concentrates at moving fronts and on routes being travelled. It also repairs a break: new walkers appear where movement resumes.
4. **Food weight follows the food** (Jones's projection weight). Make `fed_scent` a function of a pile's remaining fraction, or of its eaters versus `table`.
   - *Where:* `deposit_food`.
   - *Frame:* none.
   - *Serves:* sim, spread.
   - *Expected:* a full, under-eaten pile pulls direct spokes; a busy or dwindling one lets the network relax toward Steiner meeting points. *Beat the Ruler*'s answer then responds to how the player feeds it. The `fed_scent` dead-end (scent in the trail channel) is untouched: this keeps the separate field.
5. **Replace the global pulse clock in flow mode with something the mould makes.**
   - Today `Dish::beat()` is `1 + strength·sin(t/period)`, a clock imposed on the whole dish. The food it moves is real, but the rhythm is not the mould's.
   - Interim, low risk: give each piece its own phase, advanced by its own food intake, so pieces beat apart and a join visibly falls into step. That is real state, and a join gets a consequence.
   - Long term: see new idea 6.
   - *Where:* `beat`, `economy_flow`.
   - *Frame:* none.
   - *Serves:* sim.
6. **Show more of the simulation the drawing already has.**
   - (a) Draw trail between about 0.1 and 0.5×`tube` faintly, on the same fixed ramp: the "searching haze" where walkers explore but no tube has formed. Jones's and Jenson's pictures are mostly this. That is real data, currently thrown away at a hard cut.
   - (b) Draw walkers as one-pixel specks at 4x scale, so streaming and the back-and-forth Jones describes are visible.
   - (c) In flow mode, let the specks' brightness be their load. Food is then seen riding the walkers.
   - *Where:* `look.rs::dish`.
   - *Frame:* (a) close to free, since every cell is already visited; (b) about 2,000 pixel writes.
   - *Serves:* sim (it removes the need for a drawn pulse), fun.
   - *Note:* (a) moves what "edge" means for the smooth-edge pass, so check that the 1-in-6 edge-cell count does not balloon.
7. **Flag the drawn-only effects that remain.** Scott's rule says they should go or become simulated:
   - the default (flow off) tube pulse, `look.rs::pulse` and `PULSE_*` (a band computed from BFS distance and time);
   - `organ.rs`'s pulse ("a snapshot" riding `heart_geodesic`);
   - the join "snap" burst and the pile-arrival flash, which mark real events but are drawn graphics. These are defensible as markers of events, but they are not simulation.

## 6. New ideas

1. **Temperament as a verb: a local SA/RA field the player paints** (mist to branch out, warmth or dryness to pull tight). The rule is Jones's (RA>SA branches, RA<SA contracts). The machinery is Jenson's (parameters per walker from the cell). The result is the mould behaving differently where you painted, with a graded answer.
   - *Where:* new per-cell field in `Dish`; `move_walkers` reads SA and RA from it; a new `Tool`.
   - *Frame:* a field lookup per walker per step (small) plus a decay pass (about the cost of one `spread`).
   - *Serves:* verb, sim, spread.
   - *Note:* this is not the reverted sensor-light lamp. It changes *how* walkers turn, never subtracts from what they sense.
2. **Hunger changes the search** (Jenson's sensed-value parameters, keyed to load in flow mode). As `Walker::food` falls, raise SO and SA, so starving mould throws out long, wide-searching fingers. A cut-off branch is then *seen hunting* before it dies. The colour change becomes optional.
   - *Where:* `move_walkers` reads `w.food`.
   - *Frame:* none.
   - *Serves:* sim, spread.
   - *Note:* real plasmodia spread wider on poor food (Dussutour 2010, cited in the earlier pass), so this is also the more truthful choice.
3. **Spread-then-prune dishes, and a "let it dry" verb** (Jones's shrinkage).
   - A dish opens with the agar flooded with mould at high p. The player places food and salt. Then the dish dries, removing walkers at a rate the player sets.
   - Slow drying leaves a clean minimal network on the food. Fast drying tears it into holes and fragments. Each result is a spread over seeds.
   - It also gives the Wanderer the migration that `wander.rs` notes is missing ("the mould in this model does not migrate, it extends").
   - *Where:* a `level.rs` dish plus a removal rate in `economy_*`.
   - *Frame:* up to p 30 to 50%, so 9k to 15k walkers. **This is a real cost.** At about 1.4 ms of simulation per 1,500 walkers it could reach 8 to 14 ms. It would need to shed walkers fast, or run at a lower step rate early on.
   - *Serves:* spread, verb.
4. **A rival mould from a second trail field.** Each strain senses own trail minus k × other trail, so borders form as visible gaps and contested food changes hands.
   - *Where:* `Dish` gets a second `trail` (and `occ` already blocks both); each walker gets a strain bit.
   - *Frame:* one more `spread` pass a step (cheap), plus one more sensor read.
   - *Serves:* fun, sim, spread.
   - Already on the design record's "left out" list, and the cheapest it will ever be.
5. **Chemorepulsion as a dying state.** When a piece's food runs dry, its walkers switch from following trail to avoiding it. Jones says this regime makes regular spots: the starving branch beads up into a row of blobs rather than fading.
   - That is a simulated, graded death, not a palette. It reads as the real organism's crust of spherules (sclerotium), which the earlier pass ranked #6.
   - *Where:* one sign in `sense` for hungry walkers.
   - *Frame:* none.
   - *Serves:* sim, spread.
   - *Speculative:* Jones showed this on open ground, not in a thin tube. It needs a picture before anything else.
6. **An emergent pulse** (Jones & Adamatzky 2012). Lower the chance that a blocked walker re-orients, and give it a short refractory hold. Jamming waves then travel through dense mould as oscillations; food strengthens them and light weakens them.
   - It would replace `Dish::beat` and both drawn pulses with a rhythm the mould actually makes.
   - It is also how a blob *migrates* (amoeboid movement), and how light cleaves a blob and nearness fuses two: three behaviours the game wants.
   - *Where:* the blocked branch of `move_walkers`.
   - *Frame:* none in itself.
   - *Serves:* sim.
   - **Highest risk on the list:** shown in a large cohesive blob, not in tube networks, and with no published period to aim for. Prototype it on a blob dish first, with a counter of wave fronts beside the picture.
7. **Food that regrows across the ground** (Cherouvim's sugarscape). Replace some fixed piles with a nutrient field that grows back slowly, eaten where walkers sit.
   - The mould grazes, a grazed patch empties and the network moves on, and regrowth draws it back. *Running Dry* and the Wanderer both want this "reason to move".
   - *Where:* a field like `scent`, paid into the economy per cell.
   - *Frame:* one field update a step (cheap).
   - *Serves:* sim, spread.
8. **A "jolt" verb** (Bleuje's wave button, Cherouvim's "additional forces"). A tap briefly raises step size and sensor reach in a radius. The network convulses and re-forms, and the player sees how robust a join was.
   - *Where:* a timed per-cell multiplier read by `move_walkers`.
   - *Frame:* none.
   - *Serves:* verb.
   - *Note:* the lamp push is already a force tool, so this is a second one, not a new kind.

## 7. Anything else the comparison turned up

- **Our population is very low by Jones's standard:** about 1% at the start, about 5 to 7% at peak, against his figures of 3% to 50%. He says population sets edge thickness and the number of holes. That is why our tubes are about 12 cells wide from SO alone, and part of why the thin far reaches are fragile. Jenson's look is impossible at 2,000 walkers. A sub-cell renderer of the trail is the only way to get some of it.
- **We sense and move in one shuffled pass; Jones separates the two stages.** In ours, later walkers in a step see this step's deposits from earlier ones. This is probably harmless, but it is a difference to remember when matching his pictures.
- **Jones's food is in the trail channel; ours is a separate `scent` field.** This is already a recorded dead-end with a reason (piles wore pale rings and counted as tube). It means Jones's Steiner/MST result transfers through `food_deposit` × `fed_scent` and `scent_keep`, not through the trail. His weight range of 0.01 to 0.1 × stimulus is in the same order as our 20 per cell per step, so the lever is in range.
- **Jones repairs damage "without recovering the original pattern".** That is a graded outcome the storm could score: how *differently* the network regrew, not only whether food stayed joined.
- **Jones models light as a multiplier below 1 on sensing**, not a subtraction. That still makes walkers surge away and cleave a blob, the same failure our `light_weight` dead-end records: "an empty stretch is a cut". So the push lamp is the right answer, and Jones's own light would not have rescued the sensor version.
- **Frame cost.** Every idea except the flooded spread-then-prune dish costs a field lookup or at most one more `spread` pass. The flooded dish multiplies walker count by 5 to 7 and is the one to measure before promising (`examples/ascii.rs` does not cover this binary; `mouldprobe` reports frame times).
