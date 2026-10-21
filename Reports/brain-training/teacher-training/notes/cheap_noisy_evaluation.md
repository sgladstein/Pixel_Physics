# Cheap evaluation for noisy, expensive evolutionary / policy search

Session note on method: arxiv.org, ar5iv, alphaxiv, emergentmind, lilianweng.github.io and human-competitive.org were all unreachable from this environment (DNS failure from WebFetch, HTTP 403 from the proxy for curl). Every finding below therefore comes from search-result text quoting or summarising the primary paper, not from reading the full paper. Numbers that the search text quoted directly are marked as such. Items I know from background knowledge but could not confirm this session are listed under **Gaps**, flagged "UNVERIFIED", and should not be quoted as fact without checking.

Relevance tag used below: **[CLOSE]** = small policy, noisy episodic control, few-to-moderate evaluations (close to the ant-brain setting); **[FAR]** = deep nets, hyperparameter tuning, or huge compute.

---

## 1. Racing and adaptive resampling in noisy evolutionary search

### Takeaway
Racing is the most directly transferable lever, and the setting already measured a >2x saving from it. The published results agree on the size: adaptive per-candidate sample allocation saves roughly 25–75% of evaluation cost against the *best-tuned* fixed sample count, and much more against a badly tuned one. The bigger benefit is robustness: it removes the need to guess the right number of worlds per brain, which changes during a run. It is a constant-factor lever (about 2–4x), not an order of magnitude on its own.

