# Handoff: explore teaching the ant brain by copying (for a new session)

*Written 2026-10-20 by the session running the brain lane, at the owner's request: the owner wants a deeper discussion
of the teacher approach in its own session, without pulling the brain lane off its work. Read this first, then
`Reports/brain-training/investigation-2026-10-20.md` (parts 1-3 and 5), then the repo's `CLAUDE.md`.*

## Your job

**Explore, with the owner, whether and how to teach the trained ant brain by copying -- and design the test of it.** This
is a discussion and design lane first. The owner's question, in his words: "How do we create a teacher that has the
exact behavior that we would want it to?", followed by a caution worth taking seriously:

> "The entire first part of this project was spending weeks manually encoding behaviors and we recently switched to
> training them because manually encoding them was failing over and over. We did have some limitation on the manually
> encoded behaviors like preferring local signals and true biology over hardcoding a behavior that we wouldn't
> necessarily have to abide by for the teacher, but I worry that we are glossing over how difficult that will be."

Do not gloss over it. The brain lane already conceded two points to it: hand-wiring a teacher straight into the brain's
weights is just the approach that failed (withdrawn), and "rules are quick to change" undersold the long tail (the ant
stuck on a ledge, a pellet falling back, two rules fighting) that broke hand-encoding before.

## What the owner has decided (2026-10-20)

- **Test both label sources, side by side** (his words: "I am not sure which is better and I think we should probably
  test both"):
  - **(A) Self-imitation:** label the brain's own decisions by outcome -- keep those from trips that became full loads,
    drop those that started unpaid trips or walked past the drop point -- and train the brain to copy its better self
    (Oh et al., ICML 2018, "Self-Imitation Learning"). No rules to write; the scorer supplies the labels. It can only
    reinforce behaviour the brain sometimes shows (s161 does cut the foot 8 times a world; it also cuts the wall 3).
  - **(B) A hand-written teacher:** rules that pick from the same option list the brain gets, allowed to use privileged
    information (exact hole cells, the foot row) and free of the old biology and local-signal limits. **Time-boxed to one
    day of building**, and how far that day gets is itself part of what is measured.
- **Reinforcement learning comes after copying, not before**: copying builds the gradient-through-the-brain code that RL
  would need; RL then gets a head-to-head test against selection on one step, with a design doc first.
- **Big redesigns need a design doc and an adversarial review** before building (standing rule).
- **Earlier rejections may be revisited on the evidence** (RL, NEAT, big-network ES were rejected by reasoning in the
  design of record, never by test).

## The proposed test (the brain lane's draft -- yours to improve with the owner)

1. Same copying code for both sources: for each labelled decision the brain gives every option a probability (its
   weighted pick, `mind::decide`); training raises the probability of the labelled choice by gradient descent. Train
   offline in Python first (the brain's decision math is already reproduced exactly in Python; see below), write out a
   normal genome file.
2. Offline judge, no VMs: the copied brain against s161 gen 336 on the 24 sealed worlds at 4 and 8 rows, paired per world:
   full loads, unpaid trips, put-down distance; and fetch and trail against the fixed s91 floor (`keepref=`), so copying
   does not erase old skills (copy from a mix of digging, fetch and trail decisions).
3. Survivors get a short 8-row training run beside plain s161 (the brain lane's runs `dig-8-s171`/`s172`, due about 00:35
   UTC 2026-10-21, are that control arm). GIFs for the owner at each stage; nothing "passes" before he has seen one.
4. Known catches to design for: drift into states the teacher never visited (DAgger: let the student drive, the teacher
   labels, retrain); the brain's 12 recurrent memory units (first version: train the parts that read the current
   situation, leave memory; full training needs backprop through time, which RL would need too); decisions the brain
   cannot distinguish (check copyability first: wherever two options look identical to the brain's inputs but the
   teacher picks one, it cannot be learned -- that shows which sense is missing).

**Open questions worth the discussion:** what "the exact behaviour we want" is, concretely, for the 8-row step and for the
harder ones after it (rooms, 12 ants, brood) -- the brain lane's view is that a hand teacher may help the simple early
steps and does not scale, which is the opposite of where training gets expensive; whether self-imitation can create a
skill or only sharpen one; and whether copying a hand-written strategy crosses the owner's "one general brain, found by
selection" ruling (selection still tunes everything afterwards and can drop what does not pay -- but ask).

## What is already known that a teacher needs

- **What "right" looks like at 4 rows** (investigation part 1, s161 gen 336, 24 sealed worlds, every pellet traced):
  carrying is solved (90% of foot cuts become full loads); 37% of trips carry soil cut where nothing pays (wall cuts
  3.2 a world, mostly one row short from inside the shaft; top row 1.7; re-cuts 0.9); half of all decisions are an empty
  ant walking the surface; loads go a median 10 columns where the rule needs about 4-5; the empty ant overshoots.
- **What the brain reads** (part 2): it digs when it feels "at the nest" (`at_nest`, on along the whole dug hole --
  `NestHome::Dug`), drops when a hidden unit sees "far enough from home" (`home_dist` about 0.2), and its target scorer
  has **no live link to a cell's `DEPTH` feature** (the feature exists in the option list, so it is learnable). On an
  8-row shaft it cuts the wall 5.6 times a world against 4.3 at the foot.
- **Why fetch wears down** (part 3): pruning plus the per-connection energy cost deleted the "don't eat the cargo" link.
  The brain lane is now building protection for links the learning arena cannot test (owner-approved).
- **Copying literature** (part 5's bibliography): Nair et al. ICRA 2018 (demonstrations, ~10x on long sparse tasks, the
  learner beat the demonstrator); Taylor et al. 2011; Qiu et al. 2022 (even non-expert demonstrations help); D'Silva,
  Stanley et al. 2005 (milestone pool against forgetting).

## Tools you can use (branch `scratch/brain-instruments`, not for `main`)

`scratch-analysis/README.md` there describes them. In short: `PP_MIND_LOG=<file>` logs every mind decision with all 22
senses, 12 memories, the chosen option and its 45 features, and the soil arena's cuts and put-downs; `pellets.py`
rebuilds every pellet's fate from that log (asserting it matches the scorer); `brainmath.py` recomputes the brain's job
scores exactly from a genome file (median error 0.000025 over 105,854 scores) -- the starting point for a Python copier;
`PP_ABLATE` switches senses off. **What the log lacks for copying:** it records only the chosen option's features, not
every option's -- a copier needs the whole option list per decision, so extend the logger first.

## Coordination with the brain lane

- The brain lane (this session's author) owns: the 8-row runs on Google (`dig-8-s171`, `s172`; they fill the 64-vCPU quota
  until about 00:35 UTC), link protection in `src/train/`, and `Reports/handoff/brain-lane-handoff.md`. **Do not edit
  that handoff or `src/train/mod.rs`** while link protection is in flight; keep your notes in this file or your own.
- **No VM launches without the owner's go**; the Google budget is $100 in total, and the quota is full until the runs end.
- Repo rules that bite here: work in your own worktree; never `git add -A`; scratch code stays on `scratch/*` branches.
- Results the brain lane will want: whether either copied brain beats s161 offline, and the design for the test.
