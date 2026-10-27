# Brood and breeding (thread, 2026-10-23 to 10-24)

Started at the owner's ask "I don't love that the nest is 100% full of food". Parent: Teacher lane.

## What landed on main
- Breeding in the training bed (`examples/inrun.rs` `brood=lay`, genome chosen by selection at hatch), PR #839; ON by
  default with `lay_cap=48:8`, PR #845 (owner, 2026-10-24 00:47Z "Breeding should be on by default").
- Dead-ends entry: rotting crumbs starve the colony (the packed nest is the working larder), PR #850.
- Report `Reports/brood-in-training-2026-10-24.md` (PRs #861, #874): everything measured, including the food-distance
  test and the laygene2 result.
- Learned laying: `PP_LAYGENE` lay gate in the mind genome (ON by default, neutral at zeros), inrun `lay_cap_p`,
  colsel `capwean=R0:R1`, PR #870. Result (Teacher lane laygene2-1024): selection found no brake; `lay_cap` stays in
  training.
- Hand-off PR #952 (MERGED 2026-10-27 as 06a4ea1): the plan doc as
  `Reports/brood-plan-2026-10-23.md`, data and pictures in `Reports/brood-in-training-2026-10-24/`, pilot scripts and
  the four colsel20 carries in `scratch/inrun/brood/`.

## Owner decisions still pending
1. **How to stop the nest packing with crumbs** (asked 2026-10-24 ~06:00Z, never answered). Supply levers all failed
   (looser caps, rot, limited income, farther food). Options: (a) a colony selection case that ranks down colonies
   whose ants starve beside stored food (teaches eating the buried pile); (b) stop ants putting part-eaten loads back
   at home. Recommendation: (a) first. It is a scoring change with no physics change, and the trace shows the mind simply never
   learned to eat at home (Walk 200 of 214 choices).
2. From the plan doc, never answered: (i) the shipped nest-worker caste (`caste=4`, 1 ant in 4 by id) is a hand-made
   caste against the no-castes rule. Should it come off in training or everywhere? Recommendation: off in training first,
   measured. (ii) Is a few dozen starved adults per round acceptable as regulation? (iii) Is a training headcount cap
   near 50 acceptable for speed if kept out of the game? (In effect `lay_cap=48:8` already does this.)

## Open to-dos (none started)
- If laying regulation is retried: a bed where overshoot kills (far food, like the owner's lab box), or a colony case
  paying for low starvation per birth. The uncapped bed settles at 160-200 ants with churn and never presents the lab
  box's boom-and-crash, so nothing selects restraint.
- Food/brood separation (owner wanted separate rooms): tool `scratch/inrun/brood/separation.py` exists; it is moot
  until stored food is limited (73-100% of open nest cells touch food today).

## Files to keep
All moved into the repo by PR #952. Not kept: the pilot's raw runs in this session's scratchpad (numbers are in the
plan report).

## Jobs running
None.

## Branches
None unmerged: `claude/project-thread-6einf2` is fully merged (last, PR #952).
