# What Jenson, Cherouvim and Jones do that our mould does not (2026-11-09)

**What was read** (third pass, after Scott uploaded both Jones papers):

- **Jones 2010 (*Artificial Life* 16(2):127-153): read in full**, from the uploaded PDF in `papers/`. I read the text extraction cover to cover and opened these PDF pages for the figures:
  - Table 1 (p. 134);
  - Figs. 10, 11 and 12, scale, population and decay (pp. 139-140);
  - Fig. 13, the SA/RA map (p. 141).

  Page numbers below are the journal's (127-153). The PDF page is the journal page minus 126.
- **Jones 2015, *Applications of Multi-Agent Slime Mould Computing* (arXiv 1511.05774, 34 pages): read in full**, from the uploaded PDF. I opened Fig. 3, the continuous SA/RA map (p. 7), as an image. Page numbers below are the preprint's own (1-34).
- **Sage Jenson's post: read in full** (second pass). That means every word and every link on cargocollective.com/sagejenson/physarum, signed February 2019. **Its five pictures still could not be viewed**, because `payload.cargocollective.com` is blocked.
- **Georgios Cherouvim: every slime-mould page read in full** (second pass). These are *Slime mold* (Jan 18), *Strange Jealousy* (Jul 18) and *Sugarscape and slime mold* (Apr 17), plus the site index. **His eight videos are on Vimeo, which is blocked.**
- **Still not read in full, because they were not among the uploads:**
  - **Jones 2011**, *Influences on the formation and evolution…* (Natural Computing). This holds the growth and shrinkage rules.
  - **Jones & Adamatzky 2012**, *Emergence of self-organized amoeboid movement…*. This holds the emergent oscillation.

  What this file says about those two comes from quoted passages via SciSpace. It is marked [X], and **no other Jones claim in this file still rests on SciSpace or on memory**. The 2015 survey confirms both papers exist and what they do (p. 5 and p. 27), but gives no numbers from them.
- **Jenson's later *36 Points***: read through Bleuje's open re-implementation on GitHub.
- **Our code**, read directly:
  - all of `src/mould/mod.rs`, re-checked this pass for `Params` and `deposit_food`;
  - `look.rs`'s drawing;
  - the doc headers and key functions of the other mould files;
  - the design record and the earlier research pass.

Marks used below:

- **[P]**: read in the uploaded paper, with the page cited;
- **[R]**: read on the source's own web page;
- **[G]**: read on GitHub;
- **[X]**: quoted passage via SciSpace (Jones 2011 and 2012 only);
- **[C]**: our code, read.

## What changed after the full read

**Third pass (the two uploaded Jones papers):**

- **Table 1 is now read** (2010 p. 134). It gives:
  - lattice 200×200;
  - %p (population, as a percentage of the lattice) 3-15;
  - diffK 3 (diffusion kernel size);
  - decayT 0.1 (trail decay);
  - wProj 0.01-0.1 (how strongly food is written into the trail);
  - boundary periodic (the edges wrap round);
  - SA "22.5 or 45 deg", RA 45, SO 9 px;
  - SW (sensor width) 1, SS (step size) 1, depT (deposit) 5;
  - pCD (chance of a random turn) 0, sMin (sensing threshold) 0.

  **Our `Params` match it in every agent value except SA.** We use 45, the table's second option. The text calls RA 45 / SA 22.5 "the default sensor parameters" (p. 135).

  We differ from his set-up in three places:
  - our boundary is a fixed rim, where his is periodic;
  - our population is about 1-7%;
  - our food smell is a separate field.
- **Our setting is Jones's "computing" setting, not his "organism" setting.** It is not a mistake, and the first two passes framed it too negatively.
  - Every one of his food-network experiments runs at SA = RA: 45/45 in 2010 (Figs. 18-21), and 60/60 with SO 7 in 2015 (Figs. 4 and 9).
  - Those runs also use **%p 1.25 to 5**, which is where we are.
  - The one setting he describes as lively is 22.5/45. There "the network never stabilizes completely" (p. 135), and the population crosses every part of the lattice within 11,000 steps (p. 137).
  - So the choice is between a mould that computes (thrift, *Beat the Ruler*) and one that lives (exploring, *Behind the Wall*, toughness). **§5.1 now proposes running both in one dish's life**, using Jones's own suggestion of slowing the model over time (p. 141).
- **The Steiner-versus-direct lever is in the 2010 paper itself**, with numbers; the first two passes had it only from Jones 2011 (pp. 145-147, Fig. 20):
  - at food weight 0.05, a ring of food "snags" a loop and stops it closing;
  - at 0.01 the loop closes;
  - at 0.5 the meeting points vanish and the network becomes a minimum spanning tree.
- **Our food already "goes quiet once engulfed", which is a Jones 2015 mechanism** (p. 10, Fig. 6). A pile already smells less as it empties (`0.3 + 0.7 × fraction`) and drops to 15% once reached (`fed_scent`) [C].
  - §5.5 of the second pass ("let smell follow food") is therefore **mostly already built, and is withdrawn**.
  - What 2015 adds, and we lack, is the *inverse* set of rules:
    - piles that smell only when touched (§7.1, p. 12);
    - piles that smell louder when uncovered (§10.2, p. 19);
    - smell set by how many tubes a pile carries (§10.1, pp. 18-19).

  These are the new §5.5 and §6.12.
- **Jones's own life-cycle reading backs "hunger changes the search", with a specific direction** (p. 142). Nutrient-rich mould is reticular, poorer mould dendritic (labyrinthine), and hostile conditions give "isolated dense patterns of the sporelike sclerotium stage". In the model those three are steps along the SA/RA diagonal, "changing the cohesion of the plasmodium". §6.3 now raises SA and RA together, not SO.
- **2015 describes the sclerotium, which gives a graded death the simulation makes itself** (§13, pp. 23-26). A drying plasmodium "abandon[s] its protoplasmic tube network to form a compact, typically circular or elliptical, toughened mass" and revives when moist conditions return. Jones models it by removing stimuli and removing walkers at random (p = 0.0005 per walker per step); the blob relaxes into a disc at the shape's centroid. That is new §6.7, and it replaces chemorepulsion as the first choice for a visible death.
- **2015 supplies five new verbs or dish mechanics**:
  - the shrink-wrap band (§7.1);
  - growth that inflates a tree and then stops by itself (§7.2);
  - obstacles that repel only once uncovered (§9);
  - pins and release, where relaxation time sets how smooth the result is (§§11-12);
  - territory borders between colonies (§8).

  These are §§6.4, 6.11, 6.13 and 6.5.
- **Corrections to the second pass:**
  - **"SO must be at least 3" is Jones 2010** (pp. 132-133), not Jones 2012.
  - **Jones's wound repair for attraction networks was run at SA 22.5 with SO 3** (Fig. 23, p. 149), not at our 45/45. How repair behaves at our setting is not in the paper.
  - **"Our population is low by Jones's standard" was half wrong.** It is low against his free-pattern runs, but matches his food-network runs (§7).
  - **The 2015 parameter map (Fig. 3) is a continuous image at SO 3**, with 181,440 walkers on 720×720 (35%). It is not the 2010 9×9 grid at SO 9.
  - **Jones himself calls putting food and walker trail in one chemical "one significant simplification"** (p. 133). Our separate scent field is the closer match to the organism, not a departure to apologise for.

