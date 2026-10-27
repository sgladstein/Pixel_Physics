# Lane: Let ants evolve their own pheromones (evolvable scents)

Branch `claude/project-thread-iff53x`. State as of 2026-10-27 18:40Z.

## Where everything is (repo)
- Write-up: `Reports/brain-training/evolvable-pheromones-2026-10-25.md`. §9.8 holds the review corrections, E1/E2, the E3/E4/E5 plan with fixed bars, and the state at the move. It is on main via #943; the plan and state come with #944.
- First adversarial review: `Reports/brain-training/evolvable-pheromones-review-2026-10-25.md`.
- Dead ends: `Reports/dead-ends.md` creatures:065 (B fade 0.25 lost under selection), :066 (first emergence test), :067 (ground-law third plane, new-channel run).
- Test rules added to `Reports/how-we-test.md`:
  - a deaf death may be a missing level;
  - an emit row evolved from zero tests the operator;
  - a listening gap is not payoff.
- Harness: `scratch/pheromones/emerge.py`. `HARNESS=2` turns on the review's fixes; it is off by default.
- Data, genomes and scripts: `Reports/data/evolvable-pheromones-2026-10-27/` (PR #944, handed to the merge desk at 69ad422c).
- Switches on main, all off by default:
  - `PIXEL_PHYSICS_ALARM_GROUND=RHO:DIFFUSE`
  - `PP_SCENT_DEAF`, `PP_SCENT_SHIFT`, `PP_SCENT_CONST`
  - `PP_EMIT_FREE`
  - `PP_MIND_ALARM=train`

## Not in the repo (will be lost)
- The Claude Doc "Evolvable ant scents: results so far" (https://claude.ai/artifact/Vt8E28wJ3A5vjSfSuYPXHX). Its content is in report §9.5-9.8.
- The Ant Scent Lab page. Its HTML is in the data folder as `ant-scent-lab.html`.

## Update 2026-10-27 ~19:10Z
- The E4 readiness review finished: **not ready**. It is in the repo at `Reports/brain-training/evolvable-pheromones-e4-readiness-2026-10-27.md`, with report §9.9.
- Key finding, re-checked: "food stored" counted crumbs. In energy, new-channel replicate 2 is a tie and replicate 1 is a 1.8x loss.
- PF2: on today's founder the B trail feeds ants but does not raise stored food.
- emerge.py can now score on `SCORE=kj|home`.
- E3 was stopped because it ranked on cells; it is not rerun.
- **Owner decision pending:** what "forage better" means. Options are food delivered home (my recommendation, with starved and alive as guards), colony fed and alive, or larder kJ.
- Everything is in PR #944 (head 6190d3d0) at the merge desk.
- The Teacher lane was told that colsel's food case counts crumbs too.

## Update 2026-10-27 ~19:00Z: the owner restated the goal
- The goal is whether ants can evolve signals that communicate information between them. "Forage better" is not the goal.
- Food delivered home is the guard measure, and also the selection score (`SCORE=home`).
- Four criteria are in report §9.10: information by sender state, receiver response, real-map meaning, and social use.
- Run 1 re-scored on food delivered home: the signalling colony beats the deaf one 1.8x on 15/16 worlds and meets criteria 1-3.
- E4 is redefined as "how often such a convention forms". It still needs the readiness review's fixes and the owner's go-ahead before it runs.
- PR #944 head is 3c2c24f2.

## Open to-dos (original list; E3 and the review are superseded by the update above)
1. **E3** (running now in this session's scratchpad `e3/`, about 3.5 h, started 17:40Z): harness 1 vs 2 on the deaf arm, seeds 31/32, 40 generations. Its output is not in the repo yet. I will land its gens.csv files and the verdict if the session is still alive; otherwise rerun it with `scripts/e3_run.sh`.
2. **E4 readiness review** (independent sub-agent, started 18:20Z at the owner's ask). Writing to scratchpad `review2/review.md`; I will land it with E3.
3. **E4** itself, the decisive free vs deaf test (6 replicates × 16 colonies × 60 generations, about 240 core-hours). **Not approved.** The owner asked (18:16Z) whether the set-up is right. Known gaps:
   - E2 has no positive control.
   - The gentle operator has never run a full evolution.
   - The bed has not been checked against the training-environment review.
   - The founder is std_pool (layout 5), not the ratchet pool or the gated layouts 6-8.
4. E5 (optional): evolvable vs frozen A/B emit rows.

## Owner decisions pending
- Whether to run E4, and where. Recommendation: wait for the readiness review, fix what it finds, then run it on the VM. A card was posted and then withdrawn when the owner asked for confidence first.

## Rules learned (also in how-we-test / the report)
- Judge an emergence run by each arm's best genome on shared seeds, and by arm-level replicates. One genome per arm is n = 1.
- The deaf control still evolves the A and B signals, so it is not a no-signal control.
- Keep the alarm emit row frozen on the main ant line. An emit row opened to mutation must start dense (N(0, 0.05)) and run with a deaf arm.

## Running jobs
- E3 is running on this session's container, 4 cores, output in the scratchpad. Nothing is running on the VM.

## Branches with unmerged work
- `claude/project-thread-iff53x`: PR #944 at the merge desk.
