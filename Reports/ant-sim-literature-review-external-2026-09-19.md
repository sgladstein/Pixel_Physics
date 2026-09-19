# A Builder's Review of Ant and Superorganism Computational Simulation: Mechanisms, Models, and Parameters for Realistic Colony Games

> **Provenance.** External document, written by a reviewer who knew only that
> this project is an ant-based simulation game and surveyed the current
> literature. Reproduced here verbatim as received on 2026-09-19 so that the
> repo's own review of it ([`ant-sim-research-review-2026-09-19.md`](ant-sim-research-review-2026-09-19.md))
> can cite it by line. Nothing in it has been checked against the engine;
> that is the review's job. **Received in two parts the same day**: the
> TL;DR through §9's *Brood stages* as markdown, and the remainder from
> *Metabolic scaling* onward as plain text with the markdown structure
> flattened — headings, bullets and the four parameter tables were restored
> from their evident structure, with no word changed.

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
- **Metabolic scaling:** colony metabolic rate scales hypometrically with mass (exponent ~0.75–0.93); per-capita metabolism decreases as colonies grow (gatech) (Hou, Kaspari, Vander Zanden & Gillooly 2010, PNAS; Waters, Holbrook, Fewell & Harrison 2010, Am. Nat.; Waters et al. 2017, Proc. R. Soc. B; Fewell & Harrison 2016). Shik, Hou et al. (2012, Biol. Lett.) built a superorganism life-history model predicting colony survival, growth, and reproduction. The "group effect" vanishes when workers are isolated. (gatech) Cao & Dornhaus (2013) found larger Temnothorax colonies consume proportionally less energy per capita and have lower per-capita brood production.
- **Trophallaxis networks:** food is shared mouth-to-mouth, forming a distribution network (LeBoeuf; Sendova-Franks et al. 2010 "famine relief"). Model as flow on the interaction network.
- **Food storage:** repletes/honeypot ants, seed caches (harvesters), fungus gardens (leaf-cutters).

### 10. Collective Decision-Making: Nest-Site Selection
The best-quantified collective decision is house-hunting in Temnothorax (Pratt, Franks, Mallon, Sumpter):
- Scouts find candidate sites, assess quality (entrance size, cavity dimensions, light level), and recruit via tandem runs. When the population at a site exceeds a quorum threshold, scouts switch to rapid transport (carrying nestmates), committing the colony (Pratt, Mallon, Sumpter & Franks 2002; Pratt 2005).
- Quorum sensing is by encounter rate at the site (Pratt 2005, Behav. Ecol.), yielding a speed–accuracy tradeoff: lower quorums → faster but error-prone; higher quorums → slower but more accurate (Pratt & Sumpter 2006 "tunable algorithm," PNAS).
- Pratt, Sumpter, Mallon & Franks (2005, Anim. Behav.) built the canonical agent-based model. Colonies can choose a far-and-away better nest over an in-the-way poorer one (Springer) (Franks et al. 2008). Directly implementable and adds compelling emergent gameplay (emigrations).

### 11. Collective Transport and Self-Assembly
- **Cooperative transport:** Longhorn crazy ants (Paratrechina longicornis) — groups of hundreds of 2.5–3 mm ants haul loads "exceeding 10,000 times their own weight and more than one hundred times their body length" (Gelblum, Pinkoviezky, Fonio, Ghosh, Gov & Feinerman 2015, Nature Communications 6:7729), with the group "poised at the transition between random walk and ballistic motion." Newly-attached "informed" ants inject directional information. Fonio/Gelblum et al. (2020, eLife) mapped it to the "Ant-in-a-Labyrinth" problem, showing collective sensing-range extension. (ResearchGate) Implement as self-propelled particles attached to a shared load with individual force vectors plus alignment.
- **Fire ant rafts and towers:** Solenopsis invicta link bodies into waterproof rafts (PNAS) (Mlot, Tovey & Hu 2011, PNAS 108:7669–7673) and self-healing viscoelastic aggregates (Tennenbaum, Liu, Hu & Fernandez-Nieves 2016, Nature Materials 15:54–59). Foster et al. (2014) showed active control of spacing/orientation; Phonekeo et al. (2017) studied perpetual tower rebuilding; Wagner & Vernerey (2021, 2022) modeled treadmilling. Best modeled as active soft matter with catch-bond-like links.
- **Army ant living architecture:** Reid, Lutz, Powell, Kao, Couzin & Garnier (2015, PNAS 112:15113–15118) showed Eciton bridges lengthen, widen, and migrate (ADS) per a cost–benefit tradeoff (shortcut benefit vs workers sequestered). (ADS) Graham, Kao et al. (2017, J. Theor. Biol.) gave an optimal-construction model.
- **Circular mills:** sensory-deprived army ants can spiral into fatal "death mills" — an emergent artifact of pure trail-following, worth noting as an edge case.
- Anderson, Theraulaz & Deneubourg (2002, Insectes Soc.) reviewed self-assemblages generally.

