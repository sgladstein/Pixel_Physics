# What Jenson, Cherouvim and Jones do that our mould does not (2026-11-09)

**What was read** (second pass, after network access was granted):

- **Sage Jenson's post: read in full.** Every word of the text and every link on the page (cargocollective.com/sagejenson/physarum, signed February 2019). **Its five pictures could not be viewed.** The image host `payload.cargocollective.com` is still blocked, so I have not seen the post's diagram of "the six sub-steps of a simulation tick", nor `web.jpg`, `network_2.gif`, `ef_9.gif` or the contagion GIF `explosion.gif`.
- **Georgios Cherouvim: every relevant page read in full.** Jenson links Cherouvim's front page (`http://ch3.gr/`) and speaks of his "posts". So I read the site index and every slime-mould page on it:
  - *Slime mold* (dated "Jan 18" on the page);
  - *Strange Jealousy* ("Jul 18");
  - *Sugarscape and slime mold* ("Apr 17").

  The site's own search for "slime" returns only *Slime mold*. **His videos could not be watched**: all eight are on Vimeo, which is still blocked.
- **Jones 2010: still not read cover to cover, and I did not try to get round what stopped it.**
  - On `uwe-repository.worktribe.com`, both the paper's record page and its PDF answer HTTP 403 with a Cloudflare challenge page ("Just a moment…").
  - The PDF link that Jenson and Cherouvim both give, `eprints.uwe.ac.uk/15260/1/artl.2010.16.2.pdf`, is on a host this session does not allow.
  - What I have is quoted passages of the full text from SciSpace's extraction tool, over eleven targeted questions: the model rules, the SA/RA map (figure 13), network evolution, population and decay, the sensor-offset passages verbatim, stimuli, chemorepulsion, damage repair, shrinkage and the discussion.
  - **Table 1 (the default numbers) never came back**, despite three questions aimed at it, so defaults marked [K] are still from memory.
  - To close this gap: allow `eprints.uwe.ac.uk`, or have someone pass the UWE challenge in a browser and drop the PDF into the project files.
