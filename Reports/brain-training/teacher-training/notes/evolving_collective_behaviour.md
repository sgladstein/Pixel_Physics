# Evolving / training collective (colony-level) behaviour in neural controllers

Research conditions, so the reader can weight the evidence: in this session WebFetch and direct HTTP to publishers were blocked (plos.org, epfl.ch, arxiv.org, springer and wikipedia all refused). Full texts were read **only** for the open-access papers available through PubMed Central: Ferrante 2015, Waibel 2011, Floreano & Keller 2010, Duarte 2016 and a 2023 Frontiers review. Everything else rests on search-engine summaries of abstracts. Items marked **[unverified]** come from my own prior knowledge of the literature and were not confirmed against a source in this session.

**Citation correction.** The PLoS Biology paper is **Waibel, Floreano & Keller 2011**, "A Quantitative Test of Hamilton's Rule for the Evolution of Altruism" (PLoS Biol 9(5): e1000615). "Genetic Team Composition and Level of Selection in the Evolution of Cooperation" is **Waibel, Keller & Floreano 2009, IEEE Transactions on Evolutionary Computation 13(3): 648–660**. Both are covered below.

---

## Q1. Waibel, Keller & Floreano: team composition, level of selection, altruism (2009 IEEE TEC; 2011 PLoS Biol; Floreano & Keller 2010)

### Takeaway
For any task that needs cooperation, **genetically homogeneous (clonal) teams with team-level selection** is the combination that works. It is also the field's default. Heterogeneous teams under team-level selection did worst on cooperative tasks: when the whole team's fitness goes to non-identical genomes, each genome's credit is diluted. A colony where all ants share one genome and are scored as a colony is already in the recommended corner of this design space. The papers' warnings concern mixed-genome teams, which this project does not have.