### 12. Movement and Locomotion
- Walking speeds are strongly species- and temperature-dependent (Table 2). The Saharan silver ant Cataglyphis bombycina is the fastest ant at ~0.855 m/s (~108 body lengths/s), falling to ~0.057 m/s at 10 °C (Pfeffer, Wahl, Wittlinger & Wolf 2019, J. Exp. Biol. 222:jeb198705); the longer-legged C. fortis reaches ~0.62 m/s (~50 body lengths/s) (Pfeffer et al. 2019; Wahl, Pfeffer & Wittlinger 2015).
- **Gait:** alternating tripod; stride length increases linearly with speed (ResearchGate) (Zollikofer 1994; Wahl et al. 2015). At top speed silver ants approach a gallop.
- **Temperature dependence of activity:** thermophilic desert ants forage at ground temperatures >50 °C when competitors cannot; temperature also accelerates pheromone evaporation, limiting trail-following (a documented mechanism affecting dominant, chemically-recruiting species more than subordinate ones).
- **Random-walk models:** undirected search is well modeled as a correlated random walk (CRW) with a species-typical turning-angle distribution; biased CRW under pheromone/vector guidance.
- **Body-size effects:** larger workers walk faster and carry more; scale stats with size for polymorphic species.

### 13. Inter-Colony and Interspecies Interactions
- **Territoriality and warfare:** neighboring colonies contest space; some conduct raids. Argentine-ant supercolonies are non-aggressive internally but aggressive between supercolonies.
- **Invasive spread:** Linepithema humile forms massive supercolonies; spread models combine local budding with human-mediated jumps.
- **Slave-making (dulosis) and social parasitism:** raids on host colonies.
- **Mutualisms:** aphid tending (honeydew) and fungus farming in leaf-cutters (forage leaves → fungus garden → feed larvae; Shik et al. 2014 studied its metabolic costs).
- **Predator–prey:** army-ant raids (Franks; Schneirla).
Implemented as agent interaction rules plus population dynamics.

### 14. Empirical Data Sources and Validation
- **Barcode/tag tracking:** Mersch, Crespi & Keller (2013) tagged whole Camponotus fellah colonies with matrix barcodes (bCodes); Kay et al. (2024, Proc. R. Soc. B) showed ant social-network structure is conserved across five subfamilies (Royal Society Publishing) using ARTag barcodes.
- **Kronauer lab clonal raider ant (Ooceraea biroi):** a queenless, (bioRxiv) clonal, synchronously-reproducing (bioRxiv) species ideal for controlled tracking and neurogenetics; the lab tracks hundreds of colonies at once, produced the first transgenic ants, and a reference brain (2025). Ulrich et al. linked individual behavior to division of labor and disease exposure.
- **Trajectory datasets and trackers:** anTraX, idtracker.ai, and deep-learning trackers (Imirzian et al. 2019; Cao et al. 2020) provide trajectories for calibration.
- **Validation practice:** pattern-oriented modeling (Grimm & Railsback) — match multiple emergent patterns (trail choice, network geometry, nest shape, growth curves) simultaneously.

### 15. Recent Advances (2015–2026)
- Machine learning / RL for behavior classification and data-driven movement models; supervised learning to decode alarm-signal propagation (Guo et al.).
- Neural navigation models matured (Stone et al. 2017; Sun et al. 2020), informed by Drosophila central-complex connectomics.
- Active-matter physics of aggregates (fire ants, cooperative transport) as self-propelled particles.
- Swarm robotics inspired by ants/termites: TERMES (Werfel et al. 2014), Kilobots (Rubenstein et al.), collective-transport robots.
- GPU-scale simulation: compute-shader pheromone fields enabling millions of agents.

