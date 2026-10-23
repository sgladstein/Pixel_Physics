# Parallel training of the ant: which kind, for which task

*A living guide, edited in place (not appended), started 2026-10-23 at Scotttt's request by the "Parallel training
with weight merging" thread. It turns the measurements in
[`research/parallel-training-and-merging-2026-10-23.md`](research/parallel-training-and-merging-2026-10-23.md) into
a rule for choosing how to split training across lanes, islands or machines when a new task arrives. When a new task
is measured under one of these methods, update the method's evidence line and the task table here.*

**Standing owner rules this guide works inside:** one ant design for every ant, no hand-made castes; the colony's mix
of genomes is the product (pool, not one brain); one ant line, owned by the Teacher lane, which other lanes feed with
lessons, worlds and score rules.

## 1. The five ways to train in parallel

| | Method | What runs in parallel | How results come together | Status here |
|---|---|---|---|---|
| **A** | **Full-job islands** | several colonies or batches, same worlds, same full score | swap whole ants (refound weak colonies from strong ones) | **Works.** This is colony selection today (64 colonies, refounding). Islands drift into different kinds of colony on their own (a digger and a forager in colsel20) |
| **B** | **World-variant islands** | islands in *different worlds* (far food, plants, predators, brood) under the same full score | swap whole ants; judge migrants in the world they arrive in | Partly tested: `dists=` and moving food already vary the world per colony. Untested as separate islands |
| **C** | **Lesson lanes** | lanes that each teach one narrow lesson or new sense, **changing only that lesson's own weights** | slot union onto the current founder, then a few rounds of joint selection | **Works when the weights are disjoint.** hcs and r4 merged kept both: food 309 -> 431, 22 of 24 worlds |
| **D** | **Specialist islands, weights merged** | islands each scored on one job only | average, crossover or add the weight changes | **Does not work.** Dig-only island died out (6 of 8); merges worse than either parent; task arithmetic collapsed 12 of 12 |
| **E** | **Co-evolving opponents** | two (or more) populations scored against each other | each keeps its own pool; each trains against a stored set of the other's past and present best | Untested here. Needed for battles and evolving predators |

**The one rule behind the table:** merge *ants* freely, merge *weights* only when each parallel run was confined to its
own weights. Selection moves almost every live weight a little (mutation steps each live link at 0.03 per child), so two
selected lines are never disjoint; their merged weights land between them at best.

## 2. Five questions for a new task

Answer these in order; the first "yes" that names a method usually decides it.

1. **Does the task need an opponent that also learns** (another colony, an evolving predator)? -> **E**, with the
   colony still eating and digging in the same world.
2. **Does the task need an input or an action the ant does not have yet?** (Check first, the way the plant lane found
   the mind could not pick up living plants, and the far-home lane found the home sense was too blurry to copy onto.)
   -> build the sense or action, then teach its first use as a **C** lesson that trains only the new weights.
3. **Can an ant live through a training round doing only this task?** If not (every ant must eat), the task cannot have
   an island of its own. -> it goes into the shared job, through **A** or **B**.
4. **Is the task a different world rather than a different job?** (plants instead of a pile, predators present, food far
   away, brood to feed.) -> **B**: an island in that world, same full score, ants swapped with the others.
5. **Is success a colony result** (nest shape, headcount, a won battle) rather than an ant's act? -> judge and select it
   at colony level (colony selection's gate or lexicase case), not by an ant's score.

If none applies, the task is part of the main job: add it to **A** as a lexicase case or a gate.

**Never D.** A one-job island loses every skill it is not paid for (unpaid skills erode: dig-loss, 2026-10-22), and the
merge cannot put them back.

## 3. Before adopting a method for a task: the check

Every method above has a cheap check that says whether it is working. Run it before the long batch.

- **C (lesson merge):** count the slots each lesson changed (`build_merges.py lesson_union` refuses overlaps). Then a
  common garden (sel=off, 24+ worlds, paired by seed) of base, each lesson alone, and the merge. **Pass:** the merge is
  at least as good as each lesson on that lesson's own measure and no worse than the base on the core measures (food
  home, starved, nest, crater). Overlap or a fail -> one joint copy of both lessons, or let selection decide.
- **A / B (islands swapping ants):** after a few rounds, a common garden of each island's pool in *every* island's
  world. **Pass:** migrants are not worse than the residents in the receiving world, and no island's core measures
  (food, starved, nest) fall against the founder. An island that loses a core skill is being paid for too little.
- **E (co-evolution):** every few rounds, play the current population against a fixed panel of earlier opponents and
  the founder. **Pass:** wins against the fixed panel rise. If the population only beats the latest opponent and
  loses to older ones, it is cycling, not improving.
- **Any method:** the core skills are always measured, whatever the task. Digging and foraging must hold all game
  (owner, 2026-10-22), so a new task that wins by giving them up has failed the check.

## 4. The current tasks

| Task | Method | Why | What to watch |
|---|---|---|---|
| Digging the nest | **A** (+ **C** for narrow rules) | core job; shares the dig-or-forage decision with foraging, so it must be learned with it. Narrow rules (crater refusal, roof sense) can be C lessons | digging erodes when unpaid: measure nest and crater in the common garden every batch |
| Foraging | **A** (+ **C**) | core job, same decision as digging. The home sense was a successful C lesson | starvation, not food delivered, is the honest feeding measure (larder churn inflates deliveries) |
| Breeding (brood) | **B**, colony-level selection | changes the world (headcount, nurses, larder); success is a colony result | the boom and bust the brood lane saw; judge with laying on and off |
| Plant-only world | **C** first, then **B** | the mind could not eat living plants until `PP_MIND_GRAZE` gave it the action; grazing is a new use to teach. Then an island in the planted world, swapping ants with the pile islands | ants must still dig there; weaning (shrinking pile) is the bridge between the worlds |
| Predators (soon) | **C** for the threat sense, then **B**; **E** only if predators evolve | a fixed predator is a world variant; an evolving one is an opponent | flight must not eat foraging: a colony that hides all day starves |
| Colony battles | **E** | the opponent learns, so the score moves with it | keep a panel of past opponents (otherwise cycling); battles still in a world where colonies must eat and dig |

## 5. Not yet known

- How often islands should swap ants (method A/B's migration interval).
- Whether a world-variant island (B) trained apart for many rounds keeps its migrants useful in the main world.
- Whether a C lesson stays whole after joint selection (the crater lesson was too weak on the test build to tell).
- How large a panel of past opponents E needs here. The general literature's answer (keep a "hall of fame" of past
  champions) is from memory and has not been tested in this harness.