### Cited Findings
- **Heidrich-Meisner & Igel 2009 (ICML, "Hoeffding and Bernstein races for selecting policies in evolutionary direct policy search").** Adds Hoeffding and empirical-Bernstein racing to CMA-ES for direct policy search. The number of episodes is set per policy, just enough to make the *ranking* (not the fitness value) reliable. Clearly poor policies are rejected after a few roll-outs. Authors claim better learning speed and robustness. The motivation is that the right fixed sample size is hard to know in advance and changes during learning. — [mlanthology record](https://mlanthology.org/icml/2009/heidrichmeisner2009icml-hoeffding); [ICML PDF](https://icml.cc/imls/conferences/2009/papers/229.pdf); [UT Austin reading note](https://cs.utexas.edu/~shivaram/readings/b2hd-Heidrich-MeisnerIgel2009-races.html). **[CLOSE]** The companion paper (same authors, *J. Algorithms* 2009, "Neuroevolution strategies for episodic reinforcement learning") applies CMA-ES to neural policies on five pole-balancing variants. — [GP bibliography](https://gpbib.pmacs.upenn.edu/gp-html/HeidrichMeisner2009152.html)
- A 2021 survey of derivative-free RL calls this line of work "Race-CMA-ES": it adapts the number of roll-outs used to evaluate each parameterised policy. — [Derivative-Free RL review, arXiv 2102.05710](https://arxiv.org/pdf/2102.05710)
- **Martin & Collins 2026, AS-CMA (arXiv 2601.09594; accepted to *Evolutionary Computation*, 12 Jan 2026).** Allocates sampling time to each candidate according to its *predicted sorting difficulty*, aiming for consistent ranking precision. It uses prior knowledge of how noise falls with sample count, and it does **not** re-measure candidates already measured. On four simulated cost landscapes it converged on **98% of runs with no tuning of its one parameter**, and converged **24–65% faster with 29–76% lower total cost than each landscape's *best* static-sampling CMA-ES**. The domain is noisy robot / exoskeleton optimisation, where each evaluation is a costly physical trial. Code: GitHub RussellMMartin/AS-CMA-ES. — [arXiv abs](https://arxiv.org/abs/2601.09594); [arXiv HTML](https://arxiv.org/html/2601.09594v1); [code](https://github.com/RussellMMartin/AS-CMA-ES). **[CLOSE]**: few parameters, noisy, expensive episodes. Note that its baseline is the per-landscape *oracle* static sample count, so the saving against a guessed static count is larger.
- A 2026 follow-up, "Confidence-based Ranking with Adaptive Sampling for Noisy Black-Box Optimisation" (arXiv 2607.14936), exists. I could not read its content. — [arXiv](https://arxiv.org/html/2607.14936)
- **Liu & Teytaud (resampling rules).** They propose a rule that combines several resampling rules into a dynamic stopping rule. They report that population-size control can beat resampling rules (theirs included) in some cases, but degrades under high noise. — [SUSTech PDF](https://cse.sustech.edu.cn/faculty/~liujl/publications/simpleYetEffectiveResampling.pdf)
- **Theory (Astete-Morales, Cauwet & Teytaud).** Simple evolution strategies under additive Gaussian noise of constant variance are bounded at a log-log simple-regret slope of ≥ −1/2. A population-size-controlled ES (pcCMSA-ES) is reported to reach the lower performance bound under strong noise. — [pcCMSA-ES, ResearchGate](https://www.researchgate.net/publication/303951333_Evolution_under_Strong_Noise_A_Self-Adaptive_Evolution_Strategy_Can_Reach_the_Lower_Performance_Bound_-_the_pcCMSA-ES); [Analysis of regret in continuous noisy optimisation, arXiv 1607.06651](https://arxiv.org/pdf/1607.06651)
- **Memory-based methods.** A statistical treatment of noisy fitness found that memory-based methods (reusing past samples of the same or nearby individuals) had higher success rates than implicit averaging without memory, at higher computation cost. — [Springer chapter](https://link.springer.com/chapter/10.1007/978-3-319-26393-9_6)
- A comprehensive survey of evolutionary optimisation in noisy environments (Rakshit et al. line) exists as a Springer chapter. — [Springer](https://link.springer.com/chapter/10.1007/978-981-10-8642-7_3)

### Inferences
- The project's own race (8 worlds, drop the worse half, add worlds for the rest) is a hand-built successive-halving race. Its measured "<half the cost" sits in the same band as AS-CMA's 29–76% saving against the best static setting. Further gains are most likely from three changes:
  1. Make the race *rank-aware*: stop sampling a candidate once its confidence interval clears the truncation cut (top 25 of 100), not after a fixed half/half split. With most mutants clearly worse, most should exit after 1–4 worlds.
  2. **Carry parent samples forward** (the "no re-measurement" and memory-based ideas). An elite parent evaluated on 100+ worlds last generation does not need fresh worlds every generation.
  3. Spend extra worlds only near the cut line, where the ranking is actually in doubt.
- The 1/10 noise-to-gap ratio means a full ranking needs ~100 worlds/brain. A truncation GA needs only the top-25 *set*, and the set is much cheaper. Bernstein races exploit low variance among the clearly bad, which is exactly the "most mutants are clearly worse" regime.
- Teytaud's population-control result suggests a complementary lever: with a mean-based (μ/μ, λ) ES, a larger population averages noise implicitly and may beat resampling. A truncation GA that picks the top quartile does not average, so this applies only if the algorithm changes.

### Gaps
- UNVERIFIED (background knowledge, not confirmed this session): Hansen, Niederberger, Guzzella & Koumoutsakos 2009 (IEEE TEVC) UH-CMA-ES. It measures rank changes after re-evaluating a few candidates and, when noise is detected, raises the evaluation count or step size. It was applied to online combustion-control tuning.
- UNVERIFIED: Jin & Branke 2005 (IEEE TEVC) survey "Evolutionary optimization in uncertain environments", with its noise / robustness / approximation / dynamic taxonomy; it discusses explicit averaging (resampling) versus implicit averaging (larger population).
- Could not read Rakshit, Konar & Das 2017 (Swarm and Evolutionary Computation survey) or Cauwet & Teytaud's specific racing/portfolio papers; no numbers from them.
- No source found that measures racing against a *truncation GA* specifically (as opposed to CMA-ES).

---

## 2. Surrogate-assisted evolution and fitness approximation

### Takeaway
Surrogates have measured "several-fold" savings on small neuroevolution control tasks (cart-pole swing-up, half-cheetah). The catch is that a surrogate over raw genome distance must predict a *noisy* target, and in this setting the signal between near-final brains (1/10 of noise) is likely below what a surrogate trained on a few hundred points can resolve. It is most useful early, for screening out obviously bad mutants: a cheap pre-filter, not a ranker.

### Cited Findings
- **Gaier, Asteroth & Mouret 2018 (GECCO, Best Paper, Complex Systems track): "Data-efficient neuroevolution with kernel-based surrogate models".** Kernel (Gaussian-process-style) surrogates need only a distance between individuals. They reuse NEAT's compatibility distance, which lets variable-topology networks be modelled. On **cart-pole swing-up** and **half-cheetah running**, the surrogate-assisted NEAT (SA-NEAT) matched or beat NEAT using **several times fewer function evaluations**. — [arXiv 1804.05364](https://arxiv.org/pdf/1804.05364); [HAL](https://hal.archives-ouvertes.fr/hal-01768248); [H-BRS record](https://pub.h-brs.de/frontdoor/index/index/year/2018/docId/3646). **[CLOSE]**: small networks, episodic control. The exact ratio was not retrievable.
- **Morse & Stanley 2016 (GECCO), Limited-Evaluation EA (LEEA).** A simple EA scores each individual on only a few training examples per generation (a "minibatch" of the fitness). It was competitive with RMSProp on networks with over 1,000 weights. To counter minibatch noise it uses **fitness inheritance**: an offspring's fitness combines its current small-batch score with a decayed share of its parent's fitness. — [UCF record](https://stars.library.ucf.edu/scopus2015/4462); inheritance described in [Limited Evaluation Evolutionary Optimization of Large Neural Networks, arXiv 1806.09819](https://arxiv.org/pdf/1806.09819) and [LE cooperative co-evolution DE, arXiv 1804.07234](https://arxiv.org/pdf/1804.07234). A 2020 survey notes the approach does not generalise to the whole training sample. — [Neuroevolution in DNNs survey, arXiv 2006.05415](https://arxiv.org/pdf/2006.05415). **[FAR-ish]**: supervised tasks, but the *mechanism* is exactly "few worlds per brain plus inherited estimate".

### Inferences
- **LEEA-style inheritance maps directly onto this setting.** "Brains differ by 1/10 of per-world noise" means each child evaluated on a few worlds is a noisy estimate. A decayed running estimate (parent prior plus child's own worlds) is effectively a Bayesian shrinkage of the child toward its parent. Since mutants are close to parents (small mutation), the prior is informative. That is how LEEA reaches competitiveness on few examples per generation. Risk: it biases selection toward lineages with lucky early scores, unless the decay is fast enough.
- A kernel/GP surrogate over the ~700 weights is feasible for a few hundred archived evaluations. It is a better *pre-screen* (do not simulate mutants predicted far below the parent) than a ranker. Because "most mutants are clearly worse", a pre-screen that skips the bottom 30–50% of mutants is a direct saving. Expected saving is about 1.5–3x, consistent with "several-fold".
- Failure mode: the surrogate is trained on noisy labels, so its predicted differences between near-optimal brains will be dominated by noise. Use it only to discard, never to promote.

### Gaps
- UNVERIFIED: Jin 2011 (*Swarm and Evolutionary Computation*) survey "Surrogate-assisted evolutionary computation: recent advances and future challenges".
- UNVERIFIED: Xu et al. 2014, CMA-NeuroES with fitness prediction; no source returned and no numbers found.
- UNVERIFIED: Stork et al. surrogate neuroevolution (distance-based surrogates for neural networks, ~2019–2020); not retrieved.
- Exact evaluation counts for SA-NEAT against NEAT could not be read (arXiv blocked).

---

## 3. Multi-fidelity / low-fidelity simulation, early termination, successive halving, PBT

### Takeaway
Multi-fidelity is where an order-of-magnitude saving is most plausible for colony-scale evaluation. Two forms apply here:
1. Evaluate on *short* or *small* worlds first and promote survivors to long/large ones (successive halving / Hyperband, measured 5–30x over Bayesian optimisation).
2. Build a deliberately *minimal* simulation of only the interactions the target behaviour needs, and hide everything else under noise (Jakobi).

Early termination of hopeless episodes is the same idea inside a single episode.

### Cited Findings
- **Hyperband (Li, Jamieson, DeSalvo, Rostamizadeh & Talwalkar; ICLR 2017 / JMLR 2018).** JMLR reports Hyperband **5x to 30x faster than popular Bayesian-optimisation methods**. The ICLR version reports "more than an order of magnitude" speedups on neural-network and kernel learning problems. — [JMLR PDF](https://www.jmlr.org/papers/volume18/16-558/16-558.pdf); [ICLR 2017 PDF](https://scispace.com/pdf/hyperband-bandit-based-configuration-evaluation-for-4jylbk6zhp.pdf). **[FAR]**: hyperparameter tuning, but the mechanism is generic.
- **Successive halving** evaluates N configurations on one unit of budget, continues the best half on two units, the best half of those on four, and so on. Hyperband runs several brackets that trade the number of configurations against budget per configuration. A noted limitation is that, being random-search based, it does not use past samples to guide search. — [Hyperband, JMLR](https://www.jmlr.org/papers/volume18/16-558/16-558.pdf); [BOHB, Falkner et al.](https://aad.informatik.uni-freiburg.de/wp-content/uploads/papers/17-BayesOpt-BOHB.pdf). DEHB combines differential evolution with Hyperband. — [DEHB, arXiv 2105.09821](https://arxiv.org/pdf/2105.09821)
- **Population Based Training (Jaderberg et al. 2017, arXiv 1711.09846).** An asynchronous method that uses a fixed compute budget to train a population and its hyperparameters together, producing a hyperparameter *schedule*. It gave faster wall-clock convergence and higher final performance on DeepMind Lab, Atari and StarCraft II. Populations of 10 or fewer gave higher variance and poorer results. — [arXiv](https://arxiv.org/abs/1711.09846v2); [DeepMind blog](https://deepmind.google/discover/blog/population-based-training-of-neural-networks/); small-population caveat quoted in [Collaborating with Humans without Human Data, arXiv 2110.08176](https://arxiv.org/pdf/2110.08176). **[FAR]**
- **Jakobi's minimal simulation / radical envelope of noise (1995–1997).** Model only the robot–environment interactions needed to underpin the target behaviour, and make everything else unreliable through noise, so evolved controllers cannot rely on badly modelled details. Jakobi hypothesised that, built this way, "it does not matter how inaccurate or incomplete" the simulation is. Demonstrated with look-up-table simulations for a Khepera, the Sussex gantry robot, and an octopod walker (wander and avoid objects with IR sensors). — [Jakobi "radical" PDF (SFU mirror)](https://www2.cs.sfu.ca/~vaughan/teaching/889/papers/jakobi_radical.pdf); [Sussex CSRP 497](https://www.sussex.ac.uk/informatics/cogslib/reports/csrp/csrp497.pdf); [Jakobi et al. 1995, noise](https://cse-robotics.engr.tamu.edu/dshell/cs689/papers/jakobi95noise.pdf)
- **Failure mode (Koos et al., the "transferability" work).** Jakobi gave no generic method for choosing what to noise or how. — [arXiv 1307.1870](https://arxiv.org/pdf/1307.1870.pdf)
- **ES is less data-efficient than policy gradients, and parallelism pays for it** (Salimans et al. 2017; numbers in §5). — [arXiv 1703.03864](https://arxiv.org/pdf/1703.03864)

### Inferences
- **Fidelity ladder for colonies.** Rung 0: one ant, short episode, small world. Rung 1: 3–4 ants, medium episode. Rung 2: 12+ ants, full episode. Per-evaluation cost grows roughly with ants × steps, so rung 0 can be 20–100x cheaper than rung 2. A successive-halving race *across rungs*, not only across worlds, is where a 10x saving can come from. The condition is that rung-0 rank correlates with rung-2 rank. **That correlation must be measured (rank correlation on ~50 brains) before trusting it.** Hyperband's brackets exist precisely to hedge against low-fidelity ranks being misleading.
- **Early termination inside an episode.** If a brain has made no progress on a dense intermediate signal (e.g. no food contact by step T/4), stop the episode. This is the within-episode form of a race. The risk is killing late bloomers. Hyperband's insurance against that is to run some brackets with no early cutting.
- **Minimal simulation for colony behaviours.** For colony-level skills that depend on, say, trail following and nest return, an abstracted environment that drops irrelevant physics (no water, no plant growth) is cheaper per step. Jakobi's rule says to *randomise*, not delete, what is dropped, so brains cannot come to depend on its absence. This is also a reality-gap risk back to the full sim: transfer must be checked on full worlds at the top rung.
- PBT is the structure the project already has (truncation-and-copy); its relevant lesson is the population-size floor (~10+).

### Gaps
- UNVERIFIED: Cutler, Walsh & How 2014 (ICRA / IEEE T-RO 2015), multi-fidelity RL with a chain of simulators of increasing fidelity. It reportedly cut real-world samples on an RC car. Not retrieved.
- UNVERIFIED: multi-fidelity Bayesian optimisation (e.g. Kandasamy et al. 2016 MF-GP-UCB; BOCA). Not retrieved.
- No published measurement found of "short-episode fitness with correction" (extrapolating a full-episode score from a partial one) for neuroevolution.

---

## 4. Proxy fitness, dense shaping, novelty search and quality-diversity

### Takeaway
QD/novelty search is a lever for *deceptive* landscapes (where the 600-generation skill step looks like a plateau), not for cost per evaluation. Its best-documented payoff, Cully et al. 2015, is the opposite trade: a big *offline* simulated archive buys extreme *online* data efficiency (adaptation in under two minutes of physical trials). Dense proxy fitness is cheaper and shortens episodes, but invites reward hacking.

### Cited Findings
- **Cully, Clune, Tarapore & Mouret 2015 (*Nature* 521, cover), "Robots that can adapt like animals".** Before deployment, MAP-Elites builds a map of high-performing, behaviourally diverse gaits *in simulation*. After damage, Bayesian optimisation over that map ("Intelligent Trial and Error") finds a working behaviour with few physical trials: recovery in **under two minutes** on a hexapod with two broken legs, and on an eight-joint robotic arm. No self-diagnosis or contingency plan is needed. — [Authors' press release](https://members.loria.fr/JBMouret/ite/press_release_eng.pdf); [Recode replication notebook](https://nbviewer.jupyter.org/github/benureau/recode/blob/master/cully2015/cully2015.ipynb). **[CLOSE in structure]**: expensive trials, small controller space.
- MAP-Elites performance depends strongly on the genotype encoding. — [Imperial, "How do different encodings influence the performance of MAP-Elites?"](https://spiral.imperial.ac.uk/entities/publication/ddd2ec7b-74bc-4f64-8a9d-2d76e2faa924)

### Inferences
- The "one skill step took ~600 generations" observation has two possible causes:
  1. **Noise-limited selection**: the step exists, but the ranker cannot see it. Fix with racing / CRN (§1, §6).
  2. **Deception or plateau**: no fitness gradient toward the step. Fix with novelty / QD, or a shaped intermediate fitness.

  These need different fixes. A trace of whether the step's precursor mutants *appeared* and were *discarded*, against never appearing, would separate them.
- A behaviour descriptor that is cheap and stable (e.g. fraction of time laden, distance travelled from nest, trail cells deposited) can make MAP-Elites selection *more* noise-robust than raw fitness, because niche occupancy depends less on small fitness differences. That is my reasoning, not a published measurement.
- A dense shaped proxy correlated with the colony objective (food delivered per ant-step) lets episodes be shorter. Each proxy must be validated by rank correlation with the true long-episode objective, and watched for exploitation.

### Gaps
- UNVERIFIED: Lehman & Stanley 2011 (*Evolutionary Computation* 19(2)), "Abandoning objectives: evolution through the search for novelty alone". Its deceptive-maze and biped results showed novelty search beating objective search. Not retrieved.
- UNVERIFIED: Mouret & Clune 2015 (arXiv 1504.04909), "Illuminating search spaces by mapping elites". Not retrieved.
- UNVERIFIED: the size of the Cully 2015 archive (~13,000 gaits) and its offline compute (reported as roughly two weeks of simulation). The search could not confirm either.
- UNVERIFIED: NS-ES / NSR-ES (Conti et al. 2018), novelty with ES at scale.

---

## 5. Gradient-based alternatives and hybrids (ES vs RL, GA, ERL, CEM-RL)

### Takeaway
Per *sample*, gradient RL beats ES/GA by roughly 3–10x on the benchmarks where both were measured. Per *wall-clock*, with many cheap parallel CPU workers, ES/GA win because they skip backprop and value-function training and parallelise trivially. For a ~700-link recurrent brain in a non-differentiable pixel sim with sparse colony reward, the gradient route needs a critic, BPTT and a learnable signal. The published hybrids (ERL, CEM-RL) were shown on dense-reward MuJoCo, not on that regime. Expect no reliable order-of-magnitude saving from switching for this setting; a hybrid is a research bet.

### Cited Findings
- **Salimans, Ho, Chen & Sutskever 2017 (arXiv 1703.03864).** ES matched A3C's final performance on most Atari games using **3x to 10x as much data**. ES needs about **3x less computation per frame** because it skips backprop and value-function training. One-hour ES runs used about the same compute as one-day A3C runs, and ES was better on 23 games and worse on 28. On MuJoCo, ES matched TRPO with **no more than 10x the data**. The 3D humanoid was solved in **under 10 minutes on 1,440 workers**. **Common random numbers** (shared seeds) mean workers exchange only scalars, which is what lets it scale past a thousand workers. — [arXiv PDF](https://arxiv.org/pdf/1703.03864); [Semantic Scholar](https://www.semanticscholar.org/paper/Evolution-Strategies-as-a-Scalable-Alternative-to-Salimans-Ho/4ee802a58d32aa049d549d06be440ac947b53987); [OpenAI blog](https://openai.com/index/evolution-strategies). **[FAR]** in network size, but the sample-ratio evidence is the canonical one.
- Chrabaszcz, Loshchilov & Hutter 2018 ("Back to Basics") found a simpler canonical ES matched or beat OpenAI-ES on Atari. — [arXiv 1802.08842 (ar5iv)](https://ar5iv.labs.arxiv.org/html/1802.08842)
- **Such et al. 2017 (Uber, arXiv 1712.06567), Deep GA.** A plain truncation GA (no crossover, Gaussian mutation) trained Atari networks in about **4 hours on one desktop, or ~1 hour on 720 cores**. It was faster in wall-clock than ES, A3C and DQN, while DQN was more data-efficient. On 13 games at 1B frames, GA was best on 3 games, DQN 3, ES 3, A3C 4. The authors caution that wall-clock times are approximate. — [arXiv PDF](https://arxiv.org/pdf/1712.06567); [Uber research page](https://www.uber.com/blog/research/deep-neuroevolution-genetic-algorithms-are-a-competitive-alternative-for-training-deep-neural-networks-for-reinforcement-learning). **[relevant]**: the same algorithm class as the project's GA.
- **ERL (Khadka & Tumer 2018) and CEM-RL (Pourchot & Sigaud 2019).** ERL injects the RL actor into the evolutionary population. CEM-RL (CEM plus TD3/DDPG) instead applies gradient steps to some population members. A survey notes GA-based approaches like ERL generally do worse than evolution strategies. — [Survey: Combining evolution and deep RL for policy search, arXiv 2203.14009](https://arxiv.org/html/2203.14009); [CEM-RL ResearchGate](https://www.researchgate.net/publication/328040648_CEM-RL_Combining_evolutionary_and_gradient-based_methods_for_policy_search). Head-to-head sample numbers were not retrieved.
- **EvoRL** is a GPU-accelerated framework for evolutionary RL hybrids. — [arXiv 2501.15129](https://arxiv.org/pdf/2501.15129)

### Inferences
- Both the cost and the sample ratios point the same way for this setting: CPU-parallel GA with racing, not RL. The 3–10x sample penalty of ES/GA is paid back by parallelism and by not needing differentiability.
- **A gradient is most likely to win** for a component with a dense, local, differentiable-in-expectation signal: e.g. pre-training the sensor/steering part of the brain by imitation or supervised regression on traces of successful ants, then evolving the rest. That is an inference, not a measured result in this domain.

### Gaps
- No source found comparing ES/GA against RL on *small recurrent* policies (~10^3 parameters) under heavy noise with sparse multi-agent reward.
- ERL vs CEM-RL sample counts (CEM-RL paper, ICLR 2019) not retrieved.

---

## 6. Common random numbers / paired evaluation; antithetic sampling

### Takeaway
Evaluating all competitors on the *same* worlds (same seeds) is close to free and removes the *between-world* part of the noise from comparisons. When world difficulty dominates the variance, that can cut the worlds needed for a given ranking reliability several-fold. The failure mode is specific and matters for this project: CRN only helps while paired runs stay correlated. In an agent simulation with a single shared RNG stream, the first behavioural divergence de-synchronises every later random draw, and the correlation collapses.

### Cited Findings
- **PEGASUS (Ng & Jordan 2000)** turns a stochastic MDP/POMDP into a deterministic one by fixing the noise draws in advance (fixed scenarios), so policies are compared on identical randomness. This is the canonical policy-search CRN. — [arXiv 1301.3878](https://arxiv.org/pdf/1301.3878)
- **Sharma 2025/26, "When does pairing seeds reduce variance? Evidence from a multi-agent economic simulation" (arXiv 2512.24145).** Paired evaluation (identical seeds for competitors) gives strict variance reduction whenever outcomes are positively correlated at the seed level. It falls back to independent evaluation when correlation is absent. Empirically, it separated alternatives that stayed statistically inconclusive under independent evaluation at fixed budget. The author notes it was tested on one simulator and one intervention type. — [arXiv](https://arxiv.org/abs/2512.24145v3)
- **"Noise-free comparison of stochastic agent-based simulations using common random numbers" (arXiv 2409.02086).** In agent-based models with one centralised PRNG, the first difference between scenarios causes "a significant loss of correlation due to stochastic random number noise". The proposed fix keys random draws to *events* (per-agent / per-event streams or hashing) rather than to draw order. A 2026 follow-up proposes event-keyed hashing, noting that seed reuse assumes "the same draw index corresponds to the same modeled event across scenarios". — [arXiv 2409.02086](https://arxiv.org/html/2409.02086v2); related [arXiv 2603.11084](https://arxiv.org/pdf/2603.11084)
- **Salimans et al. 2017** use shared random seeds (common random numbers) for communication efficiency across 1,440 workers, and mirrored (antithetic) perturbations in OpenAI-ES. — [arXiv 1703.03864](https://arxiv.org/pdf/1703.03864) (antithetic detail per background knowledge; see Gaps)

### Inferences
- **Highest payoff per line of code in the whole survey, conditional on one check.** Variance decomposes into world-to-world difficulty (shared by all brains on that world, cancelled by pairing) plus within-world chaotic divergence (not cancelled). Measure the decomposition directly: evaluate two near-identical brains on the same 100 worlds and compute the correlation of their per-world scores. If ρ = 0.75, paired variance is 0.5x the unpaired; at ρ = 0.9 it is 0.2x, and the worlds needed for a given ranking reliability drop roughly 5x.
- **The pixel sim is exactly the case 2409.02086 warns about.** One `Rng` driving cell updates, weather and every ant means a single different decision by one ant re-sequences every later draw, so the "same world" stops being the same within a few hundred ticks. Two mitigations:
  1. Key the randomness by (world seed, cell/position, tick) or give each ant and subsystem its own stream, so one ant's divergence does not re-roll the weather.
  2. Score early-episode segments, which stay correlated longer.
- Even with low within-episode correlation, *initial-condition* pairing (same terrain, food and nest layout) still cancels the layout-difficulty component, which is often the largest part.
- Racing (§1) and CRN compose: race on paired differences against the current parents, not on raw scores. Both are measured in units of the same worlds.
- Antithetic sampling applies to ES-style mirrored perturbations (θ ± ε). It does not apply naturally to a truncation GA's independent Gaussian mutants, except by generating mutants in mirrored pairs.

### Gaps
- Could not retrieve 2409.02086's measured correlation-decay numbers.
- UNVERIFIED: mirrored sampling in ES (Brockhoff, Auger, Hansen et al. 2010) and its measured speedup.
- No source found measuring CRN inside a neuroevolution GA loop.

---

## 7. Simulation speed: GPU / batched / vectorised multi-agent simulation

### Takeaway
JAX/GPU vectorisation of grid and multi-agent environments has measured speedups of 10^2–10^4x. The headline numbers are best cases: thousands of environments vectorised together, simple grid logic, often with the learner co-located on the GPU. It is the only lever here that can plausibly give 100x on its own, but it requires rewriting the simulation as array operations. A cellular-automaton pixel sim with branchy per-cell rules and sequential sweeps is not trivially vectorisable.

### Cited Findings
- **JaxMARL (Rutherford et al., NeurIPS 2024 Datasets & Benchmarks).** The pipeline is "around 14 times faster than existing approaches, and up to 12500x when multiple training runs are vectorized". The arXiv version reports **21,500x per agent for QMIX on MPE and 12,500x for IPPO**. Sources conflict on whether 12,500x is per run or vectorised-many-runs; treat it as a vectorised best case. — [NeurIPS PDF](https://proceedings.neurips.cc/paper_files/paper/2024/file/5aee125f052c90e326dcf6f380df94f6-Paper-Datasets_and_Benchmarks_Track.pdf); [arXiv 2311.10090](https://arxiv.org/pdf/2311.10090); [Foerster Lab blog](https://blog.foersterlab.com/jaxmarl/); [AAMAS extended abstract](https://www.ifaamas.org/Proceedings/aamas2024/pdfs/p2444.pdf)
- SMAX (JaxMARL's StarCraft-like env) is claimed ~40,000x faster than SMAC on one 2080 GPU. This comes from a secondary summary; **unverified** against the primary.
- **JaxMARL-HFT** (financial multi-agent): up to **240x** reduction in end-to-end training time against state-of-the-art implementations on the same hardware. — [arXiv 2511.02136](https://arxiv.org/pdf/2511.02136)
- **Assistax** (JAX assistive robotics): **~139x–412x** over Assistive Gym on a single CPU, using 1,024 vectorised envs on an A100. — [arXiv 2507.21638](https://arxiv.org/pdf/2507.21638)
- **Multi-Agent Craftax** (grid world, open-ended): IPPO with 4 agents covers **250M environment steps in 57 minutes on one L40S**. No ratio is given. — [arXiv 2511.04904](https://arxiv.org/pdf/2511.04904)
- **EvoRL**: GPU-accelerated evolutionary RL framework. — [arXiv 2501.15129](https://arxiv.org/pdf/2501.15129)

### Inferences
- **Batch across worlds, not within one.** The cheapest structural win for the existing Rust sim is to run many small worlds per process with the brains' forward passes batched: 100 brains × K worlds become one matrix multiply per tick. With a 700-link RNN, brain inference is likely a minor part of the cost next to the cell sweep. **Profile first.** If the sweep dominates, GPU porting of the brain alone buys nothing.
- For colony training, a *reduced* world (smaller grid, physics subsystems not needed by the skill switched off or frozen) is the low-fidelity rung of §3 and the speed lever together. Chunk sleeping / dirty-rect skipping already exploit a settled world. Ants keep chunks awake, so per-ant cost scales with active area.
- **Cloud cost.** Spot/preemptible CPU VMs with embarrassingly parallel per-world jobs are the regime where Such et al. got ~1 hour on 720 cores. A $100 budget buys roughly 2,000–5,000 vCPU-hours at typical spot prices. That is an order-of-magnitude estimate, not a sourced figure; check current pricing.

### Gaps
- UNVERIFIED: Brax (Freeman et al. 2021) and Isaac Gym (Makoviychuk et al. 2021) speedups, often quoted as 100–1000x over CPU MuJoCo pipelines. Not retrieved.
- UNVERIFIED: Jumanji (InstaDeep) speedups; search returned nothing.
- No published speedup found for *cellular-automaton / falling-sand* physics on GPU combined with agents.

---

## Ranked levers for this setting (synthesis; payoff estimates are inferences unless cited)

1. **Common random numbers with per-entity RNG streams, plus racing on paired differences** (§6 + §1). Expected 2–5x on top of the existing race, depending on measured between-world correlation. Cost: RNG plumbing. Failure: correlation collapse from a shared RNG stream ([arXiv 2409.02086](https://arxiv.org/html/2409.02086v2)).
2. **Fidelity ladder (few ants / short episode → full colony) as successive halving** (§3). Potentially 5–30x ([Hyperband](https://www.jmlr.org/papers/volume18/16-558/16-558.pdf)). It is valid only if low-fidelity rank correlates with high-fidelity rank, which must be measured first.
3. **Smarter racing: stop at confidence of clearing the truncation cut, carry parent samples forward, spend worlds only near the cut** (§1). 1.3–3x over the current race (AS-CMA: 29–76% cost cut against the oracle static count, [arXiv 2601.09594](https://arxiv.org/abs/2601.09594)).
4. **Fitness inheritance / shrinkage toward the parent** (§2, LEEA). Lets each child use fewer worlds. Risk: lineage bias.
5. **Batched multi-world execution and a reduced world for training** (§7). 2–10x on CPU without a rewrite; 100x+ needs a GPU rewrite of the sweep (uncertain).
6. **Surrogate pre-screen of mutants** (§2). 1.5–3x if it can confidently discard the clearly-worse majority ([Gaier et al. 2018](https://arxiv.org/pdf/1804.05364): "several times fewer evaluations").
7. **QD / novelty or shaped proxies** (§4). Not a per-evaluation saving. It can collapse the 600-generation plateau *if* that plateau is deception rather than noise, so diagnose first.
8. **Switching to gradient RL** (§5). Unlikely to pay here: 3–10x better per sample on dense-reward benchmarks ([Salimans et al.](https://arxiv.org/pdf/1703.03864)), but the sim is non-differentiable, the reward sparse and multi-agent, and the parallel GA is already wall-clock competitive ([Such et al.](https://arxiv.org/pdf/1712.06567)).

Several levers multiply: (1)×(2)×(3) together plausibly give the order of magnitude asked for. That is an untested compound estimate, and each factor should be measured separately, paired within seed.
