# Curricula and skill reuse: building harder (collective) behaviours on earlier trained skills

Research conditions, so the reader can weigh the evidence. WebFetch failed with DNS errors for every host tried (ncbi, plos, arxiv), and the Consensus search quota ran out. Findings tagged **[verified-snippet]** were confirmed through WebSearch result summaries, which carry abstract-level or section-level text. Findings tagged **[from memory, unverified this session]** come from my prior knowledge of the paper; the URL is the canonical location. Treat their numbers as approximate and check them before quoting. I found no full-text numeric tables this session.

---

## 1. Incremental evolution and shaping: measured speedups compared with direct evolution

### Takeaway
Staged (incremental) evolution reliably beats direct evolution on tasks with **separable subskills and a bootstrap problem**: direct evolution either never starts or settles on a crude local optimum. Its gain is usually reported as "solves it versus doesn't" or "more general behaviour", not as an x-fold speedup. On **highly integrated tasks** it can give no gain at all (Christensen & Dorigo 2006). Alternatives that keep all subgoals active at once (multi-objective subgoals, behavioural diversity) match its results with less hand-ordering.

### Cited findings
- **Gomez & Miikkulainen 1997, *Adaptive Behavior* 5(3–4):317–342.** Prey capture with recurrent nets, in successive stages of rising prey speed and lower prey delay. Each new stage starts from the converged population using **Delta-coding** (a search in the space of weight deltas around the previous best). Direct evolution found "mechanical" strategies such as circling, which beat standing still but are not real pursuit. Incremental evolution produced "more effective and more general behavior" [verified-snippet] — [Gomez & Miikkulainen 1997 record](https://acnpsearch.tweb-dev.unibo.it/singlejournalindex/7965095); [summary](https://cs.tau.ac.il/~ruppin/ALseminar/helman.html). The detail that the direct-evolution controls failed outright on the hardest prey setting, while incremental solved it, is [from memory, unverified]. Exact generation counts were not found.
- **Christensen & Dorigo 2006 (SAB 2006, LNAI 4095:473–484).** NN controllers for s-bots on phototaxis plus hole avoidance. They tested both behavioural decomposition and environmental-complexity increase, and found that "none of the incremental evolutionary strategies performs any better than the non-incremental approach". They attribute this to the task being **highly integrated** [verified-snippet] — [Springer chapter](https://www.springerprofessional.de/incremental-evolution-of-robot-controllers-for-a-highly-integrat/1239180).
- **Mouret & Doncieux (2008 SAB; PhD thesis 2008).** They recast the assumed intermediate steps of an incremental schedule as **simultaneous objectives** in a multi-objective EA ("one objective per step"), so no fixed ordering or switching schedule is needed [verified-snippet: thesis abstract] — [Mouret PhD thesis 2008](https://www.lrde.epita.fr/dload/bulletin/theses/jean-baptiste_mouret.2008.phd.pdf). Earlier: Doncieux & Meyer 2004, "Evolution of neurocontrollers for complex systems: alternatives to the incremental approach" — title only seen — [HAL](https://hal.archives-ouvertes.fr/hal-01520951).
- **Mouret & Doncieux 2009 (CEC), "Overcoming the bootstrap problem … using behavioral diversity".** A behavioural-distance diversity objective with **no subtask staging** was compared against multi-subgoal incremental evolution on a light-seeking robot. The results were "qualitatively similar although the introduced one is less directed" [verified-snippet] — [HAL](https://hal.archives-ouvertes.fr/hal-00473147). Journal follow-up: *Evolutionary Computation* 20(1):91–133, 2012.
- **Harvey, Husbands & Cliff (1994, "Seeing the light"); Harvey's SAGA.** Incremental evolution of visually guided robots through a sequence of progressively harder target tasks, with the population carried over [from memory, unverified] — [Sussex CSRP (canonical)](https://www.researchgate.net/publication/2788557_Seeing_The_Light_Artificial_Evolution_Real_Vision).
- **Bongard 2008, "Behavior chaining: incremental behavior integration for evolutionary robotics" (ALIFE XI).** Behaviours are added one at a time, and a controller must keep **all earlier behaviours** before a new one is added. This avoids losing earlier skills during staged evolution [from memory, unverified]. No URL was verified.
- **Lessin, Fussell & Miikkulainen 2013/2014 (ESP for virtual creatures).** A hand-written "syllabus" of skills. Each learned skill's network is **frozen and encapsulated**, and later skills are evolved as new nets that read the frozen ones, so complex behaviours (for example "turn from fire, strike") built up without forgetting [from memory, unverified] — [UT NN group](https://nn.cs.utexas.edu/?lessin:alife14).
- **Whiteson, Kohl, Miikkulainen & Stone 2005, *Machine Learning* 59, keepaway soccer.** Hand-decomposed tasks with separately evolved sub-behaviours beat monolithic evolution of the whole policy. **Coevolving the parts together** was compared with layered learning that freezes lower layers [from memory, unverified: which variant won depended on the setup] — [UT NN group](https://nn.cs.utexas.edu/?whiteson:mlj05).

### Inferences
- The evidence on incremental evolution is mostly **binary-success comparisons on bootstrap-hard tasks**. I found no paper reporting a clean "N× fewer generations" figure for staged against direct evolution.
- The project's ladder (dig 4 → 8 → 12 rows; fetch; trails) has two parts. The dig steps follow the Gomez pattern of one skill at rising difficulty, which is the best-supported case. The cross-skill steps (dig → fetch → trails) follow the Christensen & Dorigo pattern of integrated skills sharing one network, where staging has been seen to give nothing.
- Mouret & Doncieux's alternative suggests a third design: **keep every rung as a concurrent objective** (or a lexicographic or Pareto rank) rather than switching arenas. It addresses ordering and forgetting at once.

### Gaps
- Generation counts and success rates for Gomez 1997, Mouret–Doncieux 2008 and Christensen–Dorigo 2006 were not retrieved, because full text was unreachable.
- Nolfi's staged and emergent-modularity work (Nolfi 1997, "Using emergent modularity to develop control systems for mobile robots", *Adaptive Behavior* 5(3–4)) was not verified this session.

---

## 2. Hierarchical and modular controllers, and skill repertoires used by a higher level

### Takeaway
Hierarchical composition works best when the lower skills are **frozen primitives** and evolution or learning only has to search an **arbitrator** over them.
- Duarte et al. report that this removes the bootstrap problem and deception, and it lets behaviours be reused across tasks.
- EvoRBC (a quality-diversity repertoire plus an evolved arbitrator) stays strong as locomotion complexity rises, where direct evolution degrades.

The cost is fixed module boundaries. That conflicts with the owner's "one general brain" rule unless the modules live *inside* one genome. Modular networks with gated output modes (Schrum) are the in-genome version.

### Cited findings
- **Duarte, Oliveira & Christensen 2015, "Evolution of hybrid robotic controllers for complex tasks", *J. Intell. Robot. Syst.* 78:463–484.** The goal task is recursively split into subtasks until each one can be evolved or hand-programmed. Behaviours are combined by evolved arbitrators. The authors claim this helps with bootstrapping, avoiding deception, and transfer to real robots, and that it **allows reuse of behaviours in different tasks**. They test a rescue task (all behaviours evolved) and a cleaning task (evolved behaviours plus a hand-coded door-opening behaviour) [verified-snippet] — [ISCTE record](https://ciencia.iscte-iul.pt/publications/evolution-of-hybrid-robotic-controllers-for-complex-tasks/17360?lang=en); [post-print PDF](https://repositorio.iscte-iul.pt/bitstream/10071/9347/6/post_print_int_journal_of_intell_and_robotic_systems_2015.pdf). The multirobot and swarm extension is in Duarte's 2016 PhD thesis — [thesis PDF](https://repositorio.iscte-iul.pt/bitstream/10071/11399/1/phd_miguel_frade_duarte.pdf).
- **EvoRBC, Duarte, Gomes, Oliveira & Christensen.** GECCO 2016, and *IEEE TEC* 22(2):314–328, 2018 (DOI 10.1109/TEVC.2017.2722101). There are two steps:
  1. A QD algorithm (MAP-Elites style) evolves a **repertoire of locomotion primitives**.
  2. A high-level arbitrator that selects from the repertoire is evolved for the task.

  On navigation and foraging, "standard evolutionary approaches are strongly affected by robot locomotion complexity, while EvoRBC consistently evolves high-performing solutions" [verified-snippet] — [GECCO'16 record](https://ciencia.iscte-iul.pt/publications/evorbc-evolutionary-repertoire-based-control-for-robots-with-arbitrary-locomotion-complexity/28863); [2018 paper PDF](https://repositorio.iscte-iul.pt/bitstream/10071/16255/5/1-s2.0-S2210650217308556-main.pdf).
- **Cully, Clune, Tarapore & Mouret 2015, *Nature* 521:503–507 ("Robots that can adapt like animals").** MAP-Elites builds an offline map of high-performing gaits. After damage, Bayesian-optimisation trial and error over the map recovers a gait in **under two minutes**. Tests: a hexapod with 5 damage types and an arm with 14 [verified-snippet] — [arXiv 1407.3501](https://arxiv.org/abs/1407.3501v4). The "~13,000 behaviours in the map" figure is [unverified].
- **Rollins & Schrum 2017 (GECCO Companion), coevolving cooperative predators.**
  - Individual-based fitness outperformed team-based fitness, even on a cooperative task.
  - **Multi-module networks** found intelligent behaviour under either fitness type.
  - Single-module team selection performed poorly, while adding a second module largely fixed it. The authors suggest a module can **hold a marginally useful behaviour in reserve** until it pays off.
  - In the best predators, one module did blocking and herding and the other did the aggressive close-in.

  [verified-snippet] — [arXiv 1703.08577](https://arxiv.org/abs/1703.08577).
- **Schrum & Miikkulainen 2016 (IEEE TCIAIG, Ms. Pac-Man).** "Module mutation" adds output modules inside one network, and preference neurons arbitrate between them. The evolved multimodal agents outperformed single-module ones [from memory, unverified] — [UT NN group](https://nn.cs.utexas.edu/?schrum:tciaig16).
- **Options framework, Sutton, Precup & Singh 1999, *Artificial Intelligence* 112:181–211.** A temporally extended action is (initiation set, policy, termination condition). It is the formal basis for treating trained skills as callable actions [from memory] — [AIJ / UMass PDF](https://people.cs.umass.edu/~barto/courses/cs687/Sutton-Precup-Singh-AIJ99.pdf).
- **Option-critic (Bacon, Harb & Precup, AAAI 2017)** learns option policies and terminations end to end by gradient — [arXiv 1609.05140](https://arxiv.org/abs/1609.05140). **FeUdal Networks (Vezhnevets et al. 2017)**: a Manager sets directional goals in a latent space, and a Worker is rewarded intrinsically for moving towards them — [arXiv 1703.01161](https://arxiv.org/abs/1703.01161). **HIRO (Nachum et al. 2018)**: off-policy two-level HRL with goal relabelling. It was reported as markedly more sample-efficient than flat RL on AntMaze/AntPush — [arXiv 1805.08296](https://arxiv.org/abs/1805.08296). All three are [from memory, unverified]. They are gradient-based and not directly transferable to a GA, and none is about reusing skills across a collective.
- **Yong & Miikkulainen 2009 (IEEE TAMD), Multiagent ESP.** Each agent's network is coevolved in its own subpopulation. This is **more efficient than evolving one central controller**, and cooperation worked best through **stigmergy and role-based responses** rather than communication [verified-snippet] — [PDF](https://www.cs.swarthmore.edu/~meeden/DevelopmentalRobotics/Yong2009.pdf).

### Inferences
- The project's brain is already partly hierarchical: per-verb job scorers act as an arbitrator, and the target scorer is a parameterised primitive. Two in-genome ways to emulate EvoRBC follow from that:
  - **freeze** (or mutation-protect) the sub-networks that trained a solo skill, and let later rungs evolve mainly the arbitration weights and new links;
  - give each verb its own module, Schrum-style.
- Freezing is the most reliable anti-forgetting device in the evolution literature (Lessin; Duarte). It does not break "one general brain found by selection", because the genome stays single and the frozen part was itself found by selection. That is an interpretation of the owner's rule, not a ruling.
- Stigmergic coordination (Yong & Miikkulainen) fits ant trails. Colony behaviour may need few *new* skills, mostly arbitration over existing ones in response to environmental cues such as crowding.

### Gaps
- Duarte's multirobot and swarm numbers and EvoRBC's success rates were not retrieved.
- I found no study where a hierarchical evolved controller's *lower* skills were later fine-tuned rather than frozen, with the forgetting measured.

---

## 3. Modularity and forgetting: what is cheap to bolt onto a GA

### Takeaway
A **connection cost on its own produces modularity, and that modularity reduces forgetting** (Clune 2013; Ellefsen 2015). But the modules found were not task-specific (Velez & Clune 2017), so connection cost is not a forgetting guarantee.

The project's case differs from Ellefsen's: its connection cost *deletes* links whose only value is in untested skills. That is exactly the setting where pruning causes forgetting, and **no modularity argument rescues it unless the old skill is still being selected for.**

The cheapest GA-compatible fixes, roughly from cheapest to dearest:
1. Keep old tasks in the fitness (rehearsal or multi-objective).
2. A milestone pool that re-injects old champions (D'Silva 2005).
3. Per-link mutation and prune protection weighted by importance to old tasks (an EWC analogue; Early 2019).
4. Freezing modules.

### Cited findings
- **Clune, Mouret & Lipson 2013, *Proc. R. Soc. B* 280:20122863.** Selecting for performance **and** low connection cost gave networks "significantly more modular and more evolvable" than performance-only selection. Without the cost, modules did not form; with it, they appeared at once. In this account faster adaptation is a *consequence* of modularity, not its cause [verified-snippet] — [arXiv 1207.2743](https://arxiv.org/pdf/1207.2743.pdf); [press materials](https://neurosciencenews.com/engineers-solve-a-biological-mystery-and-boost-artificial-intelligence/). They used multi-objective selection (performance plus connection cost, with the cost objective applied stochastically, p ≈ 0.25), not a weighted sum [from memory, unverified].
- **Ellefsen, Mouret & Clune 2015, *PLoS Comput Biol* 11(4):e1004128.** Networks with neuromodulated (learning-rate-gating) plasticity were evolved under a connection cost. They became more modular, **learned new skills faster, retained old skills better**, and developed a separate reinforcement-learning module [verified-snippet] — [PMC4383335](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4383335/); [HAL](https://hal.archives-ouvertes.fr/hal-01138901). Task: foraging edible and poisonous food in alternating "summer" and "winter" seasons within a lifetime [from memory, unverified]. Caveat in later work: these networks did **not** form functional modules per skill, so the modularity-and-forgetting hypothesis was "left partly untested" [verified-snippet, via Velez & Clune] — [PMC5690421](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC5690421/).
- **Velez & Clune 2017, *PLoS ONE*, "Diffusion-based neuromodulation can eliminate catastrophic forgetting in simple neural networks".** Simulated diffusing chemicals gate learning within spatial regions. On the Ellefsen diagnostic task this produced task-specific functional modules and **eliminated catastrophic forgetting**. Shown only on simple networks [verified-snippet] — [arXiv 1705.07241](https://arxiv.org/pdf/1705.07241). Note that this protects *lifetime learning*. The project's forgetting happens across *generations* of evolution, so its direct relevance is limited.
- **D'Silva, Janik, Chrien, Stanley & Miikkulainen 2005 (AIIDE), "Retaining Learned Behavior During Real-Time Neuroevolution" (rtNEAT/NERO).** When a skill is mastered, the fittest individuals are kept in a **milestone pool**, and their offspring are periodically re-inserted into the population so the skill is kept. The paper's main question is the pool size and the insertion frequency. It notes that the longer training on a new task runs, the more the skill is forgotten [verified-snippet] — [AAAI OJS](https://ojs.aaai.org/index.php/AIIDE/article/view/18713). Retention numbers were not retrieved.
- **Kirkpatrick et al. 2017, *PNAS* 114(13), Elastic Weight Consolidation.** A quadratic penalty pulls each weight back to its old-task value, weighted by Fisher information (importance). It kept performance on sequential permuted MNIST and on several Atari games learned in sequence, where plain SGD forgot [from memory, unverified] — [arXiv 1612.00796](https://arxiv.org/abs/1612.00796). The GA analogue needs no gradients: estimate each link's importance to old tasks by perturbation or knock-out, then lower its mutation rate or **exempt it from the connection cost and pruning**.
- **Early 2019, weight protection when evolving networks.** An importance-based protection of weights against mutation, for evolutionary rather than gradient training. I could not verify its title, venue or results — [arXiv search (unverified ID)](https://arxiv.org/abs/1904.03178). **Treat this whole entry as unverified.**
- **Soltoggio, Stanley & Risi 2018, "Born to learn" (*Neural Networks*).** A review of evolved plastic networks and of neuromodulation as a gate for task switching [from memory, unverified] — [arXiv 1703.10371](https://arxiv.org/abs/1703.10371).
- **Kashtan & Alon 2005, *PNAS*, "Spontaneous evolution of modularity and network motifs".** Goals that switch periodically between sub-goal combinations ("modularly varying goals") produce modular networks and *faster* evolution than a fixed goal [from memory, unverified] — [PNAS](https://www.pnas.org/doi/10.1073/pnas.0503610102). Relevant because **alternating arenas** is itself a cheap modularity and anti-forgetting pressure.

### Inferences
- Mechanism of the project's failure. A link that serves only an untested skill is fitness-neutral in the current arena, while the connection cost is not. So selection actively removes it, and pruning makes the removal certain. This happens regardless of modularity. The literature's answer is that **the old skill must stay in the selection signal** (Bongard behaviour chaining; Mouret–Doncieux multi-objective; Kashtan varying goals; D'Silva milestone re-injection) or the links must be shielded (EWC analogue; freezing).
- Ranked by cost for a GA:
  1. **Stochastic rehearsal**: each generation, evaluate each genome in the current arena plus one randomly drawn earlier arena, sometimes with a short episode. Cost is about 1.1–1.5× evaluation.
  2. **Retention as a gate or a separate Pareto objective**, not a weighted sum.
  3. **Importance-weighted pruning exemption**: knock out each link once per rung on the previous rung's arena and tag links whose removal drops that fitness. Cost is a one-off N_links evaluations at rung end.
  4. **Milestone re-injection** (near-free).
  5. **Freezing**, which is the strongest but limits later adaptation.
- Velez–Clune diffusion neuromodulation solves lifetime-learning forgetting, not generational forgetting. It is not a bolt-on for this problem.

### Gaps
- No measured comparison of milestone pools against rehearsal against importance-protection within one GA was found.
- The numbers for Ellefsen 2015 (fitness and retention, Q values) were unreachable.

---

## 4. Automatic curricula and open-endedness: do they reduce total compute?

### Takeaway
Automatic curricula (POET, PAIRED, teacher–student, self-paced) show that **some targets cannot be reached by direct optimisation at all, but can be reached by a curriculum plus transfer**. That is a reachability gain, not a compute saving. POET in particular is *expensive*: many environment–agent pairs plus all-pairs transfer checks, which Enhanced POET calls computationally costly. Teacher–student and learning-progress samplers are the cheap ones, and they mainly cut the wasted time spent on rungs that are too easy or too hard.

### Cited findings
- **POET (Wang, Lehman, Clune & Stanley 2019).** It coevolves 2-D bipedal-walker terrains and their ES-optimised agents, with **transfer** of agents between environments. When ES was run from scratch on three POET-generated hard environments, the best of 5 runs scored **17.9, 39.6 and 13.6**, against the **230** success threshold that POET exceeded in each. The direct agents were stuck in a local optimum: they learned to stop before obstacles to avoid the −100 fall penalty. The authors call transfer "essential" [verified-snippet] — [arXiv 1901.01753](https://arxiv.org/pdf/1901.01753); [Uber AI blog](https://www.uber.com/blog/research/paired-open-ended-trailblazer-poet-endlessly-generating-increasingly-complex-and-diverse-learning-environments-and-their-solutions).
- **Enhanced POET (Wang et al. 2020).** The original transfer mechanism is "computationally expensive and prone to false positives"; it adds a stricter transfer threshold and a domain-general novelty measure (PATA-EC) [verified-snippet] — [arXiv 2003.08536](https://arxiv.org/pdf/2003.08536).
- **Teacher–Student Curriculum Learning (Matiisen, Oliver, Cohen & Schulman; IEEE TNNLS 2019).** A bandit teacher picks the subtask with the highest *absolute slope of learning progress*, so it also revisits tasks being forgotten. On LSTM decimal addition and Minecraft mazes it matched or beat hand-designed curricula, and solved a Minecraft maze that direct training did not [from memory, unverified] — [arXiv 1707.00183](https://arxiv.org/abs/1707.00183).
- **PAIRED (Dennis et al., NeurIPS 2020).** An adversary generates environments to maximise *regret* (antagonist return minus protagonist return), so it produces solvable-but-hard levels. This gave better zero-shot transfer to hard mazes than domain randomisation or a minimax adversary [from memory, unverified] — [arXiv 2012.02096](https://arxiv.org/abs/2012.02096).
- **Portelas, Colas, Weng, Hofmann & Oudeyer 2020 (IJCAI survey), "Automatic curriculum learning for deep RL".** It classifies ACL by what is controlled (initial states, goals, environments, opponents) and by the signal used (learning progress, competence, novelty, regret) [from memory, unverified] — [arXiv 2003.04664](https://arxiv.org/abs/2003.04664).
- **Hand-designed against automatic curricula in multi-agent work.** DyMA-CL's authors state that they **manually** designed the curricula and that automatic curriculum generation "not considered in multiagent settings" at the time [verified-snippet] — [arXiv 1909.02790](https://arxiv.org/pdf/1909.02790).

### Inferences
- For a fixed hand ladder, the cheapest automatic piece to borrow is the **learning-progress bandit** (Matiisen): sample arenas in proportion to recent |Δfitness|. Retention arenas whose fitness starts falling are then automatically up-weighted. This doubles as anti-forgetting.
- Do not expect POET-style generation to save compute. Its value is finding stepping stones nobody designed, at high cost.
- Rung-length waste: "hundreds of generations per rung" is partly a stopping-rule question. Advancing on a **plateau in learning progress or a competence threshold**, rather than a fixed count, is the self-paced and teacher–student idea at its simplest.

### Gaps
- No source found gives total compute for automatic against hand curricula on the same evolutionary task.
- Klink et al. 2020 (self-paced deep RL, NeurIPS) was not located: no source or URL was confirmed. From memory only, it moves a task distribution from easy to target by trading reward against KL to the target. Unverified.

---

## 5. Curricula for multi-agent and collective behaviour: few agents, then scaling up

### Takeaway
Agent-count curricula work in MARL: DyMA-CL and EPC both beat training at full scale directly. Two lessons carry over:
1. The policy must handle a variable number of others, through permutation-invariant or attention-pooled inputs.
2. **The best small-population policy is not necessarily the best seed for the large one**, so EPC keeps *several* candidate sets per stage and promotes the most *adaptable*, not the fittest.

### Cited findings
- **DyMA-CL (Wang et al., AAAI 2020, "From Few to More").** It trains on a small-agent scenario and progressively increases the agent count, with three transfer mechanisms between stages. A Dynamic Agent-number Network (DyAN) handles input of varying size. It outperforms deep MARL baselines on large-scale StarCraft II and MAgent tasks. The curricula were hand-designed [verified-snippet] — [arXiv 1909.02790](https://arxiv.org/pdf/1909.02790); [AAAI](https://ojs.aaai.org/index.php/AAAI/article/view/6221). Win-rate numbers were not retrieved.
- **EPC (Long, Zhou, Gupta, Fang, Wu & Wang, ICLR 2020), "Evolutionary Population Curriculum".**
  - It scales agent count in stages (the population doubles per stage, built on MADDPG with population-invariant attention critics).
  - Each stage keeps **multiple agent sets**, does mix-and-match crossover and fine-tuning, and promotes "the sets of agents with the best adaptability to the next stage".
  - Motivation: "agents successfully trained in an early stage with a small population are not necessarily the best candidates for adapting to later stages".
  - Reported to outperform baselines "by a large margin as the number of agents grows exponentially", with better training stability.

  [verified-snippet] — [arXiv 2003.10423](https://arxiv.org/pdf/2003.10423); [ICLR](https://iclr.cc/virtual/2020/poster/1549). The doubling schedule detail is [from memory, unverified].
- **Emergent autocurricula from multi-agent interaction (Baker et al. 2019, hide-and-seek).** Competition alone produced about six successive strategy phases (tool use and so on) without a designed curriculum. It took very large compute (hundreds of millions of episodes) [from memory, unverified] — [arXiv 1909.07528](https://arxiv.org/abs/1909.07528).

### Inferences
- For the ant colony, the analogue of DyMA-CL and EPC is to **grow the colony size** (for example 1 → 4 → 16 → full) within the colony rungs, keeping several lineages at each stage and promoting by *adaptation speed on the next size*. "Rooms sized by crowding" in particular is a behaviour whose signal does not exist at N=1. It needs a density sense that already varies with N, and a rung that varies N.
- The ant brain's senses are local (22 senses), so it is already permutation-invariant by construction; local sensing is the GA-friendly form of DyAN. No architectural change is needed for variable N, but density or crowding must be sensed.

### Gaps
- I found no study applying agent-count curricula to *neuroevolved homogeneous* swarms with measured savings.

---

## 6. Reusing solo-trained controllers in collective settings: does individual pretraining help or hurt?

### Takeaway
I found **no direct controlled comparison** of "solo pretraining, then colony" against "evolve collectively from scratch" for neuroevolved homogeneous swarms. That is a real gap, not a search artefact I could rule out. The adjacent evidence points two ways:
1. **Individual-level selection and fitness often work better than team-level** even on cooperative tasks (Rollins & Schrum 2017). That favours solo pretraining of the individual competences.
2. A converged solo population may sit in a basin poorly placed for the collective task (EPC's core observation). That favours keeping diversity and several lineages across the transition.

For a clonal (homogeneous) colony, team-level selection of genetically identical teams is the standard recommendation for evolving cooperation (Waibel et al. 2009).

### Cited findings
- **Individual against team objectives.** On a cooperative predator task, individual-based fitness beat team-based fitness, and single-module networks under team selection did poorly unless given a second module [verified-snippet] — [arXiv 1703.08577](https://arxiv.org/abs/1703.08577).
- **Population mismatch across scale.** Small-population winners are not the best seeds for larger populations [verified-snippet] — [EPC arXiv 2003.10423](https://arxiv.org/pdf/2003.10423).
- **Decomposition into subpopulations against a central controller.** Separate per-agent subpopulations coevolved more efficiently than a single central controller, and stigmergic, role-based cooperation beat communication [verified-snippet] — [Yong & Miikkulainen 2009](https://www.cs.swarthmore.edu/~meeden/DevelopmentalRobotics/Yong2009.pdf). Nitschke 2003 likewise found a multiple-pools approach superior because it supports specialised roles [verified-snippet] — [Nitschke FLAIRS 2003](https://cdn.aaai.org/FLAIRS/2003/Flairs03-046.pdf).
- **Waibel, Keller & Floreano 2009 (*IEEE TEC* 13(3), "Genetic team composition and level of selection in the evolution of cooperation").** For tasks that need cooperation, **homogeneous teams with team-level selection** performed best. Heterogeneous teams with individual selection failed to evolve altruistic cooperation [from memory, unverified] — [IEEE/EPFL](https://infoscience.epfl.ch/record/129718).
- **Context for swarms.** The link between an individual controller and the emergent group behaviour is poorly understood, which is part of why swarms are evolved in the group setting [verified-snippet] — [arXiv 2203.11585](https://arxiv.org/pdf/2203.11585).
- **The incremental-evolution failure on integrated tasks** (Christensen & Dorigo 2006) is the main caution against assuming solo skills compose additively into a collective task [verified-snippet] — [Springer](https://www.springerprofessional.de/incremental-evolution-of-robot-controllers-for-a-highly-integrat/1239180).

### Inferences
- "One general brain, clonal colony" matches the Waibel homogeneous-team condition. Colony rungs should use **colony-level fitness** (team selection of clones), while solo rungs keep individual fitness. Rollins & Schrum's result suggests keeping an individual-competence term (or retention gate) alongside the colony term, rather than switching wholesale to a team score.
- Solo pretraining is most likely to help where colony behaviour is mostly *arbitration among existing verbs under new cues* (crowding, trail density): division of labour by response thresholds is the classic case. It is least likely to help where the colony behaviour needs a *different internal representation* (Christensen & Dorigo's integration problem).
- Practical hedge, from EPC: carry **several** solo lineages into the colony rungs, not one champion, and select the one that improves fastest in the colony arena.

### Gaps
- There is no controlled solo-first versus collective-from-scratch study with neuroevolution and compute numbers. This is the most important missing evidence for the project's plan.
- The Waibel et al. 2009 details are unverified this session.

---

## Implications for structuring a long curriculum

All of the following is inference, synthesised from the cited findings above.

1. **Never let a rung's fitness stop measuring earlier rungs.** Use stochastic rehearsal of one or two past arenas per evaluation, or retention as a gate or a Pareto objective (Mouret–Doncieux; Bongard; D'Silva; Kashtan). This is the direct fix for "don't eat the cargo" being pruned.
2. **Make pruning and connection cost importance-aware**: exempt links whose knockout hurts any retained arena (an EWC analogue). Connection cost buys modularity (Clune 2013), but in this design it is also the deletion engine.
3. **End a rung on a progress plateau, not a fixed count**, and sample arenas by learning progress (Matiisen). This also catches forgetting early.
4. **Treat the per-verb scorers as an arbitrator over primitives.** Consider protecting (low mutation) the sub-networks that implement mastered primitives, and evolving mostly arbitration for colony rungs (Duarte; EvoRBC; Lessin).
5. **For the colony phase, scale N gradually and keep several lineages** (DyMA-CL; EPC). Use homogeneous-team, colony-level selection plus an individual-competence term (Waibel; Rollins & Schrum).
6. **Expect staging to fail on tightly integrated combinations** (Christensen & Dorigo). Where two skills must interleave moment to moment, train them together from the start of that rung rather than in sequence.
