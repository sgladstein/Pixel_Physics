# A Builder's Review of Ant and Superorganism Computational Simulation: Mechanisms, Models, and Parameters for Realistic Colony Games

> **Provenance.** External document, written by a reviewer who knew only that
> this project is an ant-based simulation game and surveyed the current
> literature. Reproduced here verbatim as received on 2026-09-19 so that the
> repo's own review of it ([`ant-sim-research-review-2026-09-19.md`](ant-sim-research-review-2026-09-19.md))
> can cite it by line. Nothing in it has been checked against the engine;
> that is the review's job. **The copy received ends mid-sentence in §9
> (*Metabolic scaling*)** — the remainder, including the Tables 1–4 the TL;DR
> refers to, had not arrived when this was saved.

## TL;DR
- **Build stigmergy-first:** the single most important design decision for a realistic ant sim is to make colony behavior *emerge* from individuals following simple local rules on a shared, evaporating pheromone field — never script it top-down or use A* pathfinding for foraging. The canonical mechanisms (Deneubourg's nonlinear trail-choice, response-threshold task allocation, quorum-based nest choice, path integration, stigmergic construction) are all cheap to implement and well-validated against real colonies.
- **Species-specific parameters vary enormously and are the key to realism:** trail-pheromone half-lives range from seconds (fire ants) to minutes (Pharaoh ants ~9 min) to days/weeks (army ants); walking speeds range from ~1–3 cm/s (*Lasius niger*) to 0.855 m/s (Saharan silver ant). A Pharaoh-ant colony, a leaf-cutter, and a desert ant should behave like different games (Tables 1–4 below).
- **Most existing games (Empires of the Undergrowth, SimAnt, GPU hobbyist sims) get trail-following roughly right but omit the mechanisms that create depth and realism:** multiple pheromone types (alarm, "no-entry"/negative), quorum decisions, metabolic scaling, path integration, and topochemical stigmergic construction. That gap is exactly where realism gains lie.

## Key Findings
- **Stigmergy (Grassé) plus self-organization (the Brussels school of Deneubourg, Goss, Aron, Pasteels) is the theoretical core.** The double-bridge experiments (Goss, Aron, Deneubourg & Pasteels 1989; Deneubourg et al. 1990) show a colony selects the shorter path purely via trail deposition/following governed by a nonlinear ("Deneubourg") choice function — the foundational algorithm for any ant sim.
- **Pheromone dynamics are the heart of realism** and are best implemented as per-type scalar fields with deposition, diffusion, and species-specific evaporation. Real ants use multiple channels, including a **negative/"no-entry" pheromone** (Robinson, Jackson, Holcombe & Ratnieks 2005, *Nature*) that prevents runaway positive feedback.
- **Navigation** in non-trail species (desert ants) is dominated by **path integration** (a running home vector) plus visual route memory, with mature computational-neuroscience models of the insect central complex (Stone et al. 2017; Sun et al. 2020) that can be approximated cheaply.
- **Division of labor** emerges from **response-threshold models** (Bonabeau, Theraulaz) and **interaction-rate models** (Gordon's harvester-ant work), not from assigned jobs.
- **Collective decisions** (nest-site selection in *Temnothorax*) run on **quorum sensing** with a tunable speed-accuracy tradeoff (Pratt, Franks, Sumpter, Mallon).
- **Collective transport and self-assembly** (crazy-ant load hauling, fire-ant rafts, army-ant bridges) are increasingly modeled as active-matter physics and add spectacular emergent behavior.
- **Modern validation** rests on automated tracking (Mersch/Keller barcode tracking; Kronauer's clonal raider ant), enabling data-driven, calibrated models.

## Details

### 1. History and Evolution of the Field
The conceptual foundation is **stigmergy**, introduced by Pierre-Paul Grassé (1959) to explain termite construction: individuals modify the environment, and those modifications stimulate further work, coordinating activity indirectly. For a realistic ant sim this is the single most important concept.

The self-organization paradigm was crystallized by the Brussels school around **Jean-Louis Deneubourg**, with Simon Goss, Serge Aron, and Jacques Pasteels. Their **double-bridge experiments** with the Argentine ant (Goss, Aron, Deneubourg & Pasteels 1989, *Naturwissenschaften* 76:579–581; Deneubourg et al. 1990) showed a colony collectively selects the shorter of two paths purely through trail-pheromone deposition and following, [ResearchGate](https://www.researchgate.net/figure/Experimental-setup-for-the-double-bridge-experiment-a-Branches-have-equal-length-b_fig1_273737865) described by a nonlinear choice function. **Beckers, Deneubourg & Goss (1992, 1993)** quantified trail-laying in *Lasius niger* and showed modulation by food quality. **Franks, Gomez, Goss & Deneubourg (1991)** modeled army-ant raid patterns ("the blind leading the blind").

The field was consolidated in two landmark books: **Bonabeau, Dorigo & Theraulaz, *Swarm Intelligence: From Natural to Artificial Systems* (1999)** and **Camazine, Deneubourg, Franks, Sneyd, Theraulaz & Bonabeau, *Self-Organization in Biological Systems* (2001)**. Major research lineages: Guy Theraulaz and colleagues (Toulouse) on construction; Nigel Franks (Bristol) and Stephen Pratt on collective decision-making; Deborah Gordon (Stanford) on interaction-rate task allocation; Iain Couzin and Simon Garnier on collective motion and living architecture; Rüdiger Wehner (Zurich) on desert-ant navigation; Barbara Webb, Thomas Stone, Antoine Wystrach, and Paul Graham on neural navigation models; and Laurent Keller (Lausanne) and Daniel Kronauer (Rockefeller) on automated tracking. The trajectory has moved from minimal analytical models toward data-driven, empirically-validated agent-based models and, recently, GPU-scale simulation and neural/ML approaches.

### 2. Modeling Approaches and Tradeoffs
- **Agent-based / individual-based models (ABM/IBM):** Each ant is an agent with internal state and local sensing. The most natural fit for a game and dominant in modern realistic biology (Pratt et al. 2005 for nest choice; Khuong et al. 2016 for construction). Best emergent realism; cost scales with agent count.
- **Cellular automata (CA):** Space discretized into a grid; ants and pheromone updated by local rules. Efficient; heavily used in ant-traffic models (Chowdhury, Nishinari, Schadschneider). Good for pheromone fields and dense traffic.
- **Continuum / PDE and reaction-diffusion models:** Ant density and pheromone concentration as continuous fields (deposition, diffusion, evaporation, advection). Excellent for large-scale trail-network dynamics and analytically tractable (the pitchfork bifurcation underlying binary-bridge choice), but lose individual identity.
- **Mean-field / ODE models:** Track population fractions (ants per branch, workers per task). Cheap; good for colony demographics, poor for spatial realism.
- **Network models:** Interactions or trail topology as graphs; used for social-network analysis (Mersch et al. 2013) and trail geometry.
- **Hybrid (recommended for games):** agents for visible ants, a grid/field for pheromone (PDE-like diffusion/evaporation on GPU), and ODEs for slow colony demographics.

### 3. Pheromone and Chemical Communication
Pheromone dynamics are the core of a realistic ant sim.

**Deposition, evaporation, diffusion.** Trail pheromone is deposited as discrete drops, evaporates, and diffuses. The standard model is a scalar field per pheromone type with per-step multiplicative decay and a diffusion (blur/convolution) step, plus deposition by ants. **Beckers, Deneubourg & Goss (1992)** is the foundational empirical study of trail-laying in *Lasius niger*: foragers increase the number of depositions with distance, recruited foragers lay less than discoverers, and deposition per forager decreases with trip number. [Wiley Online Library](https://resjournals.onlinelibrary.wiley.com/doi/full/10.1111/een.12995) [Springer](https://link.springer.com/article/10.1007/s00040-026-01106-9) A commonly used modeling value is ~0.5 pheromone units deposited per second on the return from food [arxiv](https://arxiv.org/pdf/1108.3495) (Boissard, Degond & Motsch 2013, citing *L. niger* data). **Czaczkes, Olivera-Rodriguez & Poissonnier (2024, *Insectes Sociaux*)** found *L. niger* "deposit up to 22 times more pheromone within 10 cm of a food source compared to when they are about to reach the nest" — a spatial modulation worth implementing.

**Half-lives vary enormously across species — the single most important species-specific parameter.** Pharaoh ant (*Monomorium pharaonis*) trails decay with a ~9 min half-life and are strongly substrate-dependent (Jeanson, Ratnieks & Deneubourg 2003). Argentine ant synthetic pheromone ((Z)-9-hexadecenal) has a half-life of order 30 min (a lower bound); gaster extracts remain active ~4 h (Van Vorhis Key & Baker 1982; Perna et al. 2012). Army ants (*Daceton*, *Eciton*) maintain trails for days to weeks; *Aphaenogaster albisetosus* trails last minutes (Hölldobler et al. 1995). Note a modeling caveat: Choe, Villafuerte & Tsutsui (2012, *PLoS ONE*) showed the *natural* Argentine-ant trail is dominated by the iridoids dolichodial and iridomyrmecin, not the legacy synthetic (Z)-9-hexadecenal, so treat classic synthetic-compound parameters with caution.

**Multiple pheromone types** a realistic sim should support:
- *Trail/recruitment* (attractive) — guides to food.
- *Alarm* — volatile, fast-diffusing, short-range; e.g., 4-methyl-3-heptanone and 4-methyl-3-heptanol [Springer](https://link.springer.com/article/10.1007/s10886-023-01407-4) (identified in the clonal raider ant by the Kronauer lab), causing ants to become unsettled, initially attracted at low concentration then repelled at high concentration. [Springer](https://link.springer.com/article/10.1007/s10886-023-01407-4)
- *Home-range / nest-marking* — colony-specific area marking.
- *Negative / "no-entry" pheromone* — **Robinson, Jackson, Holcombe & Ratnieks (2005, *Nature* 438:442)** discovered Pharaoh ants deploy a repellent pheromone at trail bifurcations to mark unrewarding branches ("empirical evidence for such a negative trail pheromone, deployed... as a 'no entry' signal to mark an unrewarding foraging path"). **Robinson, Green, Jenner, Holcombe & Ratnieks (2008, *Insectes Sociaux* 55:246–251)** found "the repellent pheromone effect lasts more than twice as long as the attractive pheromone effect (78 min versus 33 min)... the initial effect of the repellent pheromone on branch choice is almost twice that of the attractive pheromone (48% versus 25% above control)" — note these are *behavioural-response* durations at bifurcations, not raw chemical half-lives. An agent-based model (Robinson, Ratnieks & Holcombe 2008, *J. Theor. Biol.*) showed the repellent prevents runaway positive feedback. [ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/S0022519308004281) **Czaczkes, Grüter & Ratnieks (2013, *J. R. Soc. Interface*)** documented self-organized negative feedback in *L. niger*: "a self-organized negative feedback mechanism... downregulates the production of recruitment signals in crowded parts of a network... a 5.6-fold reduction" in the number of ants depositing under crowding.

**Sensing and trail-following.** Ants sense pheromone with paired antennae, enabling **osmotropotaxis** (comparing left–right concentration to steer). Detection is extraordinarily sensitive: *Atta texana*'s trail pheromone (methyl 4-methylpyrrole-2-carboxylate; Tumlinson, Silverstein, Moser et al. 1971, the first ant trail pheromone chemically identified) is detectable at ~80 fg/cm of trail. [Wiley Online Library](https://resjournals.onlinelibrary.wiley.com/doi/10.1111/j.1365-3032.2008.00658.x) Trail-following is well modeled as a biased correlated random walk with a nonlinear (sigmoidal/Weber-law) response to the local pheromone difference; **Perna et al. (2012, *PLoS Comput. Biol.*)** showed a Weber-type individual response reproduces the classic Deneubourg choice function and generates realistic dendritic trail networks.

**Trail-network formation and optimization.** Positive feedback (deposition + following) plus decay yields shortest-path selection, network pruning, and adaptation. The Deneubourg choice function, P_A = (k+A)ⁿ / [(k+A)ⁿ + (k+B)ⁿ] with n≈2, k≈20, is the canonical implementation. Jackson, Holcombe & Ratnieks (2004, *Nature*) also showed trail *geometry* (bifurcation angle) gives networks polarity — ants read the fork angle to orient nestward vs foodward.

### 4. Foraging and Recruitment
Recruitment spans a continuum (Hölldobler & Wilson):
- **Mass recruitment** via trail pheromone (Argentine ants, *Lasius*, leaf-cutters, fire ants): scalable positive feedback; use the pheromone field above.
- **Group recruitment**: a scout leads a small group.
- **Tandem running**: one leader guides a single follower with tactile/pheromone feedback (*Temnothorax*), a form of "teaching" (Franks & Richardson 2006, *Nature*). Slow but information-rich.
- **Solitary/individual foraging** (*Cataglyphis*, many *Melophorus*): no trail; relies on navigation.

Foraging strategy sets the exploration–exploitation balance. Deneubourg, Aron, Goss & Pasteels (1990) modeled the self-organized exploratory front of Argentine ants. Fire-ant scouts deposit trail pheromone scaled to food value and colony need; foraging decays if not reinforced. For a game: scouts lay recruitment trail modulated by food quality, with a decay rate tuned so the colony tracks ephemeral vs persistent resources. **Collective choice** between sources emerges from the same nonlinear feedback: the colony concentrates on the richer/closer source (Beckers/Deneubourg modulation results). Czaczkes, Grüter, Jones & Ratnieks (2011) further showed trail pheromone and private route memory act synergistically, increasing walking speed by ~25% and straightness by ~30%.

### 5. Traffic Flow and Lane Formation
On busy trails ants self-organize traffic:
- **Couzin & Franks (2003, *Proc. R. Soc. B*)** showed army ants (*Eciton*) form three lanes — inbound (prey-laden) ants in the center, outbound ants on the margins — from simple turning/avoidance rules, protecting prey and optimizing flow.
- **Dussutour, Fourcassié, Helbing & Deneubourg (2004, *Nature*)** showed *Lasius niger* avoid jams under crowding; Dussutour et al. (2005) analyzed bi-directional traffic; Dussutour et al. (2007) found crowding can *increase* foraging efficiency in leaf-cutters.
- **Priority rules**: In *Atta colombica*, laden inbound ants get right-of-way (given way in ~80% of head-on encounters), producing lanes with laden ants in the center (Dussutour 2009, *J. Exp. Biol.*; Burd et al. 2002).
Model with CA/TASEP-type rules or force-based collision avoidance. Right-of-way and lane emergence add strong visual realism.

### 6. Navigation
Ants combine several systems, best studied in desert ants (Wehner and colleagues):
- **Path integration (dead reckoning):** the ant continuously integrates direction (celestial/polarized-light compass) and distance (stride-counting odometer) into a "home vector" (Müller & Wehner 1988, *PNAS*; Wittlinger et al. 2006 for the pedometer). *Cataglyphis fortis* returns on a near-straight line then performs a systematic spiral search if the nest isn't found (Müller & Wehner 1994). Cheap to implement: maintain a running home vector per ant.
- **Visual landmark/panorama navigation and route learning:** ants learn views and use terrestrial panoramas; local vectors and beacon-aiming override the global vector when familiar (Collett & Collett; Graham). "Learning walks" calibrate this.
- **Computational neuroscience of the insect brain:** the conserved **central complex (CX)** is the navigation center. **Stone et al. (2017)** built an anatomically-grounded CX model performing path integration; Le Möel, Stone, Lihoreau, Wystrach & Webb (2019) proposed it as a substrate for vector-based navigation; **Sun et al. (2020)** and Goulard et al. (2021) extended it to multimodal navigation via a "copy-and-shift" mechanism. [nih](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC8741217/) The **mushroom bodies** support visual familiarity-based route following (Ardin, Webb, Wystrach). For a game, a functional approximation (home vector + learned view-familiarity gradient) captures the behavior at low cost.

### 7. Nest Excavation, Construction, and Architecture
Realistic nests and mounds emerge from stigmergy:
- **Khuong et al. (2016, *PNAS* 113:1303–1308)** — the definitive data-based model of ant nest construction. In *Lasius niger*, ants pick up, transport, and deposit soil pellets marked with a building pheromone; deposition probability increases with local pellet density and pheromone (stigmergy + topochemical cue), [PNAS](https://www.pnas.org/doi/10.1073/pnas.1509829113) producing pillars → caps → chambers. The model uses ~500 ants and a pheromone mean lifetime of ~1200 s and reproduces real nest growth. This is the template for physics/pixel-based digging.
- **Buhl, Deneubourg, Grimal & Theraulaz (2005)** — self-organized digging; excavation dynamics are logistic (positive feedback via recruitment, saturating), with excavated volume nearly proportional to worker number.
- **Bardunias & Su (2009)** (Formosan termite) showed depressions act as stigmergic digging cues.
- **Underground architecture data:** Walter Tschinkel's nest casts (*Ant Architecture*, Princeton Univ. Press 2021) quantify chamber size, spacing, and how chamber area declines with depth — invaluable ground-truth for procedural nest generation. The Goldman lab (Aina, Avinery et al. 2022–2023, *J. R. Soc. Interface*, *Frontiers in Physics*) studied excavation as active matter, including clogging avoidance in confined digging.
- **Termite mounds:** Turner and Werfel/Nagpal (Bardunias, Calovi, Turner et al. 2020, *Proc. R. Soc. B*) studied Macrotermes mound humidity/airflow and construction coordination. **Werfel, Petersen & Nagpal (2014, *Science* 343:754–758)** built the **TERMES** robots that assemble 3D structures via stigmergy [Science](https://www.science.org/doi/10.1126/science.1245842) — a clean algorithmic template for emergent building.
- **Wasp nests:** Theraulaz & Bonabeau (1995, *Science*) modeled 3D wasp-nest construction with stigmergic rules on a lattice.

### 8. Division of Labor and Task Allocation
- **Response-threshold models (Bonabeau, Theraulaz, Deneubourg 1996–1998):** each worker has a threshold for each task's stimulus; when the stimulus (unattended brood, undefended nest) exceeds the threshold, the worker acts, reducing it. Individual threshold variation yields specialization and elastic reallocation — the workhorse model for a game.
- **Interaction-rate models (Gordon):** in red harvester ants (*Pogonomyrmex barbatus*), a worker's task decision depends on its recent rate of brief antennal contacts [Springer](https://link.springer.com/article/10.1007/s002650050573) (Gordon & Mehdiabadi 1999; Greene & Gordon 2007). Gordon, Holmes & Nacu (2008) modeled foraging regulation purely from the rate of returning foragers, with no spatial info. Cuticular hydrocarbons convey task identity on contact (Greene & Gordon 2003).
- **Foraging-for-work, age polyethism (temporal castes):** workers shift tasks with age. **Mersch, Crespi & Keller (2013, *Science* 340:1090–1093)** tracked six *Camponotus fellah* colonies over 41 days and, from "network analyses of more than 9 million interactions," found three age-linked functional groups (nurses → cleaners → foragers) [PubMed](https://pubmed.ncbi.nlm.nih.gov/23599264/) driven by spatial fidelity.
- **Morphological castes / polymorphism:** leaf-cutters and army ants have physical castes (minors, media, majors, soldiers) — implement as fixed roles with size-linked stats.
- **Reserve / idle workers:** a large fraction of a colony is typically inactive at any moment — a realistic feature that interacts with metabolic scaling.

### 9. Colony Life Cycle, Demography, and Energetics
Model these slow dynamics with an ODE/stage-structured layer:
- **Life cycle:** nuptial flight → mating → claustral founding → first workers (nanitics) → ergonomic growth → reproductive phase producing alates. Colony growth is typically sigmoidal.
- **Brood stages:** egg → larva (several instars, fed, often via trophallaxis) → pupa → callow adult; development times are temperature-dependent.
- **Metabolic scaling:**

*[The copy received ends here.]*