### 16. ACO — Brief Distinction from Biological Modeling
Ant Colony Optimization (Dorigo; Bonabeau, Dorigo & Theraulaz 2000, Nature) is a metaheuristic that abstracts trail-laying to solve routing/scheduling (TSP, vehicle routing). Artificial "ants" deposit "pheromone" on graph edges with evaporation and probabilistic edge choice. It is not a biological model — real ants lack global graph knowledge, deposit at biologically implausible rates, and move in continuous space. For a realistic sim it offers only two transferable ideas: (1) the evaporation + probabilistic-choice formalism for pheromone-mediated path selection, and (2) confirmation that shortest-path selection is emergent from the deposition/evaporation balance. Do not use ACO as your behavioral core.

### 17. Existing Simulation Software, Tools, and Games
**Research platforms:** NetLogo (classic Ants/AntLines models; BEEHAVE is built in it) — ideal for prototyping stigmergy, not performant at scale; MASON, Repast (Java ABM frameworks for large models); ARGoS (swarm-robotics simulator).

**Games (assessed for realism):**
- **SimAnt** (Maxis, 1991) — pioneering; modeled castes, pheromone trails, and colony-vs-colony competition; simplified but conceptually faithful.
- **Empires of the Undergrowth** (Slug Disco, Early Access 2017, full release June 2024, Unreal Engine) — RTS/base-builder; players command via pheromone markers rather than direct orders, (Steam) with underground stigmergic nest building, real species/biomes, and physical castes. It leans toward simulating real ant behaviors and is arguably the most visually realistic ant game, but it is an RTS abstraction, not an emergent individual-based sim (combat and progression are gamey; no true pheromone-field emergence).
- Various indie titles (Ant Simulator, etc.) — variable fidelity.
- Hobbyist GPU/compute-shader sims (Sebastian Lague-style ant/slime videos; numerous GitHub projects) — often implement the field correctly (deposition/diffusion/evaporation on a texture, agents sensing three points ahead) and produce beautiful emergent trails, but usually model a single generic pheromone with no colony demography, castes, or navigation beyond trail-following.

**What games get right/wrong:** most handle trail-following and basic recruitment plausibly, but omit realistic pheromone half-life differences, negative pheromones, quorum-based decisions, metabolic scaling, path integration, and stigmergic construction with topochemical cues. Adding those is where realism gains lie.

### 18. Other Superorganism Simulations (Transferable Mechanisms)
- **Honeybees:** BEEHAVE (Becher, Osborne, Thorbek, Kennedy & Grimm 2014, J. Appl. Ecol.) — a comprehensive NetLogo colony model integrating stage/cohort demography, an energetics-based foraging module on a spatial landscape, and varroa/disease dynamics. Transferable: colony demography + energetics + spatially-explicit foraging. Waggle-dance recruitment (von Frisch; Seeley) and nest-site selection by scouts with quorum (Seeley; Reina et al. 2017) parallel ant systems. Hive thermoregulation (fanning, clustering; Heinrich) and mechanical swarm adaptation (Peleg et al. 2018, Nature Physics) transfer to nest-climate mechanics.
- **Termites:** stigmergic mound building (Grassé; Turner; Bardunias/Werfel), mound airflow/thermoregulation — directly transferable to underground/mound climate and construction.
- **Social wasps:** Theraulaz & Bonabeau (1995) lattice-based stigmergic nest construction; Jeanne's work on wasp task/construction regulation.
- **Stingless bees:** Trigona use mandibular-gland odor trails with polarity (more pheromone near food) — a variant trail system.

### 19. Open Problems and What Remains Unrealistic
- **Integrating scales:** unifying individual neural navigation, pheromone fields, and colony demography in one performant simulation remains hard.
- Realistic 3D underground architecture coupled to digging physics and soil mechanics is still rare in real time.
- Multi-pheromone ecosystems (trail + alarm + no-entry + nestmate recognition) are seldom modeled together.
- Individual variation and learning (route memory, experience effects) are usually omitted.
- Validation against tracking data is improving, but most game models are uncalibrated.
- Colony-level cognition (speed–accuracy tuning, collective sensing) is understood in pieces, not as a whole.