### Cited Findings
**Waibel, Keller & Floreano 2009 (IEEE TEC), the 2×2 design**
- The study crosses two factors. Genetic team composition is either homogeneous (all team members cloned from one genome) or heterogeneous (different genomes). Level of selection is either the team (teams selected as units) or the individual (individuals selected regardless of team). That gives four treatments. Their simulated foraging task can be tuned to need more or less cooperation. — [EPFL Infoscience record](https://infoscience.epfl.ch/entities/publication/992dcfff-4683-4cad-bc2e-3616d30e487f); [ResearchGate](https://www.researchgate.net/publication/224516806_Genetic_Team_Composition_and_Level_of_Selection_in_the_Evolution_of_Cooperation)
- Headline conclusion: "the optimal combination depends on the amount of cooperation required by the task". The authors define **three types of cooperative task** and give guidelines for choosing team composition and level of selection. — [EPFL Infoscience](https://infoscience.epfl.ch/record/111745?ln=en)
- The abstract notes that most studies use teams of agents with identical control rules and select at the team level, which is the field default. — [EPFL Infoscience](https://infoscience.epfl.ch/entities/publication/992dcfff-4683-4cad-bc2e-3616d30e487f)
- Results reported by the search summary of the abstract and citing work (not seen in full text):
  - In **cooperative foraging**, homogeneous teams performed significantly better than heterogeneous teams.
  - Among heterogeneous teams, individual selection beat team selection, but still fell short of homogeneous teams.
  - In **individual foraging**, heterogeneous teams under individual selection performed about as well as homogeneous teams under either level of selection.

  — [ResearchGate / search summary](https://www.researchgate.net/publication/224516806_Genetic_Team_Composition_and_Level_of_Selection_in_the_Evolution_of_Cooperation)
- **[unverified, my recollection]** The three tasks are:
  - individual foraging: small tokens, each pushable by one robot;
  - cooperative foraging: large tokens that need two robots, with the reward going to the team;
  - altruistic cooperative foraging: pushing a large token benefits the team at a cost to the pusher.

  Recalled findings: homogeneous teams with team selection were best whenever altruism was required, and heterogeneous teams with team selection were poor because of a credit-assignment problem (one team score assigned to non-identical genomes). The exact efficiency numbers, generation counts and population sizes of the 2009 paper could not be retrieved.
- A companion conference paper (IWANN 2009) describes the setup as teams of sugar-cube robots in a foraging task whose degree of required cooperation can be tuned. — [EPFL Infoscience IWANN09](https://infoscience.epfl.ch/record/135222/files/IWANN09.pdf?version=1)

**Floreano & Keller 2010 (PLoS Biol, review essay): the cooperation and altruism experiments (full text read)**
- Setup:
  - A square arena with **10 sugar-cube robots per group**.
  - Small tokens can be pushed by one robot; large tokens need at least two.
  - A large token pushed to the white wall gives +1 fitness to **all robots in the group**; a small token gives +1 to the pusher only.
  - **Population: 100 groups of 10 robots.**
  - Sensors: 5 IR distance sensors and 2 vision sensors.

  — [Floreano & Keller 2010, PLoS Biol 8(1): e1000292](https://doi.org/10.1371/journal.pbio.1000292)
- With only large tokens in the arena, robots "readily evolved the ability to cooperatively push large tokens … in all 20 evolutionary replicates". — [Floreano & Keller 2010](https://doi.org/10.1371/journal.pbio.1000292)
- With both token sizes, the outcome depended on kin structure:
  - Unrelated robots "invariably specialised in pushing the small objects".
  - Clonal groups "primarily pushed the large tokens even though it was costly".

  — [Floreano & Keller 2010](https://doi.org/10.1371/journal.pbio.1000292)
- Costly communication, in which light-emitting robots signal a food location, evolved readily among related robots. It "never evolved in groups of unrelated robots when selection operated at the individual level". — [Floreano & Keller 2010](https://doi.org/10.1371/journal.pbio.1000292)
- The controllers had **fewer than 15 neurons**. "A few hundreds of generations" were enough for cooperation and altruism to evolve. — [Floreano & Keller 2010](https://doi.org/10.1371/journal.pbio.1000292)
- On overfitting to the training environment, the essay lists these remedies:
  - evaluate in several varied environments;
  - inject noise into the simulator;
  - co-evolve the simulator;
  - add lifetime plasticity.

  — [Floreano & Keller 2010](https://doi.org/10.1371/journal.pbio.1000292)

**Waibel, Floreano & Keller 2011 (PLoS Biol): quantitative test of Hamilton's rule (full text read)**
- Setup:
  - **8 Alice micro-robots** (2×2×4 cm) and 8 food items in a 50×50 cm arena.
  - Fixed-topology feed-forward network: 6 sensor inputs plus a bias, 3 hidden neurons, 3 outputs. The outputs are the two wheel speeds and a "share or not" decision.
  - **33 weights**, encoded as 33 genes of 8 bits each.

  — [Waibel et al. 2011](https://doi.org/10.1371/journal.pbio.1000615)
- Evolution:
  - **500 generations**, each with **200 groups × 8 = 1,600 genomes**.
  - Each group evaluated **10 times for 60 s**.
  - Roulette-wheel selection on individual inclusive fitness.
  - Crossover probability 0.005; mutation 0.005 per bit (≈0.04 per gene).
  - **25 treatments** (5 relatedness levels × 5 c/b ratios) × **20 independent replicates** each.

  — [Waibel et al. 2011](https://doi.org/10.1371/journal.pbio.1000615)
- Compute implied by those figures: 500 × 200 × 10 = **1,000,000 group trials per replicate population**, and 500 such populations in total.
- Relatedness was set by cloning:
  - r = 0: random assignment of genomes to groups.
  - r = 1: one genome cloned 8 times.
  - r ≈ 0.75: clone proportion 1:7.
  - r ≈ 0.54: proportions 6:1:1.
  - r ≈ 0.25: proportions 3:3:2.

  — [Waibel et al. 2011](https://doi.org/10.1371/journal.pbio.1000615)
- "Altruism always evolved in populations where r was greater than c/b." At r = c/b the level of altruism drifted, with a standardized variance of 0.204, against 0.018 above the threshold and 0.015 below it. — [Waibel et al. 2011](https://doi.org/10.1371/journal.pbio.1000615)
- The genotype-to-phenotype map was strongly non-linear:
  - **Epistasis**: the genetic background changed a mutation's effect in 58–59% of 4,000 tests.
  - **Pleiotropy** was widespread.
  - **Large-effect mutations were common**: 40–44% of mutations changed performance by more than 25%.

  Hamilton's rule still predicted the outcome. — [Waibel et al. 2011](https://doi.org/10.1371/journal.pbio.1000615)
- The design deliberately "prevented preferential interactions" by randomising start positions and using a network "that did not allow individuals to memorize past interactions". — [Waibel et al. 2011](https://doi.org/10.1371/journal.pbio.1000615)

### Inferences
- A colony where all ants share one genome and fitness is scored per colony is homogeneous team selection with r = 1. Under Hamilton's rule any colony-benefiting act is favoured (r = 1 > c/b for any c < b). So **there is no "altruism problem" to solve**; the obstacle is purely search and optimisation. Individual-level reward shaping inside a clonal colony is unnecessary for evolutionary stability. It is still useful as a shaping or credit signal to speed up search (see Q5).
- The cost of clonal team selection is that **each evaluation yields one fitness number for the whole colony**, so per-ant credit is lost. This is the price Waibel's heterogeneous/team-selection treatment paid, and homogeneous teams sidestep it only because all members share the credit.
- Waibel 2011 shows that a 33-weight network evolves meaningful social traits in about 500 generations at roughly 1,600 genomes per generation. A ~700-link recurrent network has a much larger search space. Extrapolating from these papers alone, budgets of hundreds to thousands of generations are typical; none of these papers evolves anything close to 700 links for a social task.

### Gaps
- Exact per-treatment numbers, team size, population size and generation count for Waibel et al. 2009 (IEEE TEC): full text not retrievable. The "three task types" are recalled, not verified.
- The Waibel et al. 2006 follow-up (ECAL/IROS-era conference papers) was not retrieved.

---

## Q2. Ferrante et al. 2015 (PLoS Comput Biol): self-organized task specialization (full text read)

### Takeaway
A clonal team of 4 robots, selected purely on group performance, evolved **dynamic** dropper/collector task partitioning from scratch in 59% of runs (13 of 22). It did so **only when the environment rewarded it**: a slope cut travel cost and let gravity move items. On flat ground, no run partitioned. The controllers were evolved rule sets (grammatical evolution), not neural networks; the authors blame neural networks for the field's earlier failures. It took 2,000 generations × 100 teams × 3 evaluations per run, on 100–200 cluster nodes, for 3 months in total. Starting from pre-built behavioural blocks gave a smooth, single-optimum fitness landscape. That is the strongest argument in this literature for staged training.

### Cited Findings
**Setup**
- Platform: ARGoS physics simulator with simulated foot-bot (MarXbot) robots. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Arena: 1.75 m × 9.75 m with an 8° slope. 5 items always present at the source, respawned when taken. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Speeds: 0.15 m/s on flat ground, 0.015 m/s climbing, 0.23 m/s descending. Dropped items slide down the slope at 1 m/s into a cache. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)

**Two scenarios**
- **(a) Pre-adapted building blocks.** Hand-coded dropper, collector and generalist strategies, with each robot assigned a strategy for life. Every mix for a 4-robot team was searched exhaustively, with 10 runs × 5,000 simulated s per mix.
  - Slope optimum: **2 droppers + 2 collectors**, scoring **144.1 ± 4.3 items**.
  - Flat optimum: all generalists, scoring **120.2 ± 1.4 items**.
  - Each landscape had "one globally attracting optimum" and was "very smooth".

  — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- **(b) From first principles (GESwarm).**
  - Primitives: phototaxis, anti-phototaxis and random walk. Obstacle avoidance is always on.
  - Actions: pick up, drop, change behaviour, set an internal variable.
  - Senses: whether the robot is in the source, slope, cache or nest area, and whether it is carrying an item.
  - Memory: **two memory state variables with evolvable meaning**.
  - Each rule fires its actions with an **evolvable probability**, so probabilistic behaviour is possible.

  — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)

**Evolution parameters**
- **22 runs, each 2,000 generations, 100 groups of 4 robots, each group evaluated 3 times.** That is 600,000 group evaluations per run, each 5,000 simulated seconds long. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Operators: single-point crossover with p = 0.3, mutation p = 0.05, 5% elitism, roulette-wheel selection. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Fitness was simply the number of items delivered to the nest in 5,000 s. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Compute: "100 to 200 nodes in parallel". The number of runs (22) was set by the "3 months of cluster time" available. A team of **4 robots was chosen "to limit the computational burden"**. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Homogeneous teams were used deliberately, "because homogeneous groups combined with team-level selection has been shown to be the most efficient approach to evolve tasks that require coordination". This cites Waibel 2009. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)

**Results**
- **13 of 22 runs (59%)** retrieved most items in a task-partitioned way. The roles were **not fixed**: allocation changed dynamically with the size of the cache. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- The mechanism was probabilistic switching from source to cache plus stigmergy. A robot collected from the cache if it met items on the way, and otherwise went on to the source. "None of our controllers succeeded in evolving" a response-threshold mechanism, even though memory variables were available. 3 of the best controllers did use memory in preconditions. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- In the flat environment, "none of the controllers evolved task partitioning". — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- The 8 best controllers (top 35%, about 100 or more items) partitioned **92% ± 0.08** of retrievals and reached **31%** of the theoretical maximum speed. The best scored **135 ± 14** items, not significantly different from the hand-coded 2+2 optimum of 144.1. These controllers used **4–11 rules**. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Robots "quickly" segregated between source and cache: about 500–1,000 s into a run, 2 droppers and 2 collectors self-organised, "without these tasks having been explicitly rewarded". — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Partitioning was evolutionarily stable: once a run reached high partitioning it "never reverted". Some runs, however, were trapped in local optima, such as generalist foraging or broken locomotion. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- **Scaling:** controllers evolved at **N = 4** were deployed at **N = 20** in a proportionally enlarged arena.
  - Example: 457 ± 72 items at N = 20 against 103 ± 24 at N = 4.
  - For the 8 best controllers, the 20/4 performance ratio was **4.4**, against a linear ideal of 5.

  — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- The authors argue that neural networks scale badly: no prior evolutionary swarm robotics study had evolved self-organised division of labour de novo, "this may be due to the fact that most … have made use of neural network-based approaches … shown to scale badly to more complex problems". — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Why specialization evolves: "task partitioning was favored only when features in the environment … could be exploited to achieve more economic transport and reduce switching costs". — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Mechanism of heterogeneity under identical genomes: "a combination of individual experience, stigmergy and stochastic switching alone were able to generate adaptive task specialization". — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- On building blocks: "optimal task allocation could be achieved more easily if optimized behaviors capable of carrying out the different subtasks were available as pre-adapted behavioral building blocks … the fitness landscape becomes smoother and also easier to explore due to its greatly reduced size". — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)

### Inferences
- **Division of labour will not evolve unless the world makes it pay.** It needs a switching cost (travel distance, tool or state changes) or an economy of specialisation. If digging, carrying and foraging can all be done equally well by one ant with no switching cost, generalists are optimal and no amount of training will produce castes. Designing the environment comes before designing the training.
- The specialisation that evolved was dynamic, built from stochastic switches plus stigmergic cues such as cache size, not fixed castes. For an ant brain that already senses crowding and scent, the cheapest likely path is the same: probabilistic switching gated by local cues.
- The project's plan to train skills solo first and then combine them matches Ferrante's "pre-adapted building blocks" condition. In that condition the colony-level search only has to find an allocation, a much smaller and smoother problem than learning skills and allocation together. A middle path is to freeze or partially freeze the skill weights and evolve only the switching and allocation weights (inference, not tested in the paper).
- Their cost was 2,000 gens × 100 teams × 3 evals with only **4 agents**, and they still needed a cluster. Training colony behaviour at small N (4–8) and deploying at large N has direct evidence behind it here: a 4.4× gain for 5× the robots.

### Gaps
- The paper does not report wall-clock time per evaluation. The total is only "3 months" on 100–200 nodes for all 22 runs.
- No head-to-head comparison of neural and grammatical evolution in the same setup. The claim that neural networks scale badly is a citation, not a result of this paper.

---

## Q3. Other evolved division of labour (digital organisms, robots) and reviews

### Takeaway
Across Avida and robots, division of labour in genetically identical groups evolves when **task-switching costs** exist and individuals can communicate or sense each other's work. Real-robot swarm evolution (Duarte 2016) works with NEAT within 100–400 generations for simple collective tasks. The automatic-design community (Birattari's group) finds that neuro-evolved swarm controllers transfer poorly to real robots compared with modular behaviour-based designs. That finding matters less for a pure simulation, but it is a warning about overfitting to the training setting.

### Cited Findings
**Goldsby et al. 2012, PNAS 109(34): 13686–13691, Avida (abstract-level only)**
- Task-switching costs promote the evolution of division of labour. Organisms doing complex tasks evolved to use results of simpler tasks done by neighbours and passed in messages. Division of labour emerged **even though group members were genetically identical**, through flexible behaviour plus communication. — [ScienceDaily press release](https://www.sciencedaily.com/releases/2012/08/120807132211.htm)
- The Dryad dataset for the central experiment crosses resource levels 250, 500 and 1000 with task-switching costs 0, 25 and 50, and measures division of labour by Shannon mutual information. — [Dryad](https://datadryad.org/dataset/doi:10.5061/dryad.f8j02)
- Follow-up work exists: Goldsby et al. 2014, PLoS ONE, "The Effect of Conflicting Pressures on the Evolution of Division of Labor". — [PLoS ONE 2014](https://journals.plos.org/plosone/article?id=10.1371%2Fjournal.pone.0102713)

**Duarte et al. 2016, PLoS ONE, real aquatic robot swarm (full text read)**
- **NEAT** with default parameters, evolving weights and topology. **10 independent runs** per task. The best controller of each generation was post-evaluated in **100 simulations**. — [Duarte et al. 2016](https://doi.org/10.1371/journal.pone.0151834)
- "Solutions for homing, dispersion, and monitoring were found within **100 generations**, while clustering required up to **400 generations**." — [Duarte et al. 2016](https://doi.org/10.1371/journal.pone.0151834)
- Robustness measures during evolution: conservative noise on sensors, actuators and environment, and multiple trials varying "the number of robots in each sample" and the initial positions. These aided transfer. — [Duarte et al. 2016](https://doi.org/10.1371/journal.pone.0151834)
- Deployment: 8 real robots, also tested with 4 and 6. In monitoring, performance "tends to scale linearly with the number of robots" when robots were removed or added mid-run. — [Duarte et al. 2016](https://doi.org/10.1371/journal.pone.0151834)
- The paper mentions hierarchical or modular control as the route to more complex behaviour. — [Duarte et al. 2016](https://doi.org/10.1371/journal.pone.0151834)

**Swarm-bot aggregation and coordinated motion (Trianni, Dorigo et al.)**
- Evolution sampled group size **randomly between 4 and 8**, and controllers stayed effective at larger group sizes. — [search summary of Dorigo/Trianni et al., "Evolving Self-Organizing Behaviors for a Swarm-bot"](https://dl.acm.org/doi/10.1023/B%3AAURO.0000033973.24945.f3)
- Trianni, Tuci, Ampatzis & Dorigo 2014 (chapter "Evolutionary Swarm Robotics"):
  - Evolved controllers generally scale, but physical interactions can stop scaling to very large N because collision-avoidance manoeuvres become more frequent.
  - Synchronisation controllers scaled well without physical interaction, though some failed at scale through "communicative interference".

  — [Trianni et al. 2014 chapter PDF](https://iridia.ulb.ac.be/~mdorigo/Published_papers/All_Dorigo_papers/TriTucAmpDor2014her.pdf) (via search summary)

**Hasselmann, Ligot, Ruddick & Birattari 2021, Nature Communications**
- Neuro-evolved swarm controllers did well in simulation but poorly on real robots, and the simulated ranking of methods disappeared on hardware. — [IRIDIA ne-swarm page](https://iridia.ulb.ac.be/ne-swarm); [PMC8285396](https://pmc.ncbi.nlm.nih.gov/articles/PMC8285396)
- Hasselmann's thesis: neuro-evolutionary approaches were "greatly affected by the reality gap and outperformed by the modular method" (AutoMoDe). — [ULB thesis record](https://difusion.ulb.ac.be/vufind/Record/ULB-DIPOT:oai:dipot.ulb.ac.be:2013/357114/TOC)

**Review: Birattari group, Frontiers in Robotics & AI 2023, "Recent trends in robot learning and evolution for swarm robotics" (full text read)**
- Evolutionary swarm robotics has traditionally optimised a neural network with a centralized evolutionary algorithm. — [Frontiers 2023](https://doi.org/10.3389/frobt.2023.1134841)
- Earlier work evolved CTRNN modules and assembled them into a hierarchical controller following a manual decomposition. — [Frontiers 2023](https://doi.org/10.3389/frobt.2023.1134841)
- On credit assignment in swarm reinforcement learning: "no generally applicable methodology exists to address this problem and most works use manual credit assignment". — [Frontiers 2023](https://doi.org/10.3389/frobt.2023.1134841)
- Joint action spaces "suffer heavily from the curse of dimensionality". — [Frontiers 2023](https://doi.org/10.3389/frobt.2023.1134841)
- Centralized learning with decentralized execution is usable in simulation. — [Frontiers 2023](https://doi.org/10.3389/frobt.2023.1134841)
- Incremental evolution and curriculum learning are named as candidate approaches. — [Frontiers 2023](https://doi.org/10.3389/frobt.2023.1134841)
- Reward shaping, the reinforcement-learning counterpart of fitness engineering, is critical because "the design process is likely to exploit any unintended local optima". — [Frontiers 2023](https://doi.org/10.3389/frobt.2023.1134841)

### Inferences
- Goldsby and Ferrante agree that **switching cost is the lever**. The project's crowding sense and age input supply exactly the cues a clonal colony needs to break symmetry: age polyethism and density-dependent switching.
- Duarte's practice of varying the number of robots per evaluation sample is a cheap way to make one genome robust across colony sizes. It costs little extra if most evaluations use small N.
- Hasselmann's reality-gap result has a simulation analogue here. A neuro-evolved colony brain can overfit to the training seed, arena or colony size. Randomising those per evaluation is the remedy.

### Gaps
- I could not open Goldsby 2012 in full (PMC returned empty text). Effect sizes per switching cost, population sizes and update counts are unknown.
- Brambilla et al. 2013 and Doncieux et al. 2015 reviews, and the Trianni 2008 book, were not retrieved in this session.
- Tuci's specific work on evolved task allocation was not retrieved.
- The neural vs modular comparison in Hasselmann et al. was not read in full; the per-method numbers are unknown.

---

## Q4. Homogeneous vs heterogeneous teams; how behavioural heterogeneity arises from identical controllers

### Takeaway
Identical genomes produce different behaviour through four routes:
- each individual's own sensory history (position, what it has met);
- stigmergic state, such as cache size or pheromone;
- stochastic decision rules;
- internal memory or state.

Ferrante's de novo specialists used only the first three. The literature's consensus default is homogeneous controllers with team-level selection. It costs less search, but it rules out fixed genetic or morphological castes unless the brain takes a caste or age input.

### Cited Findings
- Ferrante 2015 says clonal teams "precluded other mechanisms of division of labor … based, for instance, on morphological or genetic role specialization". Such castes would need heterogeneous teams or evolvable morphology. — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Ferrante 2015: identical robots "segregate quickly between the source and cache areas". The heterogeneity came from "individual experience, stigmergy and stochastic switching". — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- Waibel 2011 deliberately used a network "that did not allow individuals to memorize past interactions" to prevent preferential treatment. Memory is therefore a known source of differentiation that a stateless network removes. — [Waibel et al. 2011](https://doi.org/10.1371/journal.pbio.1000615)
- Literature exists on partially heterogeneous and clonal versus aclonal team evolution, for example "On the evolution of homogeneous two-robot teams: clonal versus aclonal approaches" (Neural Comput & Applic 2014). Only titles were seen; content was not read. — [Springer](https://link.springer.com/article/10.1007/s00521-014-1594-0); [Cooperative Coevolution of Partially Heterogeneous Multiagent Systems (AAMAS)](https://dl.acm.org/doi/10.5555/2772879.2772919)
- Hecker & Moses 2015 evolved one parameterised ant-inspired strategy for the whole swarm. Evolution favoured "more communication when sensed information was reliable and resources were tightly clustered", "more individual memory when cluster sizes varied", and "wider dispersal as swarm size grew". — [Hecker & Moses 2015, Swarm Intelligence 9(1): 43–70](https://link.springer.com/article/10.1007/s11721-015-0104-z)

### Inferences
- The ant brain has 12 recurrent memory units plus age, energy and crowding inputs. Those are enough to support stable, experience-driven role differentiation, for example "I have recently dug, so keep digging", or role by age. Fixed castes do not require separate genomes: a caste bit or age input fed to one shared genome gives "soldier versus worker" while keeping one genome (inference).
- A modest **tunable noise or temperature in the discrete option choice** is the documented cheap symmetry breaker, standing in for Ferrante's evolvable action probabilities.

### Gaps
- No quantitative study was found comparing how fast clonal-plus-memory and clonal-without-memory specialisation evolve.

---

## Q5. Credit assignment and centralized training with decentralized execution (CTDE) in multi-agent learning: what transfers to a shared genome?

### Takeaway
Multi-agent reinforcement learning has solved per-agent credit with difference rewards, counterfactual baselines (COMA), value factorisation (VDN and QMIX) and centralized critics (MADDPG, MAPPO). All of them rely on gradient-based per-agent updates. A truncation GA on one shared genome only ever sees a scalar per colony, so it gets none of that machinery directly. The ideas that do transfer are:
1. **Difference evaluations as the fitness**: compare the colony with a counterfactual where one ant or role is removed or set to default. This is reported to give large speed-ups when used inside evolutionary algorithms too.
2. **Parameter sharing**: one network for all agents. This is exactly a shared genome, and is reported as the strong default on StarCraft benchmarks.
3. **CTDE**: the trainer may use global information, such as colony-level fitness terms, while the executed brain uses only local senses. GA colony selection already is CTDE.

### Cited Findings
- **Difference rewards (Tumer group):**
  - In multi-objective multi-agent problems, difference rewards gave up to **10× faster learning** and up to **98.4% lower sensitivity to unmodelled disturbances**, and dominated team-reward solutions. — [Yliniemi & Tumer, SEAL 2014](https://link.springer.com/chapter/10.1007/978-3-319-13563-2_35)
  - Inside the NSGA-II evolutionary algorithm, "how and where difference evaluations are incorporated … is critical, and can either provide significant benefits or destroy system performance". — [AAMAS 2015 paper](https://ifaamas.org/Proceedings/aamas2015/aamas/p1635.pdf)
  - **D++** for *tightly coupled* tasks, where several agents must act together for any reward to appear, trained neural-network policies with **166% better performance and 4× faster convergence** than global (G) or plain difference (D) rewards. — [Rahmattalabi, Chung & Tumer, D++](https://robotics.fel.cvut.cz/demur16/wp-content/uploads/2016/06/demur16-rahmattalabi-chung-tumer.pdf)
  - Evolutionary reinforcement learning for multi-agent coordination, which combines evolution on the team reward with gradient learning on per-agent rewards, exists as MERL. — [arXiv 1906.07315](https://arxiv.org/pdf/1906.07315)
- **Parameter sharing:** a benchmarking paper reports that algorithms using parameter sharing performed better on SMAC, and implemented all baselines that way. — [arXiv 2408.07395](https://arxiv.org/pdf/2408.07395) (via search summary)
- The A2PO paper reports median SMAC win rates for MAPPO and QMIX with parameter sharing: near 100% on easy maps and 92.2 vs 96.9 on 2c_vs_64zg. — [arXiv 2302.06205](https://arxiv.org/pdf/2302.06205) (search summary; I could not confirm which algorithm each figure belongs to)
- **QMIX** (Rashid et al. 2018) factorises the joint action-value as a monotonic mixture of per-agent values for CTDE. — [arXiv 1803.11485](https://arxiv.org/pdf/1803.11485)
- **Benchmarking:** Papoudakis et al. compare multi-agent RL algorithms including COMA and MAPPO with and without parameter sharing. — [arXiv 2006.07869](https://arxiv.org/pdf/2006.07869)
- **[unverified, my knowledge]** COMA (Foerster et al. 2018, arXiv 1705.08926) uses a centralized critic with a per-agent counterfactual baseline, which marginalises one agent's action while holding the others fixed. That is a learned difference reward.
- **[unverified, my knowledge]** MADDPG (Lowe et al. 2017, arXiv 1706.02275) uses centralized critics that see all agents' actions, with decentralized actors.
- **[unverified, my knowledge]** MAPPO (Yu et al. 2021/22, "The Surprising Effectiveness of PPO in Cooperative Multi-Agent Games", arXiv 2103.01955) is PPO with a centralized value function and shared parameters, and is competitive with or better than off-policy methods on SMAC, MPE and Hanabi.
- **[unverified, my knowledge]** VDN (Sunehag et al. 2017) uses an additive value decomposition.

### Inferences
- For a GA with one genome per colony, a practical "difference evaluation" means extra rollouts:
  - ablate one behaviour, for example an ant that never digs, or a colony with one ant fewer;
  - measure Δ colony fitness;
  - fold it into the fitness.

  These are costly, since each one needs extra evaluations. They pay off mainly in *tightly coupled* tasks (D++), where a plain global score is nearly always 0 early in evolution.
- A cheaper analogue available to any evolutionary method is **decomposed fitness**, scored per colony as a weighted sum of locally attributable contributions such as cells dug, food delivered and trail followed. Because all ants are clones, summing per-ant contributions is still colony-level selection. It only reduces variance. Tumer's NSGA-II warning applies: badly aligned shaped terms can "destroy system performance".
- Because the ant brain picks among discrete options, a hybrid is possible:
  - evolve the genome;
  - use **within-lifetime per-ant credit** (for example a reward-modulated plasticity term) for fine adaptation.

  This is MERL's pattern, and it is not tested in this domain.
- Gradient-based CTDE (MAPPO with parameter sharing) is the strongest-documented alternative if the project could ever make the sim differentiable or batch it for reinforcement learning. It is listed as context only; it needs vastly more environment steps per update than a GA generation (no source fetched for step counts).

### Gaps
- The original Agogino & Tumer difference-reward papers (e.g. JAAMAS 2008), with their exact speed-ups when evolving neural networks, were not retrieved.
- COMA's, MADDPG's and MAPPO's sample counts were not verified in this session.

---

## Q6. Scaling: train with few agents, deploy with many; incremental/staged evolution; cost

### Takeaway
Evolving at small N and deploying at large N has repeated direct support:
- Ferrante: 4 → 20 robots, 4.4× of an ideal 5×.
- Swarm-bot aggregation: 4–8 → larger groups.
- A 2022 study: evolved at 14, held fitness at 5, 14 and 50.
- Duarte: linear scaling of coverage.

The main failure modes are crowding and physical interference, which grow with density, and communication interference. Incremental evolution from simpler tasks (Gomez & Miikkulainen 1997) produced more effective and more general behaviour than direct evolution, and avoided the trivial "mechanical" strategies direct evolution got stuck on.

### Cited Findings
- **Ferrante:**
  - 4 → 20 robots, with the arena scaled proportionally.
  - 457 ± 72 items vs 103 ± 24.
  - Top-8 average performance ratio of 4.4 against an ideal of 5.

  — [Ferrante et al. 2015](https://doi.org/10.1371/journal.pcbi.1004273)
- **Swarm-bot:** evolution used a random group size of 4–8 and the controllers remained effective for larger groups. — [Dorigo, Trianni et al. 2004, Autonomous Robots](https://dl.acm.org/doi/10.1023/B%3AAURO.0000033973.24945.f3) (via search summary)
- **2022 study, limited sensing:** controllers evolved at swarm size 14 and tested at 5, 14 and 50 were "quite scalable" with close mean fitness. — [arXiv 2203.11585](https://arxiv.org/pdf/2203.11585) (via search summary)
- **Scaling failures:** physical interactions can prevent scaling to very large N, and some synchronisation controllers fail at scale through communicative interference. — [Trianni et al. 2014 chapter](https://iridia.ulb.ac.be/~mdorigo/Published_papers/All_Dorigo_papers/TriTucAmpDor2014her.pdf) (via search summary)
- **Duarte:** varied robot count per training sample; tested at 4, 6 and 8 robots; monitoring performance scaled about linearly with robot count. — [Duarte et al. 2016](https://doi.org/10.1371/journal.pone.0151834)
- **Gomez & Miikkulainen 1997** (Adaptive Behavior 5(3–4): 317–342):
  - The task is made harder in stages, with transitions done by "Delta coding" so that even converged populations adapt.
  - On prey capture, incremental evolution produced "more effective and more general behavior" than direct evolution.
  - Direct evolution produced "mechanical" strategies such as circling.

  — [Tel Aviv seminar summary](https://cs.tau.ac.il/~ruppin/ALseminar/helman.html)
- The Frontiers 2023 review lists incremental evolution and curriculum learning as promising for swarms. — [Frontiers 2023](https://doi.org/10.3389/frobt.2023.1134841)
- **Hecker & Moses 2015:** evolved swarm-size-dependent dispersal ("wider dispersal as swarm size grew"). The best strategy itself depends on N, so N should be varied or matched in training. — [Hecker & Moses 2015](https://link.springer.com/article/10.1007/s11721-015-0104-z)
- **Cost benchmarks from the full texts read:**

  | Study | Generations | Population per generation | Evaluations | Notes |
  |---|---|---|---|---|
  | Ferrante | 2,000 | 100 teams × 4 robots | 3 per team, 5,000 s each | 22 runs; 100–200 nodes; 3 months total |
  | Waibel 2011 | 500 | 200 groups × 8 robots | 10 per group, 60 s each | |
  | Floreano & Keller 2010 | a few hundred | 100 groups of 10 | — | 20 replicates |
  | Duarte | 100–400 | NEAT defaults | — | 10 runs per task |

  Sources: [Ferrante](https://doi.org/10.1371/journal.pcbi.1004273); [Waibel 2011](https://doi.org/10.1371/journal.pbio.1000615); [Floreano & Keller](https://doi.org/10.1371/journal.pbio.1000292); [Duarte](https://doi.org/10.1371/journal.pone.0151834)

### Inferences
- **Feasibility recipe** suggested by the evidence:
  1. Freeze or seed the solo-skill weights.
  2. Evolve colony behaviour at **small N** (4–8 ants) in a small arena, varying N per evaluation.
  3. Score whole colonies with a decomposed fitness to cut noise.
  4. Validate at full N only on elites: post-evaluate the best per generation, as Duarte did with 100 simulations.
  5. Make the environment pay for specialisation, through travel or switching cost.
- **Crowding-sized rooms is a density-dependent behaviour.** Scaling breaks exactly where density-dependent interference changes, so N is not a free parameter for this skill. Train it across the target density range, not just at small N (inference from the swarm-bot and Trianni scaling caveats).
- A truncation GA keeps the elites. Ferrante's 5% elitism and Waibel's roulette wheel both evolved social behaviour. With noisy evaluations, re-evaluating elites each generation is standard practice (inference; not measured in these papers).

### Gaps
- Christensen & Dorigo (incremental evolution of a swarm-bot task) was not retrieved.
- Numeric comparisons in Gomez & Miikkulainen (success rates, generations to solve) were not retrieved.
- No paper found quantifies the evaluation cost of incremental versus direct evolution for a collective task.

---

## Q7. Ant-colony-specific evolved models: pheromone foraging, nest construction

### Takeaway
The most directly relevant ant-colony work (Hecker & Moses 2015) does **not** evolve a neural network. It evolves a small set of parameters of a hand-designed ant-inspired foraging algorithm (CPFA): memory, pheromone-like communication and movement. The evolved parameters adapt systematically to error rates, resource clustering and swarm size, and transfer to physical robots. This is the "pre-adapted building blocks, evolve only the allocation" regime from Ferrante, applied to ants.

### Cited Findings
- **Hecker & Moses 2015** (Swarm Intelligence 9(1): 43–70):
  - The swarm imitates ant behaviours for memory, communication and movement, and evolutionary tuning produced foraging strategies.
  - More communication was evolved when sensing was reliable and resources clustered; more individual memory when cluster sizes varied; wider dispersal with larger swarms.
  - The strategies were evolved in simulation and transferred to physical robots.

  — [Springer abstract](https://link.springer.com/article/10.1007/s11721-015-0104-z)
- The same group has related work on evolved ant-inspired obstacle avoidance and on autonomous charging in swarms (titles only seen). — [ASCE 1](https://ascelibrary.org/doi/10.1061/9780784479971.063); [ASCE 2](https://ascelibrary.org/doi/10.1061/9780784479971.064)
- Spiking networks evolved with a genetic algorithm for foraging showed emergent communication that enhanced foraging (title and abstract only). — [Swarm Intelligence 2023](https://link.springer.com/article/10.1007/s11721-023-00231-6)
- **[unverified, my knowledge]** Panait & Luke (2004, "Learning ant foraging behaviors", ALIFE IX; and "A pheromone-based utility model for collaborative foraging", AAMAS 2004) learned or evolved pheromone-based foraging in which ants deposit and follow two pheromones (to-food and to-nest). This is not verified in this session.

### Inferences
- Ants that already follow trails, combined with colony-level evolution of only the parameters that matter (for example the deposit/follow balance or a dispersal bias), is the ant-specific version of the building-block strategy. Hecker & Moses show it adapts to N and to resource distribution.

### Gaps
- Hecker & Moses GA parameters (population size, generations, swarm sizes, evaluation counts): full text not retrievable.
- No source was found or retrieved on **evolved nest construction** by neural ant agents, i.e. evolved stigmergic building. This is a genuine gap in this session's evidence, not proof that no such work exists.
- Panait & Luke's details are unverified.