**Second pass (web sources), kept:**

- Jenson's post has no multi-species section.
- He switched off one-walker-per-cell, preferring the patterns that arose without it.
- He added contagion, which is still the best candidate for a pulse the simulation makes (§5.6, §6.1).
- He frames foraging as two competing aims, "search a maximal area" and "optimize transport distance".
- Cherouvim drove his agents with optical-flow vectors taken from video, cites Jones 2011, and uses a sugarscape to make terrain, not food.

---

## Summary, in plain words

1. **Our mould runs at Jones's "computing" setting, which settles down and stops changing.**
   - We use sensor angle 45° and turn angle 45°. Jones uses that setting, or 60/60, whenever he wants the network to pull tight onto food and hold [P 2010 pp. 135-136, 144-147; 2015 p. 8].
   - It explains three things we have measured:
     - the lamp's bend springs back within 5 s;
     - a tube left alone moved 0 cells in 60 s (`organ.rs`);
     - the storm star is hard, because the network keeps no spare loops.
   - His default "organism" setting is sensor angle 22.5°. There the network "never stabilizes completely" and over time visits every part of the dish [P 2010 pp. 135, 137].
   - **The proposal is to run both in one dish**: lively and exploring early, settling as the dish ages. Jones himself suggests slowing the model over time to mimic the real mould sticking to its substrate [P 2010 p. 141].
   - It costs no frame time. It has never been tried here: the probe can sweep it, but there is no recorded run.
2. **Let something travel through the mould: Jenson's contagion.** A walker that picks up a "signal" lays signalled trail, and walkers that cross that trail catch the signal [R].
   - A food arrival, an alarm at a cut, or a heartbeat could then visibly run along the tubes, at the speed the walkers carry it, and stop dead at a gap.
   - This replaces our drawn pulse with one the simulation makes, which is what Scott asked for.
3. **Drying should make a sclerotium, not a grey fade.** Real slime mould that dries out abandons its tubes and pulls into a tough round ball, which comes back to life when wetted [P 2015 p. 23]. Jones's model does this with nothing but random loss of walkers: the mould shrinks into a disc at its own centre [P 2015 pp. 24-25].
   - For us, a cut-off branch would visibly gather itself up instead of greying out.
   - A "water" verb could revive it.
   - How much of it survives is a graded outcome.
4. **Hungry mould should change shape, in Jones's direction.**
   - Well fed, it is a mesh. Hungrier, it becomes fingers. Dying, it becomes isolated blobs [P 2010 p. 142].
   - In the model those are steps along one dial (sensor and turn angle together).
   - So hunger can be shown by the mould's own behaviour, not by tinting it.
5. **Let the player change the mould's temperament where they paint.** The same dial, applied locally [P 2010 pp. 135, 141; 2015 p. 6]. Mist for "branch out" and dry air for "pull tight" are verbs whose result is the mould's own behaviour. The outcome is graded, and nothing is drawn on.
6. **The real organism spreads out first and prunes later; ours only reaches out.**
   - Jones starts with mould covering the dish and removes walkers slowly at random. The sheet shrinks onto the food and leaves a clean network, the same one every run; removal that is too fast tears it into holes [P 2010 pp. 147-148].
   - In 2015, a shrinking blob finds a round trip of the food piles within 4-9% of the best possible [P 2015 p. 20]. Stones that push the mould away only once it has uncovered them steer it to a single clean path [P 2015 pp. 17-18].
   - For us this is a dish type, or a "let it dry" verb whose speed decides between a clean, repeatable network and a torn, varied one.
7. **Piles can talk back.** Jones's 2015 piles change their smell according to the mould:
   - louder when uncovered;
   - silent until touched;
   - quieter when they already carry too many tubes [P 2015 pp. 12, 18-19].

   We already have the first rule's opposite (a reached pile goes quiet) [C]. The others make new dish rules: a pile that "wants" exactly two tubes, for instance, steers the mould toward a ring through every pile.
8. **New hand tools from 2015**, each a real simulation response:
   - **lasso**: paint a ring and it contracts like a rubber band round the piles, lighting the ones it touches [P 2015 pp. 11-12];
   - **pins and release**: hold a tube in place, let go, and it relaxes into a smooth curve, smoother the longer you wait [P 2015 pp. 20-23];
   - **border**: two colonies meeting leave a no-man's-land line between them [P 2015 p. 15].
9. **Cheap looks from data we already have.**
   - Draw the faint trail below the tube line, to show the searching haze.
   - Draw the walkers as specks. Jones notes that a single walker moves back and forth along its tube, "strongly indicative of shuttle streaming" [P 2010 p. 137]. Drawing them would show the organism's best-known motion for free.

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

**Not in the post:** multiple species, 3D, interaction, or any image or force input.

**Jenson's later *36 Points* (2019 to 2022), read through Bleuje's port [G].** `points_basematrix.h` in `Bleuje/interactive-physarum` reproduces Jenson's parameter matrix. Every particle first senses the trail value `s` where it stands, then sets its own parameters from it:

```
sensor distance = SD0 + SDA * s^SDE
sensor angle    = SA0 + SAA * s^SAE
rotation angle  = RA0 + RAA * s^RAE
move distance   = MD0 + MDA * s^MDE
```

- There are 24 named presets, among them "pure_multiscale", "hex_hole_open", "vertebrata", "star_network", "waves_upturn", "massive_structure" and "clear_spaghetti".
- So one rule can make thin wanderers in sparse areas and tight bundles in dense ones, at the same time.
- Particles respawn at random places after a while.
- Bleuje adds:
  - blending between two presets across space;
  - interaction to move and spawn particles;
  - particle inertia;
  - a deposit taken from per-pixel particle counts.

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
- **"Strange Jealousy"** (Jul 18), a music video for Crøm-lus:
  - "The video was shot and edited as normal and then used to drive the behavior of a multi agent based system, obeying a set of simple rules and forces."
  - The patterns "evolve and shift between ambiguous shapes to the recognizable form of the two faces".
  - **"Off-line motion vectors were extracted from the video using Furnace in Nuke."**
  - References: **Jones (2011)**; Rong & Tan (2006), *Jump Flooding in GPU*; Thomas Diewald; David Reeves; Touch Designer; optical flow in Nuke.
- **"Sugarscape and slime mold"** (Apr 17):
  - "Terrain generated using a sugarscape, a cellular automata model with agents looking for sugar in the environment."
  - The mould is able "to effectively locate, migrate towards sources of food and over time optimize its complex network of interconnected branches".

**What Cherouvim does differently:**

- **External forces on the agents**: an optical-flow vector field from video, added on top of the trail sensing. Our lamp push is a force of this kind, but radial only.
- **An image as the field the agents grow towards**: the faces in *Strange Jealousy*.
- **Jones 2011's adaptive population as his reference**, so growth and shrinkage are in play.
- **A distance-transform method (jump flooding) in the toolkit.** The pages do not say what he used it for.

## 3. Jeff Jones

### 3a. Jones (2010), *Characteristics of pattern formation and evolution in approximations of Physarum transport networks*, Artificial Life 16(2):127-153

**Status: read in full from the uploaded PDF [P].**