## Parameter Tables

**Table 1 — Trail pheromone persistence and deposition (species-specific).**

| Species | Persistence / half-life | Notes / source |
|---|---|---|
| Monomorium pharaonis (Pharaoh) | ~9 min (short-lived attractive); repellent behavioural effect ~78 min vs short attractive ~33 min; long-lasting component persists days | Jeanson, Ratnieks & Deneubourg 2003; Robinson et al. 2008; Jackson et al. 2006 |
| Linepithema humile (Argentine) | ~30 min (synthetic (Z)-9-hexadecenal, lower bound); ~4 h from gaster extract | Van Vorhis Key & Baker 1982; Perna et al. 2012 |
| Lasius niger | pheromone mean lifetime ~1200 s used in construction model; deposition ~0.5 units/s returning; up to 22× more within 10 cm of food | Khuong et al. 2016; Boissard et al. 2013; Czaczkes et al. 2024 |
| Atta texana / leaf-cutters | very long-lasting; detectable at ~80 fg/cm | Tumlinson et al. 1971 |
| Solenopsis invicta (fire ant) | very volatile, seconds–minutes; continuous re-marking | Suckling et al. 2010 |
| Eciton / Daceton (army ants) | days to >7 days | Torgerson & Akre 1970 |
| Aphaenogaster albisetosus | minutes | Hölldobler et al. 1995 |

**Table 2 — Walking speeds and locomotion.**

| Species | Speed | Notes / source |
|---|---|---|
| Cataglyphis bombycina (silver ant) | ~0.855 m/s (fastest ant; ~108 body lengths/s); ~0.057 m/s at 10 °C | Pfeffer, Wahl, Wittlinger & Wolf 2019 |
| Cataglyphis fortis | ~0.62 m/s (~50 body lengths/s); stride length rises linearly with speed | Pfeffer et al. 2019; Wahl et al. 2015 |
| Lasius niger | order ~1–3 cm/s on trails (temperature-dependent) | traffic studies (Dussutour et al.) |
| Leaf-cutters (Atta) | ~1–4 cm/s, laden slower; right-of-way to laden | Burd et al. 2002; Dussutour 2009 |
| General rule | speed ↑ with temperature and body size | multiple |

**Table 3 — Colony size, demography, energetics.**

| Quantity | Typical value / scaling | Source |
|---|---|---|
| Colony size | Temnothorax ~100s; Lasius niger 1000s–10,000s; Atta/army ants 100,000s–millions | Khuong et al. 2016; Reid et al. 2015 |
| Active foragers | ~10% of population (springer) (Cataglyphis) | Wahl et al. 2015 |
| Colony metabolic scaling | hypometric, exponent ~0.75–0.93; per-capita metabolism ↓ with size | Hou et al. 2010; Waters et al. 2010, 2017 |
| Cooperative load (crazy ant) | >10,000× individual weight, >100× body length | Gelblum et al. 2015 (Nat. Commun. 6:7729) |
| Nest excavation | volume ∝ worker number; logistic dynamics | Buhl et al. 2005; Khuong et al. 2016 |
| Quorum threshold (Temnothorax) | correlates with number of adult workers | Pratt 2005 |

**Table 4 — Recruitment / decision parameters.**

| Parameter | Value / form | Source |
|---|---|---|
| Deneubourg choice function | P_A=(k+A)ⁿ/[(k+A)ⁿ+(k+B)ⁿ], n≈2, k≈20 | Deneubourg et al. 1990 |
| Task allocation | response-threshold; act when stimulus > individual threshold | Bonabeau et al. 1996 |
| Task decision (harvester) | function of recent antennal-contact rate | Gordon & Mehdiabadi 1999 |
| Quorum decision | switch tandem→transport at encounter-rate quorum | Pratt 2005 |
| Memory × pheromone synergy | +~25% speed, +~30% straightness | Czaczkes et al. 2011 |

## Recommendations
Staged, concrete steps for building a highly realistic ant colony simulation game:

**Stage 1 — Core stigmergic engine (do this first).**
1. Represent each pheromone type as a 2D (or layered 3D) floating-point grid/texture. Per tick: (a) deposition — ants add to their cell; (b) diffusion — separable Gaussian blur / discrete Laplacian; (c) evaporation — multiply by decay = 0.5^(dt/half_life), using a different half-life per pheromone and species (Table 1). Run the whole update as a GPU compute/fragment shader with ping-pong textures.
2. Movement: correlated random walk for search; for trail-following, sample 3 points ahead (left/center/right), steer toward the strongest with a sigmoidal/Weber response plus noise — this reproduces the Deneubourg choice function (Perna et al. 2012). Never use A* for foraging.
Benchmark that changes your approach: if trails don't dissolve after you remove food, your evaporation is too low; if the colony instantly locks onto one of two equal sources, your choice nonlinearity/noise is miscalibrated.

**Stage 2 — Multi-pheromone realism and behaviors.**
3. Add at least trail + alarm + a negative "no-entry" channel (Robinson et al. 2005); this unlocks realistic network pruning and defense and prevents runaway feedback.
4. Give foragers path integration (a per-ant home vector) plus optional learned view-familiarity for navigation, essential for solitary species (Cataglyphis, Melophorus).
5. Add traffic priority rules (right-of-way to laden ants) to get emergent lane formation on busy trails.

**Stage 3 — Colony as an organism.**
6. Allocate tasks by response thresholds + interaction rates, not scripts; keep a large inactive reserve.
7. Add a slow ODE demographic layer (brood stages, sigmoidal growth curve, hypometric energetics) beneath the fast agent layer.
8. Implement stigmergic construction (Khuong et al. 2016: pick-up/deposit with density + building-pheromone cue) for emergent nests; seed procedural nest shapes from Tschinkel cast data.
9. Add quorum-based nest-site selection/emigration (Pratt/Franks) for dramatic emergent colony decisions.

**Stage 4 — Scale and validate.**
10. Use spatial hashing/uniform grid for neighbor queries, level-of-detail (coarse/statistical rules for distant or underground ants), and a hybrid where the reserve/brood population is mean-field.
11. Respect species-specific parameters (Tables 1–4): a Pharaoh-ant sim (9-min trails, no-entry pheromone) must feel different from a silver-ant sim (solitary, path integration, 0.855 m/s) or a leaf-cutter sim (long trails, traffic lanes, castes, fungus farming).
12. Validate against published emergent patterns (trail geometry, nest shape, growth curves) and open tracking datasets.

Thresholds that should trigger a redesign: ants look robotic → replace pathfinding with stigmergy + CRW; large colonies tank performance → move the reserve to mean-field and add LOD; the colony never adapts to environmental change → increase evaporation and ensure negative feedback (crowding/ no-entry) is active.

## Caveats
- Parameters vary widely across species and contexts; the tables give representative values, but half-lives, speeds, and colony sizes differ by orders of magnitude between species and shift with temperature, substrate, humidity, and crowding. Always parameterize per modeled species.
- Some frequently-cited pheromone numbers are for synthetic compounds or are behavioural-response durations, not raw chemical half-lives. The Argentine-ant ~30 min figure is for synthetic (Z)-9-hexadecenal, which Choe et al. (2012) argue is not the dominant natural trail compound; the Pharaoh-ant ~78 min "no-entry" figure is a behavioural-effect duration measured at bifurcations (Robinson et al. 2008), not a concentration half-life. Treat accordingly.
- The ~0.5 drops/s Lasius niger deposition rate comes from a mathematical-modeling paper (Boissard et al. 2013) citing empirical work; the primary empirical source is Beckers, Deneubourg & Goss (1992/1993), and deposition is highly context-dependent (distance from food, crowding, individual, trip number).
- Neural-navigation models (central complex, mushroom body) are biologically faithful but computationally heavier than a game needs; the recommended home-vector + view-familiarity approximations capture the behavior without the full circuit.
- Games named here are assessed from public descriptions/reviews, not source code; "realism" judgments are relative to the biological literature, not exhaustive audits of each title.
- This review prioritizes peer-reviewed primary literature, but a few figures (e.g., top silver-ant speed, illustrative Atta sensitivity calculations) circulate in secondary sources; where possible the primary study is named. Citations are given in-text by author/year for the reader to consult originals.
