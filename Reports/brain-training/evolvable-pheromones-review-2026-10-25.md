# Evolvable pheromones plan: adversarial review (2026-10-25)

*Kept as written. What the owner chose after it, and what was measured since, is in the lane report [`evolvable-pheromones-2026-10-25.md`](evolvable-pheromones-2026-10-25.md). One correction already applied there: the plan's air-law reach was measured with a 240 bite, not an ant's 40 emit (H4 below).*

Reviews the first plan (now summarised in [`evolvable-pheromones-2026-10-25.md`](evolvable-pheromones-2026-10-25.md) §1-§2; the draft and its molecules sheet stayed in the project's shared folder). The owner asked for this review to be adversarial. Everything below is checked against code, measured records, or a run I made today. The runs are listed at the end. Repo state: `main` at `84988ab2`, plus the uncommitted `examples/molecules.rs`.

## Verdict in plain words

- **Physics.** The palette is sound as physics. The engine work is modest and can be made deterministic.
- **Evolution.** Under today's training regime the plan is very unlikely to produce emergent communication. The regime misses most of the conditions the literature says communication needs:
  - selection inside a colony picks individuals on their personal score;
  - founders all start with the same convention;
  - colonies never meet each other;
  - food sits in two fixed, mirrored heaps;
  - a run has about 50 to 150 lineage generations, and the emit weights mutate at a tenth of the normal rate.
- **The likely outcome** is that the molecule genes stay at their defaults while signals matter, and drift where signals do not. That drift would then be read as "signals emerging".
- **The lever is probably the wrong one.** This project's own trail records put the bottleneck in *when ants emit and what readers do*, not in the chemistry. The one informative emitter the current pool has was written by hand, and selection has not changed it since.
- **Step 1 can mislead in both directions.** Its "listening test" blinds only some of the scent inputs, and it also changes what ants emit. Its deciding measure is one a standing rule already bans.
- **What to do instead** is near the end: measure today's signals first (no engine code), then run an experiment built on the conditions the literature says communication needs. That experiment costs a few VM-hours.

Overall feasibility in the current regime: in my judgement, under 10% that a genuinely new convention (a molecule shift with matched reading and new information) appears in a 32 x 16 screen or a 64 x 48 confirm. In a redesigned regime (E2 below) it is plausible, and the palette could then be a useful extra dimension.

---

## Findings, most important first

### C1. Selection inside the colony is individual selection on personal score, which works against signals that help nestmates. Critical.

**Evidence.**
- *How the bed selects.* Births in the bed take their mind from the highest-scoring sires (`examples/inrun.rs:1625-1662`). Culling removes the lowest scorers. Between rounds, the colony carries its top 30 ants by **individual** score (`scratch/inrun/colsel.py`, `carried()`).
- *How strong it is.* In my run (std_pool, 2 colonies x 1 round, `dist=100 food=mix`) each colony-round had about 420-450 hatchings, 310-322 culls and 52-58 retirements. Deaths by cause were 360-376 culled against 18-26 starved. Most of the selection is the cull.
- *What it rewards.* An ant's score is about 87% food and 3% digging (`inrun-selection/README.md`, "Scoring review"). An emitter earns no credit when the ants it recruits deliver food.
- *The colony layer is coarse.* It is epsilon-lexicase over 7-9 cases, once per colony per 200,000 frames, with food as one case among many. "Adding a case thins the pressure on the others" (`colony-selection-log-2026-10-24.md`, farfood3b).

**Why it matters.** A signal that helps nestmates raises everyone's score by the same amount, so it gives the emitter no advantage in a within-colony ranking, and it costs the emitter energy (`mind.rs:3076-3082`). Individual selection is therefore blind to the benefit and sees only the cost. Floreano et al. (2007) found that "communication readily evolves when colonies consist of genetically similar individuals and when selection acts at the colony level". Under individual selection with unrelated robots they got deceptive signalling and worse colony performance ([doi:10.1016/j.cub.2007.01.058](https://doi.org/10.1016/j.cub.2007.01.058)). Mitri et al. (2011) found that reliability tracked relatedness over **500 generations**: unrelated robots made unreliable signals ([doi:10.1098/rspb.2010.1407](https://doi.org/10.1098/rspb.2010.1407)). Here, relatedness inside a colony is moderate to high (22-24 founder families per colony, the largest holding about half the births). But individual-score selection is the dominant force inside the colony, and colony-level selection is a thin layer on top.

**What would change my mind.** A run where colony-level selection is the only selection (`sel=off` or `nocull=1` inside the colony), and a signalling trait still loses or gains the same way it does under `sel=score`.

### C2. Pairing emit and smell in one gene, from identical founders, locks the convention. What then looks like emergence would be drift. Critical.

**Evidence.**
- *The pool has no variation to select on.* All 30 std_pool members carry almost identical emit rows. I decoded slots 2340-2444 (the `EMIT` block, `mind.rs:212-213`) of p00, p05 and p17; only the A row's memory weight differs (1.44 against 1.11). Their molecule genes would also start identical, at 1, 2 and 4.
- *Colonies never meet in training.* Each colony is its own `inrun` process (`colsel.py`, `run()`).
- *Refounding mixes colonies.* Each round, the bottom half is refounded from the top half's carried ants (`colsel.py` docstring, `lexicase()`).

**Why it matters.**
- *A mutant cannot talk.* With a shared convention, an ant whose molecule gene moves can no longer hear its nestmates and can no longer be heard. While signals pay, that is stabilising selection, so the genes stay at their defaults.
- *Genes move only where signals are useless.* There they drift, and that would look like divergence.
- *No reason for private channels.* "Different colonies may settle on different molecules (private channels)" needs colonies that meet and compete. In training they never do, so any difference between colonies is neutral drift, and refounding erases it every round anyway.
- *The literature agrees.* Floreano et al. (2007): "Once a given system of communication was well established, it constrained the evolution of more efficient communication systems" (same DOI as C1).
- *Self-signalling escapes the lock.* The one way a molecule shift *can* pay a lone mutant is when it reads its **own** trail, for example homing on its own A. That is memory, not communication, and the plan's step 5 cannot tell it apart from communication.

**What would change my mind.** Replicate runs (different lexicase seeds) where the molecule genes move away from the defaults *in the same direction*, while a `sel=random` drift control does not move them, and the move comes with a causal listening effect (see M6).

### C3. The food layout carries little information, and the plan's description of it is out of date. Critical for the "signals emerge" goal.

**Evidence.** The plan says training food is "25 columns out in a fixed place". That has been out of date since the farfood batches. Standard batches now ramp distance with `distramp=80..100:5:10`, and `food=mix` puts provisions east and fruit west (`colsel.py`, the DISTRAMP block; the laygene2 job line; `inrun.rs:751-757`). The layout is still a drip of 20 cells every 500 frames, up to 60 a side, into **two heaps at mirrored ±d columns** (`inrun.rs` STANDARD: `drip=20 cap=60 spread=10`). That is a permanent, symmetric resource whose distance changes slowly between rounds.

**Why it matters.** Recruitment signals pay when food is patchy, short-lived and unpredictable. A permanent food source at a known, symmetric place is found as well by route memory or by walking straight out. This repo already found that "a trail cannot lead to food nobody has reached" (`colony-selection-log-2026-10-24.md`, line 95). In the robot studies, food location was the information worth signalling.

**What would change my mind.** A census showing that ants in today's bed spend a large share of their trips searching for food that a nestmate already found in the same round. That is the room a signal could fill.

### C4. Too few generations, and the emit weights are protected from mutation. Critical for the compute budget.

**Evidence.**
- *Measured depth.* In my run, hatchings' lineage generation inside one round was 1 to 11, mean about 3. Carrying the top 30 into the next round gives about 3 lineage generations per round: **about 50 for a 16-round screen, about 150 for a 48-round confirm.** The robot studies above ran 500 generations, with far smaller controllers.
- *`lesson=c1:0.1` slows the emit weights 10x.* Every standard batch since colsel9b carries it (laygene2's job line included). It protects every slot that differs from c1 at 0.1x the mutation rate (`inrun.rs:1090-1097`). The pool's B-emit row is `crop +12.00, b -8.00, bias -2.00`. That is the trail-following lane's hand-written "ladenB + lay less where B is high" rule (`trail-following-2026-10-22.md` §3 and §5, arm `sup`). It differs from c1's crowd-B row (crowd +0.86, laden memory +0.87, crop +0.37, bias +0.29; same report §1), so those slots mutate at 0.1 x `kpm 0.013` per birth.
- *The record shows it.* Those three weights are still exactly 12.00 / -8.00 / -2.00 in std_pool after many batches.
- *Behaviour settles late.* "Learned behaviour settles by round 23–31" (`how-we-test.md` §7). A 16-round screen ends before a slow co-evolution could show anything.

**Why it matters.** A new convention needs an emitter mutation and a matching reader mutation to rise together. Mutation supply is not the problem: about 420 hatchings x 3 genes x 0.013 is about 16 molecule-gene mutations per colony-round. The problem is that selection is too weak and too short to carry the matching pair. And the emitter half is throttled 10x.

**What would change my mind.** A positive control showing that this regime *can* evolve something that needs two coordinated mutations within 16 rounds. Nothing in the batch logs shows that so far: every coordinated behaviour here (carry home, roofing, laden B) came from a teacher or a hand edit.

### H1. The chemical palette is probably the wrong lever. The measured bottleneck is the emitter and the reader. High.

**Evidence.**
- *Who emits decides what following does.* The trail-following record (`trail-following-2026-10-22.md` §1-5):
  - c1 and hA1 laid B as a **crowd scent**, so B peaked at the nest and "follow B" sent ants home to starve.
  - Silencing B gained food: hA1 374 against 271, 28 of 32 seeds.
  - A climb-the-gradient reader piled ants onto a runaway peak that saturated at 65,535.
  - The same reader was harmless when nobody laid B, and helped when gated to empty ants.
  - None of these problems were chemistry.
- *The three channels already cover the range.* Today's channels span two spread laws and lifetimes of about 144 frames (alarm), about 1,200 (B) and about 4,500 (A) (the plan's own table). The brain already chooses when and how much to lay on each, from context and memory (`mind.rs:2067-2078`).
- *The pool already uses all three.* My run laid A 1.34M, B 0.40M and alarm 0.50M emit-units in one colony-round (`r0.log`, "mind acts").

**Why it matters.** Within one training environment, the best molecule for a channel is a single number, so it is a **constant**. Step 1's sweep finds that constant directly. A gene adds value only when different colonies, species, or phases of a run that actually coexist need different settings. The training bed has none of these. In this regime the molecule gene is a slow, noisy way to run step 1's sweep.

**What would change my mind.** Evidence that the best B physics differs between environments one colony meets within a run. The far-food ramp could do this: the best fade at 80 columns might differ from the best at 110, if that is measured.

### H2. The molecule genes must not be body genes: their inheritance would split from the brain that reads them. High (technical).

**Evidence.**
- *Body and mind come from different parents.* Under `brood=lay` (the default), the egg's **body** comes from the ant that laid it, mutated by the game's own rule (`creature.rs:6171-6213`, `mutate_newborn`). The harness then writes the hatchling's **mind** from a separately chosen high-scoring sire (`inrun.rs:1606-1662`).
- *Ant bodies do not carry between rounds on main.* The harness carries only `.raw.txt` minds (`carried()` in `colsel.py`). `.body.txt` is carried for spiders only (`inrun.rs:498`, `src/train/predator.rs:31-41`). The ant body-genes switch (`body=`) lives only on an old branch (`body-genes-2026-10-22.md`).

**Why it matters.** Plan §4 says "per ant, mutate like body genes". As body traits, the molecule gene would follow the egg-layer's line while the weights that interpret it follow the sire's line, and it would reset to the founder body every round. **The fix:** put them in the mind genome, as the lay gate was (`mind.genome.lay`, `mind.rs:1690-1697`), so they mutate and carry with the brain.

### H3. The step-1 landscape check is mis-specified in four ways. High.

1. **`PP_SCENT_BLIND` is not a listening test.**
   - *It blinds only part of the input.* It zeroes the 3 head readings and their 3 change readings (`mind.rs:1806-1813`). It leaves the 9 per-option scent features untouched (`options.rs:944-948`: strength, difference, two-cell difference for each channel), and those are what steer a step.
   - *It also changes emission.* The pool's B-emit row reads the B input at -8. Blinded ants lose that brake and lay B at the full `crop*12-2` everywhere.
   - *It is off-distribution.* Zeroing inputs the network never saw at zero tests robustness, not information use. Lowe et al. (2019, "On the pitfalls of measuring emergent communication") draw this line between signalling and listening.
   - *The question is already answered, but the answer is confounded.* The blind arm has been run: "blinding them halves food home" (`colony-selection-log-2026-10-24.md`, line 94). So the plan's branch "if blind = control" is already decided, and the halving cannot be read as "the pool listens".
2. **The deciding measure breaks a standing rule.** "Judge feeding by tank, stored nest food and starved, never home deliveries alone: re-drops made home8 count about 11k of 13k deliveries as churn" (`colony-selection-log-2026-10-24.md`, standing rules).
3. **Some arms are already measured.** A fade 0 against 0.03 was measured twice on the old ant. Home-return rose 0.88% to 14.84% at fade 0, but intake fell by a median third, unresolved (`pheromone.rs:218-253`, §7.46). Adding a fade to A cost 34% of intake in the lab box (`trail-following-2026-10-22.md` §6). B fade 0.10-0.25 raised played-box intake (54,722 to 127,339 J) but did nothing on the gap bed (`pheromone.rs:260-270`). These are old-ant numbers, so the mind-ant arms are new. Price them against these priors, and check the bucket first (`how-we-test.md` §7).
4. **With selection off, a pool trained on today's B mostly measures how far it is from home.** A pool adapted to news-B will lose under almost any change, so a negative result says little.
   - *Unstated.* The plan gives neither seed count nor whether selection is on.
   - *Missing arms.* There is no per-channel **mute** arm. Muting is the cleaner test, since it does not touch the input distribution; hA1 with B muted was +38%.

### H4. The table and picture overstate how far an air molecule reaches from an ant. High for expectations.

**Evidence.** The air rows in `examples/molecules.rs` deposit `ALARM_DEPOSIT` (240, one bite). A mind ant's emit lays `s x DEPOSIT`, at most 40 (`mind.rs:3067-3068`). I re-implemented `Spread::ActiveSpace` and `decayed` in Python and checked them against the plan's numbers: alarm at 60 frames gives peak 27 and radius 5; call at 180 frames gives peak 49. Then:

| Deposit | alarm (0.35, 12): max radius | call (0.10, 4): max radius |
|---|---|---|
| one bite, 240 (the table's row) | 5 cells, gone by 144 frames | **19** cells, 612 frames |
| one full emit, 40 | **2** cells, 96 frames | **6** cells, 408 frames |
| ant standing still, 3 full emits per pass for 20 passes (assumed rate) | 4 cells | 18 cells |

**Why it matters.**
- *"Call" is local.* Food is 80-110 columns out, so "call" can only be a local recruitment signal. It cannot carry nest-to-food information, which only a ground trail does.
- *Old clouds flatten.* Once the front passes, the difference between neighbouring cells shrinks by `(1-fade)` each pass. At pass 15 of a 240-call the profile reads 49, 48, 47..., about 1 unit per cell, which is about 0.11 after `SCENT_GAIN` 32.
- *Square clouds make ties.* In the chessboard metric, 3 of 8 neighbours lower the distance equally, so the direction is known only to a quadrant.
- *Not fatal.* But the picture's big white square is not what an emitting ant makes.

### M1. "Defaults reproduce today's ant exactly" is false in the lab. Medium.

**Evidence.** The lab box runs B at fade 0.03 / blend **0.05** (`LAB_B_RHO`, `LAB_B_DIFFUSE`, set in `lab/scene.rs:1350-1351`). That pair is not in the palette: news is 0.03 / 0.25 and road is 0 / 0.05. The training bed uses the engine pair, so training is reproduced exactly. The `+lab` case (`labsurvive`, through the lab's own code) and the owner's lab box would change as soon as the palette is on. That change is a measured loss for the old ant: goal-box food 136,761 at the narrow pair against 104,340 at the engine pair (`pheromone.rs:1016-1021`).

**Fix:** add (0.03, 0.05) as a palette member, or report the frozen-genes arm with the palette on as a different control from today's lab.

### M2. A "frozen" arm is not today's control. Medium.

Adding three slots to the mind genome changes which slots `neutral_mutate` draws (`inrun.rs:137-160`). That is the same caveat the lay gate carries ("only the bed's mutation draws differ", `mind.rs:1644-1646`). The frozen arm must also keep those slots out of mutation. A fresh shared control is needed; a bucket control cannot be reused under §7.

### M3. Reading between molecules is penalised, so the landscape has valleys between whole molecules. Medium.

The option and change features are **absolute** differences times a gain (`options.rs:946-947`, `mind.rs:1830-1832`). An ant at gene 1.5 reads half of the nestmates' A trail, so every trained feature it sees is halved. Intermediate positions are therefore worse than either neighbour, and a gene tends to stick at whole numbers. That works against the claim that a small mutation is a small change. Two related points:
- The 3→4 interval mixes a ground molecule with an air one.
- Molecule 4 is the alarm plane, which bites write to directly (`creature.rs:11213`). A gene that leaves 4 stops hearing bites, and a gene that arrives there inherits them.

### M4. Cost is underpriced. Medium.

- *Memory.* At `u16`, double-buffered, a plane is about 84 MB at 8192x2560, so six planes are about 500 MB. Separately, the code comment at `pheromone.rs:540-547` still says 40 MB, from the u8 era.
- *Lazy allocation will not save it.* The first mutant with a fractional gene allocates two planes, so the free arm will have all six from round 0.
- *Tiles stay awake.* Road (fade 0, blend 0.05) outlives 6,000 frames, so busy routes never let their tiles sleep.
- *The pass is already expensive.* In the trainer's arenas the pheromone pass was the largest cost: libm `roundf` 14% plus the pass 45% of samples, before the 2026-10-21 optimisation (`pheromone.rs:500-505`). Nobody has measured it on the colony bed.
- *What to do.* Time `inrun` with six live planes before committing VM hours to step 4.

### M5. Steps 4 and 6 can ship an unproven mechanism. Medium.

32 x 16 "keeps 99% of large-effect calls and 85% of medium ones" (`how-we-test.md` §7). A molecule effect is likely small. Step 6 switches it on if "neutral or better", and the owner's default-on rule makes "neutral" mean "ship". So three planes could ship on a non-result. **Fix:** count the frame and memory cost as a measured harm, and name the deciding measure (tank, stored food, starved) before launch.

### M6. The signalling checks need standard metrics and a drift null. Medium.

Step 5 is the right idea. To make it a test:
- **Positive signalling:** the mutual information between an emitter's state (laden, hungry, at food, at nest) and its emission. `PP_MIND_LOG` already logs context per decision (`mind.rs:4491`, `6067-6085`).
- **Positive listening:** the causal effect of the signal on what readers do. Measure it by swapping in a counterfactual plane (time-shifted or mirrored), not by zeroing (Lowe et al. 2019).
- **Fitness:** free against muted.
- **Drift null:** the spread of the molecule genes under `sel=random`.

Without the null, "colonies converged on different molecules" cannot be read.

### L1. Points that are fine or minor. Low.

- **Determinism holds:** the plane arithmetic is integer, and the f32 split is deterministic within a build.
- **The air front's speed limit does not bind.** One cell per 12 frames is about 0.08 cells per frame, faster than a forager's net progress (`pheromone.rs:226-231` for the old ant).
- **Dropping flash is right.**
- **Off can be bit-identical** if it is gated on a switch, with integer genes that never hit the split path. The existing world-hash guard covers it.
- **Governance:** the module's standing rule is to resist a new plane "until a concrete consumer exists" (`pheromone.rs:35-36`, `stigmergy-research.md` §8). Road and call have no consumer yet.

---

## Cheaper and better experiments for the owner's two goals

**E0. What do today's three signals mean, and do they pay? No engine code; about one VM-hour.**
- *Arms.* On std_pool, with selection off, on 32 paired seeds:
  - mute each channel (zero its emit row, as `g-hA1-noB` did);
  - scramble each channel's reading. A spatially mirrored or time-lagged plane read needs a small scratch switch. Zeroing is not enough, see H3.
- *Measures.* Tank, stored food and starved, plus the lab case.
- *Signalling census.* Use `PP_MIND_LOG` to measure what each emission depends on (laden, at food, depth, memory).
- *What it delivers.* The answer to "is there communication now?", and the owner's picture: an overlay of who lays what, and when.
- *Prior.* Muting B on hA1 gained 38%. If muting a channel helps or does nothing, that channel carries no useful information, and no chemistry will fix it.

**E1. Physics as a constant.** Run a B fade/blend sweep **under selection**, 32 x 16, with one shared control and a pre-named measure. If one setting wins clearly, ship it as a world constant. That serves the foraging goal directly and needs no gene.

**E2. An emergence experiment built on the conditions the literature names.** Small harness change; a few VM-hours.
- **High relatedness:** each colony founded from one genome.
- **Colony-level selection only:** `sel=off` or `nocull=1` inside the colony, plus colsel across colonies on tank and stored food.
- **Food worth signalling about:** short-lived, depleting patches at random positions that move within a round, not mirrored heaps.
- **Emit weights free to mutate:** drop the `lesson` protection on the emit rows.
- **Controls:** a mute arm (emits banned) and a drift arm (random colony selection).
- **Length:** 150-300 colony generations. At the **50 s per 200k-frame colony-round** measured here, 32 colonies x 200 rounds is about 6,400 colony-rounds, about 90 core-hours, about 1.5 hours of the 64-core VM before gardens.
- **Emergence, defined:** the free-against-mute gap growing over rounds, plus positive signalling and positive listening.
- **The palette is optional.** It goes in as genes in the mind genome only once E2 shows signals evolving with fixed chemistry. Then it is a clean extra arm: does free chemistry speed or improve a convention that is already evolving?

---

## What would make me change the overall verdict

- E0 shows a channel with clear positive signalling and listening, **and** a physics sweep of that channel gives a large paired gain under selection: say at least 24 of 32 colonies and +20% in tank or stored food. A gene could then find that gain, though a constant would find it faster.
- The best physics is measured to differ between conditions a colony meets within a run (near against far food, fruit against provisions).
- In a pilot with colony-only selection, the molecule genes move consistently across independent replicates while the `sel=random` null stays put, and listening is causal.

## What I ran (all in my scratchpad, nothing in the repo changed)

- `cargo build --release --example inrun`, then `scratch/inrun/colsel.py <out> lex+spoil+crater 2 1 200000 assets/minds/std_pool.txt dist=100 food=mix`, with `CG_EVERY=0` and `P=2`.
  - 50.5 s per colony-round.
  - births.csv: 421 and 453 hatchings; 310 and 322 culls; lineage generation 1-11, mean about 3.
  - Deaths: 360 and 376 culled against 18 and 26 starved.
- `target/release/examples/molecules`: reproduces the plan's table exactly.
- A Python re-implementation of `Spread::ActiveSpace` and `decayed`, checked against that table, for the emit-sized air rows in H4.
- I decoded std_pool's emit rows (slots 2340-2444) from the expanded pool files.

Literature checked on PubMed: Floreano, Mitri, Magnenat & Keller 2007, *Curr Biol* 17:514 ([doi](https://doi.org/10.1016/j.cub.2007.01.058)); Mitri, Floreano & Keller 2009, *PNAS* 106:15786, on concealing information under competition ([doi](https://doi.org/10.1073/pnas.0903152106)); Mitri, Floreano & Keller 2011, *Proc R Soc B* 278:378 ([doi](https://doi.org/10.1098/rspb.2010.1407)); Wischmann, Floreano & Keller 2012, *PNAS* 109:864, on how historical contingency fixes the strategy that emerges ([doi](https://doi.org/10.1073/pnas.1104267109)). Cited from memory, not re-checked: Lowe et al. 2019, *On the pitfalls of measuring emergent communication* (AAMAS), and Skyrms 2010, *Signals* (on Lewis games, where partial-pooling equilibria are common).