**The agent (pp. 132-134, §2.1 and Fig. 3).**

- **One agent per lattice site.** Each is placed "at a randomly chosen unoccupied and habitable location" with a random heading from 0 to 360° (p. 132).
- **A moving agent stands for flowing sol, and a blocked one for the gel mesh**: "When a particle is not able to move, it can be said to represent the immobile gel matrix" (p. 132). This is what the one-per-cell rule buys: without it there is no gel.
- **Order of a step** (p. 133):
  - every agent tries to step forward;
  - *then* the whole population senses and turns;
  - agents are picked in random order in both stages, "to avoid the possibility of long term bias".
- **Success and failure** (p. 133):
  - a successful move deposits a constant amount;
  - a failed one deposits nothing and "a new orientation is randomly selected".
- **Sensor offset** (pp. 132-133):
  - The offset is large on purpose: it "mimics the overlapping actin-myosin mesh".
  - "The strong coupling (minimum distance of 3 pixels offset) is actually necessary for the complex behaviors to emerge."
- **One trail for food and walkers.** "One significant simplification with respect to the real organism is that both food sources and the changes in flux of internal protoplasm are represented by the same diffusing chemoattractant substance" (p. 133).
- **Forward-biased sensing ensures "a continuous dynamic"** (p. 133). Because walkers deposit only after a successful move, "static clustering of agents is avoided" (p. 134).
- **Diffusion** is "a pseudo-parallel simple mean filter in a 3 × 3 kernel that is subject to an adjustable decay value" (p. 132).

**Table 1 (p. 134), read off the page.**

| Parameter | Jones's typical value | Ours (`Params::default`, `mod.rs:283`) [C] |
|---|---|---|
| Lattice | 200×200 | about 30k agar cells inside an elliptical rim |
| %p (population, % of area) | 3-15 | about 1-7% (400 to 2,200 walkers) |
| diffK (kernel) | 3 | 3×3 mean |
| decayT | 0.1 | `keep` 0.90 |
| wProj (food weight) | 0.01-0.1 | separate scent field: `food_deposit` 20 × (0.3 + 0.7 × fullness) × `fed_scent` 0.15 once reached; `scent_keep` 0.97 |
| Boundary | **periodic** | **fixed rim**, no flux |
| SA | **22.5 or 45** | 45 |
| RA | 45 | 45 |
| SO | 9 px | 9 |
| SW (sensor width) | 1 px | point sample |
| SS (step) | 1 px | 1 cell |
| depT (deposit) | 5 | 5 |
| pCD (random turn) | 0 | none |
| sMin (sensing threshold) | 0 | none |

Notes on the table:

- **Five parameters matter most:** SA, RA and SO among the agent's, and %p and decayT among the framework's (p. 134).
- **decayT is read here as the share of trail lost per step.** Fig. 12's rows run from 0.0001 to 0.5, and a higher value is described as "reducing the strength of the gradient field" (p. 139). On that reading our `keep` 0.90 is his 0.1. The paper does not define it more precisely.

**Free pattern formation (pp. 134-138, §3).**

- **SA 22.5 / RA 45, his stated default** (pp. 135-136):
  - Small loops contract and disappear and larger holes grow. But "occasionally… a bifurcation will appear… and a new agent-chemoattractant sprout will branch out across the dividing space".
  - When a sprout crosses a gap, "there is a surge of movement as the two moving flows are connected".
  - "The network never stabilizes completely." The cause: "the rotation angle is significantly greater than the sensor angle. The wide rotation angle places the sensors away from the main gradient stream."
  - It "does not reach minimal configurations… network length is sacrificed for network connectivity".
  - Over time "every part of the lattice is traversed by the population… within 11,000 steps" (p. 137, Fig. 8).
- **SA 45 / RA 45, ours** (pp. 135-136):
  - "The bifurcation and sprouting of new trails does not persist… the lacunae gradually become larger as the smaller cycles are closed." The end state approximates "a regular hexagonal tiling".
  - It is "a self-balancing structure", "stable (although constantly turning over its components)".
- **Tension** (p. 136): "a contraction effect resembling surface tension". Closing a loop raises local flux, with a peak when the two paths merge (Fig. 6).
- **Shuttle streaming** (p. 137): a tracked agent "is hesitant and moves backward and forward… strongly indicative of shuttle streaming flow". This comes from random differences in path strength and from collisions.
- **Fixed boundaries** (pp. 136-138, §3.1):
  - Without periodic edges, 45/45 "condenses the network until all interior space is removed and a sheetlike mass remains". The sheet shows ripples and "dissipative vacancy islands", fewer and further apart as SO rises.
  - 22.5/45 "also condense[s] when boundary conditions are fixed, but… the branching activity prevents network condensation into a complete sheet".
- **Sensor offset** (p. 138, §3.2, Fig. 10):
  - SO is "a scaling factor"; SO 3 to 7 gives fine networks.
  - It is "an indirect coupling between separate agents… The stronger coupling results in coarser patterns with correspondingly large network structures and path thicknesses."
  - **"As the network scale increases, so does the speed of network evolution."**
  - At high population and large SO the pattern shifts to stripes and spots, because "most agents cannot move" (pp. 138-139, Fig. 11).
- **Population and decay** (p. 139, §3.3, Figs. 11-12):
  - A bigger population gives "thicker" boundaries and fewer holes.
  - Higher decay gives "coarser network edges".
  - "With very low decay… the attraction between the particles was stronger and network connectivity was reduced."
  - Fig. 12 (viewed): at %p 5-10 and decay 0.1 the 45/45 pattern is an open mesh of polygonal cells at 500 steps. At %p 40-60 it becomes a solid sheet with round holes.

**The SA/RA map (pp. 140-142, §3.4, Figs. 13-14).**

- The map is 9×9: SA and RA each run from 0 to 180° in 22.5° steps, on a 500×500 lattice at %p 15, SO 9, for 500 steps.
- Its edges are unpatterned for obvious reasons (SA 0 or 180, RA 0), with partial patterning along RA 180. "The inner sections of the quadrant show a gradual decrease in connectivity as both RA and SA increase."
- The classes run "reticular to labyrinthine to island-like".
- **Where we sit:** 45/45 is in the reticular block. The labyrinthine cells begin one step further along at 67.5, and SA ≥ 90 gives islands (my reading of Fig. 13).
- The pictures are snapshots. Evolution may be "complex (as in RA 45, SA 22.5 with dynamic remodeling), minimizing (as in RA 45, SA 45), or a simplification".
- **Two sentences worth building on** (pp. 141-142):
  - "This differs from Physarum, where the patterning is reduced by adhesion of the plasmodium to its substrate (although it would be fairly simple to slow the growth of the model over time to mimic this effect)."
  - "When the organism is in a nutrient-rich environment, growth is expansive and highly connected (reticular patterns). When nutrient concentrations are lower, growth is dendritic (labyrinthine patterns). Finally, in more hostile conditions (desiccated and/or exposed to light) there is a switch to isolated dense patterns of the sporelike sclerotium stage." Jones suggests nutrients might act by "changing the cohesion of the plasmodium".

**Chemorepulsion (pp. 142-144, §3.5, Figs. 15-16).**