- **Jones 2011** (*Influences on the formation and evolution…*, Natural Computing; Cherouvim's own reference) and **Jones & Adamatzky 2012** (*amoeboid movement*): passages extracted the same way. These hold the growth and shrinkage rules and the emergent oscillation.
- **Jenson's later *36 Points***: read through Bleuje's open re-implementation on GitHub (its README and full parameter matrix).
- **Our code**, read directly:
  - all of `src/mould/mod.rs` (the model, economy and flow);
  - the drawing in `look.rs` (the `dish` function, the pulse, the starving looks);
  - the doc headers and key functions of `render.rs`, `play.rs`, `level.rs`, `wander.rs`, `organ.rs`, `tune.rs` and `lesson.rs`;
  - the design record and the earlier research pass.

  I did not read every line of the 3,800-line `look.rs` or of `play.rs`; most of both is HUD and bots.

Marks used below: **[R]** read on the source's own page; **[X]** quoted passage of a paper's full text via SciSpace; **[G]** read on GitHub; **[S]** search summary only; **[C]** our code, read; **[K]** my own memory, not checked.

## What changed after the full read

- **Jenson's post has no multi-species section.** The first pass guessed there was one; that guess is withdrawn. "Species that follow their own trail and avoid others' trails" is a convention of later implementations. The rival-mould idea (§6.5) stays, on its own merits.
- **Jenson made two changes to Jones's model that the first pass missed entirely, and both matter to us:**
  - **He switched off Jones's rule of one walker per cell**, "preferring the patterns that arose without it". That is a large part of why his pictures look the way they do. He also says the rule is "crucial for exact mimicry" because "it approximates a sort of conservation of matter". We keep the rule. See §5.2.
  - **He added contagion.** An infected particle lays down infected trail, and that trail infects other particles, so a change of behaviour spreads through the mould by the mould's own movement. **This is the best candidate yet for a pulse the simulation actually makes**, and it is lower risk than Jones's jamming oscillation. It is now §6.1, and it changes §5.6.
- **The page's parameter list is longer than the search summaries showed.** It adds "deposit size" and "diffuse size": how big the deposit footprint and the blur kernel are, as well as how strong.
- **Jenson frames foraging as two competing aims**: "to search a maximal area" and "to optimize transport distance", achieved "through the mechanisms of growth, movement, and area reduction". Our mould has growth and movement but no area reduction. That supports the spread-then-prune idea (§6.4).
- **Cherouvim, read in full, is more specific than the summaries were:**
  - *Slime mold* says "Implementation based on this paper", linking Jones 2010.
  - *Strange Jealousy* says what the "video feed" was: **motion vectors extracted offline from the footage (optical flow, with Furnace in Nuke) acting as forces on the agents**, so the pattern drifts until it forms two faces.
  - His reference list is **Jones 2011**, the paper with the growth and shrinkage rules, plus jump flooding (a GPU distance-transform and Voronoi method).
  - In *Sugarscape and slime mold*, the sugarscape **generates the terrain**. Nothing says the mould eats it. So "food as a landscape that regrows" (§6.8) is our extension of his idea, not his.
- **Jones's sensor-offset passages, now quoted verbatim, say *why* SO sets tube width**: it is the reach of the coupling between walkers (§3).
- **Unchanged:** the core finding (we sit in the corner of Jones's map that minimises and then stops changing), the Jones column of the comparison table, and the order of §5, apart from the contagion insert.

---

## Summary, in plain words

1. **Our mould is set to the one corner of Jones's map that settles down and stops changing.**
   - We run "sensor angle 45°, turn angle 45°". In Jones's paper that setting closes its small loops, pulls tight into a few simple paths, and then holds still [X].
   - That explains three things we have already measured: the lamp's bend springs back within 5 s; a tube left alone moved 0 cells in 60 s (`organ.rs`); and the storm star is hard because the network keeps no spare loops.
   - Changing one number (sensor angle to 22.5°) gives Jones's other main setting: a network that keeps branching, keeps rebuilding itself, and keeps its spare loops [X].
   - **On screen:** a mould that stays busy when left alone. **In play:** loops survive the storm more often, but networks run longer.
   - It costs no frame time, and it has never been tried: the probe can already sweep it, but there is no recorded run.
2. **Let something travel through the mould: Jenson's contagion.** A walker that picks up a "signal" lays signalled trail, and walkers that cross that trail catch the signal [R]. A food arrival, an alarm at a cut, or a heartbeat could then visibly run along the tubes at the speed the walkers themselves carry it, and stop dead at a gap. This replaces our drawn pulse with one the simulation makes, which is what Scott asked for.
3. **Let the player change the mould's temperament where they paint.** Jones shows that the ratio of sensor angle to turn angle decides between "pull tight" and "branch out" [X]. A mist or warmth tool that shifts those angles locally is a verb whose result is the mould's own behaviour. The answer is graded, and nothing is drawn on.
4. **Hungry mould should search differently, not just turn grey.** Starving walkers reach wider and turn harder, so a branch that has been cut off is seen fanning out to hunt before it dies.
5. **Jones keeps tubes thin by removing walkers where they crowd, and grows new walkers only where they are moving forward** [X]. Cherouvim cites exactly this paper [R]. We have the growth half, loosely, and none of the crowding half. Adding it should give thinner, more even tubes, stop the fat blob round every pile, and repair a break by growing into it.
6. **The real organism spreads out first and prunes later; ours only reaches out.**
   - Jones's "shrinkage" run starts with mould covering the whole dish and removes walkers slowly at random. The sheet shrinks onto the food and leaves a clean network; removal that is too fast tears it into holes [X].
   - Jenson names "area reduction" as one of the organism's three mechanisms [R].
   - For us this could be a dish type, or a "let it dry" verb whose speed decides between a clean network and a torn one.
7. **How strongly food smells decides the network's shape.** Faint food gives meeting points in empty space (Steiner trees); strong food gives direct spokes [X, Jones 2011]. That is exactly what *Beat the Ruler* asks the player to build. Tie the smell strength (`fed_scent`) to how full a pile is.
8. **Cheap looks from data we already have.**
   - Draw the faint trail below the tube line, so you can see where the mould is searching.
   - Draw the walkers as moving specks, so you can see the streaming.
   - Jenson's look comes from a continuous trail field, millions of particles, and no one-per-cell limit [R]. Ours is a thresholded silhouette from about 2,000 walkers.
9. **Let the player's hand be a force field, as in Cherouvim's work.** His agents are pushed by motion vectors taken from video [R]. Our lamp push is already one such force. A "drag" or "breath" tool that pushes along the direction of the stroke is the obvious second one.

---

## 1. Sage Jenson, "physarum" (cargocollective.com/sagejenson/physarum)

**Status: text read in full [R]. The five images could not be loaded.**

What the page says, in order:

- **Why Jenson did it.** "This February I spent a bit of time simulating slime mold… I saw some incredible posts by Georgios Cherouvim with reference to a 2010 paper by Jeff Jones… When I read the paper, I was excited to learn that the model combined continuum and agent-based simulation systems in a way that I hadn't seen before." Cherouvim's name links to `http://ch3.gr/`; the paper links to `eprints.uwe.ac.uk/15260/1/artl.2010.16.2.pdf`.
- **The biology framing.** "There are two requirements of efficiency in foraging behavior: 1.) to search a maximal area and 2.) to optimize transport distance." Physarum "excels at these two competing tasks through the mechanisms of growth, movement, and area reduction."
- **The model.** There is "an agent-based layer (the data map) and a continuum-based layer (the trail map)". "The particles of the data map deposit material onto the trail map, while those same particles sense values from the trail map in order to determine aspects of their locomotion."
  - Each particle has "a heading angle, a location, and three sensors (front left, front, front right)", and rotates left or right or keeps straight on.
  - "A simple 3-by-3 mean filter is applied to simulate diffusion of the particle trail, and then a multiplicative decay factor is applied to simulate trail dissipation over time."
  - A diagram shows "the six sub-steps of a simulation tick". I could not load it.
- **Parameters.** "Many of the parameters of this simulation are configurable, including sensor distance, sensor size, sensor angle, step size, rotation angle, deposition amount, decay factor, deposit size, diffuse size, decay factor, etc."
  - The post gives **no numerical values**.
- **His departures from Jones.**
  - **"In Jones's original paper there is a collision detection step that ensures that there is at most one particle in each grid square. For my implementations I usually ignored this step, preferring the patterns that arose without it. However, this step is crucial for exact mimicry of the behavior of Physarum polycephalum, as it approximates a sort of conservation of matter."** Dropping it also "removes any sort of sequential dependence", which helps the GPU.
  - **"I also added a contagion behavior that alters properties of the simulation. When a particle is infected, it lays down an infected trail which then spreads to other particles."** A GIF shows contagion spreading from the centre of the image. I could not load it.
- **Implementation.** C++ and GLSL in openFrameworks, with all computation and rendering on the GPU. It runs in real time on a GTX 1070, "with most of the examples comprised of between 5 and 10 million particles".
- He hopes "to post a CPU version… soon". No such version is linked.

**Not in the post:** multiple species, 3D, interaction, or any image or force input. The first pass's guesses about these are withdrawn.

**Jenson's later *36 Points* (2019 to 2022), read through Bleuje's port [G].** `points_basematrix.h` in `Bleuje/interactive-physarum` reproduces Jenson's parameter matrix. Every particle first senses the trail value `s` where it stands, then sets its own parameters from it:

```
sensor distance = SD0 + SDA * s^SDE
sensor angle    = SA0 + SAA * s^SAE
rotation angle  = RA0 + RAA * s^RAE
move distance   = MD0 + MDA * s^MDE
```

- There are 24 named presets ("pure_multiscale", "hex_hole_open", "vertebrata", "star_network", "waves_upturn", "massive_structure", "clear_spaghetti" and others).
- So one rule can make thin wanderers in sparse areas and tight bundles in dense ones, at the same time.
- Particles respawn at random places after a while.
- Bleuje adds blending between two presets across space, interaction to move and spawn particles, particle inertia, and a deposit taken from per-pixel particle counts.

**What Jenson does that we do not:**

- no one-per-cell limit, so particles stack up;
- contagion: a state spread by trail and by particles;
- sensor size, deposit size and kernel size as parameters;
- fractional step sizes;
- millions of particles, with the trail drawn directly;
- in 36 Points, parameters that depend on the sensed value.

## 2. Georgios Cherouvim (ch3.gr)

**Status: all three slime-mould pages read in full [R]. The videos (Vimeo) could not be watched.**

- **"Slime mold"** (Jan 18):
  - "An agent based feedback loop system that simulates the behavior of Physarum polycephalum (slime mold) and the natural process of network optimization. Implementation based on this paper", linking Jones 2010.
  - "At later iteration it is also combined with additional forces and a video feed to affect its growth and behavior. Rendered in realtime (300k partilces @60fps) using GLSL compute shaders within Touch Designer."
  - Three videos, which I could not watch.
- **"Strange Jealousy"** (Jul 18), a music video for Crøm-lus. This is the fullest account of the method:
  - "The video was shot and edited as normal and then used to drive the behavior of a multi agent based system, obeying a set of simple rules and forces. The most characteristic behavior is the slime mold… which creates a distinct network of interconnected branches."
  - The patterns "evolve and shift between ambiguous shapes to the recognizable form of the two faces".
  - It was built in TouchDesigner, with GLSL compute shaders, "and can also be performed in real time". **"Off-line motion vectors were extracted from the video using Furnace in Nuke."**
  - References: **Jones (2011)**, *Influences on the formation and evolution of Physarum polycephalum inspired emergent transport networks*; Rong & Tan (2006), *Jump Flooding in GPU with Applications to Voronoi Diagram and Distance Transform*; Thomas Diewald; David Reeves; Touch Designer; optical flow in Nuke.
  - Screened at Ars Electronica 2019 and ÉCU 2019.
- **"Sugarscape and slime mold"** (Apr 17):
  - "Terrain generated using a sugarscape, a cellular automata model with agents looking for sugar in the environment. Based on Growing Artificial Societies."
  - The slime mould is again "a multi agent based system, obeying a set of simple rules and forces". It is able "to effectively locate, migrate towards sources of food and over time optimize its complex network of interconnected branches, to effectively transport the nutrients over its constantly changing body".
  - Same references as above, plus "simpler cellular automata systems" (Game of Life renders), and 3D renders.
  - "This research led to the creation of this music video." It was commissioned by Agnieszka Kurant.

**What Cherouvim does differently:**

- **External forces on the agents**: an optical-flow vector field from video, added on top of the trail sensing. Our lamp push is a force of this kind, but radial only.
- **An image as the field the agents grow towards**: the faces in *Strange Jealousy*.
- **Jones 2011's adaptive population as his reference**, so growth and shrinkage are in play.
- **A distance-transform method (jump flooding) in the toolkit.** The pages do not say what he used it for.

## 3. Jeff Jones (2010), *Characteristics of pattern formation and evolution in approximations of Physarum transport networks*, Artificial Life 16(2):127-153

**Status: passages of the full text via SciSpace, not a cover-to-cover read.** The UWE copy refuses this session with a Cloudflare challenge, and the eprints host is not allowed (see the top). SciSpace record: <https://scispace.com/papers/characteristics-of-pattern-formation-and-evolution-in-3sr6e5wv1h>.

**The agent [X].**

- Each lattice site holds at most one agent.
- Every scheduler step, every agent first tries to step forward along its heading; then the whole population senses.
- **If the step succeeds** (the next site is empty), the agent moves and deposits a constant amount of attractant.
- **If it fails**, the agent stays put, deposits nothing, and picks a random new heading.
- Agents are picked in **random order** in both the motor and the sensory stage, "to avoid long-term bias".
- The trail is then diffused by "a pseudoparallel simple mean filter in a 3 x 3 kernel" with "an adjustable decay value".
- There is also a "passive" variant, in which agents sense but do not deposit.
- The parameters that matter most are SA (sensor angle), RA (rotation angle), SO (sensor offset), p (the population, as a percentage of sites) and the decay.
- The habitat is a grayscale image whose values mark habitable ground, obstacles and food.
- *Table 1's defaults, from memory [K], still unverified:* SA 22.5°, RA 45°, SO 9 px, sensor width 1, step 1, deposit 5, decay 0.1, chance of a random turn 0, sensitivity threshold 0. If that is right, our defaults are Jones's except that our SA is 45°.

**Sensor offset, verbatim [X].**

- "The parameter SO… acts as a scaling factor. Smaller SO values exhibit a more fine-grained structure, while large values show wider patterns and coarser network paths… SO distances of 3, 9, 15, and 25 pixels."
- "The distance from the agent location to the position of the sensors reflects an indirect coupling between separate agents. When SO is small, the agent receives sensory input from the chemical cues of only nearby agents and the coupling is weak. For large SO the increased distance represents a strong coupling between distant agents. The stronger coupling results in coarser patterns with correspondingly large network structures and path thicknesses."
- "Very small sensor offset distances (SO 3 to 7) result in fine-grained network formation."
- Jones 2012 adds: "a minimum distance of 3 pixels is required for strong local coupling".

**The pattern map [X].**

- Figure 13 varies SA and RA each from 0 to 180° in 22.5° steps, at 500 steps a run. The pictures are snapshots: "the patterns continue to evolve".
- Evolution "may remain disordered…, complex (as in RA 45, SA 22.5 with dynamic remodeling), minimizing (as in 45, SA 45), or a simplification of the initial patterning".
- "When both parameters are increased together there is a trend from reticular to labyrinthine to island-like patterns"; "low values show higher connectivity, whereas higher values result in more isolated patterns".
- Jones 2011 [X]: RA = SA contracts the network; RA < SA "increases" contraction; RA > SA gives "spontaneous branching", because "the larger rotation angle places particle sensors outside the chemoattractant trail path".

**How networks evolve [X].**

- **SA 45 / RA 45.** "Bifurcation and sprouting of new trails do not persist… lacunae gradually become larger as smaller cycles are closed", ending in "a simple network" that approximates "a regular hexagonal tiling". It is "a self-balancing structure".
- **SA 22.5 / RA 45.** "Does not reach minimal configurations… network length is sacrificed for network connectivity." With fixed boundaries the network still condenses, but branching stops it collapsing into a solid sheet.
- **Population.** A bigger population gives thicker edges and fewer lacunae (the holes between paths).
- **Decay.** Faster decay gives coarser edges. Very slow decay gives stronger attraction and *less* connectivity.

**Single agents [X].** Jones followed one agent for 3,000 steps. It shows "hesitation" and moves back and forth along its path, "strongly indicative of shuttle streaming flow", from random differences in path strength and from collisions.

**Quasi-physical behaviour [X].**

- Tension-like effects.
- Minimisation, with junctions settling to three-way nodes at 120° (Plateau's laws).
- Repair. Attraction networks repair "by interdigitating migration… gradually reestablishing the network, but without recovering the original pattern". Repulsion patterns close the wound and restore the original pattern.

**Food and other stimuli [X].**

- Food is written into the *same* trail layer every step, as stimulus × weight, with the weight "typically 0.01 to 0.1". It then diffuses like the trail.
- A stronger food weight pulls the network from a Steiner tree (meeting points in empty space) toward a minimum spanning tree (direct links between foods).
- Jones 2011: a weight of 0.005 gives Steiner trees, and a weight of 5 favours direct connections (20 runs × 10,000 steps at each).
- With more food nodes, "cyclic areas are less likely to be closed".

**Chemorepulsion [X].** Agents that turn *away* from the trail make "regular periodic arrays of spots" and stripes. Local patches of pattern compete until one covers the whole area.

**Shrinkage [X].**

- The run starts from a solid sheet: p 50 on a 300x300 lattice, 45,000 agents.
- Agents are removed at random, with probability **0.00025 per agent per step**, and never re-admitted.
- Slow removal lets "particle flux… adapt", and the sheet "maintains a minimal area as it shrinks". Three runs gave identical networks.
- Faster removal tears the sheet into holes.

**Growth and shrinkage (Jones 2011) [X].**

- **Division.** If an agent has *just moved successfully* and there are 1 to 10 particles in the 9x9 window round it, it divides into an empty neighbouring site.
- **Removal.** If its 5x5 window holds more than 20 particles, it is removed.
- Both tests run every 3 steps.
- Growth therefore happens at the edges, and where food stimulates movement. Removal under crowding "keeps network paths thin".
- The adaptive population "prevents snapping": new particles repair a break.

**Emergent oscillation (Jones & Adamatzky 2012) [X].**

- Oscillation emerges from parts that do not oscillate. Disordered local domains compete and then lock into travelling waves.
- Lowering `pID` to 0.01 "restricts particle movement fluidity" and makes the oscillations much stronger. I believe [K] `pID` is the chance that a blocked agent re-orients.
- The incompressible blob migrates (amoeboid movement).
- Food strengthens the oscillation and simulated light weakens it. Light there is implemented as sensitivity × a weight below 1.
- A band of light cleaves a blob in two; nearby blobs fuse.
- A small SO fragments the collective.

**Briefly, other Jones work [K].** *From Pattern Formation to Material Computation* (Springer 2015) adds growth from a single inoculum, food that is "engulfed" (its stimulus is suppressed once covered), and wave transport of objects. *Towards Physarum Engines* (Tsuda, Jones & Adamatzky 2012) pushes objects with oscillating domains [X, abstract].

---

## 4. Our model against theirs

| Mechanism | Jones 2010/11/12 | Jenson (post [R], 36 Points [G]) | Cherouvim [R] | Ours (`src/mould/`) [C] |
|---|---|---|---|---|
| Sensor angle SA | Swept 0-180°. Default 22.5° [K]. 45 minimises | Configurable; in 36 Points, set from the sensed trail | Implements Jones | **45°** fixed (`Params::sensor_angle`), not on the T panel |
| Rotation RA | Swept 0-180°; 45 typical | Configurable | | 45° fixed |
| Sensor offset SO | Scale factor and coupling reach: 3/9/15/25 shown; 3 is the minimum for cohesion | "Sensor distance" | | 9, which gives tubes about 12 cells wide (`TUBE_WIDTH`) |
| Sensor width | Not recovered | "Sensor size" | | Point sample |
| Step | One site | "Step size" | | One cell |
| One walker per cell | **Yes**: the collision step | **Usually off**, by preference; "crucial for exact mimicry… conservation of matter" | Not stated | **Yes** (`occ`) |
| Deposit | Constant, only on a successful move | "Deposition amount" and "deposit size" | | 5, only on a successful move ✓ |
| Diffuse / decay | 3x3 mean + decay | 3x3 mean + multiplicative decay; "diffuse size" configurable | | 3x3 mean, keep 0.90, no flux at walls ✓ |
| Update order | Random; motor and sensory stages separate | Parallel (no collision step, so no order) | GPU | Shuffled; sense and move in one pass |
| Population | p as % of sites; 3 to 50% shown | 5-10 M particles | 300k at 60 fps | 400 to about 2,200 on about 30k agar cells: **p ≈ 1 to 7%** |
| Boundary | Periodic or fixed; fixed boundaries condense | Not stated | | Fixed rim (ellipse) |
| Food | Written into the trail layer, weight 0.01-0.1; weight sets Steiner vs MST | Not discussed | A sugarscape makes the terrain | Separate **scent** field (keep 0.97), 20/cell/step, 15% once eaten (`fed_scent`); fixed piles |
| Signal spreading | None | **Contagion**: infected trail infects particles | | None simulated. The pulse is a clock (`Dish::beat`) or drawn (`look.rs::pulse`) |
| External forces | Light lowers sensitivity | | **Optical-flow vectors from video**; "additional forces" | Lamp **push** (radial); salt is a wall |
| Growth | Every 3 steps: 9x9 holds 1-10 *and* just moved, then divide | Not discussed | Cites Jones 2011 | Chance 0.03/step if fed and its 5x5 holds ≤ 7; paid from food; no "moved" condition |
| Removal | 5x5 holds > 20 → removed; random removal for shrinkage | | Cites Jones 2011 | Starvation only (`starve`, `hungry`) |
| Oscillation | Emerges from jamming (2012) | | | Global sine clock in flow mode; a drawn band otherwise |
| Memory | Trail only | | | `wall` field (flow level 3) |
| Rendering | Grayscale trail or particles | Trail map, millions of particles, stacked | Real-time particles | Trail cut at 0.5×`tube`, log ramp, emboss and glow; walkers never drawn |

**Where we sit on Jones's map.** SA = RA = 45°, SO 9, a low population and a fixed boundary: **the corner that minimises and then balances itself**. Our own record shows that corner's signature three times:

- The organ probe measured **0 cells of tube-length change over 60 s** untouched, and a lamp bend was "gone within 5 s of letting go on 8 of 8 seeds" (`organ.rs`).
- Toughness is the hardest star, because minimising throws away the redundant loop that would survive a cut.
- Thrift is reachable, because the mould minimises by itself.

**What moving each lever should do, by Jones's own words** (none of this is measured on our dish yet):

| Move | Expected pattern | On screen and in play |
|---|---|---|
| SA 45 → 22.5 (RA 45) | Dynamic remodelling: constant branching, loops kept | Busier network, more loops, more storm survival, worse thrift |
| RA 45 → 22.5 (SA 45) | RA < SA: more contraction | Tighter, straighter trunks that snap back faster |
| Both up (67.5 and beyond) | Reticular → labyrinthine → islands | Wormy labyrinths, then separate blobs |
| SO 9 → 5 | Weak coupling, so a fine network | More, thinner tubes; every width-calibrated constant moves |
| SO 9 → 15 | Strong coupling, so coarse ropes | Fewer, fatter tubes |
| More walkers (p up) | Thicker edges, fewer holes | Fuller look; simulation cost rises linearly |
| Slower decay | Stronger attraction, less connectivity | Clumpier |
| Food weight down once eaten | Steiner meeting points | *Beat the Ruler*'s goal |
| Food weight up | Direct spokes | Straight lines from food to food |
| One-per-cell rule off (Jenson) | Walkers stack, thin bright filaments | A Jenson-like look; breaks `occ`, the blocked-move rule and the growth window |

`mouldprobe sensor_angle=… rotate_angle=… sensor_offset=…` already sweeps the first five rows. The design record holds no run of it.

**Where the earlier research pass (`mould-game-brainstorm-2026-11-08/research.md`) went shallow.**

- **It never placed our settings on Jones's map**, nor suggested the one-number change that flips the behaviour.
- **It read no text from any source**: only search summaries and abstracts. It missed the SA/RA results, the growth and shrinkage rules, the effect of food weight, and the shrinkage and repulsion experiments.
- **Its "visible heartbeat" was drawn-only.** It was built, and then rejected by Scott. Jones & Adamatzky 2012's emergent oscillation was cited but not followed. Jenson's contagion, a cheaper simulated signal, was not found at all.
- **It concluded that art versions get their beauty "from particle count and colour ramps, not new rules", then proposed no rendering change.** It also missed that Jenson's look comes partly from *dropping* a rule, the one-per-cell limit.
- **It did not look at Cherouvim.**
- **Most of its ranked borrows have since been built** (wall memory, the pulse). So the next layer has to come from the model's own dynamics.

---

## 5. Improvements to what we already have

Ranked by value against risk.

- **Frame** is the risk to the frame budget. Today the mean frame is 3.3 ms under the old rule and 5.0 ms in flow mode; the worst frames are 8.8 to 17 ms (design record).
- **Serves** names Scott's rules:
  - **sim**: everything on screen is simulated;
  - **fun**: fun over fidelity;
  - **spread**: a graded outcome, not a yes/no;
  - **verb**: a player action with a visible result.

1. **Put SA, RA and SO (and a new random-turn chance) on the T tuning panel, and run SA 22.5° against 45° as a paired A/B with pictures.**
   - *Where:* rows in `tune.rs`; `Params`; one random-turn roll in `move_walkers`.
   - *Frame:* none.
   - *Serves:* fun, spread.
   - *Note:* if the default moves, re-derive the constants calibrated at 45/45: `TUBE_WIDTH` 12, `OATS_PULL` 28 / `OATS_MAYBE` 36, `THRIFT_BAR` 1.15, the level star bars, and the `STORM_CUTS` dead-end's re-test condition ("if the network gets denser").
   - **This is the cheapest high-value experiment here.**
2. **A switch for the one-walker-per-cell rule (Jenson's departure).** With it off, walkers may stack up to some cap per cell, and the look moves toward Jenson's.
   - *Where:* `occ` handling in `move_walkers`, `push_light` and births.
   - *Frame:* none.
   - *Serves:* fun.
   - *Note:* Jenson says the rule stands for conservation of matter, and our economy leans on it in three places: the blocked-move turn, the `crowd` window, and the measured feeding bands. **So this is a look experiment, not a default.** Show it on an A/B page first.
3. **Jones's crowding removal**: a walker whose 5x5 window holds more than N others dies, and its food goes to the walkers near it.
   - *Where:* `economy_flow` and `economy_by_piece`.
   - *Frame:* small, about the cost of the existing `crowd` test.
   - *Serves:* sim, fun.
   - *Expected:* thinner, more even tubes, and no fat blob at piles. `table` and `eat` will need re-tuning.
4. **Grow only after a successful move**, with a sparse-window test like Jones's 9x9 window holding 1 to 10, in place of "5x5 ≤ 7".
   - *Where:* the births loops; plus a per-walker `moved` bit set in `move_walkers`.
   - *Frame:* none.
   - *Serves:* sim, spread.
   - *Expected:* growth at the moving fronts, and breaks repaired by growth.
5. **Let a pile's smell follow its food** (Jones's projection weight): `fed_scent` set from the pile's remaining fraction, or from its eaters against `table`.
   - *Where:* `deposit_food`.
   - *Frame:* none.
   - *Serves:* sim, spread.
   - *Expected:* a full pile pulls direct spokes; a busy or dwindling one lets the network relax toward Steiner meeting points.
6. **Replace the pulse with contagion** (Jenson). Today the flow-mode beat is a global sine clock, and the default pulse is drawn on.
   - Piles mark the walkers that load food. Marked walkers lay a "signal" trace in a short-lived field, and walkers that cross the trace catch the mark.
   - The beat then travels at the walkers' own speed, thins out along long tubes, and stops at a cut.
   - It is drawn as brightness on the marked walkers and their trace. That is real state, not a band computed from distance.
   - *Where:* a mark bit on `Walker`, one more field with fast decay, and a catch test in `move_walkers`.
   - *Frame:* one more `spread` pass on a small field, plus a lookup per walker. Small.
   - *Serves:* **sim**.
   - It retires §5.8's first two drawn-only items. See §6.1 for the playable version.
7. **Show more of the simulation the drawing already holds.**
   - (a) Draw the trail between about 0.1 and 0.5×`tube` faintly, on the same fixed ramp: the searching haze, which is real data cut off today.
   - (b) Draw the walkers as moving specks.
   - (c) In flow mode, make a speck's brightness its load.
   - *Where:* `look.rs::dish`.
   - *Frame:* (a) close to free; (b) about 2,000 pixel writes.
   - *Serves:* sim, fun.
   - *Note:* (a) changes what counts as an "edge" cell; check the 1-in-6 edge-cell count.
8. **The drawn-only effects that remain**, flagged against Scott's rule:
   - the default tube pulse, `look.rs::pulse` and `PULSE_*` (a band computed from BFS distance and time);
   - `organ.rs`'s pulse, a snapshot riding `heart_geodesic`;
   - the global clock `Dish::beat` in flow mode, which moves real food but has an imposed rhythm;
   - the join "snap" burst and the pile-arrival flash, which mark real events but are drawn graphics.

## 6. New ideas

1. **Signals the player can send through the mould (contagion as a verb).** Jenson's contagion [R], made into a tool. Tap a walker or a pile to "infect" it with a signal, which then travels by the walkers' own movement.
   - **Alarm**: infected walkers pull back. The mould retreats from a threat along its own tubes, fastest along thick trunks.
   - **Feast**: infected walkers grow more freely. A growth spurt spreads outward from the food.
   - **Tempo** (for the organ): the heart's beat is a contagion wave, so a bell rings when the signal actually arrives through the tube. That is no longer a snapshot.
   - The outcome is graded: how far and how fast a signal spreads depends on the network the player built, and a gap stops it.
   - *Where:* the §5.6 mark bit, the signal field and the catch rule, plus one effect per signal type on `move_walkers` or births.
   - *Frame:* small.
   - *Serves:* verb, sim, spread.
2. **Temperament as a verb: a local SA/RA field the player paints** (mist to branch out, dry to pull tight). The rule is Jones's (RA > SA branches, RA < SA contracts); the machinery is like 36 Points' per-walker parameters.
   - *Where:* a per-cell field read by `move_walkers`, and a new `Tool`.
   - *Frame:* a lookup per walker, plus a decay pass.
   - *Serves:* verb, sim, spread.
   - *Note:* this is not the reverted sensor-light lamp. It changes how a walker turns, never what it senses.
3. **Hunger changes the search.** As `Walker::food` falls, raise SO and SA, so starving mould throws out long, wide-searching fingers.
   - *Where:* `move_walkers`.
   - *Frame:* none.
   - *Serves:* sim, spread.
4. **Spread-then-prune dishes, and a "let it dry" verb** (Jones's shrinkage; Jenson's "area reduction").
   - The dish opens with mould covering the agar. The player places food and salt, then sets the drying speed. Slow drying leaves a clean network; fast drying tears it.
   - It also gives the Wanderer the migration `wander.rs` says is missing ("the mould in this model does not migrate, it extends").
   - *Where:* a `level.rs` dish, plus a removal rate in `economy_*`.
   - *Frame:* **the real cost on this list.** p 30 to 50% means 9k to 15k walkers. At about 1.4 ms of simulation per 1,500 walkers that is 8 to 14 ms. It would need fast shedding, or a lower step rate early on.
   - *Serves:* spread, verb.
5. **A rival mould from a second trail field.** Each strain senses its own trail minus k × the other's.
   - *Where:* a second `trail`; a strain bit on `Walker`.
   - *Frame:* one more `spread` pass.
   - *Serves:* fun, sim, spread.
   - This is a convention of later implementations, not of Jenson's post.
6. **A drag force** (Cherouvim's optical-flow forces). A stroke pushes trail and walkers *along* its direction, as the lamp pushes them outward. The mould can then be combed and swept, not only shoved.
   - *Where:* the same shape as `push_light`, with a direction field taken from the stroke.
   - *Frame:* as the lamp.
   - *Serves:* verb.
7. **Chemorepulsion as a way to die.** When a piece runs dry, its walkers avoid trail instead of following it. Jones says this regime makes regular spots, so a starving branch beads up rather than fading.
   - *Where:* one sign in `sense`.
   - *Frame:* none.
   - *Serves:* sim, spread.
   - *Speculative:* it needs a picture first.
8. **Food that regrows across the ground.** This is our extension of Cherouvim's sugarscape terrain, not his. A nutrient field grows back slowly and is eaten where walkers sit.
   - *Where:* a field like `scent`, wired into the economy.
   - *Frame:* one field update a step.
   - *Serves:* sim, spread.
9. **An emergent pulse from jamming** (Jones & Adamatzky 2012). Blocked walkers rarely re-orient, so waves of jamming travel through dense mould. Food strengthens them and light weakens them; blobs migrate, cleave and fuse.
   - *Where:* the blocked branch of `move_walkers`.
   - *Frame:* none.
   - *Serves:* sim.
   - **Highest risk on the list**: it was shown in blobs, not in tubes. Since contagion (§6.1) covers the pulse more cheaply, this matters mainly for migration.
10. **A jolt** (Bleuje's wave button). A tap briefly raises step size and sensor reach in a radius, and the network convulses and re-forms.
    - *Where:* a timed per-cell multiplier.
    - *Frame:* none.
    - *Serves:* verb.

## 7. Anything else the comparison turned up

- **Our population is low by Jones's standard**: about 1 to 7% of sites, against 3 to 50% in his figures. His SO passage explains our tube width. SO is the reach of the coupling, so at SO 9 tubes run about 12 cells wide whatever the number of walkers.
- **Jenson's look is not reachable at 2,000 walkers with one per cell.** The two ways toward it are the one-per-cell switch (§5.2) and drawing the faint trail (§5.7). Neither is the walker count.
- **We sense and move in one shuffled pass; Jones separates the two stages.** It is probably harmless, but worth remembering when matching his pictures.
- **Jones puts food in the trail channel; ours is a separate `scent` field.** That is a recorded dead-end: piles wore pale rings and counted as tube. So his Steiner/MST lever reaches us through `food_deposit` × `fed_scent`, and his weight range sits in the same order as ours.
- **Jones's attraction networks repair "without recovering the original pattern".** The storm could score how *differently* the mould regrew, which is a graded outcome.
- **Jones's light is a multiplier below 1 on sensing.** It still makes walkers surge away and cleave a blob: the `light_weight` dead-end's failure ("an empty stretch is a cut"). The push lamp stays the right answer.
- **Cherouvim lists jump flooding**, a GPU distance-transform method. It gives straight-line distance, not distance through a tube, so it cannot replace `organ.rs`'s `heart_geodesic`. It is noted only because the organ probe found the four-neighbour feeding search blind to a tube's shape.
- **Frame cost.** Every idea except the flooded dish (§6.4) costs a field lookup or at most one more `spread` pass. The flooded dish multiplies the walker count by 5 to 7, and is the one to measure before promising (`mouldprobe` reports frame times).
