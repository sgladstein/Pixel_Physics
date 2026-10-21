# Imitation learning from privileged teachers: copying and distillation to speed up training of small neural controllers, and when it fails

Method note: direct arXiv / Semantic Scholar access was blocked from this session (DNS / proxy refusal). Abstracts and citation counts were retrieved through the SciSpace paper index; the links point to the canonical arXiv / proceedings pages. Claims marked **[recalled]** come from my own reading of the paper body: the abstract I retrieved does not state them, and I could not re-check them in this session. Verify those before quoting them as numbers. Everything else is in the abstract I retrieved.

---

## Q1. Privileged teacher -> sensor-limited student: what made it work, and how big were the gains?

### Takeaway
"Train a cheating teacher, then copy it into a student with limited senses" is the main recipe in driving and legged robotics. It works when (a) the teacher's privileged inputs mostly remove a *perception* burden, not *information the student can never reach*; (b) the student is trained on its **own** state distribution (DAgger-style relabelling by a teacher that can be queried anywhere); and (c) the student has **history/memory** that can stand in for the missing state. The reported gains are large: 100% CARLA success, an order of magnitude fewer infractions, and zero-shot sim-to-real legged robots. These are gains in final quality and in what can be learned at all. They are not wall-clock speedups.

### Cited Findings
- **Learning by Cheating (Chen, Zhou, Koltun, Krähenbühl, CoRL 2019).** A privileged agent "cheats by observing the ground-truth layout of the environment and the positions of all traffic participants". It then teaches a purely vision-based agent. Results: "100% success rate on all tasks in the original CARLA benchmark", a new record on NoCrash, and "reduces the frequency of infractions by an order of magnitude compared to the prior state of the art". ~323 citations (SciSpace record of the arXiv version). — [arXiv 1912.12294](https://arxiv.org/abs/1912.12294); [PMLR v100](http://proceedings.mlr.press/v100/chen20a.html)
- LbC mechanics **[recalled]**:
  - The privileged agent is itself trained by behaviour cloning on a bird's-eye-view map.
  - It can be queried on *any* state, including states the student visits, so the student stage is effectively on-policy DAgger.
  - The teacher labels all high-level commands for every frame ("white-box" supervision), which multiplies the labels per sample.
  - The authors attribute the gain to splitting "learning to see" from "learning to act". — [arXiv 1912.12294](https://arxiv.org/abs/1912.12294)
- **Lee et al. 2020 (Science Robotics), blind quadrupedal locomotion.** The controller "acts on a stream of proprioceptive signals". It was trained in simulation and generalised zero-shot to mud, snow, rubble, vegetation and water. ~1,216 citations. — [Science Robotics 10.1126/scirobotics.abc5986](https://www.science.org/doi/10.1126/scirobotics.abc5986); [arXiv 2010.11251](https://arxiv.org/abs/2010.11251)
- Lee et al. teacher-student detail **[recalled]**: the teacher is trained by RL with privileged terrain and contact information. The student is a temporal-convolution network over proprioceptive history. It is trained to match the teacher's action and its latent embedding of the privileged state, on the student's own rollouts. — [arXiv 2010.11251](https://arxiv.org/abs/2010.11251)
- **RMA, Rapid Motor Adaptation (Kumar, Fu, Pathak, Malik, RSS 2021) [recalled; abstract not retrieved].** The base policy is conditioned on an encoded "extrinsics" vector computed from privileged environment parameters. An adaptation module is then trained by supervised regression to predict that latent from recent state-action history, on the student's own rollouts. Note that the *latent* is copied, not the action. — [arXiv 2107.04034](https://arxiv.org/abs/2107.04034)
- **Miki et al. 2022 (Science Robotics), perceptive locomotion [recalled; abstract not retrieved].**
  - The teacher sees noiseless terrain heights.
  - The student gets noisy, partly occluded height samples plus proprioception, through a recurrent "belief encoder" with a gated attention path.
  - An auxiliary decoder reconstructs the privileged state, so the student learns *when to distrust* its exteroception.
  - — [arXiv 2201.08117](https://arxiv.org/abs/2201.08117)
- **Asymmetric actor-critic (Pinto et al. 2017) [recalled].** The critic gets the full simulator state and the actor gets only images. Privileged information enters through the value estimate only, so the actor is never asked to copy an action it cannot justify. — [arXiv 1710.06542](https://arxiv.org/abs/1710.06542)
- A later line confirms the bottleneck is the teacher-student *asymmetry*, not the method. "Coaching a Teachable Student" (Zhang, Huang, Ohn-Bar, CVPR 2023) blames student shortfall on "inherent differences between the input, modeling capacity, and optimization processes of the two agents". It aligns student features to the teacher's bird's-eye-view space and reports **+20.6% driving score** over prior models "without ... on-policy data aggregation or reinforcement learning". — [arXiv 2306.10014](https://arxiv.org/abs/2306.10014)
- TAR (Mousa et al., IROS 2025) names three failure sources of standard teacher-student locomotion: "representation misalignment", "covariate shift due to behavioral cloning" and "lack of deployable adaptation". Its contrastive alignment fix reports **2x faster** training to peak and **+40%** out-of-distribution generalisation. The student surpasses the privileged teacher out of distribution. — [arXiv 2503.20839](https://arxiv.org/abs/2503.20839)
- Concurrent training of teacher and student under RL, instead of the two-stage pipeline, is reported to beat two-stage training on quadruped and biped hardware (Wang et al. 2024, preprint). — [arXiv 2405.10830](https://arxiv.org/abs/2405.10830)

### Inferences
- Every success above has the same structure. The privileged information is either (i) *perceptual* — the student could in principle compute it from its senses, as with LbC's map from camera pixels — or (ii) *slowly varying and identifiable from history*, as with RMA's terrain and friction or Lee's terrain. Neither case is information the student can never reach.
- For the ant, the teacher's "exact world layout, pathfinding, exact hole geometry" is mostly **non-recoverable** from 22 local senses. A route around an unseen obstacle is not inferable from local senses or a short history. That puts the ant in the *imitation-gap* regime of Q2, not in the LbC/RMA regime.
- RMA and Lee copy a **latent** (a compact summary of privileged state), not just an action. For the ant, a teacher that emits targets like "direction to nest", "is there food within N cells" or "is this cell diggable and load-bearing" may be more copyable than "pick option k". Those targets could serve as auxiliary prediction heads on the recurrent memory units.
- The headline gains in these papers are about reaching a quality RL alone could not, or training on sensors where RL is impractical. None reports a clean "x-fold fewer environment steps than RL from scratch" number for the privileged-teacher stage itself.

### Gaps
- No exact wall-clock or sample speedup numbers were retrievable for Lee 2020, RMA or Miki 2022 (abstracts do not state them). Their method details above are [recalled].
- No paper found applies privileged-teacher distillation to a network as small as ~700 weights. All the cited students are deep networks (CNN/TCN/GRU) with roughly 10^5–10^7 parameters.

---

## Q2. The imitation gap: what happens when the teacher's decision depends on information the student cannot sense, and what are the fixes?

### Takeaway
When the teacher acts on information the student lacks, supervised copying **marginalises that information**. The student learns the teacher's *average* action over every world consistent with what it sees. That can be worse than either policy, and it never learns the information-gathering behaviour (exploring, probing) that the teacher never needed. The documented fixes all do one of three things:
- down-weight imitation where the student cannot follow, and switch to reward (ADVISOR, TGRL);
- change the teacher so it only does things the student can copy (A2D, student-informed teacher);
- make the student infer the hidden context from history, *trained on-policy* (Swamy et al.).

### Cited Findings
- **Weihs et al., NeurIPS 2021, "Bridging the Imitation Gap by Adaptive Insubordination".**
  - Mechanism: "when the teaching agent makes decisions with access to privileged information that is unavailable to the student, this information is marginalized during imitation learning, resulting in an 'imitation gap' and, potentially, poor results."
  - Prior work bridges the gap by moving gradually from imitation to RL, but "gradual progression fails for tasks that require frequent switches between exploration and memorization".
  - ADVISOR "dynamically weights imitation and reward-based reinforcement learning losses during training". It beats pure imitation, pure RL, and their sequential and parallel combinations on gridworlds (MiniGrid), multi-agent particle environments and 3D simulators.
  - — [arXiv 2007.12173](https://arxiv.org/abs/2007.12173); [NeurIPS 2021](https://proceedings.neurips.cc/paper/2021/hash/9fc664916bce863561527f06a96f5ff3-Abstract.html)
- ADVISOR weighting **[recalled]**: an auxiliary policy is trained by imitation alone. The imitation weight at each state falls with the divergence between the teacher's distribution and the auxiliary policy's, roughly w = exp(−α·d). Where the student provably cannot match the teacher, the reward loss takes over. — [arXiv 2007.12173](https://arxiv.org/abs/2007.12173)
- **Warrington et al., ICML 2021, "Robust Asymmetric Learning in POMDPs" (A2D).** Identifies "a serious flaw: the expert does not know what the trainee cannot see, and so may encourage actions that are sub-optimal, even unsafe, under partial information." The fix is to train the *expert* "to maximize the expected reward of the imitating agent policy" — adaptive asymmetric DAgger, which trains expert and agent jointly. It "produces an expert policy that the agent can safely imitate, in turn outperforming policies learned by imitating a fixed expert." — [arXiv 2012.15566](https://arxiv.org/abs/2012.15566)
- **Swamy, Choudhury, Bagnell, Wu, NeurIPS 2022, "Sequence Model Imitation Learning with Unobserved Contexts".**
  - When the expert has privileged information, the learner may be able to "eventually identify the hidden context" from history; the problem is "asymptotically realizable".
  - They prove that **on-policy** imitation (with or without a queryable expert) handles this, and **off-policy** imitation (behaviour cloning) does not. The reason: off-policy methods "treat their suboptimal past actions as though they came from the expert", which "manifests as a latching behavior: a naive repetition of past actions."
  - A toy bandit shows "sharp phase transitions" in whether off-policy learners match the expert.
  - On continuous control, "off-policy approaches actually perform worse when given access to history".
  - — [arXiv 2208.02225](https://arxiv.org/abs/2208.02225)
- **Student-Informed Teacher Training (Messikommer, Xing, Aljalbout, Scaramuzza, 2024/25).** "The student might be unable to imitate the teacher's behavior due to partial observability ... because the teacher is trained without considering if the student is capable of imitating." The fix adds "the approximated action difference between teacher and student as a penalty term to the reward function of the teacher" plus a supervised alignment step. It is demonstrated on a maze, vision-based quadrotor flight and manipulation. — [arXiv 2412.09149](https://arxiv.org/abs/2412.09149)
- **TGRL (Shenfeld, Hong, Agrawal, ICML 2023):** "due to information asymmetry, it is not always feasible for the student to perfectly mimic the teacher". The student "dynamically balances between following the teacher's guidance and utilizing reinforcement learning". — [TGRL record](https://arxiv.org/abs/2307.03186) (abstract retrieved via SciSpace; the arXiv ID is [recalled])
- **Vuorio et al. 2024, "A Bayesian Solution to the Imitation Gap".** "If the expert observes the Markov state and the agent does not, then the expert will not demonstrate the information-gathering behavior needed by the agent", and "a naive application of IL can fail catastrophically." Their BIG method infers a reward posterior from demonstrations, so the agent explores when a gap exists and imitates when none does. — [arXiv 2407.00495](https://arxiv.org/abs/2407.00495)
- **Causal-imitation theory (Zhang, Kumor, Bareinboim, NeurIPS 2020; Kumor et al., NeurIPS 2021).** When "sensory inputs of the learner and the expert differ", imitation can be infeasible. They give a graphical criterion that is necessary and sufficient for imitability. "If a demonstrator has access to information unavailable to the imitator ... no matter how perfectly the imitator models its perceived environment ... attempting to reproduce the demonstrator's behavior can lead to poor outcomes." — [NeurIPS 2020](https://proceedings.neurips.cc/paper/2020/hash/8fdd149fcaa7058caccc9c4ad5b0d89a-Abstract.html); [arXiv 2208.06276](https://arxiv.org/abs/2208.06276)
- **Deconfounded Imitation Learning (Vuorio, Brehmer, ... de Haan 2022)**: partial observability "gives rise to hidden confounders". The fix trains an inference model over the latent jointly with a latent-conditional policy, and alternates belief updates with acting. — [arXiv 2211.02667](https://arxiv.org/abs/2211.02667)

### Inferences
- **The likeliest failure mode for the planned ant teacher** is the god-like teacher (exact layout, pathfinding, hole geometry). At a junction where the correct turn depends on unseen geometry, the student sees identical senses across worlds where the teacher turns left and worlds where it turns right. Cross-entropy then trains it toward the teacher's empirical marginal, about 50/50. That is a *graded* outcome but not a useful one. The student also never sees the teacher *search*, because the teacher never needs to.
- Swamy's "latching" result bears directly on the ant's **12 recurrent memory units**. If the student is trained off-policy (on teacher-driven trajectories) with memory, memory can make things *worse*: the student learns to repeat its past actions. Memory pays off only if training data comes from the **student's own rollouts**, relabelled by the teacher (DAgger).
- The cheapest fix fits the project's evolutionary loop: **make the teacher student-aware** (A2D / student-informed teacher in spirit). The hand-written teacher should be restricted to the information the ant can sense or could have stored in memory, so it is a *privileged-computation* teacher rather than a *privileged-information* teacher. Example: it may run exact pathfinding over the cells the ant has already seen, not over the whole map.
- An ADVISOR-style per-state weight maps naturally onto the GA. Keep fitness (reward) as the outer loop. Apply the copy loss only at decisions where a probe ("can a student-sized network predict the teacher's label here from the student's own input history?") says the label is learnable. Everywhere else, fall back to evolution.

### Gaps
- ADVISOR's exact weighting formula and A2D's quantitative results were not retrievable here (the ADVISOR formula is [recalled]).
- No study found measures the imitation gap for a network of the ant's size.

---

## Q3. Causal confusion, copycat problems, compounding error and DAgger: how many teacher queries are needed?

### Takeaway
Behaviour cloning on off-policy data has three documented pathologies:
- **compounding error**: the error bound grows quadratically with horizon, against linearly for DAgger;
- **causal confusion**: more input information can *hurt*;
- **the copycat problem**: with history inputs, the learner predicts the expert's previous action.

The standard cure for all three is on-policy relabelling (DAgger) or targeted interventions. Published query counts are task-specific; I found no general rule.

### Cited Findings
- **de Haan, Jayaraman, Levine, NeurIPS 2019, "Causal Confusion in Imitation Learning".** Ignoring causality "is particularly damaging because of the distributional shift in imitation learning ... it leads to a counter-intuitive 'causal misidentification' phenomenon: access to more information can yield worse performance." The fix is "targeted interventions — either environment interaction or expert queries — to determine the correct causal model". It is shown on benchmark control and driving, against DAgger. ~262 citations. — [arXiv 1905.11979](https://arxiv.org/abs/1905.11979)
- **Wen et al., NeurIPS 2020, "Fighting Copycat Agents in Behavioral Cloning from Observation Histories".** "A common instance of this causal confusion occurs in partially observed settings when expert actions are strongly correlated over time: the imitator learns to cheat by predicting the expert's previous action, rather than the next action." Their fix is an adversarial feature representation that "removes excess information about the previous expert action". — [arXiv 2010.14876](https://arxiv.org/abs/2010.14876)
- **Pfrommer et al. 2023**: "Causally confused agents produce low open-loop supervised loss but poor closed-loop performance upon deployment." — [arXiv 2307.15980](https://arxiv.org/abs/2307.15980)
- **OREO (Park et al., NeurIPS 2021)**: randomly masking whole semantic objects during cloning prevents the policy "exploiting nuisance variables strongly correlated with expert actions". It improves behaviour cloning on Atari and CARLA. — [arXiv 2110.14118](https://arxiv.org/abs/2110.14118)
- **DAgger (Ross, Gordon, Bagnell, AISTATS 2011) [recalled; abstract not retrieved].**
  - Behaviour cloning's cost bound grows as O(T²ε) over horizon T, because errors take the learner to states the expert never visited.
  - DAgger iteratively runs the learner's policy, has the expert label the visited states, aggregates all data and retrains. This gives a bound linear in T.
  - It was demonstrated on Super Tux Kart and Super Mario Bros.
  - — [arXiv 1011.0686](https://arxiv.org/abs/1011.0686)
- **Swamy et al. 2022** (above) extends this argument. With hidden context, on-policy methods "provably learn to recover from their initially suboptimal actions", and off-policy methods do not. — [arXiv 2208.02225](https://arxiv.org/abs/2208.02225)
- **Active querying**: "query the demonstrator only in hard-to-learn and uncertain regions of the state space", prioritising states where expert and policy disagree most, outperformed prior imitation methods on MuJoCo exploration efficiency (Bougie & Ichise 2021). — [AAMAS journal 10.1007/s10458-021-09527-5](https://link.springer.com/article/10.1007/s10458-021-09527-5)

### Inferences
- **Practical query budget for the ant.** The teacher is a hand-written program run inside the simulator, so a query costs only teacher CPU time, not human effort. DAgger's main practical cost (expert labelling time) is therefore near zero. The real cost is the simulation rollouts the student must run to generate its own state distribution. The obvious design: label **every** decision the student makes on its own rollouts, aggregate across iterations, and run a handful of DAgger rounds. The source papers give no general count; 5–20 rounds is common in the DAgger literature, but this is not verified here.
- The ant's inputs include recurrent memory and probably "what I did last" signals, such as carrying state and last move direction. That is exactly the copycat set-up. Expect low training loss with poor closed-loop behaviour if trained on teacher-driven trajectories.
- **Check that will catch it**: measure *closed-loop* fitness of the copied brain, not label accuracy. Low open-loop loss is explicitly documented as uninformative (Pfrommer).
- Causal confusion argues for **not** feeding the student inputs that are effects of the teacher's choices. If a sense is a consequence of the previous action (e.g. "moved last tick"), consider masking or dropping it during copying, in the spirit of OREO.

### Gaps
- No paper found states a general count of DAgger iterations or teacher queries needed. Counts are task-specific. I did not retrieve DAgger's own iteration counts.
- I could not find work on causal confusion specifically in recurrent controllers with *discrete-option* outputs.

---

## Q4. Demonstrations to accelerate RL or evolution: measured gains, including neuroevolution hybrids

### Takeaway
In gradient RL, adding demonstrations gives large, well-measured speedups:
- DQfD: a strong DQN baseline needs on average **83M steps** to catch up;
- Nair et al.: an **order-of-magnitude** speedup;
- kickstarting: **~10x fewer steps** (Q5).

The common design is to *mix* the imitation loss with the task objective and let the task objective win. The mixing mechanism lets the learner surpass the demonstrator; DQfD did so on 14/42 games. For **neuroevolution**, the evidence is thinner and older but consistent:
- Lamarckian write-back of locally learned weights speeds evolution;
- Baldwinian (learning without write-back) was inefficient on hard problems;
- human-assisted neuroevolution helps on some tasks and not others.

### Cited Findings
- **DQfD (Hester et al., AAAI 2018).** It combines temporal-difference updates with "supervised classification of the demonstrator's actions". It "starts with better scores on the first million steps on 41 of 42 games", and "on average it takes PDD DQN 83 million steps to catch up to DQfD's performance". It "learns to out-perform the best demonstration given in 14 of 42 games". The earlier version also showed DQfD "learns faster than DQN even when given poor demonstration data". ~871 citations. — [arXiv 1704.03732](https://arxiv.org/abs/1704.03732); [AAAI](https://ojs.aaai.org/index.php/AAAI/article/view/11757)
- **Nair et al., ICRA 2018, "Overcoming Exploration in RL with Demonstrations".** Built on DDPG + HER, it "provides an order of magnitude of speedup over RL on simulated robotics tasks" for sparse-reward block stacking, using "a small set of demonstrations". It "solve[s] tasks not solvable by either RL or behavior cloning alone, and often ends up outperforming the demonstrator policy." ~993 citations. — [arXiv 1709.10089](https://arxiv.org/abs/1709.10089)
- Nair et al. mechanism **[recalled]**: the behaviour-cloning loss uses a **Q-filter**. It is applied only at states where the critic judges the demonstrator's action better than the current policy's, so bad demonstrations stop pulling the policy. — [arXiv 1709.10089](https://arxiv.org/abs/1709.10089)
- **DAPG (Rajeswaran et al., RSS 2018) [recalled; abstract not retrieved].**
  - It pre-trains by behaviour cloning on a small set of virtual-reality demonstrations (~25 per task), then fine-tunes with natural policy gradient.
  - It adds an augmented loss on the demonstrations whose weight decays over training.
  - On a 24-DoF simulated hand it reports roughly an order of magnitude lower sample complexity than RL from scratch.
  - — [arXiv 1709.10087](https://arxiv.org/abs/1709.10087)
- **Self-Imitation Learning (Oh, Guo, Singh, Lee, ICML 2018) [recalled; abstract not retrieved].** The agent imitates its own past actions only where the observed return exceeded its value estimate. It uses a clipped advantage, (R − V)₊, so only "better than expected" decisions are copied. It improved A2C and PPO, notably on hard-exploration Atari games such as Montezuma's Revenge. — [arXiv 1806.05635](https://arxiv.org/abs/1806.05635)
- **Choice of demonstrator matters**: when choosing among demonstrators of varying skill, "it is not always the best policy to draw demonstrations from the best performing demonstrator". Careful selection gave sample-efficiency gains across nine Atari games ("Zone of Proximal Development", Seita et al. 2019). — [arXiv 1910.12154](https://arxiv.org/abs/1910.12154)
- **Lamarckian vs Baldwinian neurocontrollers (Ku & Mak, CEC 2000).** "While the Baldwinian and the two-phase approaches are inefficient in improving the evolution process for difficult problems, the Lamarckian approach is able to speed up the training process." Also, "allowing the local search to learn another related task can assist the evolutionary search." — [IEEE 10.1109/CEC.2000.870840](https://ieeexplore.ieee.org/document/870840/)
- **Rasmussen 2023 (simulation study)**: with low learning cost, Lamarckian beats Baldwinian and Darwinian. "The evolutionary advantage decreases for Lamarckian and Baldwinian strategies as their learning costs increase". — [arXiv 2305.00491](https://arxiv.org/abs/2305.00491)
- **Human-assisted neuroevolution (Karpov, Valsalam, Miikkulainen, GECCO 2011).** In a human-subject experiment comparing advice, shaping and demonstration added to neuroevolution: "(1) human-assisted evolution outperforms a manual scripting approach, (2) unassisted evolution performs consistently well across domains, and (3) different methods of assisting neuroevolution outperform unassisted evolution on different tasks." — [GECCO 2011 PDF](http://nn.cs.utexas.edu/downloads/papers/ikarpov.gecco11.pdf)
- **Evolution + gradient hybrids**: CEM-RL (Pourchot & Sigaud, ICLR 2019) combines the cross-entropy method with TD3 gradient steps applied to half the population. It "offers a satisfactory trade-off between performance and sample efficiency" against competitors. — [arXiv 1810.01222](https://arxiv.org/abs/1810.01222)
- **ERL (Khadka & Tumer, NeurIPS 2018) [recalled]**: a gradient-trained RL actor is periodically copied into the evolutionary population (Lamarckian injection) and survives only if its fitness is competitive. — [arXiv 1805.07917](https://arxiv.org/abs/1805.07917)
- **Evolution choosing *what to imitate*** (Eliya & Herrmann 2020, "Evolutionary Selective Imitation"):
  - Fitness is computed by training a fresh network on an evolving set of samples by supervised learning and running it.
  - The best trajectories' samples replace the set each iteration.
  - It solved BipedalWalker by "imitating an evolutionarily selected set of only 25 samples with a NN with only a few thousand parameters".
  - — [arXiv 2009.08403](https://arxiv.org/abs/2009.08403)
- **Neuroevolution can itself imitate**: co-evolutionary adversarial imitation evolved recurrent networks matching pre-trained agents' scores on 8 Gym control tasks (Le Clei & Bellec 2023). — [arXiv 2304.12432](https://arxiv.org/abs/2304.12432)

### Inferences
- The best-supported pattern for the project is **Lamarckian copying inside the GA**: periodically take elite brains, run a few gradient steps of teacher-copy loss, write the weights back, and let truncation selection judge them on noisy fitness. Selection then acts as the Q-filter or ADVISOR weight. A copy that hurts fitness is simply not selected. That is the ERL / CEM-RL pattern with imitation in place of the RL gradient.
- Fitness is very noisy (~100 worlds per reliable ranking). The value of a dense supervised signal is that a **copy loss has no world-to-world noise**. Each labelled decision is a deterministic target, so gradient copying extracts far more signal per simulated tick than a single scalar fitness per world. This is the core reason distillation should beat pure evolution here (inference, not measured).
- Self-imitation of the brain's own successful trips should use the SIL rule: copy only decisions on trips that did better than that brain's (or the population's) expected return. Copying every successful trip uncritically reinforces lucky noise. With per-world noise ~10x the between-brain differences, a "successful" trip is weak evidence that the decisions in it were good.
- Seita et al.'s ZPD result supports a teacher that is **not maximally strong**. A teacher whose behaviour a 700-link brain can represent may teach better than an optimal one.

### Gaps
- No peer-reviewed study found reports a measured speedup for **imitation-seeded or imitation-guided GA training of small recurrent controllers with very noisy fitness**, which is the project's exact regime. ERL/CEM-RL use RL gradients, not teacher labels.
- DAPG and SIL numbers are [recalled] and should be checked against the papers.

---

## Q5. Policy distillation into small networks, and kickstarting (an annealed teacher signal so the student can surpass the teacher)

### Takeaway
Distillation compresses policies dramatically:
- Rusu et al.: students "dramatically smaller" at expert level;
- a later Atari study: full distillation at **1.7%** of the teacher's size in most games.

Kickstarting shows that an imitation term with a **decaying, self-tuned weight** gives roughly **10x data efficiency** and a student that ends **42% above** a from-scratch agent. The common thread: a soft-target / KL loss, plus a mechanism that turns the teacher off once it stops helping.

### Cited Findings
- **Policy Distillation (Rusu et al., ICLR 2016).** It extracts a DQN policy into "a new network that performs at the expert level while being dramatically smaller and more efficient". "The multi-task distilled agent outperforms the single-task teachers as well as a jointly-trained DQN agent." ~425 citations. — [arXiv 1511.06295](https://arxiv.org/abs/1511.06295)
- Rusu et al. loss detail **[recalled]**: KL divergence against the teacher's Q-values sharpened with a **low temperature** worked better than mean-squared error on Q-values or negative log-likelihood on the argmax action. The soft targets carry the teacher's preference *ranking*, not only its top choice. — [arXiv 1511.06295](https://arxiv.org/abs/1511.06295)
- **Kickstarting (Schmitt et al., 2018, DeepMind).**
  - The method "places no constraints on the architecture of the teacher or student agents, and it regulates itself to allow the students to surpass their teachers".
  - On DMLab-30 with task-specialist teachers, the student matched a from-scratch agent "in almost 10x fewer steps, and surpass[ed] its final performance by 42 percent".
  - It uses population-based training to tune the imitation weight. ~120 citations.
  - — [arXiv 1803.03835](https://arxiv.org/abs/1803.03835)
- **Real-time policy distillation (Sun & Fazli 2019)** trains teacher and student simultaneously. This halves distillation time and "make[s] it possible for extremely small student models to learn skills at the expert level": "full distillation in most games, even with compression ratios up to 1.7%." — [arXiv 1912.12630](https://arxiv.org/abs/1912.12630)
- **Ensemble Policy Distillation (Sun & Zhang, IJCNN 2022)**: with an ensemble of teachers, "highly compressed student networks ... even outperform teachers for numerous Atari games". It also notes that the ensemble causes a data-distribution mismatch, which it addresses. — [IEEE 10.1109/IJCNN55064.2022.9892503](https://ieeexplore.ieee.org/document/9892503)
- **Capacity gap in distillation (Cho & Hariharan, ICCV 2019) [recalled]**: in supervised image classification, a larger, more accurate teacher does not always give a better small student. An early-stopped (less sharp) teacher can teach better. — [arXiv 1910.01348](https://arxiv.org/abs/1910.01348)
- **Soft targets (Hinton, Vinyals, Dean 2015) [recalled]**: the teacher's full output distribution at raised temperature carries "dark knowledge" about relative preferences. That information is lost with hard labels. — [arXiv 1503.02531](https://arxiv.org/abs/1503.02531)

### Inferences
- The ant's scorer outputs a score per discrete option, followed by a softmax-like choice. That is exactly the distillation set-up, so the teacher should emit **a score or distribution over all options**, not just the argmax. Examples: a cost-to-go for each candidate dig cell, or each walk target. Train with KL at a tuned temperature. This gives the student a graded ranking, matching the project's "distribution, not binary" law, and gives more bits per decision.
- **Kickstarting is the template for "teacher signal annealed away".** In GA terms: fitness = task fitness − λ·(copy loss), or Lamarckian copy steps at rate λ, with λ itself evolved or decayed. Kickstarting's population-based tuning of λ is directly an evolutionary mechanism, so it fits a truncation GA without new machinery.
- A 700-link student is extreme even by the 1.7% compression standard. The teacher should be made representable (Q2) before compression is attempted. Combine with the ZPD and capacity-gap findings: start with a deliberately weak, local teacher.

### Gaps
- Exact Rusu compression and temperature numbers are [recalled]. Their table values were not retrieved.
- No study found distils into networks with fewer than ~10^4 parameters with recurrent memory.

---

## Q6. Imitation in multi-agent settings: a centralised expert teaching decentralised agents, and decisions that depend on group state

### Takeaway
The standard pattern trains decentralised policies by imitating a **centralised expert that sees global state**:
- Tolstaya et al. 2019, flocking;
- Li et al. 2020, multi-robot path planning;
- later work using the Hungarian algorithm for assignment.

The documented fix for group-level information is **communication**: graph neural networks that aggregate multi-hop neighbour information, or delayed or temporal aggregation. Without it, purely local imitation of a centralised controller is reported as **insufficient**. Results reach "close to expert" and generalise to teams 5–100x larger than in training. RL fine-tuning after imitation is a common second stage.

### Cited Findings
- **Tolstaya et al., CoRL 2019, "Learning Decentralized Controllers for Robot Swarms with Graph Neural Networks".** They "learn local controllers that require only local information and communications at test time by imitating the policy of centralized controllers using global information at training time". A single shared local controller "exploits information from distant teammates using only local communication interchanges" via aggregation GNNs over time-varying graphs, applied to flocking. "A decreasing communication radius and faster velocities increase the value of multi-hop information." ~106 citations. — [arXiv 1903.10527](https://arxiv.org/abs/1903.10527); [PMLR v100](http://proceedings.mlr.press/v100/tolstaya20a.html)
- **Li, Gama, Ribeiro, Prorok, IROS 2020, "Graph Neural Networks for Decentralized Multi-Robot Path Planning".** A CNN encodes local observations and a GNN "communicates these features among robots". It is trained "to imitate an expert algorithm" (a centralised planner) and achieves "a performance close to that of our expert algorithm". It generalises "to previously unseen cases (involving larger environments and larger robot teams)". ~207 citations. — [arXiv 1912.06095](https://arxiv.org/abs/1912.06095); [IEEE 10.1109/IROS45743.2020.9341668](https://ieeexplore.ieee.org/document/9341668)
- **MAGAT (Li, Lin, Liu, Prorok, RA-L 2021)** adds message-dependent attention. It is "close to that of a coupled centralized expert algorithm" and achieves "a 47% improvement over the benchmark success rate, even in very large-scale instances that are ×100 larger than the training instances." — [arXiv 2011.13219](https://arxiv.org/abs/2011.13219)
- **Gama, Li, Tolstaya, Prorok, Ribeiro, IEEE TSP 2022**: GNNs "learn appropriate decentralized controllers by means of imitation learning, leverage their permutation invariance properties to successfully scale to larger teams and transfer to unseen scenarios", on flocking and multi-agent path planning. — [arXiv 2012.14906](https://arxiv.org/abs/2012.14906)
- **The limit of local-only imitation**: "relying solely on the states of immediate neighbors is insufficient to imitate a centralized control policy". L-hop delayed aggregation helps but "can lead to a lack of consensus among distant flock members and the formation of small clusters". Adding temporal history of immediate neighbours (a spatio-temporal GNN) fixes this and was deployed on Crazyflie drones (Chen et al. 2023). — [arXiv 2309.17437](https://arxiv.org/abs/2309.17437)
- **Assignment / division-of-labour analogue (Muthusamy et al. 2024).** In unlabelled motion planning each robot knows only its k-nearest robots and targets. A GNN is trained by imitation "with the centralized Hungarian algorithm as the expert policy, and further fine-tune[d] ... using reinforcement learning to avoid collisions and enhance performance". Trained on 100 robots, it generalises to 500 and outperforms the state of the art by 8.6% on average. — [arXiv 2409.19829](https://arxiv.org/abs/2409.19829)
- **Heterogeneous segregation (Omotuyi & Kumar 2022)**: imitating a centralised controller with a multi-hop aggregation GNN "outperformed a local controller that considers only immediate neighbors and achieved similar performance to the centralized controller". — [IEEE 10.1109/MARSS55884.2022.9870482](https://ieeexplore.ieee.org/document/9870482)
- ADVISOR (Q2) was also evaluated on **multi-agent particle environments**, which places the imitation-gap problem in multi-agent settings too. — [arXiv 2007.12173](https://arxiv.org/abs/2007.12173)

### Inferences
- **Direct warning for colony behaviour.** A centralised teacher that assigns tasks from colony-level quantities ("the colony needs 3 more diggers") has the imitation gap at its worst, unless the ant has a channel that carries that quantity. In the robotics work that channel is explicit message passing. In ants it would be **stigmergy and contact**: pheromone or trail concentration, encounter rate with nestmates of each role, stock levels at the nest. A colony-level teacher will only be copyable if some sense the ant has is a usable *proxy* for the group quantity.
- **Before writing a colony teacher**, check copyability: can the label be predicted from the ant's own 22 senses plus memory history (e.g. the encounter-rate integral)? If not, add a sense (a local signal field) rather than a bigger network. This is the multi-agent version of "privileged computation, not privileged information".
- The literature's two-stage pattern — imitate the centralised expert, then fine-tune with the task objective — maps onto "copy, then evolve". The Hungarian example shows the fine-tune stage is where collision-like group failures get fixed.

### Gaps
- No multi-agent imitation study found where agents communicate only through a shared environment field (stigmergy), not explicit messages. That is the case relevant to ants.
- Paulos et al. (named in the brief) was not found as a separate retrievable paper. James Paulos is a co-author of Tolstaya et al. 2019, which may be the intended reference.

---

## Q7. Practical recommendations: label quality vs quantity, soft vs hard targets, partial labelling, recurrent memory, checking copyability

### Takeaway
The evidence converges on a short list:
- label on the **student's own** state distribution (DAgger), with full option-ranking **soft targets**;
- let the teacher **abstain**, or down-weight labels, where the student cannot follow;
- for memory, train on long enough student-driven sequences, and beware copycat/latching shortcuts;
- **measure copyability before training**: the imitation gap is the first-order risk;
- keep the task objective (fitness) as the final arbiter and anneal the teacher away.

### Cited Findings
- Label distribution beats label volume. Off-policy behaviour cloning on history "perform[s] worse when given access to history", while on-policy learners use it to identify context (Swamy et al.). — [arXiv 2208.02225](https://arxiv.org/abs/2208.02225)
- Low open-loop loss does not imply good closed-loop behaviour: "causally confused agents produce low open-loop supervised loss but poor closed-loop performance". — [arXiv 2307.15980](https://arxiv.org/abs/2307.15980)
- More input information can lower performance under behaviour cloning (de Haan et al.). — [arXiv 1905.11979](https://arxiv.org/abs/1905.11979)
- Partial labelling, or weighting where the teacher is followable, is the core of ADVISOR (per-state loss weighting), TGRL (dynamic balance) and Nair et al. ([recalled] Q-filter). — [arXiv 2007.12173](https://arxiv.org/abs/2007.12173); [arXiv 1709.10089](https://arxiv.org/abs/1709.10089)
- Querying only uncertain or disagreeing regions improves efficiency (Bougie & Ichise). — [10.1007/s10458-021-09527-5](https://link.springer.com/article/10.1007/s10458-021-09527-5)
- The strongest teacher is not always the best teacher (Seita et al., ZPD). — [arXiv 1910.12154](https://arxiv.org/abs/1910.12154)
- Make the teacher student-aware: penalise the teacher for actions the student cannot reproduce (Messikommer et al.), or train the expert for the student's reward (A2D). — [arXiv 2412.09149](https://arxiv.org/abs/2412.09149); [arXiv 2012.15566](https://arxiv.org/abs/2012.15566)
- Annealing the teacher out with a self-tuned weight lets the student surpass it (kickstarting: 10x fewer steps, +42% final). — [arXiv 1803.03835](https://arxiv.org/abs/1803.03835)
- Imitating an expert's previous action through history inputs is a known shortcut (copycat). — [arXiv 2010.14876](https://arxiv.org/abs/2010.14876)

### Inferences (concrete guidance for the ant brain; my synthesis, not measured)
1. **Copyability probe first (cheap, decisive).**
   - Log (student-observation history, teacher label) pairs from student-driven rollouts.
   - Fit the *same-capacity* network (or a slightly larger one as an upper bound) by supervised learning.
   - Report held-out agreement **per decision type** (dig target, walk target, drop), with two references: a "teacher with the same senses" upper bound and a "majority label" baseline.
   - Positive control: include one decision whose label is a pure function of current senses, which must reach ~100%.
   - Negative control: shuffle labels across worlds, which must fall to baseline.
   - Decision types near baseline mark imitation gaps; exclude them from the copy loss, or redesign the teacher.
2. **Teacher design.** Give the teacher privileged *computation* (exact pathfinding, exact hole geometry) only over information the ant has sensed or could hold in memory. A god-map teacher should be at most a *reference* for measuring the gap, not the label source. Emit a full **score per option** (cost-to-go), not an argmax. Let the teacher **abstain** when its preferred option's margin is small or when it relies on unseen cells.
3. **Loss.** Use KL from the teacher's softmax over option scores at a tuned temperature to the ant's choice distribution. Mask options the teacher abstains on. Per-decision weight in the ADVISOR spirit: down-weight decision types the probe flags.
4. **Data.** Use DAgger: roll out the *current student*, label every decision with the teacher, aggregate across rounds. Never train only on teacher-driven trajectories when the student has memory (latching/copycat risk).
5. **Memory.** Run backpropagation through time (BPTT) over windows that cover the dependency the memory must carry, e.g. a full outbound-return trip. Re-start windows from stored hidden states taken from the student's own rollouts. Consider auxiliary heads that predict teacher-side latents (nest direction, food-nearby) from the memory units, as RMA/Miki copy latents.
6. **Integration with the GA (Lamarckian).** Each generation, apply k gradient copy steps to the elites (or to offspring), write the weights back, then select on noisy fitness. Evolve or decay the copy weight λ, as in kickstarting. Selection, not the copy loss, is the final judge: a copy that lowers fitness dies.
7. **Self-imitation.** Copy only decisions from trips whose return exceeded the brain's own expected return (SIL's (R − V)₊). With per-world noise this large, require the advantage to clear a noise bar estimated from repeat runs.
8. **Measure the right thing.** Report closed-loop fitness over ~100 worlds, paired against the un-copied parent on the same seeds, beside label agreement. Agreement alone is documented as misleading.

### Gaps
- No source found gives guidance specific to truncated BPTT window length for imitation in recurrent controllers. Item 5 is inference.
- No source found quantifies label quality against quantity for discrete-option controllers. Seita et al. and the capacity-gap result are indirect.
- No retrievable study combines a copyability probe with an imitation-gap measurement as a standard pre-training check. Item 1 is my synthesis of Weihs/Swamy/Warrington.