- Agents turn *away* from the trail.
- At 45/45 with SO 40 they make honeycomb spots, each "surrounded by six weaker regions… similar to… a revolver". At RA 67.5, SA 112.5 and SO 13 they make stripes.
- Local patches compete until one orientation covers everything.

**Food (pp. 144-147, §4).**

- **How food enters the trail.** Food pixels are written into the same trail layer every step, as stimulus × weight, with the weight "typically… 0.01 to 0.1 (depending on the area of the stimulus source)". The weight sets "both the steepness and the area" of the gradient (Fig. 17).
- **Two foods, then several.** With two food points the network keeps the shortest path between them. With more points it minimises toward a Steiner tree, with three-way junctions at 120° (Plateau's laws) (p. 144). It converges on the same tree "despite often following very different dynamic graph trajectories".
- **Snagging (Fig. 18).** A small population (%p 2) gets "snagged" on food points.
- **Weight as a lever (pp. 145-147, Fig. 20; 2,000 agents, 45/45).**
  - At weight 0.05, a ring of food "cancels out the contraction force" and the loop cannot close.
  - Reducing to 0.01 lets it close: "a loosening of the tension".
  - Randomly removing agents (down to about 600) "mimic[s]… cellular apoptosis", and the network keeps "minimal connections".
  - Raising to 0.5, "tightening", removes the Steiner points and leaves a minimum spanning tree.
- **Populations in the food runs.** Figs. 18-20 use **%p 1.25, 2 and 5**.

**Shrinkage (pp. 147-148, §4.2, Fig. 21).**

- The run starts from a solid sheet: %p 50 on 300×300, 45,000 agents.
- Agents are removed at random with probability **0.00025 per agent per step**, and never re-admitted.
- Slow removal lets "particle flux… adapt in time". Faster removal makes "a tearing of the sheet and holes".
- **Three shrinkage runs gave identical networks; three condensation runs at %p 5 differed.** So shrinkage trades away variety for repeatability.

**Damage (pp. 148-150, §4.3, Figs. 22-23).**

- **How a wound is made.** Agents in a square are removed and re-seeded elsewhere, then let back in.
- **Repulsion patterns** (45/45, SO 27): the wound closes by uniform inflow, and the original pattern is restored exactly.
- **Attraction networks** (**SA 22.5, SO 3**): the wound is crossed by "a gradual insertion of branchlike processes into the wound margin". The pattern is re-established but differs from the original.

### 3b. Jones (2015), *Applications of Multi-Agent Slime Mould Computing* (arXiv 1511.05774; IJPEDS survey)

**Status: read in full from the uploaded PDF [P].** This is a survey of how the 2010 model was later used as a "virtual material" that is "programmed" by placing attractants and repellents (p. 7). What follows is what it adds.

**The model and its regimes (pp. 5-8, §§3-4).**

- **Later extensions.** The model was later given "growth and shrinkage in response to environmental stimuli" [Jones 2011 and 2015]. Its coarsening was compared against real Physarum and found to match closely.
- **Material behaviour.** The collective behaves like a soap film: Plateau angles, T1 and T2 relaxation, and von Neumann's law (p. 5).
- **"At low SA and RA values the patterns are reticulate and adaptive, constantly changing their topology. As SA and RA increase the networks undergo minimisation, reducing the number of edges. Further increases result in labyrinthine patterns and the formation of circular minimal configurations"** (p. 6).
- **Fig. 3 (p. 7, viewed)** is a continuous SA/RA sweep: SA across, RA down, 0-180°, 720×720 with 181,440 walkers (35%), SO 3, at t = 400. What the image shows:
  - fine, noisy mesh at the top-left;
  - a field of round holes (cells) through the left and centre;
  - worm-like labyrinths and stripes across the right half;
  - noise along the edges.

  It is a texture, not a labelled grid, so I cannot read an exact position off it.
- **Fig. 2 (p. 6)** repeats the 2010 default, SA 22.5 / RA 45 / SO 9 / %p 15, for "spontaneous formation and evolution".
- **Fig. 4 (p. 8)**, the standard computing demonstration, runs at **SA 60, RA 60, SO 7** with 4,000 particles. Six food nodes in a circular arena end as a Steiner minimum tree by t = 5,000. Stimuli "constrain the natural minimisation of the material, which would otherwise typically condense the initial inoculation pattern into a minimal configuration" (p. 6).

**Growth toward food, and engulfment (pp. 9-11, §6).**

- **How the real slime grows** (p. 9):
  - Nutrient-poor conditions give pseudopodia toward nearby sources.
  - Nutrient-rich conditions give radial growth "in all directions".
  - A weak stimulus from one direction gives "a conical foraging pseudopodium"; strong stimuli all round give radial growth (p. 11).
- **Engulfment** (p. 10, Fig. 6). The real slime grows toward a node, "engulfing it, and stopping (or at least reducing) the diffusion of chemoattractant from the node", then moves on to the next. The model does the same: a small population inoculated on one node grows out and builds a spanning tree, and the figure shows each node's gradient vanish as it is covered.
- **Where the mould starts decides the graph** (p. 10):
  - Inoculated at one node, the real slime builds a spanning tree and keeps adding links toward the relative neighbourhood graph.
  - Inoculated at all nodes at once, it builds the denser Gabriel graph and Delaunay triangulation, "thicker" on Gabriel edges.
- **Real slime is never finished** (p. 9). It forages on, abandons tubes ("leaving empty remnants of tubes"), sprouts from old tubes, and battles "competition and predation".

**Shape: shrink-wrap and inflate (pp. 11-14, §7).**

- **Convex hull** (pp. 11-12, Fig. 8):
  - A ring of material placed outside the points shrinks like "a ring of deformable elastic material… The minimising properties of the paths ensure that the edges of the Hull are straight and convex."
  - Points inside the hull can pull the band inward through their diffused attractant. The fix: **nodes "project stimuli only when they have been directly contacted by particles"**.
  - A side benefit: the hull's nodes end up "highlighted".
- **Concave hull by shrinking** (p. 13, Fig. 9; 18,000 particles, SA 60, RA 60, SO 7):
  - The run starts as a solid mass inside the convex hull, and the population is reduced "slowly… [to prevent] hole defects".
  - The reduction is done by random removal or "by adjusting the growth and shrinkage parameters to bias adaptation towards shrinkage".
- **Concave hull by growing** (pp. 13-14, Fig. 10):
  - Inoculate on the minimum spanning tree and bias toward growth. The model "'inflates' the MST and **automatically halts its growth (maintaining a constant population size)**", stabilising after about 10,000 steps.
  - "The stabilisation point could be altered by a parameter which effectively tuned the concavity."

**Territory: Voronoi diagrams (pp. 14-17, §8).**

- **Two real-slime methods** (p. 15):
  - Repellents placed on a grown plasmodium, with attractants round the rim.
  - Several plasmodia inoculated on rich agar. "When two or more plasmodia meet they do not immediately fuse. There is a period where the growth is inhibited… and the substrate at these positions is not occupied." Later they fuse.
- **The model** (pp. 15-17, Figs. 11-14):
  - Seven repellent sources inside, with attractant at the border, push a uniform mass into the Voronoi bisectors.
  - Merging fronts on a rich background mark the bisector only as a *denser* band, because "the model does not explicitly incorporate the inhibition at the touching growth fronts", and it fades. **A territory border needs an explicit mutual-avoidance rule.**
  - Lowering repellent strength lets minimisation "exert its influence", giving hybrid cells that wrap their sources.
  - Slowly raising it "grow[s]" cellular Voronoi diagrams.
  - Bigger repellent sources give weighted diagrams.

**Path planning (pp. 17-18, §9, Fig. 15).**

- The whole arena is filled with mould, attractant is placed at the start and the end, and the population is reduced.
- Repellent from the walls centres the path.
- **Obstacles project repellent "only when they were partially uncovered by the shrinking blob"**, which pushes the blob off them and leaves a single path.

**Feedback and the shrinking blob (pp. 18-20, §10).**

- **Node-degree feedback** (§10.1, pp. 18-19):
  - A program counts each node's tubes. "If a node had d > 2 the concentration of attractant projected at that node was reduced. If a node had d <= 2 the concentration of attractant was increased."
  - Less attractant makes the network "detach"; more attaches it and "increase[s] the tension of the material in this location".
  - The evolution was "extremely complex" and the method "blind": there was no guarantee of a valid tour.
- **Shrinking blob for the travelling salesman** (§10.2, pp. 19-20, Fig. 17):
  - The blob starts as the convex hull, and **a city's projection is "damped if the city location was completely covered"**.
  - As the blob shrinks, uncovered cities strengthen and the blob "adhere[s]" to them. "The shrinkage process was halted when all cities were uncovered."
  - The tour is the blob's outline, "between only 4% and 9% longer than optimal".

**Relaxation: smoothing and splines (pp. 20-23, §§11-12).**

- **Smoothing** (Fig. 18):
  - Material is laid on a jagged line and held by attractant. When released it relaxes into the moving average.
  - **"The amount of smoothing… was dependent on the length of time that the material adapted for"**: a kernel of 19 by t = 189, and of 79 by t = 979. The material also shortens.
- **Low-pass filtering** (Fig. 19). Keeping a *weak* copy of the original stimulus (0.255) gives a low-pass filter instead: the material clings to steady stretches and lets go of sharp turns, without shortening.
- **Splines** (Figs. 20-21):
  - Material laid on a polyline is released, with attractant kept at the two ends to "clamp" it. It relaxes into a B-spline.
  - Longer relaxation matches a higher spline degree: degree 2 at t = 436, degree 5 at t = 1,788.
  - Closed shapes can be clamped at a single point.

**Sclerotium (pp. 23-26, §13).**

- **Biology** (p. 23):
  - Entry is "provoked by adverse environmental conditions, particularly by a gradual reduction in humidity".
  - "The mass of plasmodium aggregates together, abandoning its protoplasmic tube network to form a compact, typically circular or elliptical, toughened mass… re-entering the plasmodium stage when moist conditions return."
  - Computationally it is "freezing or halting".
- **Model** (pp. 24-26, Figs. 22-24):
  - Material is held in a shape by attractant for 50 steps, then the stimulus is removed, and **particles are removed at random at p = 0.0005**. The run stops below 50 particles.
  - The blob relaxes toward a disc that stays on the shape's centroid: within about 2 px on a lizard shape, worse on strongly concave shapes (error up to 10.5 px).
  - On a jagged 1D line it smooths first, then shrinks from both ends to the mean.

**Halting, oscillation and other remarks (pp. 27-28, §14).**

- A computation can be judged finished "when a stable pattern state is reached… Another indicator… is when the population size stabilises."
- "A simple change in particle movement was sufficient to reproduce the emergence and dynamical transitions of oscillation patterns", and those oscillations "could be harnessed to generate travelling waves… enabling collective transport and amoeboid movement" [Jones & Adamatzky 2012; Tsuda, Jones & Adamatzky 2012]. **No numbers are given here**; the details stay [X] below.
- Particles "repelled by chemoattractant" also make complex patterns, and agents with more or fewer sensors are an open field.

**Real-slime results it cites, usable as game behaviour (pp. 2-3)** [P, as reported by Jones; the primary papers were not read]:

- **"Strong long-distance attractant stimuli combined with short-distance repulsive stimuli"** make the plasmodium follow "attractor cycles around simple stimuli and limit-cycle motion with more complex stimuli" (Adamatzky 2011, herbal tablets).
- **Under simulated contamination** (salt diffusing in a region), "the plasmodium migrated away from contaminated regions… before re-establishing network connectivity when the damaged areas were contamination-free" (Adamatzky & Jones 2010, motorways).
- **Fine early networks give way to "thicker and more sparse"** later ones (Strano et al. 2011, Roman roads).
- **Chaotic oscillation helps the plasmodium avoid deadlock** (Aono et al.).

### 3c. Jones 2011 and Jones & Adamatzky 2012 (not uploaded; [X], quoted passages via SciSpace)

**Growth and shrinkage (Jones 2011) [X].**

- **Division.** If an agent has *just moved successfully* and its 9×9 window holds 1 to 10 particles, it divides into an empty neighbouring site.
- **Removal.** If its 5×5 window holds more than 20 particles, it is removed.
- Both tests run every 3 steps.
- Removal under crowding "keeps network paths thin". The adaptive population "prevents snapping".
- Food weight 0.005 gives Steiner trees; weight 5 favours direct connections. This agrees in direction with 2010 Fig. 20 [P].

**Emergent oscillation (Jones & Adamatzky 2012) [X].**

- Oscillation emerges from non-oscillating parts. Disordered local domains compete and then lock into travelling waves.
- Lowering `pID` to 0.01 "restricts particle movement fluidity" and strengthens the oscillation. I take `pID` to be the chance that a blocked agent re-orients; that is not confirmed.
- The blob migrates.
- Food strengthens the oscillation and light weakens it. Light there is sensitivity × a weight below 1.
- A band of light cleaves a blob, and nearby blobs fuse.

---

## 4. Our model against theirs

| Mechanism | Jones 2010 [P] / 2015 [P] / 2011-12 [X] | Jenson (post [R], 36 Points [G]) | Cherouvim [R] | Ours (`src/mould/`) [C] |
|---|---|---|---|---|
| Sensor angle SA | Table 1: 22.5 or 45; text default 22.5 (2010 p. 135); computing runs use 45 (2010) or 60 (2015) | Configurable; in 36 Points, set from the sensed trail | Implements Jones | **45°** fixed, not on the T panel |
| Rotation RA | 45 (Table 1); 60 in 2015 computing runs | Configurable | | 45° |
| Sensor offset SO | 9 (Table 1); scale and coupling reach; ≥ 3 needed; bigger evolves faster (2010 pp. 132, 138) | "Sensor distance" | | 9; tubes about 12 cells wide (`TUBE_WIDTH`) |
| Sensor width | 1 px | "Sensor size" | | Point sample |
| One walker per cell | **Yes**; a blocked walker is "gel" (2010 p. 132) | **Usually off**; "crucial for exact mimicry… conservation of matter" | Not stated | **Yes** (`occ`) |
| Deposit | 5, only on a successful move | "Deposition amount", "deposit size" | | 5, only on a successful move ✓ |
| Diffuse / decay | 3×3 mean, decayT 0.1 | 3×3 mean + multiplicative decay | | 3×3 mean, keep 0.90 ✓ |
| Update order | All move, then all sense; random order each stage (2010 p. 133) | Parallel | GPU | Shuffled; sense and move in one pass |
| Population | %p 3-15 typical; 1.25-5 in food runs; 50 for shrinkage | 5-10 M particles | 300k at 60 fps | **%p ≈ 1-7** |
| Boundary | **Periodic** by default; fixed boundaries condense 45/45 to a sheet (2010 p. 136) | Not stated | | Fixed elliptical rim |
| Food | Written into the trail, wProj 0.01-0.1; weight sets Steiner (0.01) or MST (0.5) (2010 pp. 144-147) | Not discussed | Sugarscape terrain | Separate scent field; fades with fullness and drops to 15% once reached |
| Engulfment | Covered node stops smelling (2015 p. 10) | | | **Yes**, via `fed_scent` ✓ |
| Node rules | Smell only when touched; louder when uncovered; set by degree (2015 pp. 12, 18-19) | | | None |
| Repellent | Salt-like sources push; obstacles repel only when uncovered (2015 pp. 15-18) | | | Salt is a wall |
| Signal spreading | None | **Contagion** | | None simulated; pulse is a clock or drawn |
| External forces | Light lowers sensitivity [X] | | **Optical-flow vectors** | Lamp **push** (radial) |
| Growth | 9×9 holds 1-10 *and* just moved [X]; biasable to self-halting growth (2015 p. 13) | | Cites 2011 | Chance 0.03/step if fed and 5×5 ≤ 7; paid from food |
| Removal | 5×5 > 20 [X]; random 0.00025 (2010) or 0.0005 (2015) for shrinkage | | Cites 2011 | Starvation only |
| Drying / death | Sclerotium: shrink to a disc, revive when moist (2015 p. 23) | | | Grey fade, then gone |
| Oscillation | Emerges from jamming [X; 2015 p. 27] | | | Global sine clock in flow mode; drawn band otherwise |
| Rendering | Grayscale trail or particle dots | Trail, millions of particles | Real-time particles | Trail cut at 0.5×`tube`; walkers never drawn |

**Where we sit on Jones's map.**

- **Fig. 13:** SA = RA = 45, SO 9 is the reticular block. Its text calls it the "minimizing" regime, ending in a hexagon-like mesh that holds still while its walkers turn over (2010 pp. 135-136, 141).
- **The table differences** (fixed rim, %p 1-7, separate scent) put us in Jones's *food-network* experiments, not his free-pattern ones. That is Figs. 18-21: 45/45, SO 9, %p 1.25-5, with boundaries.
- **What his food runs predict for us:**
  - the network snags on food;
  - it minimises toward a Steiner tree or a spanning tree, depending on food strength;
  - it converges on the same shape from different starts.
- **What we have measured matches that:**
  - 0 cells of change over 60 s;
  - the lamp bend gone within 5 s;
  - thrift easy and toughness hard.
- **What Jones does not cover.** He never shows a fixed-rim, food-free 45/45 run at our low %p. At %p 15-20 it becomes a sheet. Below that, our own pictures are the evidence.

**What moving each lever should do, by Jones's own words** (none of this is measured on our dish yet):

| Move | Expected pattern | Source | On screen and in play |
|---|---|---|---|
| SA 45 → 22.5 (RA 45) | Never settles; sprouts cross holes; whole dish visited over time; loops kept | 2010 pp. 135-137 | Busier network, more loops, finds hidden food; worse thrift |
| SA and RA both up (60 → 67.5 → 90+) | Minimising → labyrinthine → islands | 2010 p. 141; 2015 p. 6 | Wormy fingers, then separate blobs: the "hungry" and "dying" looks |
| SO 9 → 5 | Finer, slower-evolving network | 2010 p. 138 | More, thinner tubes; every width-calibrated constant moves |
| SO 9 → 15 | Coarser, faster-evolving | 2010 p. 138 | Fewer, fatter tubes that re-route quicker |
| More walkers | Thicker edges, fewer holes; very high gives a sheet with holes | 2010 p. 139, Fig. 12 | Fuller look; cost rises linearly |
| Slower decay | Stronger attraction, less connectivity | 2010 p. 139 | Clumpier |
| Food weight down | Loops close; Steiner meeting points | 2010 pp. 145-146 | *Beat the Ruler*'s goal |
| Food weight up | Steiner points vanish; spanning tree | 2010 p. 147 | Straight spokes |
| Random removal, slow | Sheet shrinks onto food; same network every run | 2010 pp. 147-148 | Clean, repeatable |
| Random removal, fast | Sheet tears into holes | 2010 p. 148 | Varied, ragged |
| One-per-cell off (Jenson) | Walkers stack; thin bright filaments | Jenson [R] | Jenson-like look; no "gel"; breaks `occ`, the blocked-move rule and the growth window |

`mouldprobe sensor_angle=… rotate_angle=… sensor_offset=…` already sweeps the first four rows. The design record holds no run of it.

**Where the earlier research pass (`mould-game-brainstorm-2026-11-08/research.md`) went shallow.**

- It never placed our settings on Jones's map.
- It read no text from any source.
- Its "visible heartbeat" was drawn-only, and Scott rejected it.
- It missed Jenson's contagion and his dropped collision rule.
- It did not look at Cherouvim.
- It did not know the 2015 applications: engulfment, the node rules, the sclerotium, and relaxation.

---

## 5. Improvements to what we already have

Ranked by value against risk.

- **Frame** is the risk to the frame budget. Today the mean frame is 3.3 ms under the old rule and 5.0 ms in flow mode; the worst frames are 8.8 to 17 ms (design record).
- **Serves** names Scott's rules:
  - **sim**: everything on screen is simulated;
  - **fun**: fun over fidelity;
  - **spread**: a graded outcome, not a yes/no;
  - **verb**: a player action with a visible result.

1. **Put SA, RA, SO and pCD on the T tuning panel, and run SA 22.5 against 45 as a paired A/B with pictures. Then try "lively early, settled later".**
   - Jones's default organism is 22.5/45. His computing runs are 45/45 or 60/60. He suggests slowing the model over time to mimic substrate adhesion (2010 p. 141).
   - So the variant to try is **SA easing from 22.5 to 45 as the dish ages**, or as each patch of mould ages. It explores and finds food first, then pulls tight, which is Jenson's "search a maximal area" then "optimize transport distance".
   - *Where:* rows in `tune.rs`; `Params`; a per-dish age term in `move_walkers`; one random-turn roll for pCD.
   - *Frame:* none.
   - *Serves:* fun, spread.
   - *Note:* if the default moves, re-derive every constant calibrated at 45/45:
     - `TUBE_WIDTH` 12;
     - `OATS_PULL` 28 / `OATS_MAYBE` 36;
     - `THRIFT_BAR` 1.15;
     - the level star bars;
     - the `STORM_CUTS` dead-end's re-test condition.
   - **This is the cheapest high-value experiment here.**
2. **A switch for the one-walker-per-cell rule (Jenson's departure)**, as a look experiment.
   - Jones gives the rule a meaning: a blocked walker *is* the gel (2010 p. 132). Turning it off removes the gel, the "most agents cannot move" spot regime (p. 138), and our blocked-move turn.
   - *Where:* `occ` handling in `move_walkers`, `push_light` and births.
   - *Frame:* none.
   - *Serves:* fun.
   - **A look experiment, not a default.** Show it on an A/B page first.
3. **Jones's crowding removal** [X, 2011]: a walker whose 5×5 window holds more than N others dies, and its food goes to its neighbours.
   - *Where:* `economy_flow` and `economy_by_piece`.
   - *Frame:* about the cost of the existing `crowd` test.
   - *Serves:* sim, fun.
   - *Expected:* thinner, even tubes and no fat blob at piles. `table` and `eat` will need re-tuning.
4. **Grow only after a successful move**, with a sparse-window test like Jones's (9×9 window holding 1-10) [X, 2011], in place of "5×5 ≤ 7".
   - *Where:* the births loops; a per-walker `moved` bit.
   - *Frame:* none.
   - *Serves:* sim, spread.
   - *Expected:* growth at moving fronts; breaks repaired by growth.
5. **Pile rules beyond engulfment** (2015 §§7.1, 10). We already have engulfment, since a reached pile drops to 15% and a pile smells less as it empties [C], and 2015 p. 10 confirms that this is Jones's mechanism. Add one optional rule per dish:
   - **(a) Degree feedback.** A pile carrying more than two tubes smells less; one carrying two or fewer smells more (2015 pp. 18-19). Tubes then detach from busy piles and tighten onto lonely ones. Jones calls the result "extremely complex", which is good for watching and bad for guaranteeing anything, so use it as a dish twist, not a solver.
   - **(b) Louder when uncovered.** A pile covered all round goes quiet; one at the mould's edge is loud (2015 p. 19). This keeps an engulfing blob from sitting on its food, and pulls it out to the rim.
   - *Where:* `deposit_food`, plus a degree count, which `census` already has the pieces for.
   - *Frame:* the degree count needs the tube graph once a second or so, not every step.
   - *Serves:* sim, spread.
6. **Replace the pulse with contagion** (Jenson).
   - Piles mark the walkers that load food. Marked walkers lay a "signal" trace in a short-lived field, and walkers that cross the trace catch the mark.
   - The beat then travels at the walkers' own speed, thins along long tubes, and stops at a cut.
   - It is drawn as brightness on the marked walkers and their trace.
   - *Where:* a mark bit on `Walker`, one more fast-decaying field, and a catch test in `move_walkers`.
   - *Frame:* one more `spread` pass on a small field, plus a lookup per walker.
   - *Serves:* **sim**.
   - It retires §5.8's first two drawn-only items. The playable version is §6.1.
7. **Show more of the simulation the drawing already holds.**
   - (a) Draw the trail between about 0.1 and 0.5×`tube` faintly, on the same fixed ramp.
   - (b) Draw the walkers as moving specks. Jones's tracked walker shuttles back and forth (2010 p. 137). Specks would show shuttle streaming, the real organism's signature motion, plus the real flux surge when a sprout joins or a loop closes (pp. 135-137). Today that surge is a drawn "snap".
   - (c) In flow mode, make a speck's brightness its load.
   - *Where:* `look.rs::dish`.
   - *Frame:* (a) close to free; (b) about 2,000 pixel writes.
   - *Serves:* sim, fun.
   - *Note:* (a) changes what counts as an "edge" cell, so check the 1-in-6 edge-cell count.
8. **The drawn-only effects that remain**, flagged against Scott's rule:
   - the default tube pulse, `look.rs::pulse` and `PULSE_*`;
   - `organ.rs`'s pulse, a snapshot riding `heart_geodesic`;
   - the global clock `Dish::beat` in flow mode, which moves real food but on an imposed rhythm;
   - the join "snap" burst and the pile-arrival flash, which mark real events but are drawn graphics. Jones's surge on joining (2010 p. 135) is what §5.7(b) would show instead.

## 6. New ideas

1. **Signals the player can send through the mould (contagion as a verb).** Jenson's contagion [R], made into a tool: tap a walker or a pile to "infect" it.
   - **Alarm**: infected walkers pull back.
   - **Feast**: infected walkers grow more freely.
   - **Tempo** (for the organ): the heart's beat is a contagion wave, so a bell rings when the signal actually arrives.
   - The outcome is graded: how far and fast a signal spreads depends on the network built.
   - *Where:* the §5.6 machinery.
   - *Frame:* small.
   - *Serves:* verb, sim, spread.
2. **Temperament as a verb: a local SA/RA field the player paints.** Mist branches the mould out; dry air pulls it tight.
   - Jones: RA > SA keeps branching (2010 p. 135). Raising both moves toward labyrinths and islands (p. 141; 2015 p. 6).
   - *Where:* a per-cell field read by `move_walkers`, and a new `Tool`.
   - *Frame:* a lookup per walker, plus a decay pass.
   - *Serves:* verb, sim, spread.
   - This is not the reverted sensor-light lamp: it changes how a walker turns, never what it senses.
3. **Hunger changes the shape, along Jones's life-cycle axis.** As `Walker::food` falls, raise SA and RA together, from a mesh, to fingers (labyrinthine), to isolated blobs (2010 p. 142).
   - A starving branch visibly changes kind before it dies.
   - This revises the second pass, which raised SO; Jones ties nutrient level to *cohesion*, which is the angle pair.
   - *Where:* `move_walkers`.
   - *Frame:* none.
   - *Serves:* sim, spread.
4. **Spread-then-prune dishes, and a "let it dry" verb.** This draws on Jones 2010 §4.2, 2015 §§7.2, 9 and 10.2, and Jenson's "area reduction".
   - **The dish:**
     - It opens with mould covering the agar.
     - The player places food and sets the drying speed.
     - **Slow drying gives the same clean network every run; fast drying tears it, differently each time** (2010 p. 148). That is the spread rule, as a dial.
   - **Variants straight from 2015:**
     - **Round trip**: piles go quiet while covered and loud when uncovered, and drying stops itself once every pile is uncovered. The blob's outline is then a tour within 4-9% of the best (p. 20). A natural sibling to *Beat the Ruler*.
     - **Bitter stones**: stones under the mould are inert until uncovered, then repel, pushing the shrinking mould to a single path (pp. 17-18).
     - **Inflate**: start the mould as a thin tree between piles and bias toward growth. It fills out and stops growing by itself (pp. 13-14).
   - It also gives the Wanderer the migration that `wander.rs` lacks.
   - *Where:* a `level.rs` dish, plus a removal rate in `economy_*`.
   - *Frame:* **the real cost on this list.** %p 30-50 means 9k to 15k walkers. At about 1.4 ms per 1,500 walkers that is 8 to 14 ms. It needs fast early shedding, or a lower step rate early on. Jones's own 2015 hull run used 18,000 particles (Fig. 9).
   - *Serves:* spread, verb.
5. **A rival mould, and the border between colonies.**
   - Real plasmodia meeting on rich agar "do not immediately fuse": growth stops, a strip stays empty, and later they merge (2015 p. 15).
   - Jones's model lacks that inhibition, so its border is only a fading dense band (p. 16). **So a territory line needs an explicit rule: each strain senses its own trail minus k × the other's.**
   - Optionally, fuse after a delay.
   - *Where:* a second `trail`; a strain bit on `Walker`.
   - *Frame:* one more `spread` pass.
   - *Serves:* fun, sim, spread.
6. **A drag force** (Cherouvim's optical-flow forces). A stroke pushes trail and walkers *along* its direction.
   - *Where:* like `push_light`, with a direction field from the stroke.
   - *Frame:* as the lamp.
   - *Serves:* verb.
7. **Drying into a sclerotium: a graded death and a revival** (2015 §13).
   - **The rule:** when a piece is cut off from food, or its ground dries, stop paying upkeep deaths at the normal rate. Instead, shed walkers slowly at random (Jones uses p = 0.0005 per walker per step). With no food anchoring it, the piece minimises by itself and gathers into a disc at its own centre (pp. 24-25).
   - **A "water" verb revives it.** The ball sends out tubes again, as the real organism re-enters the plasmodium stage when moist (p. 23).
   - **Graded:** how big the ball is when water arrives decides how much comes back.
   - **The fallback:** chemorepulsion beads, where a starving branch turns away from trail and breaks into spots (2010 pp. 142-144). Show both on one page.
   - *Where:* `economy_*` (death mode), and a `Tool`.
   - *Frame:* none.
   - *Serves:* sim, spread, verb.
8. **Food that regrows across the ground.** This is our extension of Cherouvim's sugarscape, not his.
   - Rich ground should make the mould grow radially, and poor ground make it send directed fingers (2015 pp. 9-11).
   - *Where:* a field like `scent`.
   - *Frame:* one field update a step.
   - *Serves:* sim, spread.
9. **An emergent pulse from jamming** [X, 2012; confirmed in outline, 2015 p. 27].
   - *Where:* the blocked branch of `move_walkers`.
   - *Frame:* none.
   - *Serves:* sim.
   - **Highest risk on the list**: it was shown in blobs, not tubes. Contagion (§6.1) covers the pulse more cheaply, so this matters mainly for migration.
10. **A jolt** (Bleuje's wave button). A tap briefly raises step size and sensor reach in a radius.
    - Jones: a larger SO also makes the network evolve faster (2010 p. 138), so the jolt shows up as faster re-routing, not only as a shake.
    - *Where:* a timed per-cell multiplier.
    - *Frame:* none.
    - *Serves:* verb.
11. **Lasso** (2015 §7.1).
    - The player paints a closed ring of mould round a group of piles. It contracts like a rubber band, and its edges straighten by the model's own minimisation.
    - Piles smell only once the band touches them. Those it catches light up and hold it, so the final shape is the hull of what was caught.
    - **Graded:** piles near the band can pull it into a dent.
    - *Where:* a stroke-to-walkers spawn, plus a per-pile "touched" flag in `deposit_food`.
    - *Frame:* walkers only.
    - *Serves:* verb, sim, spread.
12. **Pins and release** (2015 §§11-12).
    - The player sets pins, small attractant points, that hold a tube in a shape. Releasing a pin lets the tube relax.
    - **How smooth it gets depends on how long it is left**: a kernel of 19 by step 189 and of 79 by step 979 in Jones. A weak leftover pin gives a low-pass curve that keeps its long sweeps and drops its kinks.
    - This makes our lamp snap-back (gone in 5 s) into a deliberate tool. Pins at the ends clamp it.
    - *Where:* pins as tiny piles with no food.
    - *Frame:* none.
    - *Serves:* verb, sim.
13. **Seeding decides the network's kind** (2015 p. 10).
    - A spore placed on one pile grows a sparse tree first. Spores on every pile at once grow the denser, triangulated network.
    - That gives a free level lever: "one spore" versus "a spore per pile" dishes.
    - *Where:* `level.rs`.
    - *Frame:* none.
    - *Serves:* spread.
14. **An orbiting lure, and salt that washes out** (real-slime results cited in 2015 pp. 2-3; not modelled in the papers read).
    - **Orbiting lure:** a lure that attracts from far but repels up close should make the mould circle it. That is real motion from simulation.
    - **Spreading salt:** salt that diffuses and slowly washes out should make the mould retreat and then reconnect once the ground is clean. That is graded, unlike today's salt wall.
    - Both are speculative until pictured.
    - *Where:* a scent with a negative core; a decaying salt field.
    - *Frame:* one field each.
    - *Serves:* sim, spread, verb.

## 7. Anything else the comparison turned up

- **Population.** Our %p of 1-7 is low against Jones's free-pattern runs (3-15 typical), but **inside the range he uses with food (%p 1.25-5, 2010 Figs. 18-20)**. That is the regime our dishes are in. The second pass's "low by Jones's standard" was too strong.
- **Tube width comes from SO, not from walker count.** SO is the coupling reach (2010 p. 138), so at SO 9 tubes are about 12 cells wide whatever the walker count. Fig. 10 shows SO 3 to 25 at 45/45.
- **Boundary.** Jones's default edges wrap round; ours are a fixed rim. Under fixed edges, food-free 45/45 condenses into a sheet at %p 15-20 (2010 p. 136). With food and low %p, it minimises onto the food instead.
- **Jenson's look is not reachable at 2,000 walkers with one per cell.** The two ways toward it are the one-per-cell switch (§5.2) and drawing the faint trail (§5.7).
- **We sense and move in one shuffled pass; Jones moves everyone, then senses** (2010 p. 133). It is probably harmless, but worth remembering when matching his pictures.
- **Jones calls putting food in the trail channel "one significant simplification"** (2010 p. 133). Our separate scent field is a recorded fix (piles wore pale rings and counted as tube), and it is closer to the organism. His Steiner/MST weight lever reaches us through `food_deposit` × `fed_scent`. Because the fields differ, his numbers do not transfer; only the direction does.
- **Engulfment is already ours** (2015 p. 10 against `deposit_food` [C]). We arrived at it independently: the comment on `fed_scent` says that without it "the mould never explores".
- **Shrinkage trades away variety for repeatability** (2010 p. 148: three runs, identical networks). Under Scott's spread rule, a spread-then-prune dish should expose the drying speed, so that the player chooses between a sure result and a varied one.
- **Repair at our setting is unmeasured.** Jones's attraction-network repair was at SA 22.5, SO 3 (2010 p. 149). The storm could score how *differently* the mould regrew, but our own pictures at 45/45 are the evidence, not his.
- **A "settled" signal for levels**: Jones calls a computation finished when the pattern is stable or "the population size stabilises" (2015 p. 27). Both are cheap to read, and could end a dish or award a star, rather than a timer.
- **Real slime leaves "empty remnants of tubes"** (2015 p. 9). Our `wall` memory field (flow level 3) is the same idea, and it could be drawn as faint husks: real state, not decoration.
- **Jones's light is a multiplier below 1 on sensing** [X, 2012]. It still makes walkers surge away, which was the `light_weight` dead-end's failure. The push lamp stays the right answer.
- **Cherouvim lists jump flooding**, a GPU distance-transform method. It gives straight-line distance, not distance through a tube, so it cannot replace `organ.rs`'s `heart_geodesic`.
- **Frame cost.** Every idea except the flooded dish (§6.4) costs a field lookup, a flag, or at most one more `spread` pass. The flooded dish multiplies the walker count by 5 to 7, and is the one to measure before promising (`mouldprobe` reports frame times). §5.5(a)'s degree count should run on a timer, not every step.
